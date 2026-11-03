//! The model and sign-in dialog: pick a model, or connect a provider by API key.

use egui::{RichText, Ui};

use super::theme::{self, Palette};
use crate::state::AppState;

/// Built-in providers that can be connected with a key: (id, name, key hint).
pub const PROVIDERS: &[(&str, &str, &str)] = &[
    ("soket", "Soket", "your Soket API key"),
    ("openrouter", "OpenRouter", "sk-or-…"),
    ("openai", "OpenAI", "sk-…"),
];

/// What the dialog asks for.
#[derive(Debug, Clone)]
pub enum ConnectAction {
    /// Use this model.
    SelectModel {
        /// Provider id.
        provider: String,
        /// Model id.
        id: String,
    },
    /// Verify and save this key.
    Login {
        /// Provider preset id.
        provider: String,
        /// The key.
        api_key: String,
    },
}

/// Draw the dialog when it is open.
pub fn show(ctx: &egui::Context, state: &mut AppState, p: &Palette) -> Option<ConnectAction> {
    if !state.connect.open {
        return None;
    }
    let mut action = None;
    let mut open = true;
    egui::Window::new("Models and sign-in")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(440.0, 340.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(p.surface)
                .stroke(theme::hairline(p))
                .inner_margin(theme::margin(theme::SPACE_16)),
        )
        .show(ctx, |ui| {
            models_section(ui, state, p, &mut action);
            ui.add_space(theme::SPACE_12);
            ui.separator();
            ui.add_space(theme::SPACE_12);
            login_section(ui, state, p, &mut action);
        });
    if !open {
        state.connect.open = false;
        state.connect.key.clear();
    }
    action
}

fn models_section(
    ui: &mut Ui,
    state: &mut AppState,
    p: &Palette,
    action: &mut Option<ConnectAction>,
) {
    ui.label(RichText::new("Model").strong().color(p.text));
    ui.add_space(theme::SPACE_4);
    if state.models.is_empty() {
        ui.label(
            RichText::new("No provider connected yet. Add your API key below.")
                .color(p.muted),
        );
        return;
    }
    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        for m in &state.models {
            let spec = format!("{}/{}", m.provider, m.id);
            let selected = state.current_model.as_deref() == Some(spec.as_str());
            let text = RichText::new(&spec).font(theme::caption_font());
            let row = ui.add_sized(
                [ui.available_width(), 26.0],
                egui::SelectableLabel::new(selected, text),
            );
            if row.clicked() && !selected {
                *action = Some(ConnectAction::SelectModel {
                    provider: m.provider.clone(),
                    id: m.id.clone(),
                });
            }
        }
    });
}

fn login_section(
    ui: &mut Ui,
    state: &mut AppState,
    p: &Palette,
    action: &mut Option<ConnectAction>,
) {
    ui.label(RichText::new("Connect a provider").strong().color(p.text));
    ui.add_space(theme::SPACE_4);
    let c = &mut state.connect;
    egui::ComboBox::from_id_salt("provider")
        .selected_text(PROVIDERS[c.provider.min(PROVIDERS.len() - 1)].1)
        .show_ui(ui, |ui| {
            for (i, (_, name, _)) in PROVIDERS.iter().enumerate() {
                ui.selectable_value(&mut c.provider, i, *name);
            }
        });
    let (id, name, hint) = PROVIDERS[c.provider.min(PROVIDERS.len() - 1)];
    ui.add_space(theme::SPACE_4);
    let edit = ui.add_sized(
        [ui.available_width(), 28.0],
        egui::TextEdit::singleline(&mut c.key)
            .password(true)
            .hint_text(hint),
    );
    ui.add_space(theme::SPACE_8);
    let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let ready = !c.key.trim().is_empty() && !c.busy;
    ui.horizontal(|ui| {
        let label = if c.busy { "Checking…" } else { "Connect" };
        let clicked = ui
            .add_enabled(
                ready,
                egui::Button::new(RichText::new(label).color(p.bg)).fill(p.accent),
            )
            .clicked();
        if (clicked || (enter && ready)) && ready {
            c.busy = true;
            c.message = None;
            *action = Some(ConnectAction::Login {
                provider: id.to_owned(),
                api_key: std::mem::take(&mut c.key),
            });
        }
        if ui.button("Done").clicked() {
            c.open = false;
            c.key.clear();
        }
        if let Some((ok, msg)) = &c.message {
            let color = if *ok { p.success } else { p.danger };
            ui.label(RichText::new(msg).font(theme::caption_font()).color(color));
        }
    });
    ui.add_space(theme::SPACE_4);
    ui.label(
        RichText::new(format!(
            "The key is checked with {name}, then stored in ~/.loop/agent/auth.json (owner-only)."
        ))
        .font(theme::caption_font())
        .color(p.muted),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, state: &mut AppState) -> Option<ConnectAction> {
        let mut out = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            out = show(ctx, state, &theme::LIGHT);
        });
        out
    }

    #[test]
    fn dialog_is_a_window_only_while_open() {
        let ctx = egui::Context::default();
        let mut state = AppState::default();
        frame(&ctx, &mut state);
        assert!(!ctx.memory(|m| m.areas().visible_last_frame(&egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("Models and sign-in")
        ))));
        state.connect.open = true;
        for _ in 0..3 {
            frame(&ctx, &mut state);
        }
        let visible = ctx.memory(|m| {
            m.areas().visible_layer_ids().iter().any(|l| l.order == egui::Order::Middle)
        });
        assert!(visible, "an open dialog should be a visible window");
    }
}
