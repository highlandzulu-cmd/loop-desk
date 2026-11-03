//! The right-hand "Changes" panel: the list of files the agent changed and a
//! unified diff view for the selected one.

use egui::{Layout, RichText, Ui};

use super::{diff, theme};
use crate::state::{AppState, ChangedFile};

/// Draw the whole panel. Returns true when the user closed it.
pub fn show(ui: &mut Ui, state: &mut AppState) -> bool {
    let p = palette(ui);
    let mut close = false;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Changes")
                .size(theme::TEXT_UI)
                .strong()
                .color(p.text),
        );
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if super::icons::button(ui, super::icons::Icon::Cross, p.muted).clicked() {
                close = true;
            }
        });
    });
    ui.add_space(theme::SPACE_8);

    if state.changed_files.is_empty() {
        ui.label(
            RichText::new("No file changes yet")
                .font(theme::caption_font())
                .color(p.muted),
        );
        return close;
    }

    // File list, newest change first.
    let selected = state.selected_change.unwrap_or(0).min(state.changed_files.len() - 1);
    for (index, change) in state.changed_files.iter().enumerate() {
        if file_row(ui, change, index == selected, &p) {
            state.selected_change = Some(index);
        }
    }

    ui.add_space(theme::SPACE_12);
    ui.separator();

    // Diff of the selected file.
    if let Some(change) = state.changed_files.get(selected).cloned() {
        ui.add_space(theme::SPACE_8);
        diff::show(ui, &change.path, &change.diff);
    }
    close
}

/// One row of the file list: path plus added/removed counts.
fn file_row(ui: &mut Ui, change: &ChangedFile, active: bool, p: &theme::Palette) -> bool {
    let width = ui.available_width();
    let height = theme::SESSION_ROW_HEIGHT;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let inner = rect.shrink2(egui::vec2(theme::SPACE_4, 0.0));

    if active {
        ui.painter()
            .rect_filled(inner.expand(0.5), theme::chip_radius(), p.accent_soft);
    }

    // Path on the left, +N -M on the right.
    let stats_w = 64.0;
    let path_w = (inner.width() - theme::SPACE_16 - stats_w).max(40.0);
    let galley = ui.ctx().fonts(|f| {
        f.layout_job(egui::text::LayoutJob::simple(
            short_path(&change.path),
            theme::muted_font(),
            if active { p.text } else { p.muted },
            path_w,
        ))
    });
    let pos = egui::pos2(
        inner.left() + theme::SPACE_8,
        inner.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, egui::Color32::WHITE);

    let stats = format!("+{} −{}", change.added, change.removed);
    let stats_galley = ui.ctx().fonts(|f| {
        f.layout_job(egui::text::LayoutJob::simple(
            stats,
            theme::caption_font(),
            p.muted,
            stats_w,
        ))
    });
    let spos = egui::pos2(
        inner.right() - theme::SPACE_8 - stats_galley.size().x,
        inner.center().y - stats_galley.size().y / 2.0,
    );
    ui.painter().galley(spos, stats_galley, egui::Color32::WHITE);

    let response = ui.interact(rect, ui.id().with(("change", &change.path)), egui::Sense::click());
    let clicked = response.clicked();
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
}

/// Last two components of a path, so long paths still fit one row.
fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    match parts.len() {
        0 | 1 => path.to_owned(),
        n => parts[n - 2..].join("/"),
    }
}

fn palette(ui: &Ui) -> theme::Palette {
    if ui.visuals().dark_mode {
        theme::DARK
    } else {
        theme::LIGHT
    }
}
