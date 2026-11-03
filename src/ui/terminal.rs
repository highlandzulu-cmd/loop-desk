//! The terminal dock: a real shell in the project folder, provided by the
//! `loop-desk-term` crate (portable-pty + vt100).

use egui::Color32;
use loop_desk_term::{TermTheme, TerminalPanel};

use super::theme::{self, Palette};

/// Where the dock sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSide {
    /// Right edge, next to the conversation.
    Right,
    /// Under the whole window.
    Bottom,
}

impl DockSide {
    /// Toggle between the two sides.
    pub fn toggle(self) -> Self {
        match self {
            Self::Right => Self::Bottom,
            Self::Bottom => Self::Right,
        }
    }

    /// Label for the side switch button.
    pub fn label(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Bottom => "bottom",
        }
    }
}

/// Everything the dock needs: the panel plus its persisted layout.
pub struct TerminalDock {
    /// The embedded terminal; `None` until it is first opened.
    panel: Option<TerminalPanel>,
    /// Project folder the shells start in.
    cwd: std::path::PathBuf,
    /// True while the dock is visible.
    pub open: bool,
    /// Which edge the dock occupies.
    pub side: DockSide,
    /// Panel size in px (width on the right, height at the bottom).
    pub size: f32,
    /// Set when the panel failed to start; shown instead of the terminal.
    error: Option<String>,
}

/// Default dock width on the right.
const DEFAULT_WIDTH: f32 = 480.0;
/// Default dock height at the bottom.
const DEFAULT_HEIGHT: f32 = 260.0;

impl TerminalDock {
    /// A closed dock for `cwd`.
    pub fn new(cwd: std::path::PathBuf) -> Self {
        Self {
            panel: None,
            cwd,
            open: false,
            side: DockSide::Right,
            size: DEFAULT_WIDTH,
            error: None,
        }
    }

    /// Open the dock, starting the first shell when needed.
    pub fn open_panel(&mut self, ctx: &egui::Context) {
        if self.panel.is_none() && self.error.is_none() {
            match TerminalPanel::new(self.cwd.clone(), ctx.clone()) {
                Ok(panel) => self.panel = Some(panel),
                Err(err) => self.error = Some(format!("terminal failed to start: {err}")),
            }
        }
        self.open = true;
    }

    /// Toggle the dock.
    pub fn toggle(&mut self, ctx: &egui::Context) {
        if self.open {
            self.open = false;
        } else {
            self.open_panel(ctx);
        }
    }

    /// Draw the dock's toolbar row (always shown above the terminal).
    ///
    /// Returns true when the user closed the dock.
    pub fn toolbar(&mut self, ui: &mut egui::Ui, p: &Palette) -> bool {
        let mut close = false;
        let size = egui::vec2(ui.available_width(), 28.0);
        ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let count = self.panel.as_ref().map(TerminalPanel::tab_count).unwrap_or(0);
            let label = if count == 1 {
                "Terminal".to_owned()
            } else {
                format!("Terminal ({count})")
            };
            ui.label(
                egui::RichText::new(label)
                    .size(theme::TEXT_UI)
                    .strong()
                    .color(p.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if super::icons::button(ui, super::icons::Icon::Cross, p.muted).clicked() {
                    close = true;
                }
                let side_label = format!("move to {}", self.side.toggle().label());
                if small_button(ui, p, &side_label) {
                    self.side = self.side.toggle();
                }
                let plus = small_button(ui, p, "+ tab");
                if plus {
                    if let Some(panel) = &mut self.panel {
                        if let Err(err) = panel.new_tab() {
                            self.error = Some(format!("could not start a shell: {err}"));
                        }
                    }
                }
            });
        });
        close
    }

    /// Draw the terminal itself.
    pub fn body(&mut self, ui: &mut egui::Ui, mode: theme::ThemeMode) {
        let term_theme = term_theme(mode.palette());
        let Some(panel) = &mut self.panel else {
            let text = self
                .error
                .clone()
                .unwrap_or_else(|| "terminal unavailable".to_owned());
            ui.label(
                egui::RichText::new(text)
                    .font(theme::caption_font())
                    .color(mode.palette().danger),
            );
            return;
        };
        panel.ui(ui, &term_theme);
    }

    /// Current default size for the active side.
    pub fn default_size(&self) -> f32 {
        match self.side {
            DockSide::Right => DEFAULT_WIDTH,
            DockSide::Bottom => DEFAULT_HEIGHT,
        }
    }
}

/// Map the app palette onto the terminal's palette, keeping the terminal
/// readable on both themes (the shell mostly draws its own colours).
fn term_theme(p: Palette) -> TermTheme {
    let base = if p.bg.r() > 128 {
        TermTheme::light()
    } else {
        TermTheme::dark()
    };
    TermTheme {
        bg: p.surface_2,
        fg: p.text,
        cursor: p.accent,
        selection: Color32::from_rgba_premultiplied(
            p.accent.r(),
            p.accent.g(),
            p.accent.b(),
            90,
        ),
        font_size: theme::TEXT_MONO,
        ..base
    }
}

/// A quiet text button used in the dock toolbar.
fn small_button(ui: &mut egui::Ui, p: &Palette, label: &str) -> bool {
    let btn = ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .font(theme::caption_font())
                .color(p.muted),
        )
        .fill(p.surface_2)
        .corner_radius(theme::chip_radius()),
    );
    let clicked = btn.clicked();
    btn.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
}
