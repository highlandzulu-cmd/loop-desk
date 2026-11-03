//! Icons drawn from simple shapes. No icon font and no special Unicode glyphs, so they look the
//! same everywhere (the default font has no glyphs for most symbols).

use egui::{vec2, Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui};

use super::theme;
use crate::state::ToolKind;

/// Every icon the app draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Read,
    Edit,
    Delete,
    Move,
    Search,
    Execute,
    Fetch,
    Think,
    Other,
    ChevronDown,
    ChevronRight,
    Copy,
    Check,
    Cross,
    Dots,
    Sun,
    Moon,
}

/// Icon for a tool kind.
pub fn tool(kind: ToolKind) -> Icon {
    match kind {
        ToolKind::Read => Icon::Read,
        ToolKind::Edit => Icon::Edit,
        ToolKind::Delete => Icon::Delete,
        ToolKind::Move => Icon::Move,
        ToolKind::Search => Icon::Search,
        ToolKind::Execute => Icon::Execute,
        ToolKind::Fetch => Icon::Fetch,
        ToolKind::Think => Icon::Think,
        ToolKind::Other => Icon::Other,
    }
}

/// Allocate an icon-sized square and draw `icon` in it.
pub fn paint(ui: &mut Ui, icon: Icon, color: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(theme::ICON_SIZE, theme::ICON_SIZE), Sense::hover());
    draw(ui.painter(), rect, icon, color);
    response
}

