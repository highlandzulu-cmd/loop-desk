//! The built-in demo conversation and the screenshot mode.
//!
//! Nothing here touches the engine or the network; it only fills an
//! [`AppState`] and draws it with the same code as the live app.

use crate::state::{
    AppState, ChatRow, DiffContent, NoticeLevel, SessionEntry, ToolCard, ToolKind, ToolStatus,
};
use crate::ui::{theme, ChatView};

/// Options for one screenshot.
#[derive(Debug, Clone)]
pub struct DemoOptions {
    /// Where to write the PNG.
    pub out_path: String,
    /// Dark palette when true.
    pub dark: bool,
    /// Window width in px.
    pub width: f32,
    /// Window height in px.
    pub height: f32,
    /// Open the model and sign-in dialog.
    pub dialog: bool,
}

/// Parse `--demo --screenshot out.png [--dark] [--width W] [--height H]`
/// from the command line. Returns `None` when this is not a demo run.
pub fn demo_options_from(args: &[String]) -> Option<DemoOptions> {
    if !args.iter().any(|a| a == "--demo") {
        return None;
    }
    let mut out = "loop-desk.png".to_owned();
    let mut dark = false;
    let dialog = args.iter().any(|a| a == "--dialog");
    let mut width = 1200.0;
    let mut height = 800.0;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--demo" | "--screenshot" => {}
            "--dark" => dark = true,
            "--width" => {
                if let Some(v) = args.get(i + 1).and_then(|v| v.parse().ok()) {
                    width = v;
                }
            }
            "--height" => {
                if let Some(v) = args.get(i + 1).and_then(|v| v.parse().ok()) {
                    height = v;
                }
            }
            other if other.ends_with(".png") => out = other.to_owned(),
            _ => {}
        }
        i += 1;
    }
    Some(DemoOptions {
        out_path: out,
        dark,
        width,
        height,
        dialog,
    })
}

