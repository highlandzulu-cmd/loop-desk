//! The conversation column: rows, thinking lines, notices and the working
//! indicator.

use egui::{Layout, RichText, Ui};

use super::{icons, markdown, theme, tool_card};
use crate::state::{
    AppState, ChatRow, NoticeLevel, PermissionOutcome, PermissionPrompt,
};

/// Draw all rows inside the scroll area. Returns a clicked example prompt.
pub fn show(ui: &mut Ui, state: &mut AppState) -> Option<ChatAction> {
    let mut action = None;
    if state.rows.is_empty() {
        empty_state(ui, state);
        return None;
    }
    // Spacing between blocks: 24 px for messages and other standalone rows,
    // 8 px when tool cards run in a row.
    let mut prev_was_tool = false;
    for row in state.rows.clone() {
        let is_tool = matches!(row, ChatRow::Tool(_));
        if prev_was_tool && is_tool {
            ui.add_space(theme::SPACE_8);
        } else {
            ui.add_space(theme::SPACE_24);
        }
        match row {
            ChatRow::User { text, .. } => user_message(ui, &text),
            ChatRow::Agent { text, .. } => agent_message(ui, &text),
            ChatRow::Thought { id, text, seconds } => {
                thought_row(ui, state, &id, &text, seconds)
            }
            ChatRow::Tool(card) => {
                let expanded = *state.expanded_tools.get(&card.id).unwrap_or(&false);
                if tool_card::show(ui, &card, expanded) {
                    state
                        .expanded_tools
                        .insert(card.id.clone(), !expanded);
                }
            }
            ChatRow::Permission(prompt) => {
                if let Some(outcome) = permission_card(ui, &prompt) {
                    action = Some(ChatAction::Approve {
                        id: prompt.id.clone(),
                        outcome,
                    });
                }
            }
            ChatRow::Notice { level, text } => notice(ui, level, &text),
        }
        prev_was_tool = is_tool;
    }
    if state.working {
        ui.add_space(theme::SPACE_8);
        working(ui);
    }
    action
}

/// Something the user clicked in the conversation column.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatAction {
    /// The user answered an approval.
    Approve {
        /// Which request.
        id: String,
        /// The decision.
        outcome: PermissionOutcome,
    },
}

fn user_message(ui: &mut Ui, text: &str) {
    let p = palette(ui);
    // Right-aligned bubble, at most 80% of the column, wrapping its text.
    let avail = ui.available_width();
    let max_w = (avail * 0.8).min(theme::CHAT_MAX_WIDTH * 0.8);
    let galley = ui.ctx().fonts(|f| {
        f.layout_job(egui::text::LayoutJob::simple(
            text.to_owned(),
            theme::body_font(),
            p.text,
            egui::vec2(max_w - theme::SPACE_32, f32::INFINITY).x,
        ))
    });
    let size = galley.size() + egui::vec2(theme::SPACE_32, theme::SPACE_24);
    let width = size.x.max(theme::SPACE_64);
    ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, size.y), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, theme::soft_radius(), p.surface_2);
        let pos = rect.left_top() + egui::vec2(theme::SPACE_16, theme::SPACE_12);
        ui.painter().galley(pos, galley, egui::Color32::WHITE);
    });
}

fn agent_message(ui: &mut Ui, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    markdown::show(ui, text);
}

fn thought_row(ui: &mut Ui, state: &mut AppState, id: &str, text: &str, seconds: f32) {
    let p = palette(ui);
    let expanded = *state.expanded_thoughts.get(id).unwrap_or(&false);
    let label = if seconds > 0.0 {
        format!("Thought for {:.0}s", seconds)
    } else {
        "Thought".to_owned()
    };
    let row = ui.horizontal(|ui| {
        icons::paint(
            ui,
            if expanded { icons::Icon::ChevronDown } else { icons::Icon::ChevronRight },
            p.muted,
        );
        ui.label(RichText::new(label).font(theme::caption_font()).color(p.muted));
    });
    let btn = ui.interact(row.response.rect, ui.id().with(("thought", id)), egui::Sense::click());
    let clicked = btn.clicked();
    btn.on_hover_cursor(egui::CursorIcon::PointingHand);
    if clicked {
        state
            .expanded_thoughts
            .insert(id.to_owned(), !expanded);
    }
    if expanded {
        ui.add_space(theme::SPACE_4);
        ui.label(
            RichText::new(text)
                .font(theme::muted_font())
                .color(p.muted)
                .italics()
                .line_height(theme::line_height(theme::TEXT_UI)),
        );
    }
}

