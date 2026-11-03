//! Left sidebar: wordmark, new session button, grouped session list.

use egui::{Layout, RichText, Ui};

use super::theme;
use crate::state::SessionEntry;

/// What the user did in the sidebar this frame.
#[derive(Debug, Default)]
pub enum SidebarAction {
    /// Nothing happened.
    #[default]
    None,
    /// The user asked for a fresh session.
    NewSession,
    /// The user clicked this session.
    Select(String),
}

/// Draw the sidebar. The panel already carries the 12 px inner padding.
pub fn show(
    ui: &mut Ui,
    sessions: &[SessionEntry],
    active: &str,
    project_name: &str,
) -> SidebarAction {
    let p = palette(ui);
    let mut action = SidebarAction::None;

    // Wordmark.
    ui.label(RichText::new("Loop").strong().size(theme::TEXT_TITLE));
    ui.add_space(theme::SPACE_12);

    if new_session_button(ui, &p) {
        action = SidebarAction::NewSession;
    }
    ui.add_space(theme::SPACE_16);

    // Sessions grouped by day, each row a compact 36 px line directly under
    // its group label.
    let today = chrono::Local::now().date_naive();
    let mut last_group = "";
    for session in sessions {
        let group = session.day_group(today);
        if group != last_group {
            if !last_group.is_empty() {
                ui.add_space(theme::SPACE_8);
            }
            ui.add_space(theme::SPACE_4);
            ui.label(
                RichText::new(group)
                    .font(theme::caption_font())
                    .color(p.muted),
            );
            ui.add_space(theme::SPACE_4);
            last_group = group;
        }
        if let Some(id) = session_row(ui, session, session.id == active, &p) {
            action = SidebarAction::Select(id);
        }
    }

    // Project name pinned to the bottom.
    ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.separator();
        ui.label(
            RichText::new(project_name)
                .font(theme::caption_font())
                .color(p.muted),
        );
    });
    action
}

fn new_session_button(ui: &mut Ui, p: &theme::Palette) -> bool {
    let width = ui.available_width();
    let btn = ui.add(
        egui::Button::new(
            RichText::new("+  New session")
                .font(theme::muted_font())
                .color(p.text),
        )
        .fill(p.accent_soft)
        .corner_radius(theme::soft_radius())
        .min_size(egui::vec2(width, 34.0)),
    );
    let clicked = btn.clicked();
    btn.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
}

fn session_row(
    ui: &mut Ui,
    session: &SessionEntry,
    is_active: bool,
    p: &theme::Palette,
) -> Option<String> {
    let width = ui.available_width();
    let height = theme::SESSION_ROW_HEIGHT;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let inner = rect.shrink2(egui::vec2(theme::SPACE_4, 0.0));

    if is_active {
        ui.painter().rect_filled(
            inner.expand(0.5),
            theme::chip_radius(),
            p.accent_soft,
        );
    }

    // Activity dot when a turn runs in this session.
    let text_x = inner.left() + theme::SPACE_8;
    let mut label_x = text_x;
    if session.busy {
        let pulse = (ui.ctx().input(|i| i.time) * 2.0).sin().abs();
        let center = egui::pos2(
            text_x + 3.0,
            inner.center().y,
        );
        ui.painter()
            .circle_filled(center, 3.0 + pulse as f32, p.accent);
        label_x = text_x + theme::SPACE_12;
    }

    let galley = ui.ctx().fonts(|f| {
        f.layout_job(
            egui::text::LayoutJob::simple(
                session.title.clone(),
                theme::muted_font(),
                if is_active { p.text } else { p.muted },
                (inner.right() - label_x).max(0.0),
            ),
        )
    });
    let pos = egui::pos2(label_x, inner.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, egui::Color32::WHITE);

    let response = ui.interact(rect, ui.id().with(("session", &session.id)), egui::Sense::click());
    let clicked = response.clicked();
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if clicked {
        Some(session.id.clone())
    } else {
        None
    }
}

fn palette(ui: &Ui) -> theme::Palette {
    if ui.visuals().dark_mode {
        theme::DARK
    } else {
        theme::LIGHT
    }
}