/// Run the app in screenshot mode: render one frame, save it, exit.
///
/// This uses eframe's `__screenshot` support: setting `EFRAME_SCREENSHOT_TO`
/// makes eframe save the second frame's pixels and call `std::process::exit`.
pub fn run_screenshot(opts: DemoOptions) -> anyhow::Result<()> {
    std::env::set_var("EFRAME_SCREENSHOT_TO", &opts.out_path);
    let mode = if opts.dark {
        theme::ThemeMode::Dark
    } else {
        theme::ThemeMode::Light
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([opts.width, opts.height])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    let mut state = demo_state();
    if opts.dialog {
        state.connect.open = true;
        state.current_model = Some("soket/GLM-5.3".to_owned());
        state.models = ["GLM-5.3", "qwen3-8-27b", "glm-5.3-flash"]
            .iter()
            .map(|id| crate::state::ModelEntry { provider: "soket".into(), id: (*id).into() })
            .collect();
    }
    eframe::run_native(
        "loop-desk",
        options,
        Box::new(move |cc| {
            theme::install_fonts(&cc.egui_ctx);
            Ok(Box::new(DemoApp {
                view: ChatView::new(mode),
                state,
            }) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow::anyhow!("screenshot failed: {e}"))
}

/// The eframe app used only by the demo.
struct DemoApp {
    view: ChatView,
    state: AppState,
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Windows fade in; the screenshot is taken on the second pass.
        ctx.style_mut(|st| st.animation_time = 0.0);
        let view = std::mem::replace(&mut self.view, ChatView::new(theme::ThemeMode::Light));
        let _ = view.clone().show(ctx, &mut self.state, None);
        self.view = view;
    }
}

/// A rich, static conversation that exercises every row type.
pub fn demo_state() -> AppState {
    let now = chrono::Local::now().timestamp_millis();
    let minute = 60_000;
    let mut state = AppState {
        project_dir: "/Users/demo/projects/loop-desk".to_owned(),
        active_session: "demo-1".to_owned(),
        session_title: "Add a session list to the sidebar".to_owned(),
        model_label: Some("GLM-5.3".to_owned()),
        working: true,
        auto_scroll: false,
        permission_mode: crate::state::PermissionMode::Ask,
        ..Default::default()
    };

    state.rows.push(ChatRow::User {
        id: "d-user-1".into(),
        text: "Add a sidebar that lists past sessions for this project, \
               grouped by day, and loads one when I click it."
            .into(),
    });
    state.rows.push(ChatRow::Thought {
        id: "d-thought-1".into(),
        text: "The runtime keeps every session in its SQLite store. I can list \
               them with the harness's session store, then load one through the \
               backend so its history replays on the update stream. The sidebar \
               needs the id, a title and the created date; the first user prompt \
               makes a good title."
            .into(),
        seconds: 6.0,
    });
    state.rows.push(ChatRow::Tool(ToolCard {
        id: "d-tool-1".into(),
        name: Some("read".into()),
        title: "src/ui/sidebar.rs".into(),
        kind: ToolKind::Read,
        status: ToolStatus::Completed,
        output: "// Left sidebar: wordmark, new session button, grouped session\n// list.\npub fn show(ui: &mut Ui, sessions: &[SessionEntry], ...) {\n    ...\n}"
            .into(),
        input: "{\n  \"path\": \"src/ui/sidebar.rs\"\n}".into(),
        diff: None,
    }));
    state.rows.push(ChatRow::Tool(ToolCard {
        id: "d-tool-2".into(),
        name: Some("edit".into()),
        title: "src/state.rs".into(),
        kind: ToolKind::Edit,
        status: ToolStatus::Completed,
        output: String::new(),
        input: "{\n  \"path\": \"src/state.rs\",\n  \"old_text\": \"pub struct AppState {\",\n  \"new_text\": \"pub struct AppState {\\n    pub sessions: Vec<SessionEntry>,\"\n}".into(),
        diff: Some(DiffContent {
            path: "src/state.rs".into(),
            diff: "@@ -12,6 +12,7 @@ use std::collections::HashMap;\n /// Everything the UI needs to draw one frame.\n #[derive(Debug, Default)]\n pub struct AppState {\n+    /// Known sessions for the project, newest first.\n+    pub sessions: Vec<SessionEntry>,\n     /// Rows of the active conversation, oldest first.\n     pub rows: Vec<ChatRow>,".into(),
        }),
    }));
    state.rows.push(ChatRow::Permission(
        crate::state::PermissionPrompt {
            id: "d-perm-1".into(),
            kind: "bash".into(),
            tool_name: "bash".into(),
            summary: "cargo build --tests".into(),
            detail: "/Users/demo/projects/loop-desk".into(),
            resolved: false,
            outcome: None,
        },
    ));
    state.rows.push(ChatRow::Tool(ToolCard {
        id: "d-tool-3".into(),
        name: Some("bash".into()),
        title: "cargo build".into(),
        kind: ToolKind::Execute,
        status: ToolStatus::Failed,
        output: "error[E0308]: mismatched types\n --> src/engine.rs:42:26\n  |\n42 |     let metas: Vec<SessionMetadata> = store.list(None).await?;\n  |                -------------------   ^^^^^^^^^^^^^^^^^^^^^ expected `Vec<SessionMetadata>`, found `()`".into(),
        input: "{\n  \"command\": \"cargo build\"\n}".into(),
        diff: None,
    }));
    state.rows.push(ChatRow::Agent {
        id: "d-agent-1".into(),
        text: "The sidebar is wired up. Here is what changed:\n\n- `state.rs` gained a `sessions` list and a `SessionEntry` per row.\n- `engine.rs` lists sessions from the runtime's SQLite store and loads one \
through the backend when you click it, so its full history replays.\n\nSessions are grouped **Today**, **Yesterday** and **Earlier**, newest first:\n\n```rust\nlet metas = store.list(Some(cwd)).await?;\nmetas.sort_by(|a, b| b.created_at.cmp(&a.created_at));\n```\n\n> Clicking a session keeps the running turn alive in its own session; the \
sidebar shows an activity dot on it.".into(),
        done: false,
    });
    state.rows.push(ChatRow::Notice {
        level: NoticeLevel::Info,
        text: "Context compacted automatically: 118,092 → 22,990 tokens".into(),
    });

    state.sessions = vec![
        SessionEntry {
            id: "demo-1".into(),
            title: "Add a session list to the sidebar".into(),
            created_at: now - 5 * minute,
            busy: true,
        },
        SessionEntry {
            id: "demo-2".into(),
            title: "Explain the approval bridge".into(),
            created_at: now - 3 * 60 * minute,
            busy: false,
        },
        SessionEntry {
            id: "demo-3".into(),
            title: "Why does the pty resize lag?".into(),
            created_at: now - 26 * 60 * minute,
            busy: false,
        },
        SessionEntry {
            id: "demo-4".into(),
            title: "Write tests for the session store".into(),
            created_at: now - 3 * 24 * 60 * minute,
            busy: false,
        },
    ];
    state.usage.total_tokens = 22_990;
    state.usage.context_window = Some(200_000);

    // The edit above also feeds the Changes panel.
    let changes: Vec<(String, String)> = state
        .rows
        .iter()
        .filter_map(|row| match row {
            ChatRow::Tool(card) => card
                .diff
                .as_ref()
                .map(|d| (d.path.clone(), d.diff.clone())),
            _ => None,
        })
        .collect();
    for (path, diff) in changes {
        state.record_change(path, diff);
    }
    state
}
