# loop-desk-term

An embeddable, multi-tab terminal panel for [egui](https://github.com/emilk/egui) 0.31 / eframe 0.31.
It runs your login shell in a real pseudo-terminal (`portable-pty`), parses its
output with `vt100`, and renders it as a styled cell grid inside any `egui::Ui`.

The crate is deliberately small and self-contained (a handful of modules, no
other project dependencies) so you can either depend on it by path or copy the
sources straight into your app as `src/ui/terminal.rs`.

## Layout

| File | Contents |
| --- | --- |
| `src/lib.rs` | `TerminalPanel`: tabs, focus, public API |
| `src/pty.rs` | `TermTab`: pty spawn, reader thread, resize, scrollback |
| `src/keys.rs` | Pure key event → byte-sequence mapping |
| `src/palette.rs` | `TermTheme` plus the 256-colour cube / truecolor mapping |
| `src/render.rs` | Tab strip + terminal view |
| `src/grid.rs` | Cell grid drawing (runs of equal style, cursor) |
| `src/input.rs` | Keyboard/mouse handling |
| `src/bin/term_demo.rs` | Demo window (build only) |

## Public API

```rust
pub struct TerminalPanel { /* tabs */ }

pub struct TermTheme {
    pub bg: egui::Color32,
    pub fg: egui::Color32,
    pub cursor: egui::Color32,
    pub selection: egui::Color32,
    pub ansi: [egui::Color32; 16],
    pub font_size: f32,
}

impl TermTheme {
    pub fn light() -> Self;
    pub fn dark() -> Self;
}

impl TerminalPanel {
    /// Starts the first tab with the user's login shell ($SHELL, or /bin/zsh)
    /// running with `-l`, `TERM=xterm-256color`, in `cwd`.
    pub fn new(cwd: std::path::PathBuf, ctx: egui::Context) -> anyhow::Result<Self>;

    /// Draws the tab strip and the active terminal; handles input, resizing,
    /// scrolling and the exit banner.
    pub fn ui(&mut self, ui: &mut egui::Ui, theme: &TermTheme);

    pub fn new_tab(&mut self) -> anyhow::Result<()>;
    pub fn close_active_tab(&mut self);
    pub fn tab_count(&self) -> usize;
}
```

A few extra read-only helpers (`active_tab_index`, `terminal_size`) are also
public because they are handy for embedding apps.

## Embedding

### As a path dependency

```toml
[dependencies]
loop-desk-term = { path = "../loop-desk-term" }
```

```rust
use loop_desk_term::{TermTheme, TerminalPanel};

struct MyApp {
    term: TerminalPanel,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        Ok(Self { term: TerminalPanel::new(cwd, cc.egui_ctx.clone())? })
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Your own shortcuts next to the panel:
            if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::T)) {
                let _ = self.term.new_tab();
            }
            if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::W)) {
                self.term.close_active_tab();
            }

            self.term.ui(ui, &TermTheme::dark());
        });
    }
}
```

### By copying the sources

Copy `src/{lib,pty,keys,palette,render,grid,input}.rs` into your crate (e.g.
under `src/ui/terminal/`), make `lib.rs` a `mod terminal;`, add the
dependencies from `[dependencies]` below, and `pub use terminal::TerminalPanel;`.
Nothing in the crate reaches outside its own modules.

## Behaviour notes

- **Shell**: `$SHELL` (fallback `/bin/zsh`), started with `-l`,
  `TERM=xterm-256color`, in the directory given to `TerminalPanel::new`.
- **Rendering**: monospace font from egui; cell size derived from the font;
  per-cell foreground/background, bold (brightened), italic, underline,
  inverse and wide characters; runs of equal style are drawn as one text job.
  The cursor is a filled block when focused and hollow otherwise.
- **Size**: cols/rows derive from the available rect; changes call
  `master.resize` and `parser.set_size`.
- **Input** (when the terminal has focus): UTF-8 text, Enter, Backspace, Tab,
  Esc, Delete, arrows (application-cursor sequences when the screen asks for
  them), Home/End, PageUp/PageDown, F1–F12, Ctrl+letter control codes,
  Alt+key as an ESC prefix, and paste wrapped in bracketed paste mode when
  enabled by the application.
- **Scrolling**: the mouse wheel scrolls the vt100 scrollback; any key snaps
  back to the bottom.
- **Selection** (bonus): drag to select, `Cmd/Ctrl+C` copies the selection
  (and does nothing without one).
- **Tabs**: title from the working directory name (else "zsh"), a "+" button,
  and an "×" on hover. Closing the last tab opens a new one.
- **Exit**: when the shell exits the tab shows `[process exited]` in muted
  text and Enter starts a fresh shell.
- No panics on bad input; nothing that can fail at runtime is `unwrap()`ed.

## Demo binary

`src/bin/term_demo.rs` is a window containing only the panel (dark theme,
`Cmd+T` new tab, `Cmd+W` close):

```sh
cargo run --bin term_demo
```

## Tests

`cargo test` runs everything without opening a window:

- `src/keys.rs` — letters, `Ctrl+C` = `0x03`, Enter/Backspace, arrows in
  normal and application mode, `Alt+x`, F5, paste with and without brackets.
- `tests/pty.rs` — spawns `/bin/sh -c 'printf "hello\033[31m red\033[0m"'`
  through portable-pty and asserts the parsed screen contains `hello red`
  with a red foreground on the cells of `red`; resizes to 100x30 and checks
  both the parser and the shell agree on the size.
- `tests/render.rs` — headless egui frames driving `TerminalPanel::ui`,
  checking that drawing works and the pty gets resized.
- `src/palette.rs` — the 256-colour cube, grayscale ramp, truecolor and
  default colour mapping.

## Dependencies

```toml
[dependencies]
eframe = "0.31"
egui = "0.31"
portable-pty = "0.8"
vt100 = "0.15"
anyhow = "1"
parking_lot = "0.12"
```
