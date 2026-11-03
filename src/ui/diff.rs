//! Unified diff rendering with tinted added and removed lines.

use egui::{Color32, RichText, Ui};

use super::theme;

/// One line of a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    /// Context line.
    Context,
    /// Added line.
    Add,
    /// Removed line.
    Remove,
    /// Hunk header.
    Header,
}

impl LineKind {
    fn bg(self, p: &theme::Palette) -> Option<Color32> {
        match self {
            Self::Add => Some(p.diff_add),
            Self::Remove => Some(p.diff_del),
            _ => None,
        }
    }

}

/// Render a unified diff for `path`.
pub fn show(ui: &mut Ui, path: &str, diff: &str) {
    let p = palette(ui);
    ui.label(
        RichText::new(path)
            .font(theme::caption_font())
            .color(p.muted),
    );
    ui.add_space(theme::SPACE_4);
    egui::Frame::new()
        .fill(p.surface_2)
        .corner_radius(theme::chip_radius())
        .inner_margin(theme::margin(theme::SPACE_8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::ScrollArea::vertical()
                .id_salt(ui.id().with(path))
                .max_height(240.0)
                .show(ui, |ui| {
                    for line in diff.lines() {
                        let kind = classify(line);
                        let text = format!("{}{}", line_number_prefix(line), line);
                        let mut label = RichText::new(text)
                            .font(theme::mono_font())
                            .color(if kind == LineKind::Remove {
                                p.danger
                            } else if kind == LineKind::Add {
                                p.success
                            } else {
                                p.muted
                            });
                        if kind == LineKind::Header {
                            label = label.color(p.accent);
                        }
                        let row = egui::Label::new(label).selectable(false);
                        if let Some(bg) = kind.bg(&p) {
                            let size = egui::vec2(
                                ui.available_width(),
                                theme::TEXT_MONO * theme::LINE_HEIGHT,
                            );
                            let (rect, _) =
                                ui.allocate_exact_size(size, egui::Sense::hover());
                            ui.painter().rect_filled(rect, 0.0, bg);
                            ui.put(rect, row);
                        } else {
                            ui.add(row);
                        }
                    }
                });
        });
}

fn classify(line: &str) -> LineKind {
    if line.starts_with("+++")
        || line.starts_with("---")
        || line.starts_with("@@")
    {
        LineKind::Header
    } else if line.starts_with('+') {
        LineKind::Add
    } else if line.starts_with('-') {
        LineKind::Remove
    } else {
        LineKind::Context
    }
}

/// A blank prefix keeps all lines the same width.
fn line_number_prefix(_line: &str) -> &'static str {
    " "
}

fn palette(ui: &Ui) -> theme::Palette {
    if ui.visuals().dark_mode {
        theme::DARK
    } else {
        theme::LIGHT
    }
}
