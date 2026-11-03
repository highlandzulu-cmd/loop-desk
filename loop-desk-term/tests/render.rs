//! Tests for the rendering path that need no window: we drive egui with a
//! headless [`egui::Context`] (real font atlas, no GPU) and check that the
//! panel draws without panicking and resizes its pty.

use std::path::PathBuf;

use loop_desk_term::{TermTheme, TerminalPanel};

fn temp_cwd() -> PathBuf {
    std::env::temp_dir()
}

#[test]
fn panel_ui_draws_and_resizes_headless() {
    let ctx = egui::Context::default();
    let mut panel = TerminalPanel::new(temp_cwd(), ctx.clone()).expect("panel");
    assert_eq!(panel.tab_count(), 1);

    // A couple of frames at different sizes; fonts are loaded for real.
    for size in [[600.0, 400.0], [900.0, 600.0]] {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size[0], size[1]),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let theme = TermTheme::dark();
                panel.ui(ui, &theme);
            });
        });
    }

    // The panel reports the live terminal size through its public API; it
    // must have moved away from the 80x24 default to a size derived from the
    // available rect and the monospace cell metrics.
    let (rows, cols) = panel.terminal_size();
    assert!(
        (cols, rows) != (80, 24),
        "expected the pty to be resized away from the default 80x24, got {cols}x{rows}"
    );
    assert!(cols >= 2 && rows >= 2, "got {cols}x{rows}");
}

#[test]
fn new_and_close_tabs() {
    let ctx = egui::Context::default();
    let mut panel = TerminalPanel::new(temp_cwd(), ctx).expect("panel");

    panel.new_tab().expect("second tab");
    assert_eq!(panel.tab_count(), 2);
    assert_eq!(panel.active_tab_index(), 1);

    panel.close_active_tab();
    assert_eq!(panel.tab_count(), 1);
    assert_eq!(panel.active_tab_index(), 0);

    // Closing the last tab opens a new one.
    panel.close_active_tab();
    assert_eq!(panel.tab_count(), 1);
}
