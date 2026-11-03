//! loop-desk: a desktop app for the Loop coding agent.

mod change_log;
mod demo;
mod engine;
#[cfg(test)]
mod layout_tests;
mod protocol;
#[cfg(test)]
mod live_test;
mod state;
#[cfg(test)]
mod tests;
mod ui;

use std::path::PathBuf;

use protocol::{UiCommand, UiEvent};
use state::AppState;
use ui::{theme, ChatView};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Some(Cli::Screenshot(opts)) => {
            if let Err(err) = demo::run_screenshot(opts) {
                eprintln!("loop-desk: {err}");
                std::process::exit(1);
            }
        }
        Some(Cli::Help) => print_help(),
        None => run_app(project_dir_from(&args)),
    }
}

enum Cli {
    Screenshot(demo::DemoOptions),
    Help,
}

/// Route the arguments: help, demo screenshot or a normal run.
fn parse_args(args: &[String]) -> Option<Cli> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Some(Cli::Help);
    }
    demo::demo_options_from(args).map(Cli::Screenshot)
}

fn print_help() {
    println!(
        "loop-desk — a desktop app for the Loop coding agent\n\n\
         Usage: loop-desk [PROJECT_DIR]\n\n\
         Options:\n  \
         --demo --screenshot out.png [--dark] [--width 1200] [--height 800]\n    \
         Render the built-in demo conversation once and save it as a PNG.\n  \
         -h, --help  Show this help."
    );
}

/// The first non-flag argument is the project directory.
fn project_dir_from(args: &[String]) -> PathBuf {
    args.iter()
        .find(|a| !a.starts_with('-') && !a.ends_with(".png"))
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

struct LoopDeskApp {
    view: ChatView,
    state: AppState,
    engine: Option<engine::EngineHandle>,
    events: tokio::sync::mpsc::UnboundedReceiver<UiEvent>,
    /// Terminal dock; created lazily with the project dir.
    dock: ui::terminal::TerminalDock,
    /// Last window title sent to the OS.
    title_sent: String,
    /// Kept alive for the app's lifetime; the engine task runs on it.
    _runtime: tokio::runtime::Runtime,
}

impl LoopDeskApp {
    fn new(cc: &eframe::CreationContext<'_>, cwd: PathBuf) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let ctx = cc.egui_ctx.clone();
        let (engine, events) =
            runtime.block_on(async move { engine::EngineHandle::spawn(cwd, ctx) });
        let dock_cwd = std::env::current_dir().ok().unwrap_or_else(|| PathBuf::from("."));
        let mut dock = ui::terminal::TerminalDock::new(dock_cwd);
        load_dock_layout(&mut dock);
        Self {
            _runtime: runtime,
            view: ChatView::new(initial_theme(&cc.egui_ctx)),
            title_sent: String::new(),
            state: AppState {
                project_dir: String::new(),
                permission_mode: crate::state::PermissionMode::Ask,
                ..Default::default()
            },
            engine: Some(engine),
            events,
            dock,
        }
    }

    /// Move every pending engine event into the UI state.
    fn drain_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            self.apply(event);
        }
    }

    fn apply(&mut self, event: UiEvent) {
        match event {
            UiEvent::Started {
                session_id,
                model_label,
                notices,
                project_dir,
                sessions,
            } => {
                self.state.active_session = session_id;
                self.state.model_label = model_label;
                self.state.startup_notices = notices;
                self.state.project_dir = project_dir;
                self.state.sessions = sessions;
            }
            UiEvent::Failed(text) => self.state.error = Some(text),
            UiEvent::Row(row) => self.state.upsert_row(row),
            UiEvent::AgentText { id, delta } => {
                self.state.append_text(&id, &delta, true);
            }
            UiEvent::ThoughtText { id, delta } => self.state.append_thought(&id, &delta),
            UiEvent::MessagesEnded => self.state.finish_messages(),
            UiEvent::TurnStarted => self.state.working = true,
            UiEvent::TurnEnded { reason, error } => {
                self.state.working = false;
                self.state.finish_messages();
                self.command(UiCommand::RefreshSessions);
                if let Some(error) = error {
                    self.state.error = Some(error);
                } else if reason.contains("Error") {
                    self.state.error = Some(format!("turn ended: {reason}"));
                }
            }
            UiEvent::Notice { level, text } => {
                self.state.rows.push(state::ChatRow::Notice { level, text });
            }
            UiEvent::Usage(usage) => self.state.usage = usage,
            UiEvent::SessionLoaded {
                session_id,
                title,
                rows,
            } => {
                self.state.reset_conversation(session_id, title, rows);
            }
            UiEvent::SessionCreated { session_id } => {
                self.state
                    .reset_conversation(session_id, "New session".into(), Vec::new());
            }
            UiEvent::Sessions(sessions) => self.state.sessions = sessions,
            UiEvent::BusySessions(busy) => {
                for session in &mut self.state.sessions {
                    session.busy = busy.contains(&session.id);
                }
            }
            UiEvent::ApprovalRequested(row) => self.state.upsert_row(row),
            UiEvent::Models { list, current, needs_login } => {
                if let Some(spec) = &current {
                    self.state.model_label = Some(spec.clone());
                }
                self.state.models = list;
                self.state.current_model = current;
                // First run: no key yet, so open the sign-in dialog.
                if needs_login {
                    self.state.connect.open = true;
                }
            }
            UiEvent::LoginResult { ok, message } => {
                self.state.connect.busy = false;
                self.state.connect.message = Some((ok, message));
            }
            UiEvent::PermissionModeChanged(mode) => {
                if let Some(mode) = parse_permission_mode(&mode) {
                    self.state.permission_mode = mode;
                }
            }
            UiEvent::FileChanged { path, diff } => {
                self.view.changes_open = true;
                self.state.record_change(path, diff);
            }
        }
    }

    /// The session `step` places away from the active one, if any.
    fn neighbour_session(&self, step: i32) -> Option<String> {
        let sessions = &self.state.sessions;
        let index = sessions.iter().position(|s| s.id == self.state.active_session)?;
        let target = index as i32 + step;
        if target < 0 || target >= sessions.len() as i32 {
            return None;
        }
        Some(sessions[target as usize].id.clone())
    }

    /// Switch to a session by id, clearing the view until it loads.
    fn select_session(&mut self, id: String) {
        self.state.active_session = id.clone();
        self.state.rows.clear();
        self.state.session_title.clear();
        self.state.working = false;
        self.state.pending_approval = None;
        self.state.changed_files.clear();
        self.command(UiCommand::SelectSession(id));
    }

    /// Send a command to the engine when there is one.
    fn command(&self, command: UiCommand) {
        if let Some(engine) = &self.engine {
            engine.send(command);
        }
    }
}

