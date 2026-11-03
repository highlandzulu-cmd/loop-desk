//! Plain data the UI draws. No egui types live here.

use std::collections::HashMap;

pub use crate::change_log::{ChangedFile, PermissionMode};

/// One row of the conversation.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRow {
    /// A user message.
    User { id: String, text: String },
    /// An assistant message (streaming while `done` is false).
    Agent { id: String, text: String, done: bool },
    /// The agent's reasoning, collapsed by default.
    Thought { id: String, text: String, seconds: f32 },
    /// A tool call card.
    Tool(ToolCard),
    /// The agent asks permission to run something.
    Permission(PermissionPrompt),
    /// Something that happened outside the conversation.
    Notice { level: NoticeLevel, text: String },
}

impl ChatRow {
    /// Rows with the same key belong together and are replaced, not appended.
    pub fn key(&self) -> &str {
        match self {
            Self::User { id, .. }
            | Self::Agent { id, .. }
            | Self::Thought { id, .. } => id,
            Self::Permission(prompt) => &prompt.id,
            Self::Tool(card) => &card.id,
            Self::Notice { text, .. } => text,
        }
    }
}

/// An approval the agent is waiting on.
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionPrompt {
    /// Row id; unique per request.
    pub id: String,
    /// `file edit`, `bash` or `web`.
    pub kind: String,
    /// The tool that wants to run.
    pub tool_name: String,
    /// Primary line: the path or command summary.
    pub summary: String,
    /// Secondary line: the full command or absolute path.
    pub detail: String,
    /// True once answered; the card stays as a record of the decision.
    pub resolved: bool,
    /// What the user answered, once resolved.
    pub outcome: Option<PermissionOutcome>,
}

/// What the user did with a permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOutcome {
    /// Allowed this one call.
    AllowOnce,
    /// Allowed every call of this kind for the session.
    AllowSession,
    /// Refused.
    Deny,
}

impl PermissionOutcome {
    /// Short label shown on the resolved card.
    pub fn label(self) -> &'static str {
        match self {
            Self::AllowOnce => "allowed",
            Self::AllowSession => "allowed for this session",
            Self::Deny => "denied",
        }
    }
}

/// Severity of a notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    /// Status or progress.
    Info,
    /// Something to act on.
    Warning,
    /// Something went wrong.
    Error,
}

/// Where a tool call stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// Waiting to start.
    Pending,
    /// Running.
    InProgress,
    /// Finished successfully.
    Completed,
    /// Failed or rejected.
    Failed,
}

/// What kind of action a tool performs; picks its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    /// Reads files or data.
    Read,
    /// Changes files.
    Edit,
    /// Deletes files.
    Delete,
    /// Moves or renames files.
    Move,
    /// Searches files or data.
    Search,
    /// Runs a command.
    Execute,
    /// Fetches from the network.
    Fetch,
    /// Internal reasoning step.
    Think,
    /// Anything else.
    Other,
}

/// A compact card describing one tool call.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCard {
    /// Tool call id.
    pub id: String,
    /// The tool's own name, when known.
    pub name: Option<String>,
    /// Short description, such as `edit src/main.rs`.
    pub title: String,
    /// Icon kind.
    pub kind: ToolKind,
    /// Current status.
    pub status: ToolStatus,
    /// Output so far, as plain text.
    pub output: String,
    /// The arguments JSON, pretty printed when available.
    pub input: String,
    /// A unified diff, when the tool changed a file.
    pub diff: Option<DiffContent>,
}

/// A change a tool made to one file.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffContent {
    /// File path.
    pub path: String,
    /// Unified diff text.
    pub diff: String,
}

impl ToolCard {
    /// Fold a partial update into this card, keeping fields the update left
    /// empty. `status` always wins because updates carry real end states.
    pub fn merge_update(&mut self, new: ToolCard) {
        if new.name.is_some() {
            self.name = new.name;
        }
        if !new.title.is_empty() {
            self.title = new.title;
        }
        if new.kind != ToolKind::Other {
            self.kind = new.kind;
        }
        self.status = new.status;
        if !new.output.is_empty() {
            self.output = new.output;
        }
        if !new.input.is_empty() {
            self.input = new.input;
        }
        if new.diff.is_some() {
            self.diff = new.diff;
        }
    }
}

