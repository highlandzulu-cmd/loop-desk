//! Demo window for the terminal panel.
//!
//! Build only — do not run this in an unattended session.

use eframe::egui;
use loop_desk_term::{TermTheme, TerminalPanel};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_title("term_demo"),
        ..Default::default()
    };
    eframe::run_native(
        "term_demo",
        options,
        Box::new(|cc| {
            // The first tab runs in the current directory.
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
            let panel = TerminalPanel::new(cwd, cc.egui_ctx.clone()).ok();
            Ok(Box::new(DemoApp { panel }))
        }),
    )
}

struct DemoApp {
    panel: Option<TerminalPanel>,
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(panel) = self.panel.as_mut() else {
                ui.label("Failed to start a shell.");
                return;
            };

            // Cmd+T / Cmd+W (Ctrl on non-Mac).
            let cmd = ui.input(|i| i.modifiers.command);
            if cmd && ui.input(|i| i.key_pressed(egui::Key::T)) {
                let _ = panel.new_tab();
            }
            if cmd && ui.input(|i| i.key_pressed(egui::Key::W)) {
                panel.close_active_tab();
            }

            let theme = TermTheme::dark();
            panel.ui(ui, &theme);
        });
    }
}
