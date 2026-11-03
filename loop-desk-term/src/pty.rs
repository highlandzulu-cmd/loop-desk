//! PTY management: one [`TermTab`] per terminal, each with its own reader
//! thread feeding a shared `vt100::Parser`.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use vt100::Parser;

/// Shared parser type used by both the UI thread and the reader thread.
pub type SharedParser = Arc<Mutex<Parser>>;

const INITIAL_ROWS: u16 = 24;
const INITIAL_COLS: u16 = 80;
/// How many lines of scrollback to keep per tab.
pub const SCROLLBACK_LINES: usize = 10_000;

/// The user's login shell, or `/bin/zsh` if `$SHELL` is unset.
pub fn login_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_owned())
}

fn pty_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// A single terminal tab: a pty, its child process and the shared parser.
pub struct TermTab {
    parser: SharedParser,
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    title: String,
    cwd_hint: PathBuf,
    exited: bool,
    /// Rows/cols last applied to the pty and parser.
    size: (u16, u16),
    /// Current scrollback offset (0 = live screen).
    scroll_offset: usize,
    /// Mouse selection as (row, col) start/end, if any.
    selection: Option<((u16, u16), (u16, u16))>,
}

impl TermTab {
    /// Spawns the login shell (with `-l`) in `cwd` on a new pty.
    pub fn spawn(cwd: PathBuf, ctx: egui::Context) -> Result<Self> {
        let parser: SharedParser =
            Arc::new(Mutex::new(Parser::new(INITIAL_ROWS, INITIAL_COLS, SCROLLBACK_LINES)));

        let pair = native_pty_system()
            .openpty(pty_size(INITIAL_ROWS, INITIAL_COLS))
            .map_err(|e| anyhow!("failed to open pty: {e}"))?;

        let mut cmd = CommandBuilder::new(login_shell());
        cmd.arg("-l");
        cmd.env("TERM", "xterm-256color");
        if let Some(dir) = cwd.to_str() {
            cmd.env("PWD", dir);
        }
        if cwd.is_dir() {
            cmd.cwd(&cwd);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| anyhow!("failed to spawn shell: {e}"))?;
        drop(pair.slave); // closing our end delivers EOF when the child exits

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| anyhow!("failed to clone pty reader: {e}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| anyhow!("failed to take pty writer: {e}"))?;

        spawn_reader_thread(reader, Arc::clone(&parser), ctx);

        let title = cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "zsh".to_owned());

        Ok(Self {
            parser,
            master: pair.master,
            writer,
            child,
            title,
            cwd_hint: cwd,
            exited: false,
            size: (INITIAL_ROWS, INITIAL_COLS),
            scroll_offset: 0,
            selection: None,
        })
    }

    /// The shared parser (also written to by the reader thread).
    pub fn parser(&self) -> &SharedParser {
        &self.parser
    }

    /// Tab title shown in the tab strip.
    ///
    /// Prefers the last path component of a title the shell set via OSC
    /// (zsh's `precmd` typically emits `user@host: dir`), falling back to the
    /// directory the tab was started in.
    pub fn title(&self) -> String {
        let osc = self.parser.lock().screen().title().to_owned();
        let tail = osc
            .rsplit([':', '—'])
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned();
        if !tail.is_empty() {
            return tail;
        }
        self.title.clone()
    }

    /// Directory the shell was started in (used when restarting the tab).
    pub fn cwd_hint(&self) -> &PathBuf {
        &self.cwd_hint
    }

    /// Whether the child process has exited.
    pub fn exited(&mut self) -> bool {
        if self.exited {
            return true;
        }
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            self.exited = true;
        }
        self.exited
    }

    /// Writes bytes to the shell. Errors are swallowed: a dead pty must never
    /// take the UI down.
    pub fn write(&mut self, bytes: &[u8]) {
        if self.exited {
            return;
        }
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    /// Resizes the pty and the parser; only acts when the size changed.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        if rows == 0 || cols == 0 || (rows, cols) == self.size {
            return;
        }
        self.size = (rows, cols);
        let _ = self.master.resize(pty_size(rows, cols));
        self.parser.lock().set_size(rows, cols);
    }

    /// Restarts the shell in this tab after it exited.
    pub fn restart(&mut self, ctx: &egui::Context) -> Result<()> {
        *self = Self::spawn(self.cwd_hint.clone(), ctx.clone())?;
        Ok(())
    }

    /// Kills the child process (called when a tab is closed).
    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }

    /// Current scrollback offset (0 means the live screen is shown).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Sets the scrollback offset, clamped to what is available.
    pub fn set_scroll_offset(&mut self, rows: usize) {
        let max = self.parser.lock().screen().scrollback();
        self.scroll_offset = rows.min(max);
        self.parser.lock().set_scrollback(self.scroll_offset);
    }

    /// Snaps back to the live screen.
    pub fn snap_to_bottom(&mut self) {
        if self.scroll_offset != 0 {
            self.set_scroll_offset(0);
        }
    }

    /// Current mouse selection, if any, as normalized cell coordinates.
    pub fn selection(&self) -> Option<((u16, u16), (u16, u16))> {
        self.selection
    }

    /// Sets or clears the mouse selection (start <= end after normalizing).
    pub fn set_selection(&mut self, sel: Option<((u16, u16), (u16, u16))>) {
        self.selection = sel.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });
    }

    /// Text of the current selection (empty when there is none).
    pub fn selected_text(&self) -> Option<String> {
        let ((r1, c1), (r2, c2)) = self.selection?;
        if r1 == r2 && c1 == c2 {
            return None;
        }
        let guard = self.parser.lock();
        let screen = guard.screen();
        let (_, cols) = screen.size();
        let mut out = String::new();
        for row in r1..=r2 {
            let start = if row == r1 { c1 } else { 0 };
            let end = if row == r2 { c2 + 1 } else { cols };
            for col in start..end.min(cols) {
                if let Some(cell) = screen.cell(row, col) {
                    out.push_str(&cell.contents());
                }
            }
            if row != r2 {
                out.push('\n');
            }
        }
        Some(out)
    }
}

impl Drop for TermTab {
    fn drop(&mut self) {
        self.kill();
    }
}

fn spawn_reader_thread(
    mut reader: Box<dyn Read + Send>,
    parser: SharedParser,
    ctx: egui::Context,
) {
    let _ = std::thread::Builder::new()
        .name("term-reader".into())
        .spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        parser.lock().process(&buf[..n]);
                        ctx.request_repaint();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            ctx.request_repaint();
        });
}
