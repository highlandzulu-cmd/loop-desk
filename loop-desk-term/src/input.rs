//! Keyboard and mouse input for the terminal view.

use egui::{Pos2, Rect};

use crate::keys;
use crate::pty::TermTab;

/// Consumes keyboard/mouse events for the terminal.
pub(crate) fn handle_input(
    tab: &mut TermTab,
    response: &egui::Response,
    rect: Rect,
    cell_w: f32,
    cell_h: f32,
) {
    // Mouse wheel scrolls the scrollback.
    let wheel = response.ctx.input(|i| i.smooth_scroll_delta);
    if wheel.y.abs() > 0.0 {
        let lines = (-wheel.y / cell_h).round() as isize;
        if lines != 0 {
            let next = tab.scroll_offset() as isize + lines;
            tab.set_scroll_offset(next.max(0) as usize);
        }
    }

    // Mouse selection (drag to select).
    handle_selection(tab, response, rect, cell_w, cell_h);

    if !response.has_focus() {
        return;
    }

    let (app_cursor, bracketed) = {
        let guard = tab.parser().lock();
        let screen = guard.screen();
        (screen.application_cursor(), screen.bracketed_paste())
    };

    let events: Vec<egui::Event> = response.ctx.input(|i| i.events.to_vec());

    for event in events {
        match event {
            egui::Event::Text(text) => {
                if tab.exited() {
                    continue;
                }
                if let Some(bytes) = keys::text_to_bytes(&text, egui::Modifiers::NONE) {
                    tab.write(&bytes);
                    tab.snap_to_bottom();
                }
            }
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if modifiers.command || modifiers.mac_cmd {
                    continue; // application shortcuts (Cmd+T, Cmd+W, ...)
                }
                if tab.exited() {
                    if key == egui::Key::Enter {
                        let ctx = response.ctx.clone();
                        let _ = tab.restart(&ctx);
                    }
                    continue;
                }
                if let Some(bytes) = keys::key_to_bytes(key, modifiers, app_cursor) {
                    tab.write(&bytes);
                    tab.snap_to_bottom();
                }
            }
            egui::Event::Paste(text) => {
                if tab.exited() {
                    continue;
                }
                let bytes = keys::paste_bytes(&text, bracketed);
                tab.write(&bytes);
                tab.snap_to_bottom();
            }
            egui::Event::Copy => {
                if let Some(text) = tab.selected_text() {
                    if !text.is_empty() {
                        response.ctx.copy_text(text);
                    }
                }
            }
            _ => {}
        }
    }
}

fn handle_selection(
    tab: &mut TermTab,
    response: &egui::Response,
    rect: Rect,
    cell_w: f32,
    cell_h: f32,
) {
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    let primary = egui::PointerButton::Primary;
    if response.drag_started_by(primary) || response.dragged_by(primary) {
        let (row, col) = pos_to_cell(pos, rect, cell_w, cell_h);
        let anchor_id = response.id.with("sel-anchor");
        let anchor = response
            .ctx
            .memory(|m| m.data.get_temp::<(u16, u16)>(anchor_id))
            .unwrap_or((row, col));
        response
            .ctx
            .memory_mut(|m| m.data.insert_temp(anchor_id, anchor));
        tab.set_selection(Some((anchor, (row, col))));
    } else if response.clicked() {
        tab.set_selection(None);
        response
            .ctx
            .memory_mut(|m| m.data.remove_temp::<(u16, u16)>(response.id.with("sel-anchor")));
    }
}

fn pos_to_cell(pos: Pos2, rect: Rect, cell_w: f32, cell_h: f32) -> (u16, u16) {
    let col = (((pos.x - rect.min.x) / cell_w).floor().max(0.0)) as u16;
    let row = (((pos.y - rect.min.y) / cell_h).floor().max(0.0)) as u16;
    (row, col)
}