impl eframe::App for LoopDeskApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();

        let last_dock_side = self.dock.side;
        let last_dock_size = self.dock.size;

        // Keyboard shortcuts.
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::N) {
                self.command(UiCommand::NewSession);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::B) {
                self.view.sidebar_open = !self.view.sidebar_open;
            }
            // Cmd+[ newer, Cmd+] older (the sidebar lists newest first).
            let step = if i.modifiers.command && i.key_pressed(egui::Key::OpenBracket) {
                Some(-1)
            } else if i.modifiers.command
                && i.key_pressed(egui::Key::CloseBracket)
            {
                Some(1)
            } else {
                None
            };
            if let Some(step) = step {
                if let Some(target) = self.neighbour_session(step) {
                    self.select_session(target);
                }
            }
        });

        // Handle what the user asked for during drawing.
        let view = std::mem::replace(&mut self.view, ChatView::new(theme::ThemeMode::Light));
        let action = view.clone().show(ctx, &mut self.state, Some(&mut self.dock));
        if let Some(open) = self.state.pending_changes_open.take() {
            self.view.changes_open = open;
        }
        self.view = view;
        // Persist the dock layout when it moved or changed side.
        if self.dock.side != last_dock_side
            || (self.dock.size - last_dock_size).abs() > 1.0
        {
            save_dock_layout(&self.dock);
        }
        match action {
            ui::ViewAction::None => {}
            ui::ViewAction::NewSession => self.command(UiCommand::NewSession),
            ui::ViewAction::SelectSession(id) => self.select_session(id),
            ui::ViewAction::Composer(ui::composer::ComposerAction::Send(text)) => {
                // The engine task echoes the user row and names the session.
                self.state.working = true;
                self.command(UiCommand::Prompt(text));
            }
            ui::ViewAction::Composer(ui::composer::ComposerAction::Stop) => {
                self.command(UiCommand::Stop);
            }
            ui::ViewAction::Approve { id, outcome } => {
                self.state.resolve_approval(&id, outcome);
                self.command(UiCommand::Approve { id, outcome });
            }
            ui::ViewAction::SetPermissionMode(mode) => {
                self.state.permission_mode = mode;
                self.command(UiCommand::SetPermissionMode(mode.as_str().to_owned()));
            }
            ui::ViewAction::Connect(ui::connect::ConnectAction::SelectModel { provider, id }) => {
                self.command(UiCommand::SelectModel { provider, id });
            }
            ui::ViewAction::Connect(ui::connect::ConnectAction::Login { provider, api_key }) => {
                self.command(UiCommand::Login { provider, api_key });
            }
            ui::ViewAction::ToggleTheme => {
                self.view.mode = match self.view.mode {
                    theme::ThemeMode::Light => theme::ThemeMode::Dark,
                    theme::ThemeMode::Dark => theme::ThemeMode::Light,
                };
                save_setting("theme", match self.view.mode {
                    theme::ThemeMode::Light => "light",
                    theme::ThemeMode::Dark => "dark",
                }.into());
            }
            ui::ViewAction::ToggleTerminal => self.dock.toggle(ctx),
            ui::ViewAction::ToggleChanges => {
                self.view.changes_open = !self.view.changes_open;
            }
        }

        // Window title: session, project and git branch.
        let title = window_title(&self.state);
        if title != self.title_sent {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title_sent = title;
        }

        // Keep repainting while a turn runs so streaming updates land fast.
        if self.state.working || self.dock.open {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

/// Parse the engine's permission-mode string.
fn parse_permission_mode(value: &str) -> Option<crate::state::PermissionMode> {
    crate::state::PermissionMode::from_str(value)
}

/// File the dock layout is persisted to (`~/.loop/agent/loop-desk.json`).
fn dock_layout_path() -> Option<PathBuf> {
    let dir = loop_app_core::get_agent_dir();
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("loop-desk.json"))
}