/// One past session in the sidebar.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionEntry {
    /// Session id.
    pub id: String,
    /// Display name (first user prompt or a stored name).
    pub title: String,
    /// Unix ms.
    pub created_at: i64,
    /// True when a turn is running in this session.
    pub busy: bool,
}

impl SessionEntry {
    /// "Today", "Yesterday" or "Earlier".
    pub fn day_group(&self, today: chrono::NaiveDate) -> &'static str {
        let Some(date) = unix_ms_to_date(self.created_at) else {
            return "Earlier";
        };
        if date == today {
            "Today"
        } else if date == today.pred_opt().unwrap_or(today) {
            "Yesterday"
        } else {
            "Earlier"
        }
    }
}

/// Convert unix milliseconds to a local calendar date.
pub fn unix_ms_to_date(ms: i64) -> Option<chrono::NaiveDate> {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.with_timezone(&chrono::Local).date_naive())
}

/// Everything the UI needs to draw one frame.
/// One selectable model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    /// Provider id.
    pub provider: String,
    /// Model id.
    pub id: String,
}

/// State of the model and sign-in dialog.
#[derive(Debug, Default)]
pub struct ConnectDialog {
    /// True while the dialog is visible.
    pub open: bool,
    /// Index into the provider list.
    pub provider: usize,
    /// The key being typed; cleared after every attempt.
    pub key: String,
    /// True while a login request is in flight.
    pub busy: bool,
    /// Result of the last attempt: (ok, message).
    pub message: Option<(bool, String)>,
}

#[derive(Debug, Default)]
pub struct AppState {
    /// Set by the view when the user toggled the changes panel; main applies it.
    pub pending_changes_open: Option<bool>,
    /// Models the user can switch to.
    pub models: Vec<ModelEntry>,
    /// `provider/id` of the selected model.
    pub current_model: Option<String>,
    /// The model and sign-in dialog.
    pub connect: ConnectDialog,
    /// Rows of the active conversation, oldest first.
    pub rows: Vec<ChatRow>,
    /// Known sessions for the project, newest first.
    pub sessions: Vec<SessionEntry>,
    /// Id of the active session.
    pub active_session: String,
    /// Title of the active session.
    pub session_title: String,
    /// Absolute project directory.
    pub project_dir: String,
    /// Label of the model prompts go to, when known.
    pub model_label: Option<String>,
    /// True while a turn runs in the active session.
    pub working: bool,
    /// Token usage of the latest request.
    pub usage: UsageSnapshot,
    /// Notices shown once at startup.
    pub startup_notices: Vec<String>,
    /// True when the engine could not start or failed; the text explains why.
    pub error: Option<String>,
    /// Composer draft.
    pub draft: String,
    /// Expanded tool cards, by id.
    pub expanded_tools: HashMap<String, bool>,
    /// Expanded thought rows, by id.
    pub expanded_thoughts: HashMap<String, bool>,
    /// True while the chat should follow new content.
    pub auto_scroll: bool,
    /// Files the agent changed in this session, newest first.
    pub changed_files: Vec<ChangedFile>,
    /// The change selected in the right-hand panel, when open.
    pub selected_change: Option<usize>,
    /// The permission mode in effect.
    pub permission_mode: PermissionMode,
    /// The pending approval the engine is waiting on, if any.
    pub pending_approval: Option<PermissionPrompt>,
}

impl AppState {
    /// Add a row, replacing an earlier row with the same key.
    ///
    /// Tool cards merge instead of replacing: a partial update keeps the
    /// fields the earlier card already had.
    pub fn upsert_row(&mut self, row: ChatRow) {
        // A fresh approval request becomes the pending prompt; a resolved one
        // only refreshes its card.
        if let ChatRow::Permission(prompt) = &row {
            if !prompt.resolved {
                self.pending_approval = Some(prompt.clone());
            }
        }
        let key = row.key().to_owned();
        if let Some(slot) = self.rows.iter_mut().find(|r| r.key() == key) {
            match (slot, row) {
                (ChatRow::Tool(old), ChatRow::Tool(new)) => old.merge_update(new),
                (dst, src) => *dst = src,
            }
        } else {
            self.rows.push(row);
        }
    }