/// Same as [`paint`] but the square reacts to clicks and shows a pointing hand.
pub fn button(ui: &mut Ui, icon: Icon, color: Color32) -> Response {
    let size = theme::ICON_SIZE + 6.0;
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let color = if response.hovered() { ui.visuals().strong_text_color() } else { color };
    let inner = Rect::from_center_size(rect.center(), vec2(theme::ICON_SIZE, theme::ICON_SIZE));
    draw(ui.painter(), inner, icon, color);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Draw `icon` inside `rect` with `color`.
pub fn draw(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let stroke = Stroke::new(1.4_f32, color);
    // Point at fractional position (0..1) of the rect.
    let p = |x: f32, y: f32| Pos2::new(rect.min.x + x * rect.width(), rect.min.y + y * rect.height());
    let line = |a: Pos2, b: Pos2| painter.line_segment([a, b], stroke);
    let poly = |pts: Vec<Pos2>| painter.add(Shape::line(pts, stroke));
    let r = rect.width();
    match icon {
        Icon::Read => {
            painter.rect_stroke(Rect::from_min_max(p(0.2, 0.08), p(0.8, 0.92)), CornerRadius::same(2), stroke, StrokeKind::Inside);
            line(p(0.34, 0.34), p(0.66, 0.34));
            line(p(0.34, 0.52), p(0.66, 0.52));
            line(p(0.34, 0.70), p(0.55, 0.70));
        }
        Icon::Edit => {
            line(p(0.2, 0.8), p(0.78, 0.22));
            line(p(0.26, 0.86), p(0.84, 0.28));
            line(p(0.78, 0.22), p(0.84, 0.28));
            line(p(0.12, 0.92), p(0.32, 0.92));
        }
        Icon::Delete => {
            line(p(0.2, 0.28), p(0.8, 0.28));
            line(p(0.4, 0.28), p(0.4, 0.14));
            line(p(0.4, 0.14), p(0.6, 0.14));
            line(p(0.6, 0.14), p(0.6, 0.28));
            poly(vec![p(0.27, 0.28), p(0.32, 0.9), p(0.68, 0.9), p(0.73, 0.28)]);
        }
        Icon::Move => {
            line(p(0.1, 0.35), p(0.9, 0.35));
            poly(vec![p(0.72, 0.18), p(0.9, 0.35), p(0.72, 0.52)]);
            line(p(0.1, 0.68), p(0.9, 0.68));
            poly(vec![p(0.28, 0.51), p(0.1, 0.68), p(0.28, 0.85)]);
        }
        Icon::Search => {
            painter.circle_stroke(p(0.44, 0.44), r * 0.28, stroke);
            line(p(0.65, 0.65), p(0.9, 0.9));
        }
        Icon::Execute => {
            poly(vec![p(0.12, 0.25), p(0.42, 0.5), p(0.12, 0.75)]);
            line(p(0.52, 0.78), p(0.88, 0.78));
        }
        Icon::Fetch => {
            painter.circle_stroke(p(0.5, 0.5), r * 0.4, stroke);
            line(p(0.1, 0.5), p(0.9, 0.5));
            line(p(0.5, 0.1), p(0.5, 0.9));
        }
        Icon::Think => {
            line(p(0.5, 0.08), p(0.5, 0.92));
            line(p(0.08, 0.5), p(0.92, 0.5));
            line(p(0.22, 0.22), p(0.78, 0.78));
            line(p(0.78, 0.22), p(0.22, 0.78));
        }
        Icon::Other => {
            painter.circle_filled(p(0.5, 0.5), r * 0.14, color);
        }
        Icon::ChevronDown => {
            poly(vec![p(0.22, 0.38), p(0.5, 0.66), p(0.78, 0.38)]);
        }
        Icon::ChevronRight => {
            poly(vec![p(0.38, 0.22), p(0.66, 0.5), p(0.38, 0.78)]);
        }
        Icon::Copy => {
            painter.rect_stroke(Rect::from_min_max(p(0.3, 0.3), p(0.9, 0.92)), CornerRadius::same(2), stroke, StrokeKind::Inside);
            poly(vec![p(0.12, 0.7), p(0.12, 0.08), p(0.66, 0.08)]);
        }
        Icon::Check => {
            poly(vec![p(0.14, 0.54), p(0.4, 0.78), p(0.88, 0.24)]);
        }
        Icon::Dots => {
            for x in [0.2, 0.5, 0.8] {
                painter.circle_filled(p(x, 0.5), r * 0.09, color);
            }
        }
        Icon::Sun => {
            painter.circle_stroke(p(0.5, 0.5), r * 0.2, stroke);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let (dx, dy) = (a.cos(), a.sin());
                line(p(0.5 + dx * 0.33, 0.5 + dy * 0.33), p(0.5 + dx * 0.46, 0.5 + dy * 0.46));
            }
        }
        Icon::Moon => {
            // A crescent: the long arc of a circle, closed by a smaller arc bulging inward.
            let arc = |cx: f32, cy: f32, rad: f32, from: f32, to: f32| -> Vec<Pos2> {
                (0..=16)
                    .map(|i| {
                        let a = from + (to - from) * i as f32 / 16.0;
                        p(cx + rad * a.cos(), cy + rad * a.sin())
                    })
                    .collect()
            };
            let mut pts = arc(0.5, 0.5, 0.38, 0.9, 5.4);
            pts.extend(arc(0.66, 0.4, 0.32, 5.0, 1.2));
            poly(pts);
        }
        Icon::Cross => {
            line(p(0.2, 0.2), p(0.8, 0.8));
            line(p(0.8, 0.2), p(0.2, 0.8));
        }
    }
}

/// Three pulsing dots, for "working" states. Requests repaints while visible.
pub fn working_dots(ui: &mut Ui, color: Color32) {
    let time = ui.ctx().input(|i| i.time);
    let (rect, _) = ui.allocate_exact_size(vec2(26.0, theme::ICON_SIZE), Sense::hover());
    for i in 0..3 {
        let phase = ((time * 2.4 - i as f64 * 0.33).rem_euclid(1.0) * std::f64::consts::TAU).sin() as f32;
        let alpha = (0.35 + 0.65 * (phase * 0.5 + 0.5)).clamp(0.0, 1.0);
        let c = color.gamma_multiply(alpha);
        ui.painter().circle_filled(Pos2::new(rect.min.x + 4.0 + i as f32 * 9.0, rect.center().y), 2.4, c);
    }
    ui.ctx().request_repaint();
}