/// Load the dock side and size from the settings file.
fn load_dock_layout(dock: &mut ui::terminal::TerminalDock) {
    let Some(path) = dock_layout_path() else { return };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    if let Some(side) = value.get("dockSide").and_then(|v| v.as_str()) {
        if side == "bottom" {
            dock.side = ui::terminal::DockSide::Bottom;
        } else if side == "right" {
            dock.side = ui::terminal::DockSide::Right;
        }
    }
    if let Some(size) = value.get("dockSize").and_then(|v| v.as_f64()) {
        if (120.0..=2000.0).contains(&size) {
            dock.size = size as f32;
        }
    }
}

/// Read the whole settings object (`~/.loop/agent/loop-desk.json`), empty when missing or broken.
fn read_settings() -> serde_json::Map<String, serde_json::Value> {
    dock_layout_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

/// Set one key in the settings file without losing the others, best effort.
fn save_setting(key: &str, value: serde_json::Value) {
    let Some(path) = dock_layout_path() else { return };
    let mut map = read_settings();
    map.insert(key.to_owned(), value);
    if let Ok(text) = serde_json::to_string_pretty(&serde_json::Value::Object(map)) {
        let _ = std::fs::write(path, text);
    }
}

/// Save the dock side and size, best effort.
fn save_dock_layout(dock: &ui::terminal::TerminalDock) {
    let side = match dock.side {
        ui::terminal::DockSide::Right => "right",
        ui::terminal::DockSide::Bottom => "bottom",
    };
    save_setting("dockSide", side.into());
    save_setting("dockSize", dock.size.into());
}

/// The saved theme, else the operating system's, else light.
fn initial_theme(ctx: &egui::Context) -> theme::ThemeMode {
    match read_settings().get("theme").and_then(|v| v.as_str()) {
        Some("dark") => theme::ThemeMode::Dark,
        Some("light") => theme::ThemeMode::Light,
        _ => match ctx.system_theme() {
            Some(egui::Theme::Dark) => theme::ThemeMode::Dark,
            _ => theme::ThemeMode::Light,
        },
    }
}

/// `Session title — project (branch)`.
fn window_title(state: &AppState) -> String {
    let project = ui::project_name(&state.project_dir);
    let session = if state.session_title.is_empty() { "Loop".to_owned() } else { state.session_title.clone() };
    match git_branch(&state.project_dir) {
        Some(branch) if !project.is_empty() => format!("{session} — {project} ({branch})"),
        _ if !project.is_empty() => format!("{session} — {project}"),
        _ => session,
    }
}

/// Current git branch of `dir`, read from `.git/HEAD`.
fn git_branch(dir: &str) -> Option<String> {
    if dir.is_empty() {
        return None;
    }
    let head = std::fs::read_to_string(std::path::Path::new(dir).join(".git/HEAD")).ok()?;
    head.trim().strip_prefix("ref: refs/heads/").map(str::to_owned)
}

fn run_app(cwd: PathBuf) {    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 800.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    if let Err(err) = eframe::run_native(
        "Loop",
        options,
        Box::new(|cc| Ok(Box::new(LoopDeskApp::new(cc, cwd)))),
    ) {
        eprintln!("loop-desk: {err}");
        std::process::exit(1);
    }
}