    /// Append text to the row with this id, creating it when missing.
    pub fn append_text(&mut self, id: &str, text: &str, agent: bool) {
        if let Some(ChatRow::Agent {
            text: existing, ..
        }) = self.rows.iter_mut().find(|r| r.key() == id)
        {
            existing.push_str(text);
            return;
        }
        // A stream arrives as many chunks, possibly under changing ids; the
        // open message at the end of the chat is the one they belong to.
        if let Some(ChatRow::Agent { text: existing, done: false, .. }) = self.rows.last_mut() {
            existing.push_str(text);
            return;
        }
        if agent {
            self.rows.push(ChatRow::Agent {
                id: id.to_owned(),
                text: text.to_owned(),
                done: false,
            });
        }
    }

    /// Append text to the thought row with this id, creating it when missing.
    pub fn append_thought(&mut self, id: &str, text: &str) {
        if let Some(ChatRow::Thought { text: existing, .. }) =
            self.rows.iter_mut().find(|r| r.key() == id)
        {
            existing.push_str(text);
            return;
        }
        if let Some(ChatRow::Thought { text: existing, .. }) = self.rows.last_mut() {
            existing.push_str(text);
            return;
        }
        self.rows.push(ChatRow::Thought {
            id: id.to_owned(),
            text: text.to_owned(),
            seconds: 0.0,
        });
    }

    /// Mark every unfinished agent row as done.
    pub fn finish_messages(&mut self) {
        for row in &mut self.rows {
            if let ChatRow::Agent { done, .. } = row {
                *done = true;
            }
        }
    }

    /// Record a file change from a tool call's diff, replacing the entry for
    /// the same path so the panel lists each file once with its latest diff.
    pub fn record_change(&mut self, path: String, diff: String) {
        let change = ChangedFile::from_diff(path.clone(), diff);
        self.changed_files.retain(|c| c.path != path);
        self.changed_files.insert(0, change);
        if self.selected_change.is_none() {
            self.selected_change = Some(0);
        }
    }

    /// Replace the conversation wholesale (session switch or new session).
    pub fn reset_conversation(&mut self, session_id: String, title: String, rows: Vec<ChatRow>) {
        self.active_session = session_id;
        self.session_title = title;
        self.rows = rows;
        self.working = false;
        self.pending_approval = None;
        // Rebuild the changed-file list from the loaded history's diffs.
        self.changed_files.clear();
        self.selected_change = None;
        let changes: Vec<(String, String)> = self
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
            self.record_change(path, diff);
        }
    }

    /// Answer the pending approval, marking its card resolved.
    pub fn resolve_approval(&mut self, id: &str, outcome: PermissionOutcome) {
        let Some(mut prompt) = self.pending_approval.take() else {
            return;
        };
        if prompt.id != id {
            // A different request arrived meanwhile; keep it pending.
            self.pending_approval = Some(prompt);
            return;
        }
        prompt.resolved = true;
        prompt.outcome = Some(outcome);
        self.upsert_row(ChatRow::Permission(prompt));
    }

    /// A short title for a session from its first user prompt.
    pub fn title_from_prompt(prompt: &str) -> String {
        let mut line = prompt
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("New session")
            .trim()
            .to_owned();
        if line.chars().count() > 42 {
            let cut: String = line.chars().take(42).collect();
            line = format!("{cut}…");
        }
        line
    }
}

/// Token usage of the latest request.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UsageSnapshot {
    /// Tokens in the context after the latest request.
    pub total_tokens: u64,
    /// The model's context window, when known.
    pub context_window: Option<u64>,
    /// Cost of the latest request in US dollars, when known.
    pub cost_usd: Option<f64>,
}

#[cfg(test)]
mod stream_tests {
    use super::*;

    #[test]
    fn chunks_under_changing_ids_form_one_message() {
        let mut state = AppState::default();
        for (i, piece) in ["Hey", "!", " I", "'m ready"].iter().enumerate() {
            state.append_text(&format!("m{i}"), piece, true);
        }
        let agents: Vec<_> = state
            .rows
            .iter()
            .filter_map(|r| match r {
                ChatRow::Agent { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(agents, vec!["Hey! I'm ready".to_owned()]);
        state.finish_messages();
        state.append_text("next", "New message", true);
        assert_eq!(state.rows.len(), 2, "a finished message is not extended");
    }
}
