//! Owns the engine `Runtime` on a tokio task and translates between the
//! protocol and the UI's plain data.

use std::path::PathBuf;
use std::sync::Arc;

use loop_app_core::{
    bootstrap,
    permissions::PermissionMode as CorePermissionMode,
    tool_approval::{permissions_from_settings, ToolApprovalBridge},
    BootstrapOpts,
};
use loop_protocol::{
    ContentBlock, Origin, Role, SessionRef, SessionSpec, SessionUpdate, UpdateEnvelope,
};
use tokio::sync::mpsc;

use crate::protocol::{UiCommand, UiEvent};
use crate::state::{
    ChatRow, NoticeLevel as UiNotice, ToolCard, ToolKind as UiToolKind,
    ToolStatus as UiToolStatus, UsageSnapshot,
};

mod approvals;
mod sessions;
mod translate;

use sessions::{
    list_sessions, open_store_reader, transcript_rows,
};
use translate::{
    card_from_call, diff_content, notice_level, pretty_json, text_content, tool_kind,
    tool_status,
};

/// Handle the UI thread uses to talk to the engine task.
#[derive(Clone)]
pub struct EngineHandle {
    /// Commands to the engine task.
    pub commands: mpsc::UnboundedSender<UiCommand>,
}

impl EngineHandle {
    /// Start the engine task for `cwd` and return it with its event receiver.
    pub fn spawn(
        cwd: PathBuf,
        ctx: egui::Context,
    ) -> (Self, mpsc::UnboundedReceiver<UiEvent>) {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<UiCommand>();
        let (evt_tx, evt_rx) = mpsc::unbounded_channel::<UiEvent>();
        let send = move |event: UiEvent| {
            let _ = evt_tx.send(event);
            ctx.request_repaint();
        };
        let notify = send.clone();
        tokio::spawn(async move {
            let actor = match Actor::new(cwd, notify).await {
                Ok(actor) => actor,
                Err(err) => {
                    send(UiEvent::Failed(format!("engine failed to start: {err}")));
                    // Drain commands so the UI never blocks sending.
                    while cmd_rx.recv().await.is_some() {}
                    return;
                }
            };
            actor.run(cmd_rx, send).await;
        });
        (Self { commands: cmd_tx }, evt_rx)
    }

    /// Send a command; errors only mean the app is shutting down.
    pub fn send(&self, command: UiCommand) {
        let _ = self.commands.send(command);
    }
}

/// The engine task's state.
struct Actor {
    rt: loop_app_core::Runtime,
    updates: tokio::sync::broadcast::Receiver<UpdateEnvelope>,
    approvals: mpsc::UnboundedReceiver<loop_app_core::tool_approval::ApprovalPrompt>,
    sessions_db: PathBuf,
    /// Sessions with a turn running.
    busy: Vec<String>,
    /// Reply channel of the approval being asked, when any.
    pending_response_tx:
        Option<tokio::sync::oneshot::Sender<loop_app_core::tool_approval::ApprovalDecision>>,
    /// Row id of the approval the reply channel belongs to.
    pending_approval_id: Option<String>,
    /// The approval row as the UI last saw it, to replay after a switch.
    pending_approval_row: Option<ChatRow>,
    /// Session the pending approval belongs to.
    pending_approval_session: Option<String>,
    /// Monotonic counter for approval row ids.
    approval_seq: usize,
}

impl Actor {
    async fn new(
        cwd: PathBuf,
        notify: impl Fn(UiEvent) + Send + 'static,
    ) -> anyhow::Result<Self> {
        let mut rt = match bootstrap(opts(&cwd, false)).await {
            Ok(rt) => rt,
            // No key yet: start anyway so the sign-in dialog can add one.
            Err(err) if err.to_string().starts_with("No LLM provider connected") => {
                bootstrap_without_provider(&cwd).await?
            }
            Err(err) => return Err(err),
        };
        let updates = rt.backend.subscribe();

        // Approvals: build the bridge the tool gate asks through and drain its
        // prompts in the main loop, so the UI can answer inline.
        let env = rt.harness.tool_env().await?;
        let (bridge, approvals) = ToolApprovalBridge::new(
            env,
            rt.settings.diff_editor.clone(),
            permissions_from_settings(&rt.settings.tool_permissions),
            Default::default(),
        );
        // Review file edits in-app; never spawn an external diff editor.
        bridge.set_open_external_diff(false);
        rt.tool_gate.set_approver(Some(Arc::new(bridge)));

        let notices = std::mem::take(&mut rt.startup_notices);
        let sessions = list_sessions(&rt).await;
        let mode = rt.permission_mode();
        notify(UiEvent::Started {
            session_id: rt.session_id.clone(),
            model_label: model_label(&rt),
            notices,
            project_dir: cwd.to_string_lossy().into_owned(),
            sessions,
        });
        notify(UiEvent::PermissionModeChanged(mode.as_str().to_owned()));
        notify(models_event(&rt).await);
        Ok(Self {
            updates,
            approvals,
            sessions_db: rt.sessions_db.clone(),
            busy: Vec::new(),
            pending_response_tx: None,
            pending_approval_id: None,
            pending_approval_row: None,
            pending_approval_session: None,
            approval_seq: 0,
            rt,
        })
    }