/// An inline permission request: what wants to run, then the three buttons.
///
/// A resolved card only records the decision.
fn permission_card(ui: &mut Ui, prompt: &PermissionPrompt) -> Option<PermissionOutcome> {
    let p = palette(ui);
    let mut outcome = None;
    egui::Frame::new()
        .fill(p.accent_soft)
        .corner_radius(theme::card_radius())
        .inner_margin(theme::margin(theme::SPACE_12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(if prompt.resolved {
                    format!("Permission — {}", prompt.outcome.map(PermissionOutcome::label).unwrap_or("answered"))
                } else {
                    "Permission needed".to_owned()
                })
                .size(theme::TEXT_UI)
                .strong()
                .color(p.text),
            );
            ui.add_space(theme::SPACE_4);
            if !prompt.detail.is_empty() && prompt.detail != prompt.summary {
                ui.label(
                    RichText::new(&prompt.detail)
                        .font(theme::mono_font())
                        .color(p.muted),
                );
                ui.add_space(theme::SPACE_4);
            }
            ui.label(
                RichText::new(&prompt.summary)
                    .font(theme::mono_font())
                    .color(p.text),
            );
            if !prompt.resolved {
                ui.add_space(theme::SPACE_8);
                ui.horizontal(|ui| {
                    if pill_button(ui, &p, "Allow once", true) {
                        outcome = Some(PermissionOutcome::AllowOnce);
                    }
                    ui.add_space(theme::SPACE_8);
                    if pill_button(ui, &p, "Allow for this session", true) {
                        outcome = Some(PermissionOutcome::AllowSession);
                    }
                    ui.add_space(theme::SPACE_8);
                    if pill_button(ui, &p, "Deny", false) {
                        outcome = Some(PermissionOutcome::Deny);
                    }
                });
            }
        });
    outcome
}

/// A small rounded button; `primary` picks the accent fill.
fn pill_button(ui: &mut Ui, p: &theme::Palette, label: &str, primary: bool) -> bool {
    let btn = ui.add(
        egui::Button::new(
            RichText::new(label)
                .font(theme::muted_font())
                .color(if primary { p.bg } else { p.text }),
        )
        .fill(if primary { p.accent } else { p.surface })
        .stroke(theme::hairline(p))
        .corner_radius(theme::chip_radius()),
    );
    let clicked = btn.clicked();
    btn.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
}

fn notice(ui: &mut Ui, level: NoticeLevel, text: &str) {
    let p = palette(ui);
    let color = match level {
        NoticeLevel::Info => p.muted,
        NoticeLevel::Warning => p.accent,
        NoticeLevel::Error => p.danger,
    };
    ui.horizontal(|ui| {
        let line = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(line * 0.25, 1.0),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(rect, 0.0, p.border);
        ui.label(
            RichText::new(text)
                .font(theme::caption_font())
                .color(color),
        );
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(line * 0.25, 1.0),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(rect, 0.0, p.border);
    });
}

fn working(ui: &mut Ui) {
    let p = palette(ui);
    let dots = ((ui.ctx().input(|i| i.time) * 3.0) as usize % 3) + 1;
    ui.label(
        RichText::new(format!("Working{}", ".".repeat(dots)))
            .font(theme::caption_font())
            .color(p.muted),
    );
}

fn empty_state(ui: &mut Ui, state: &mut AppState) {
    let p = palette(ui);
    ui.add_space(theme::SPACE_32);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("What are we building today?")
                .size(theme::TEXT_GREETING),
        );
        ui.add_space(theme::SPACE_8);
        ui.label(
            RichText::new(&state.project_dir)
                .font(theme::caption_font())
                .color(p.muted),
        );
    });
    ui.add_space(theme::SPACE_24);
    for prompt in EXAMPLE_PROMPTS {
        let btn = ui.add(
            egui::Button::new(
                RichText::new(prompt.to_owned())
                    .font(theme::muted_font())
                    .color(p.text),
            )
            .fill(p.surface)
            .stroke(theme::hairline(&p))
            .corner_radius(theme::card_radius())
            .min_size(egui::vec2(ui.available_width(), 38.0)),
        );
        let clicked = btn.clicked();
        btn.on_hover_cursor(egui::CursorIcon::PointingHand);
        if clicked {
            state.draft = (*prompt).to_owned();
        }
    }
}

/// Example prompts shown on the empty screen.
pub const EXAMPLE_PROMPTS: [&str; 4] = [
    "Explain this codebase",
    "Find and fix a bug",
    "Write tests for a module",
    "Review my last commit",
];

fn palette(ui: &Ui) -> theme::Palette {
    if ui.visuals().dark_mode {
        theme::DARK
    } else {
        theme::LIGHT
    }
}
