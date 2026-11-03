//! PTY integration tests (no window needed): spawn real processes through
//! portable-pty and check the vt100 parser state.

use std::io::Read;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use vt100::Parser;

const SCROLLBACK_LINES: usize = 10_000;

fn pty_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

type SharedParser = Arc<Mutex<Parser>>;

/// Drains `reader` into `parser` on a background thread; the returned handle
/// joins once the pty hits EOF.
fn drain(
    mut reader: Box<dyn Read + Send>,
    parser: SharedParser,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => parser.lock().process(&buf[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    })
}

fn wait_until(deadline_s: u64, mut f: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(deadline_s);
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn pty_output_is_parsed_with_colors() {
    let parser: SharedParser = Arc::new(Mutex::new(Parser::new(24, 80, SCROLLBACK_LINES)));
    let pair = native_pty_system().openpty(pty_size(24, 80)).expect("openpty");

    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.arg("-c");
    cmd.arg("printf \"hello\\033[31m red\\033[0m\"; exit 0");
    cmd.env("TERM", "xterm-256color");
    let mut child = pair.slave.spawn_command(cmd).expect("spawn");
    drop(pair.slave);

    let reader = pair.master.try_clone_reader().expect("reader");
    let handle = drain(reader, Arc::clone(&parser));
    let _ = child.wait().expect("child exits");
    handle.join().expect("reader finishes");

    let guard = parser.lock();
    let screen = guard.screen();
    let text = screen.contents();
    assert!(text.contains("hello red"), "screen text was {text:?}");

    let row: String = screen.rows(0, 80).next().unwrap_or_default();
    let col = row.find("red").expect("'red' on the first row") as u16;
    for offset in 0..3 {
        assert_eq!(
            screen.cell(0, col + offset).expect("cell").fgcolor(),
            vt100::Color::Idx(1),
            "cell {offset} of 'red' should be red"
        );
    }
    assert_eq!(
        screen.cell(0, 0).expect("cell").fgcolor(),
        vt100::Color::Default
    );
}

#[test]
fn resize_is_reflected_in_parser() {
    let parser: SharedParser = Arc::new(Mutex::new(Parser::new(24, 80, SCROLLBACK_LINES)));
    let pair = native_pty_system().openpty(pty_size(24, 80)).expect("openpty");

    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.env("TERM", "xterm-256color");
    let mut child = pair.slave.spawn_command(cmd).expect("spawn /bin/sh");
    drop(pair.slave);

    let reader = pair.master.try_clone_reader().expect("reader");
    let handle = drain(reader, Arc::clone(&parser));

    // Resize both sides and wait for the shell to notice.
    pair.master.resize(pty_size(30, 100)).expect("resize");
    parser.lock().set_size(30, 100);
    assert_eq!(parser.lock().screen().size(), (30, 100));

    // Ask the shell to report its own idea of the size.
    use std::io::Write as _;
    let mut writer = pair.master.take_writer().expect("writer");
    let _ = writer.write_all(b"stty size\n");
    let _ = writer.flush();
    assert!(
        wait_until(10, || parser
            .lock()
            .screen()
            .contents()
            .contains("30 100")),
        "shell should report '30 100', got {:?}",
        parser.lock().screen().contents()
    );

    let _ = child.kill();
    let _ = child.wait();
    drop(writer);
    let _ = handle.join();
}

#[test]
fn term_tab_spawn_write_and_resize() {
    let ctx = egui::Context::default();
    let mut tab =
        loop_desk_term::TermTab::spawn(std::env::temp_dir(), ctx).expect("spawn tab");
    assert_eq!(tab.parser().lock().screen().size(), (24, 80));

    tab.write(b"echo loop-desk-term-ok\n");
    assert!(
        wait_until(10, || tab
            .parser()
            .lock()
            .screen()
            .contents()
            .contains("loop-desk-term-ok")),
        "shell did not echo our command"
    );

    tab.resize(40, 120);
    assert_eq!(tab.parser().lock().screen().size(), (40, 120));
    tab.kill();

    // Selection helpers used by the renderer's copy support: the selected
    // text must match exactly the cells the parser reports for that range.
    tab.set_selection(Some(((0, 2), (0, 4))));
    let expected: String = {
        let guard = tab.parser().lock();
        (2..=4)
            .map(|c| guard.screen().cell(0, c).map(|cell| cell.contents()).unwrap_or_default())
            .collect()
    };
    assert_eq!(tab.selected_text().as_deref(), Some(expected.as_str()));
    tab.set_selection(None);
    assert_eq!(tab.selected_text(), None);
    // A degenerate single-cell selection yields nothing.
    tab.set_selection(Some(((0, 3), (0, 3))));
    assert_eq!(tab.selected_text(), None);

    // Tab title: falls back to the directory name when the shell set no
    // OSC title, and uses the OSC tail when it did.
    let fallback = tab.title();
    assert!(!fallback.is_empty());
    tab.parser().lock().process(b"\x1b]0;user@host: /tmp/somewhere\x07");
    assert_eq!(tab.title(), "/tmp/somewhere");
}
