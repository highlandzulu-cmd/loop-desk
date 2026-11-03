//! Grid drawing: the `vt100` screen rendered as a grid of styled cells, plus
//! the cursor.

use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};

use crate::palette::{resolve_color, TermTheme};
use crate::pty::SharedParser;

#[derive(Clone, Copy, PartialEq, Eq)]
struct CellStyle {
    color: Color32,
    italic: bool,
    underline: bool,
}

/// Renders the visible grid: selection and backgrounds first, then one text
/// job per run of equal style.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_grid(
    painter: &egui::Painter,
    parser: &SharedParser,
    rect: Rect,
    theme: &TermTheme,
    font_id: &FontId,
    cell_w: f32,
    cell_h: f32,
    selection: Option<((u16, u16), (u16, u16))>,
) {
    let guard = parser.lock();
    let screen = guard.screen();
    let (rows, cols) = screen.size();

    // Selection highlight (drawn under the text).
    if let Some(((r1, c1), (r2, c2))) = selection {
        for row in r1..=r2.min(rows.saturating_sub(1)) {
            let y = rect.min.y + f32::from(row) * cell_h;
            let start = if row == r1 { c1 } else { 0 };
            let end = if row == r2 { c2 + 1 } else { cols };
            let x = rect.min.x + f32::from(start) * cell_w;
            let w = f32::from(end.saturating_sub(start)) * cell_w;
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, cell_h)),
                0.0,
                theme.selection,
            );
        }
    }

    // Background colours: one rect per run of equal background.
    for row in 0..rows {
        let y = rect.min.y + f32::from(row) * cell_h;
        if y + cell_h > rect.max.y {
            break;
        }
        let mut col = 0u16;
        while col < cols {
            let Some(cell) = screen.cell(row, col) else {
                col += 1;
                continue;
            };
            let bg = cell_bg(cell, theme);
            let start = col;
            let mut width = 1.0f32;
            col += 1;
            while col < cols {
                match screen.cell(row, col) {
                    Some(next)
                        if !next.is_wide_continuation() && cell_bg(next, theme) == bg =>
                    {
                        width += if next.is_wide() { 2.0 } else { 1.0 };
                        col += 1;
                    }
                    _ => break,
                }
            }
            if bg != theme.bg {
                let x = rect.min.x + f32::from(start) * cell_w;
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(x, y), Vec2::new(width * cell_w, cell_h)),
                    0.0,
                    bg,
                );
            }
        }
    }

    // Text runs: consecutive cells with identical style become one job.
    for row in 0..rows {
        let y = rect.min.y + f32::from(row) * cell_h;
        if y + cell_h > rect.max.y {
            break;
        }
        let mut col = 0u16;
        while col < cols {
            let Some(cell) = screen.cell(row, col) else {
                col += 1;
                continue;
            };
            if cell.is_wide_continuation() || !cell.has_contents() {
                col += 1;
                continue;
            }
            let style = cell_style(cell, theme);
            let start = col;
            let mut text = String::new();
            let mut advance = 0.0f32;
            while col < cols {
                match screen.cell(row, col) {
                    Some(next)
                        if !next.is_wide_continuation()
                            && next.has_contents()
                            && cell_style(next, theme) == style =>
                    {
                        text.push_str(&next.contents());
                        advance += if next.is_wide() { 2.0 } else { 1.0 };
                        col += 1;
                    }
                    _ => break,
                }
            }
            if text.is_empty() {
                col += 1;
                continue;
            }
            let x = rect.min.x + f32::from(start) * cell_w;
            let layout =
                painter.layout_job(text_job(text, style, font_id.clone(), advance * cell_w));
            painter.galley(Pos2::new(x, y), layout, style.color);
        }
    }
}

/// Draws the block cursor (hollow when the terminal is not focused).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_cursor(
    painter: &egui::Painter,
    parser: &SharedParser,
    rect: Rect,
    theme: &TermTheme,
    cell_w: f32,
    cell_h: f32,
    focused: bool,
    scroll: usize,
) {
    let guard = parser.lock();
    let screen = guard.screen();
    if screen.hide_cursor() || scroll != 0 {
        return;
    }
    let (row, col) = screen.cursor_position();
    let cell_rect = Rect::from_min_size(
        Pos2::new(
            rect.min.x + f32::from(col) * cell_w,
            rect.min.y + f32::from(row) * cell_h,
        ),
        Vec2::new(cell_w, cell_h),
    );
    if cell_rect.right() > rect.right() || cell_rect.bottom() > rect.bottom() {
        return;
    }
    if focused {
        painter.rect_filled(cell_rect, 0.0, theme.cursor);
    } else {
        painter.rect_stroke(
            cell_rect.expand(0.5),
            0.0,
            Stroke::new(1.5_f32, theme.cursor),
            egui::StrokeKind::Inside,
        );
    }
}

/// Builds a single-line [`egui::text::LayoutJob`] for a run of cells.
fn text_job(text: String, style: CellStyle, font_id: FontId, width: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple(text, font_id, style.color, f32::INFINITY);
    job.wrap = egui::text::TextWrapping {
        max_width: width + 1.0,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: None,
    };
    if let Some(section) = job.sections.first_mut() {
        section.format.italics = style.italic;
        section.format.underline = if style.underline {
            Stroke::new(1.0_f32, style.color)
        } else {
            Stroke::NONE
        };
    }
    job
}

fn cell_style(cell: &vt100::Cell, theme: &TermTheme) -> CellStyle {
    let fg = resolve_color(cell.fgcolor(), theme, theme.fg);
    let bg = cell_bg(cell, theme);
    let color = if cell.inverse() { bg } else { fg };
    CellStyle {
        color: if cell.bold() {
            lerp(color, Color32::WHITE, 0.18)
        } else {
            color
        },
        italic: cell.italic(),
        underline: cell.underline(),
    }
}

fn cell_bg(cell: &vt100::Cell, theme: &TermTheme) -> Color32 {
    if cell.inverse() {
        resolve_color(cell.fgcolor(), theme, theme.fg)
    } else {
        resolve_color(cell.bgcolor(), theme, theme.bg)
    }
}

/// Darkens (or lightens) a colour towards grey by `factor`.
pub(crate) fn dim(color: Color32, factor: f32) -> Color32 {
    let mix = |c: u8| (f32::from(c) * (1.0 - factor) + 40.0 * factor).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(mix(color.r()), mix(color.g()), mix(color.b()))
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let ch = |x: u8, y: u8| {
        let (x, y) = (f32::from(x), f32::from(y));
        (x + (y - x) * t).round().clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}
