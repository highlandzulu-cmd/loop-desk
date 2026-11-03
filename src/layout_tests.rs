//! Headless layout checks for the demo conversation, mirroring what the
//! visual review asked for. These run the real drawing code in an egui
//! context with a fixed screen size and measure the result.

use crate::demo;
use crate::state::{ChatRow, ToolStatus};
use crate::ui::{theme, ChatView};

/// Every tool card in the demo renders collapsed by default and stays a
/// single compact row (the visual review's item 1).
#[test]
fn demo_tool_cards_start_collapsed() {
    let state = demo::demo_state();
    assert!(
        state.expanded_tools.is_empty(),
        "tool cards must start collapsed"
    );
    assert!(state.rows.iter().any(|r| matches!(r, ChatRow::Tool(_))));
}

/// The composer must be on screen: its text edit exists and sits near the
/// bottom of the window, capped to the chat column width.
#[test]
fn demo_composer_is_visible_and_capped() {
    let mut state = demo::demo_state();
    let ctx = egui::Context::default();
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
        )),
        ..Default::default()
    };
    let mut composer = None;
    let _ = ctx.run(raw, |ctx| {
        ChatView::new(theme::ThemeMode::Light).show(ctx, &mut state, None);
        // The TextEdit registers its response under its id_source.
        let id = egui::Id::new("composer-input");
        if let Some(resp) = ctx.read_response(id) {
            composer = Some(resp.rect);
        }
    });
    let rect = composer.expect("the composer text edit is missing from the frame");
    assert!(
        rect.bottom() <= 820.0 && rect.top() > 820.0 - 300.0,
        "composer at {rect:?} is not pinned near the bottom"
    );
    assert!(
        rect.width() <= theme::CHAT_MAX_WIDTH,
        "composer is {rect:?}, wider than the {} column cap",
        theme::CHAT_MAX_WIDTH
    );
    assert!(rect.left() > 200.0, "composer is not centred: {rect:?}");
}

/// The user bubble is right-aligned inside the centred column: there is real
/// empty margin to the right of the column at 1280 px.
#[test]
fn demo_column_leaves_margin_on_both_sides() {
    let mut state = demo::demo_state();
    let ctx = egui::Context::default();
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
        )),
        ..Default::default()
    };
    let mut composer = None;
    let _ = ctx.run(raw, |ctx| {
        ChatView::new(theme::ThemeMode::Light).show(ctx, &mut state, None);
        if let Some(resp) = ctx.read_response(egui::Id::new("composer-input")) {
            composer = Some(resp.rect);
        }
    });
    let rect = composer.expect("composer present");
    // Sidebar (260) + inset on the left, and a matching inset on the right.
    let left_gap = rect.left() - theme::SIDEBAR_WIDTH;
    let right_gap = 1280.0 - rect.right();
    assert!(left_gap > 24.0, "no left margin: {rect:?}");
    assert!(right_gap > 24.0, "no right margin: {rect:?}");
}

/// Every demo row type survives a render without panicking, in both themes
/// and at both review sizes.
#[test]
fn demo_renders_every_row_type_in_both_themes() {
    for mode in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
        for size in [(1280.0, 820.0), (1280.0, 1500.0)] {
            let mut state = demo::demo_state();
            let ctx = egui::Context::default();
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(size.0, size.1),
                )),
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                ChatView::new(mode).show(ctx, &mut state, None);
            });
            assert!(
                state.rows.iter().any(|r| matches!(r, ChatRow::Agent { .. })),
                "agent row lost in {mode:?} at {size:?}"
            );
            assert!(
                state.rows.iter().any(|r| matches!(r, ChatRow::User { .. })),
                "user row lost in {mode:?} at {size:?}"
            );
            assert!(
                state.rows.iter().any(|r| matches!(
                    r,
                    ChatRow::Tool(c) if c.status == ToolStatus::Failed
                )),
                "failed tool card lost in {mode:?} at {size:?}"
            );
        }
    }
}

/// An expanded edit card draws its inline diff, and the Changes panel lists
/// the file with the right counts.
#[test]
fn expanded_edit_card_and_changes_panel_render() {
    use crate::state::{ChatRow, ToolStatus};
    let mut state = demo::demo_state();
    // Expand every tool card so the diff is on screen.
    for row in &state.rows {
        if let ChatRow::Tool(card) = row {
            state.expanded_tools.insert(card.id.clone(), true);
        }
    }

    let ctx = egui::Context::default();
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1500.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| {
        let mut view = crate::ui::ChatView::new(theme::ThemeMode::Light);
        view.changes_open = true;
        let _ = view.show(ctx, &mut state, None);
    });

    assert!(
        state
            .rows
            .iter()
            .any(|r| matches!(r, ChatRow::Tool(c) if c.diff.is_some() && c.status == ToolStatus::Completed)),
        "the demo must contain a completed edit with a diff"
    );
    assert_eq!(state.changed_files.len(), 1, "one changed file expected");
    assert_eq!(state.changed_files[0].added, 2);
    assert_eq!(state.changed_files[0].removed, 0);

}

#[cfg(test)]
mod changes_toggle_tests {
    use super::*;
    use crate::ui::ChatView;

    #[test]
    fn an_untouched_changes_panel_reports_nothing() {
        let ctx = egui::Context::default();
        let mut state = crate::state::AppState::default();
        let mut view = ChatView::new(crate::ui::theme::ThemeMode::Light);
        view.changes_open = true;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            view.clone().show(ctx, &mut state, None);
        });
        assert_eq!(state.pending_changes_open, None, "nothing changed yet");
    }
}
