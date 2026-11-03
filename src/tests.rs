//! Tests for the pure, window-free parts: state merging, markdown block
//! parsing, the demo fixture and CLI parsing.

use crate::demo;
use crate::state::{
    AppState, ChatRow, NoticeLevel, SessionEntry, ToolCard, ToolKind, ToolStatus,
};
use crate::ui::markdown;

#[test]
fn upsert_row_replaces_by_key() {
    let mut state = AppState::default();
    state.upsert_row(ChatRow::User {
        id: "u1".into(),
        text: "hello".into(),
    });
    state.upsert_row(ChatRow::User {
        id: "u1".into(),
        text: "hello again".into(),
    });
    assert_eq!(state.rows.len(), 1);
    assert_eq!(
        state.rows[0],
        ChatRow::User {
            id: "u1".into(),
            text: "hello again".into()
        }
    );
}

#[test]
fn append_text_merges_streamed_deltas() {
    let mut state = AppState::default();
    state.append_text("m1", "Hel", true);
    state.append_text("m1", "lo", true);
    assert_eq!(
        state.rows.last(),
        Some(&ChatRow::Agent {
            id: "m1".into(),
            text: "Hello".into(),
            done: false,
        })
    );
    state.finish_messages();
    assert!(matches!(state.rows.last(), Some(ChatRow::Agent { done: true, .. })));
}

#[test]
fn title_from_prompt_is_short_and_single_line() {
    assert_eq!(AppState::title_from_prompt("\n\n  hi there  \nmore"), "hi there");
    let long = "x".repeat(80);
    let title = AppState::title_from_prompt(&long);
    assert!(title.chars().count() <= 43);
    assert!(title.ends_with('…'));
}

#[test]
fn tool_card_updates_merge_without_losing_fields() {
    let mut state = AppState::default();
    state.upsert_row(ChatRow::Tool(ToolCard {
        id: "t1".into(),
        name: Some("edit".into()),
        title: "edit src/main.rs".into(),
        kind: ToolKind::Edit,
        status: ToolStatus::InProgress,
        output: String::new(),
        input: "{}".into(),
        diff: None,
    }));
    // A partial update with only content and an end status.
    state.upsert_row(ChatRow::Tool(ToolCard {
        id: "t1".into(),
        name: None,
        title: String::new(),
        kind: ToolKind::Other,
        status: ToolStatus::Completed,
        output: "done".into(),
        input: String::new(),
        diff: None,
    }));
    match &state.rows[0] {
        ChatRow::Tool(card) => {
            assert_eq!(card.name.as_deref(), Some("edit"));
            assert_eq!(card.title, "edit src/main.rs");
            assert_eq!(card.kind, ToolKind::Edit);
            assert_eq!(card.status, ToolStatus::Completed);
            assert_eq!(card.output, "done");
        }
        other => panic!("expected a tool card, got {other:?}"),
    }
}

#[test]
fn day_groups_use_local_dates() {
    let now = chrono::Local::now();
    let today_ms = now.timestamp_millis();
    let yesterday_ms = (now - chrono::Duration::days(1)).timestamp_millis();
    let old_ms = (now - chrono::Duration::days(9)).timestamp_millis();
    let today = now.date_naive();
    let entry = |ms| SessionEntry {
        id: "s".into(),
        title: "t".into(),
        created_at: ms,
        busy: false,
    };
    assert_eq!(entry(today_ms).day_group(today), "Today");
    assert_eq!(entry(yesterday_ms).day_group(today), "Yesterday");
    assert_eq!(entry(old_ms).day_group(today), "Earlier");
}

#[test]
fn markdown_parse_handles_core_blocks() {
    let blocks = markdown::parse_for_test(
        "# Title\n\ntext here\n\n- one\n- two\n\n1. first\n\n> quoted\n\n```rust\nfn main() {}\n```\n",
    );
    let kinds: Vec<&str> = blocks.iter().map(|b| b.kind_name.as_str()).collect();
    assert_eq!(
        kinds,
        vec!["heading", "paragraph", "list", "ordered", "quote", "code"]
    );
}

#[test]
fn markdown_parse_keeps_code_fence_open_to_the_end() {
    let blocks = markdown::parse_for_test("before\n```py\nprint(1)\n");
    // An unclosed fence still yields its code block.
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].kind_name, "code");
}

#[test]
fn demo_state_exercises_every_row_type() {
    let state = demo::demo_state();
    assert!(state.rows.iter().any(|r| matches!(r, ChatRow::User { .. })));
    assert!(state
        .rows
        .iter()
        .any(|r| matches!(r, ChatRow::Permission { .. })));
    assert!(state.rows.iter().any(|r| matches!(r, ChatRow::Agent { .. })));
    assert!(state.rows.iter().any(|r| matches!(r, ChatRow::Thought { .. })));
    assert!(state
        .rows
        .iter()
        .any(|r| matches!(r, ChatRow::Tool(c) if c.status == ToolStatus::Failed)));
    assert!(state.rows.iter().any(|r| matches!(
        r,
        ChatRow::Tool(c) if c.diff.is_some()
    )));
    assert!(state.rows.iter().any(
        |r| matches!(r, ChatRow::Notice { level: NoticeLevel::Info, .. })
    ));
    assert!(state.sessions.len() >= 3);
}

