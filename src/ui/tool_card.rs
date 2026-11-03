//! Compact, expandable cards for tool activity.

use egui::{Layout, RichText, Sense, Ui};

use super::{icons, theme};
use crate::state::{ToolCard, ToolStatus};

/// Draw one tool card. Returns true when the user toggled it.
///
/// The collapsed card is a single compact row (icon, name, summary, status);
/// only the expanded body grows, capped by a scroll area.
pub fn show(ui: &mut Ui, card: &ToolCard, expanded: bool) -> bool {
    let mut toggled = false;
    let p = palette(ui);
    let status_color = match card.status {
        ToolStatus::Completed => p.success,
        ToolStatus::Failed => p.danger,
        _ => p.muted,
    };

    let outer = ui
        .scope(|ui| {
            ui.set_min_width(ui.available_width());
            egui::Frame::new()
                .fill(p.surface)
                .stroke(theme::hairline(&p))
                .corner_radius(theme::card_radius())
                .inner_margin(theme::card_margin())
                .show(ui, |ui| {
                    // One horizontal header row: icon, name + summary, status.
                    // `horizontal` sizes the row to its content; a raw
                    // `with_layout(left_to_right(Center))` would stretch to
                    // the full available height inside the frame.
                    ui.horizontal(|ui| {
                        icons::paint(ui, icons::tool(card.kind), p.accent);
                        ui.label(
                            RichText::new(card.name.clone().unwrap_or_else(|| "tool".into()))
                                .strong()
                                .font(theme::muted_font())
                                .color(p.text),
                        );
                        if !card.title.is_empty() {
                            ui.label(
                                RichText::new(one_line(&card.title))
                                    .font(theme::caption_font())
                                    .color(p.muted),
                            );
                        }

                        // Right side: status glyph and chevron.
                        ui.with_layout(
                            Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                icons::paint(
                                    ui,
                                    if expanded { icons::Icon::ChevronDown } else { icons::Icon::ChevronRight },
                                    p.muted,
                                );
                                ui.add_space(theme::SPACE_8);
                                status_glyph(ui, card.status, status_color);
                            },
                        );
                    });

                    if expanded {
                        ui.add_space(theme::SPACE_8);
                        if let Some(diff) = &card.diff {
                            super::diff::show(ui, &diff.path, &diff.diff);
                        }
                        if !card.input.is_empty() {
                            section(ui, "input", &card.input);
                        }
                        if !card.output.is_empty() {
                            section(ui, "output", &card.output);
                        }
                    }
                });
        })
        .response
        .rect;

    // The whole card toggles expansion.
    let response = ui.interact(outer, ui.id().with(("tool", &card.id)), Sense::click());
    let clicked = response.clicked();
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if clicked {
        toggled = true;
    }
    toggled
}

fn section(ui: &mut Ui, label: &str, body: &str) {
    let p = palette(ui);
    ui.label(
        RichText::new(label.to_uppercase())
            .font(theme::caption_font())
            .color(p.muted),
    );
    egui::ScrollArea::vertical()
        .id_salt(ui.id().with(label))
        .max_height(theme::TOOL_OUTPUT_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Frame::new()
                .fill(p.surface_2)
                .corner_radius(theme::chip_radius())
                .inner_margin(theme::margin(theme::SPACE_8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.add(
                        egui::Label::new(RichText::new(body).font(theme::mono_font())).wrap(),
                    );
                });
        });
}

fn status_glyph(ui: &mut Ui, status: ToolStatus, color: egui::Color32) {
    match status {
        ToolStatus::InProgress | ToolStatus::Pending => icons::working_dots(ui, color),
        ToolStatus::Completed => {
            icons::paint(ui, icons::Icon::Check, color);
        }
        ToolStatus::Failed => {
            icons::paint(ui, icons::Icon::Cross, color);
        }
    }
}

fn palette(ui: &Ui) -> theme::Palette {
    let light = !ui.visuals().dark_mode;
    if light {
        theme::LIGHT
    } else {
        theme::DARK
    }
}

/// First line of a summary, trimmed to fit one row.
fn one_line(text: &str) -> String {
    let mut line = text.lines().next().unwrap_or("").to_owned();
    if line.chars().count() > 60 {
        let cut: String = line.chars().take(60).collect();
        line = format!("{cut}…");
    }
    line
}
