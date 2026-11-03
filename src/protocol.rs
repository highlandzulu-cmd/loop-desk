//! Messages between the UI thread and the engine task.

use crate::state::{
    ChatRow, ModelEntry, NoticeLevel, PermissionOutcome, SessionEntry, UsageSnapshot,
};

/// What the UI asks the engine to do.
#[derive(Debug, Clone)]
pub enum UiCommand {
    /// Send a prompt in the active session.
    Prompt(String),
    /// Stop the running turn.
    Stop,
    /// Start a fresh session.
    NewSession,
    /// Make this stored session the active one and load its history.
    SelectSession(String),
    /// Refresh the session list from the store.
    RefreshSessions,
    /// Answer the pending approval.
    Approve {
        /// Which request (the id the engine sent).
        id: String,
        /// The decision.
        outcome: PermissionOutcome,
    },
    /// Switch the permission mode.
    SetPermissionMode(String),
    /// Use this model from now on.
    SelectModel {
        /// Provider id.
        provider: String,
        /// Model id.
        id: String,
    },
    /// Verify and save an API key for a built-in provider.
    Login {
        /// Provider preset id.
        provider: String,
        /// The key; never logged.
        api_key: String,
    },
}

/// What the engine task tells the UI.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// The engine is up; this is the active session.
    Started {
        /// Session id.
        session_id: String,
        /// Model label, when known.
        model_label: Option<String>,
        /// Notices shown once at startup.
        notices: Vec<String>,
        /// Absolute project directory.
        project_dir: String,
        /// Sessions for the project, newest first.
        sessions: Vec<SessionEntry>,
    },
    /// The engine could not start or failed.
    Failed(String),
    /// A row was added or replaced.
    Row(ChatRow),
    /// The models the user can pick from.
    Models {
        /// Choices, Soket first.
        list: Vec<ModelEntry>,
        /// `provider/id` of the selected one.
        current: Option<String>,
        /// True when no provider is connected yet.
        needs_login: bool,
    },
    /// The outcome of a login attempt.
    LoginResult {
        /// True when the key was accepted and saved.
        ok: bool,
        /// Short message for the dialog.
        message: String,
    },
    /// Text appended to a streaming assistant message.
    AgentText {
        /// Message id.
        id: String,
        /// Delta text.
        delta: String,
    },
    /// Text appended to a thinking row.
    ThoughtText {
        /// Message id.
        id: String,
        /// Delta text.
        delta: String,
    },
    /// All open messages are complete.
    MessagesEnded,
    /// A turn started in this session.
    TurnStarted,
    /// A turn ended in this session.
    TurnEnded {
        /// Why it ended, as text.
        reason: String,
        /// Failure details, when the turn failed.
        error: Option<String>,
    },
    /// A notice for the conversation.
    Notice {
        /// Severity.
        level: NoticeLevel,
        /// Text.
        text: String,
    },
    /// Token usage of the latest request.
    Usage(UsageSnapshot),
    /// The active session changed; replace the conversation with these rows.
    SessionLoaded {
        /// Session id.
        session_id: String,
        /// Title.
        title: String,
        /// Full history, oldest first.
        rows: Vec<ChatRow>,
    },
    /// A new session was created and is now active (empty history).
    SessionCreated {
        /// Session id.
        session_id: String,
    },
    /// The session list changed.
    Sessions(Vec<SessionEntry>),
    /// Which sessions have a turn running.
    BusySessions(Vec<String>),
    /// The agent asks permission to run something.
    ApprovalRequested(ChatRow),
    /// The permission mode changed.
    PermissionModeChanged(String),
    /// A tool call finished with a file diff.
    FileChanged {
        /// File path.
        path: String,
        /// Unified diff text.
        diff: String,
    },
}