    async fn run(
        mut self,
        mut cmd_rx: mpsc::UnboundedReceiver<UiCommand>,
        send: impl Fn(UiEvent) + Clone + Send + 'static,
    ) {
        loop {
            tokio::select! {
                command = cmd_rx.recv() => {
                    let Some(command) = command else { break };
                    self.handle(command, &send).await;
                }
                envelope = self.updates.recv() => {
                    match envelope {
                        Ok(envelope) => self.forward(envelope, &send).await,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                approval = self.approvals.recv() => {
                    match approval {
                        Some(prompt) => self.on_approval(prompt, &send),
                        None => continue,
                    }
                }
            }
        }
    }

    async fn handle(&mut self, command: UiCommand, send: &impl Fn(UiEvent)) {
        match command {
            UiCommand::Prompt(text) => self.prompt(text, send).await,
            UiCommand::Stop => self.stop(send),
            UiCommand::NewSession => self.new_session(send).await,
            UiCommand::SelectSession(id) => self.select_session(id, send).await,
            UiCommand::RefreshSessions => {
                send(UiEvent::Sessions(list_sessions(&self.rt).await));
            }
            UiCommand::Approve { id, outcome } => self.approve(id, outcome),
            UiCommand::SelectModel { provider, id } => {
                match self.rt.select_model(&provider, &id).await {
                    Ok(_) => {
                        send(models_event(&self.rt).await);
                        send(UiEvent::Notice {
                            level: UiNotice::Info,
                            text: format!("Model switched to {provider}/{id}"),
                        });
                    }
                    Err(err) => send(UiEvent::Failed(format!("{err:#}"))),
                }
            }
            UiCommand::Login { provider, api_key } => {
                let request = loop_app_core::config::ProviderLoginRequest::Preset {
                    id: provider,
                    api_key: api_key.trim().to_owned(),
                };
                match self.rt.connect_provider(&request).await {
                    Ok(done) => {
                        // Use the provider's first model when none works yet.
                        let models = self.rt.available_models().await;
                        let usable = self
                            .rt
                            .selected_model_spec()
                            .is_some_and(|spec| {
                                models.iter().any(|m| format!("{}/{}", m.provider, m.id) == spec)
                            });
                        if !usable {
                            if let Some(m) = models.iter().find(|m| m.provider == done.id) {
                                let _ = self.rt.select_model(&m.provider, &m.id).await;
                            }
                        }
                        send(UiEvent::LoginResult {
                            ok: true,
                            message: format!("{} connected, {} models", done.name, done.model_count),
                        });
                        send(models_event(&self.rt).await);
                    }
                    Err(err) => send(UiEvent::LoginResult {
                        ok: false,
                        message: format!("{err:#}"),
                    }),
                }
            }
            UiCommand::SetPermissionMode(mode) => {
                let Some(mode) = CorePermissionMode::parse(&mode) else {
                    return;
                };
                self.rt.set_permission_mode(mode).await;
                send(UiEvent::PermissionModeChanged(mode.as_str().to_owned()));
            }
        }
    }

    async fn prompt(&mut self, text: String, send: &impl Fn(UiEvent)) {
        let trimmed = text.trim().to_owned();
        if trimmed.is_empty() {
            return;
        }
        let session = self.rt.session_ref();
        let id = self.rt.session_id.clone();
        // Name the session after its first prompt so the sidebar stays useful.
        if let Ok(reader) = open_store_reader(&self.sessions_db, &id).await {
            let unnamed = reader.metadata().name.as_deref().unwrap_or("").is_empty();
            if unnamed {
                let title = crate::state::AppState::title_from_prompt(&trimmed);
                let _ = self.rt.harness.set_session_name(title).await;
            }
        }
        send(UiEvent::Row(ChatRow::User {
            id: format!("user-{id}-{}", chrono::Utc::now().timestamp_millis()),
            text: trimmed.clone(),
        }));
        match self
            .rt
            .backend
            .prompt(&session, vec![ContentBlock::text(trimmed)])
            .await
        {
            Ok(handle) => {
                self.busy.push(id.clone());
                send(UiEvent::BusySessions(self.busy.clone()));
                // Only await the outcome so the stream keeps flowing in the
                // select loop of `run`; detach the handle.
                tokio::spawn(async move {
                    let outcome = handle.outcome().await;
                    let _ = outcome;
                });
            }
            Err(err) => send(UiEvent::Failed(format!("could not send the prompt: {err}"))),
        }
    }

    fn stop(&mut self, send: &impl Fn(UiEvent)) {
        self.rt.backend.cancel(&self.rt.session_ref());
        send(UiEvent::Notice {
            level: UiNotice::Info,
            text: "Stopping…".to_owned(),
        });
    }

    async fn new_session(&mut self, send: &impl Fn(UiEvent)) {
        if let Err(err) = self.rt.new_session().await {
            send(UiEvent::Failed(format!("could not start a session: {err}")));
            return;
        }
        let id = self.rt.session_id.clone();
        send(UiEvent::SessionCreated {
            session_id: id.clone(),
        });
        send(UiEvent::Sessions(list_sessions(&self.rt).await));
    }

    async fn select_session(&mut self, id: String, send: &impl Fn(UiEvent)) {
        let backend = Arc::clone(&self.rt.backend);
        let sr = SessionRef {
            instance: backend.instance().clone(),
            key: loop_protocol::SessionKey(id.clone()),
            native_id: id.clone(),
        };
        let spec = SessionSpec {
            cwd: Some(self.rt.cwd.clone()),
            name: None,
        };
        if let Err(err) = backend.load_session(&sr, spec).await {
            send(UiEvent::Failed(format!("could not open the session: {err}")));
            return;
        }
        // `load_session` re-adopted the session; mirror it on the runtime.
        self.rt.session_id = id.clone();
        self.rt.backend = backend;
        let title = self
            .session_title(&id)
            .await
            .unwrap_or_else(|| "Session".to_owned());
        let rows = transcript_rows(&self.rt).await;
        send(UiEvent::SessionLoaded {
            session_id: id,
            title,
            rows,
        });
        // An approval that was asked in this session still needs an answer.
        if let (Some(row), Some(session)) =
            (self.pending_approval_row.clone(), self.pending_approval_session.clone())
        {
            if session == self.rt.session_id {
                send(UiEvent::ApprovalRequested(row));
            }
        }
    }

    async fn session_title(&self, id: &str) -> Option<String> {
        let reader = open_store_reader(&self.sessions_db, id).await.ok()?;
        reader.metadata().name.clone()
    }

    /// Translate one protocol envelope into UI events.
    async fn forward(&mut self, envelope: UpdateEnvelope, send: &impl Fn(UiEvent)) {
        // Busy state tracks every session, so a turn that ends after the user
        // switched away still clears its activity dot.
        if matches!(envelope.update, SessionUpdate::TurnEnded { .. }) {
            let id = envelope.session.0.clone();
            self.busy.retain(|b| b != &id);
            send(UiEvent::BusySessions(self.busy.clone()));
        }
        let active = self.rt.session_ref().key == envelope.session;
        if !active {
            return;
        }
        // Live history is already on screen; replay replaces it wholesale.
        if envelope.origin == Origin::Replay {
            return;
        }
        match envelope.update {
            SessionUpdate::TurnStarted { .. } => send(UiEvent::TurnStarted),
            SessionUpdate::MessageChunk {
                message_id,
                role,
                content,
            } => {
                if role == Role::Agent {
                    if let Some(text) = content.as_text() {
                        send(UiEvent::AgentText {
                            id: message_id.0,
                            delta: text.to_owned(),
                        });
                    }
                }
            }
            SessionUpdate::ThoughtChunk { message_id, content } => {
                if let Some(text) = content.as_text() {
                    send(UiEvent::ThoughtText {
                        id: message_id.0,
                        delta: text.to_owned(),
                    });
                }
            }
            SessionUpdate::MessageEnded { .. } => send(UiEvent::MessagesEnded),
            SessionUpdate::ToolCall(call) => {
                send(UiEvent::Row(ChatRow::Tool(card_from_call(&call))));
            }
            SessionUpdate::ToolCallUpdate(update) => {
                // Partial output arrives without a status; end states arrive
                // with one. Forward whichever fields the update carries.
                if update.status.is_some() || update.content.is_some() {
                    let content = update.content.as_deref().unwrap_or(&[]);
                    let card = ToolCard {
                        id: update.id.0,
                        name: update.name,
                        title: update.title.unwrap_or_default(),
                        kind: update.kind.map(tool_kind).unwrap_or(UiToolKind::Other),
                        status: update
                            .status
                            .map(tool_status)
                            .unwrap_or(UiToolStatus::InProgress),
                        output: text_content(content),
                        input: pretty_json(update.raw_input.as_ref()),
                        diff: diff_content(content),
                    };
                    // A finished edit or write also feeds the Changes panel.
                    if let Some(diff) = &card.diff {
                        if matches!(card.status, UiToolStatus::Completed) {
                            send(UiEvent::FileChanged {
                                path: diff.path.clone(),
                                diff: diff.diff.clone(),
                            });
                        }
                    }
                    send(UiEvent::Row(ChatRow::Tool(card)));
                }
            }
            SessionUpdate::Notice { level, text } => send(UiEvent::Notice {
                level: notice_level(level),
                text,
            }),
            SessionUpdate::Usage(usage) => send(UiEvent::Usage(UsageSnapshot {
                total_tokens: usage.total_tokens,
                context_window: usage.context_window,
                cost_usd: usage.cost_usd,
            })),
            SessionUpdate::TurnEnded { stop, error, .. } => {
                send(UiEvent::TurnEnded {
                    reason: format!("{stop:?}"),
                    error: error.map(|e| e.message),
                });
                send(UiEvent::Sessions(list_sessions(&self.rt).await));
            }
            _ => {}
        }
    }
}

/// A short label for the selected model.
fn opts(cwd: &std::path::Path, interactive: bool) -> BootstrapOpts {
    BootstrapOpts {
        cwd: cwd.to_path_buf(),
        provider: None,
        model: None,
        theme: None,
        system_prompt: None,
        append_system_prompt: None,
        no_context_files: false,
        interactive,
        session_id: None,
        no_web: false,
        permission_mode: None,
        agent: None,
    }
}

/// Start the runtime with no provider connected. The only way the core allows
/// that is `interactive`, which may also ask about project trust on stdin: a
/// window has no stdin, so point it at /dev/null and put the trust file back
/// as it was, so the unanswered question is asked again once a key exists.
async fn bootstrap_without_provider(cwd: &std::path::Path) -> anyhow::Result<loop_app_core::Runtime> {
    let trust = loop_app_core::config::paths::trust_path(&loop_app_core::config::paths::get_agent_dir());
    let before = std::fs::read(&trust).ok();
    if let Ok(null) = std::fs::File::open("/dev/null") {
        use std::os::fd::AsRawFd;
        // SAFETY: both descriptors are valid; this only redirects stdin.
        unsafe { libc::dup2(null.as_raw_fd(), 0) };
    }
    let result = bootstrap(opts(cwd, true)).await;
    match before {
        Some(bytes) => {
            let _ = std::fs::write(&trust, bytes);
        }
        None => {
            let _ = std::fs::remove_file(&trust);
        }
    }
    result
}

async fn models_event(rt: &loop_app_core::Runtime) -> UiEvent {
    let list = rt
        .available_models()
        .await
        .into_iter()
        .map(|m| crate::state::ModelEntry {
            provider: m.provider.to_string(),
            id: m.id.to_string(),
        })
        .collect::<Vec<_>>();
    UiEvent::Models {
        needs_login: list.is_empty(),
        list,
        current: rt.selected_model_spec(),
    }
}

fn model_label(rt: &loop_app_core::Runtime) -> Option<String> {
    rt.active_model_label().or_else(|| rt.selected_model_spec())
}