/// Lay out one collapsed tool card the way the chat column does and measure
/// how much vertical space it takes.
#[test]
fn collapsed_tool_card_is_compact() {
    let card = ToolCard {
        id: "t".into(),
        name: Some("read".into()),
        title: "src/ui/sidebar.rs".into(),
        kind: ToolKind::Read,
        status: ToolStatus::Completed,
        output: "line\nline\nline".into(),
        input: "{}".into(),
        diff: None,
    };
    let ctx = egui::Context::default();
    let mut measured = f32::INFINITY;
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width(1000.0);
            ui.set_height(800.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // The chat column is a content-sized child UI, never one
                    // that asks for the remaining height.
                    ui.scope(|ui| {
                        ui.set_max_width(760.0);
                        let top = ui.min_rect().bottom();
                        crate::ui::tool_card::show(ui, &card, false);
                        measured = ui.min_rect().bottom() - top;
                    });
                });
        });
    });
    assert!(
        measured < 64.0,
        "collapsed tool card must stay compact, was {measured}px"
    );
}

/// A pending approval becomes the state's prompt; answering it resolves the
/// card and clears the prompt.
#[test]
fn approvals_flow_from_row_to_decision() {
    use crate::state::{ChatRow, PermissionOutcome, PermissionPrompt};

    let mut state = AppState::default();
    state.upsert_row(ChatRow::Permission(PermissionPrompt {
        id: "approval-0".into(),
        kind: "bash".into(),
        tool_name: "bash".into(),
        summary: "cargo build".into(),
        detail: "/tmp/proj".into(),
        resolved: false,
        outcome: None,
    }));
    assert!(state.pending_approval.is_some());

    state.resolve_approval("approval-0", PermissionOutcome::AllowSession);
    assert!(state.pending_approval.is_none());
    match state.rows.last() {
        Some(ChatRow::Permission(prompt)) => {
            assert!(prompt.resolved);
            assert_eq!(prompt.outcome, Some(PermissionOutcome::AllowSession));
        }
        other => panic!("expected a permission row, got {other:?}"),
    }

    // Answering an unknown id does nothing.
    state.resolve_approval("approval-9", PermissionOutcome::Deny);
    assert!(state.pending_approval.is_none());
}

/// Tool diffs feed the changed-file list, one entry per path with the newest
/// diff last-writer-wins.
#[test]
fn changed_files_track_latest_diff_per_path() {
    use crate::state::{ChangedFile, DiffContent};

    let mut state = AppState::default();
    state.record_change(
        "src/a.rs".into(),
        "@@ -1 +1 @@\n-old\n+new\n".into(),
    );
    state.record_change(
        "src/b.rs".into(),
        "@@ -1,2 +1,3 @@\n ctx\n-gone\n ctx\n+added\n+more\n".into(),
    );
    assert_eq!(
        state.changed_files,
        vec![
            ChangedFile {
                path: "src/b.rs".into(),
                added: 2,
                removed: 1,
                diff: "@@ -1,2 +1,3 @@\n ctx\n-gone\n ctx\n+added\n+more\n".into(),
            },
            ChangedFile {
                path: "src/a.rs".into(),
                added: 1,
                removed: 1,
                diff: "@@ -1 +1 @@\n-old\n+new\n".into(),
            },
        ]
    );

    // A second change to the same path replaces its entry.
    state.record_change("src/a.rs".into(), "@@ -2 +2 @@\n-x\n+y\n".into());
    assert_eq!(state.changed_files.len(), 2);
    assert_eq!(state.changed_files[0].path, "src/a.rs");

    // Loading a session rebuilds the list from its history.
    let rows = vec![crate::state::ChatRow::Tool(ToolCard {
        id: "t1".into(),
        name: Some("edit".into()),
        title: "edit src/c.rs".into(),
        kind: ToolKind::Edit,
        status: ToolStatus::Completed,
        output: String::new(),
        input: String::new(),
        diff: Some(DiffContent {
            path: "src/c.rs".into(),
            diff: "+c".into(),
        }),
    })];
    state.reset_conversation("s2".into(), "title".into(), rows);
    assert_eq!(state.changed_files.len(), 1);
    assert_eq!(state.changed_files[0].path, "src/c.rs");
    assert_eq!(state.changed_files[0].added, 1);
}

/// The permission mode labels stay stable for the engine round trip.
#[test]
fn permission_mode_labels_round_trip() {
    use crate::state::PermissionMode;
    assert_eq!(PermissionMode::default(), PermissionMode::Ask);
    for mode in PermissionMode::ALL {
        assert_eq!(PermissionMode::from_str(mode.as_str()), Some(mode));
        assert!(!mode.label().is_empty());
        assert!(!mode.description().is_empty());
    }
}

#[test]
fn cli_parsing_defaults_and_overrides() {    let base = vec![
        "--demo".to_owned(),
        "--screenshot".to_owned(),
        "out.png".to_owned(),
    ];
    let opts = demo::demo_options_from(&base).expect("demo args parse");
    assert_eq!(opts.out_path, "out.png");
    assert!(!opts.dark);
    assert_eq!(opts.width, 1200.0);
    assert_eq!(opts.height, 800.0);

    let full = vec![
        "--demo".to_owned(),
        "--screenshot".to_owned(),
        "dark.png".to_owned(),
        "--dark".to_owned(),
        "--width".to_owned(),
        "900".to_owned(),
        "--height".to_owned(),
        "600".to_owned(),
    ];
    let opts = demo::demo_options_from(&full).expect("full args parse");
    assert!(opts.dark);
    assert_eq!(opts.width, 900.0);
    assert_eq!(opts.height, 600.0);

    assert!(demo::demo_options_from(&["--help".to_owned()]).is_none());
}
