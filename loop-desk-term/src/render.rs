//! Rendering: the tab strip, the terminal view and input handling.

use egui::{Align2, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::grid::{dim, draw_cursor, draw_grid};
use crate::input::handle_input;
use crate::palette::TermTheme;
use crate::TerminalPanel;

/// Extra vertical padding between font rows (fraction of the row height).
const LINE_SPACING: f32 = 0.15;
/// Height of the tab strip.
const STRIP_HEIGHT: f32 = 22.0;

// ---------------------------------------------------------------------------
// Tab strip
// ---------------------------------------------------------------------------

/// Draws the tab strip at the top of the panel.
pub fn tab_strip(panel: &mut TerminalPanel, ui: &mut Ui, theme: &TermTheme) {
    let desired = Vec2::new(ui.available_width(), STRIP_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 0.0, theme.bg);
    painter.hline(
        rect.left()..=rect.right(),
        rect.bottom(),
        Stroke::new(1.0_f32, dim(theme.fg, 0.25)),
    );

    // "+" button on the far right.
    let plus_rect = Rect::from_min_max(
        Pos2::new(rect.right() - STRIP_HEIGHT, rect.min.y),
        Pos2::new(rect.right(), rect.max.y),
    );
    let plus_id = ui.id().with("term-new-tab");
    let plus_resp = ui.interact(plus_rect, plus_id, Sense::click());
    painter.text(
        plus_rect.center(),
        Align2::CENTER_CENTER,
        "+",
        FontId::proportional(theme.font_size),
        if plus_resp.hovered() {
            theme.cursor
        } else {
            theme.fg
        },
    );
    if plus_resp.clicked() {
        let _ = panel.new_tab();
    }

    let mut close_idx: Option<usize> = None;
    let mut select_idx: Option<usize> = None;
    let active = panel.active_tab_index();

    let mut x = rect.min.x;
    for (idx, title) in panel.tab_titles().into_iter().enumerate() {
        let galley = painter.layout_no_wrap(
            format!(" {title} "),
            FontId::proportional(theme.font_size * 0.85),
            theme.fg,
        );
        let size = Vec2::new(galley.size().x + STRIP_HEIGHT, STRIP_HEIGHT);
        let tab_rect = Rect::from_min_size(Pos2::new(x, rect.min.y), size);
        if tab_rect.right() > plus_rect.left() {
            break; // out of room
        }

        let is_active = idx == active;
        let id = ui.id().with("term-tab").with(idx);
        let resp = ui.interact(tab_rect, id, Sense::click());
        if resp.hovered() || is_active {
            painter.rect_filled(
                tab_rect,
                4.0,
                dim(theme.fg, if is_active { 0.18 } else { 0.08 }),
            );
        }
        painter.galley(
            Pos2::new(
                tab_rect.min.x + 4.0,
                tab_rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            theme.fg,
        );

        // "x" shown on hover (or for the active tab).
        if resp.hovered() || is_active {
            let close_rect = Rect::from_min_max(
                Pos2::new(tab_rect.right() - STRIP_HEIGHT * 0.8, tab_rect.min.y),
                Pos2::new(tab_rect.right(), tab_rect.max.y),
            );
            let close_id = ui.id().with("term-close-tab").with(idx);
            let close_resp = ui.interact(close_rect, close_id, Sense::click());
            painter.text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                "×",
                FontId::proportional(theme.font_size * 0.8),
                if close_resp.hovered() {
                    theme.cursor
                } else {
                    theme.fg
                },
            );
            if close_resp.clicked() {
                close_idx = Some(idx);
            }
        }

        if resp.clicked() {
            select_idx = Some(idx);
        }
        x = tab_rect.right() + 2.0;
    }

    if let Some(idx) = select_idx {
        panel.select_tab(idx);
    }
    if let Some(idx) = close_idx {
        panel.close_tab_at(idx);
    }
}

// ---------------------------------------------------------------------------
// Terminal
// ---------------------------------------------------------------------------

/// Draws the active terminal and handles its input.
pub fn terminal(panel: &mut TerminalPanel, ui: &mut Ui, theme: &TermTheme) {
    if panel.tab_count() == 0 {
        ui.weak("[no terminal]");
        return;
    }

    let available = ui.available_size();
    let (id, rect) = ui.allocate_space(available);
    let response = ui.interact(rect, id, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.bg);

    // Cell metrics from the monospace font.
    let font_id = FontId::monospace(theme.font_size);
    let (cell_w, cell_h) = cell_metrics(ui, &font_id);
    if cell_w <= 0.0 || cell_h <= 0.0 {
        return;
    }

    let cols = ((rect.width() / cell_w).floor() as u16).max(2);
    let rows = ((rect.height() / cell_h).floor() as u16).max(2);
    if let Some(tab) = panel.active_tab_mut() {
        tab.resize(rows, cols);
    }

    let focused = response.has_focus();
    if !focused && response.clicked() {
        response.request_focus();
    }

    if let Some(tab) = panel.active_tab_mut() {
        handle_input(tab, &response, rect, cell_w, cell_h);
    }

    let (parser, scroll, exited, selection) = {
        let Some(tab) = panel.active_tab_mut() else {
            return;
        };
        let exited = tab.exited();
        (
            tab.parser().clone(),
            tab.scroll_offset(),
            exited,
            tab.selection(),
        )
    };

    draw_grid(
        &painter,
        &parser,
        rect,
        theme,
        &font_id,
        cell_w,
        cell_h,
        selection,
    );
    draw_cursor(
        &painter,
        &parser,
        rect,
        theme,
        cell_w,
        cell_h,
        focused,
        scroll,
    );

    if exited {
        draw_exit_banner(&painter, rect, theme);
    }
}

fn draw_exit_banner(painter: &egui::Painter, rect: Rect, theme: &TermTheme) {
    let text = "[process exited — press Enter for a new shell]";
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::proportional(theme.font_size * 0.9),
        dim(theme.fg, 0.55),
    );
    let pos = Pos2::new(rect.min.x + 6.0, rect.max.y - galley.size().y - 6.0);
    painter.rect_filled(
        Rect::from_min_size(pos, galley.size() + Vec2::splat(8.0)),
        4.0,
        theme.bg,
    );
    painter.galley(pos, galley, theme.fg);
}

fn cell_metrics(ui: &Ui, font_id: &FontId) -> (f32, f32) {
    let ctx = ui.ctx().clone();
    ctx.fonts(|fonts| {
        (
            fonts.glyph_width(font_id, 'M').max(1.0),
            (fonts.row_height(font_id) * (1.0 + LINE_SPACING)).max(1.0),
        )
    })
}
