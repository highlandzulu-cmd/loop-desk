//! The floating composer pinned under the conversation.

use egui::{Layout, RichText, Ui};

use super::theme;
use crate::state::AppState;

/// What the user did in the composer.
#[derive(Debug, Clone, PartialEq)]
pub enum ComposerAction {
    /// Send this prompt.
    Send(String),
    /// Stop the running turn.
    Stop,
}

/// Draw the composer.
pub fn show(ui: &mut Ui, state: &mut AppState) -> Option<ComposerAction> {
    let p = palette(ui);
    let mut action: Option<ComposerAction> = None;

    egui::Frame::new()
        .fill(p.surface)
        .stroke(theme::hairline(&p))
        .corner_radius(theme::soft_radius())
        .inner_margin(theme::margin_sides(
            theme::SPACE_16,
            theme::SPACE_16,
            theme::SPACE_12,
            theme::SPACE_8,
        ))
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 24,
            spread: 0,
            color: shadow_color(&p),
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Auto-growing text area, 1 to 8 lines.
            let rows = state.draft.lines().count().clamp(1, 8);
            let output = egui::TextEdit::multiline(&mut state.draft)
                .id(egui::Id::new("composer-input"))
                .desired_rows(rows)
                .desired_width(f32::INFINITY)
                .font(theme::body_font())
                .hint_text(
                    RichText::new("Ask Loop to make a change...")
                        .font(theme::body_font())
                        .color(p.muted),
                )
                .frame(false)
                .lock_focus(true)
                .show(ui);
            let has_focus = output.response.has_focus();

            ui.add_space(theme::SPACE_4);

            // Chips on the left, send/stop on the right.
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if state.working {
                    if stop_button(ui, &p) {
                        action = Some(ComposerAction::Stop);
                    }
                } else if send_button(ui, &p, !state.draft.trim().is_empty()) {
                    action = Some(ComposerAction::Send(std::mem::take(&mut state.draft)));
                }
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    let label = state.model_label.as_deref().unwrap_or("choose a model");
                    if chip(ui, &p, &format!("{label}  ▾")) {
                        state.connect.open = true;
                    }
                    ui.add_space(theme::SPACE_8);
                    let _ = chip(ui, &p, state.permission_mode.label());
                });
            });

            // Enter sends while the text area owns the keyboard; Shift+Enter
            // inserts a newline (TextEdit already does that).
            let enter_sends = has_focus
                && !state.working
                && !state.draft.trim().is_empty()
                && ui.input(|i| {
                    i.key_pressed(egui::Key::Enter)
                        && i.modifiers.contains(egui::Modifiers::NONE)
                });
            if enter_sends {
                action = Some(ComposerAction::Send(std::mem::take(&mut state.draft)));
            }
        });

    // Cmd+Enter always sends, focused or not.
    let cmd_enter_sends = !state.working
        && !state.draft.trim().is_empty()
        && ui.input(|i| {
            i.key_pressed(egui::Key::Enter)
                && i.modifiers.contains(egui::Modifiers::COMMAND)
        });
    if cmd_enter_sends {
        return Some(ComposerAction::Send(std::mem::take(&mut state.draft)));
    }

    // Esc stops the running turn from anywhere in the window.
    if state.working && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        return Some(ComposerAction::Stop);
    }
    action
}

fn send_button(ui: &mut Ui, p: &theme::Palette, enabled: bool) -> bool {
    let btn = ui.add(
        egui::Button::new(
            RichText::new("↑").size(theme::TEXT_UI).color(if enabled {
                p.bg
            } else {
                p.muted
            }),
        )
        .fill(if enabled { p.accent } else { p.surface_2 })
        .corner_radius(egui::CornerRadius::same(15))
        .min_size(egui::vec2(30.0, 30.0)),
    );
    let clicked = btn.clicked();
    btn.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked && enabled
}

fn stop_button(ui: &mut Ui, p: &theme::Palette) -> bool {
    let btn = ui.add(
        egui::Button::new(
            RichText::new("■")
                .size(theme::TEXT_UI - 2.0)
                .color(p.bg),
        )
        .fill(p.danger)
        .corner_radius(theme::chip_radius())
        .min_size(egui::vec2(30.0, 30.0)),
    );
    let clicked = btn.clicked();
    btn.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
}

fn chip(ui: &mut Ui, p: &theme::Palette, label: &str) -> bool {
    let btn = ui.add(
        egui::Button::new(
            RichText::new(label)
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

fn shadow_color(p: &theme::Palette) -> egui::Color32 {
    // A soft, large-blur shadow; nearly transparent in light mode.
    let alpha = if p.bg.r() > 128 { 14 } else { 90 };
    egui::Color32::from_black_alpha(alpha)
}

fn palette(ui: &Ui) -> theme::Palette {
    if ui.visuals().dark_mode {
        theme::DARK
    } else {
        theme::LIGHT
    }
}
