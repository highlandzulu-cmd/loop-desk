//! Turning protocol updates into the UI's plain data.

use loop_protocol::{ToolCallStatus, ToolContent};

use crate::state::{
    NoticeLevel as UiNotice, ToolCard, ToolKind as UiToolKind, ToolStatus as UiToolStatus,
};

/// Build a card from a full tool call.
pub(crate) fn card_from_call(call: &loop_protocol::ToolCall) -> ToolCard {
    ToolCard {
        id: call.id.0.clone(),
        name: call.name.clone(),
        title: call.title.clone(),
        kind: tool_kind(call.kind),
        status: tool_status(call.status),
        output: text_content(&call.content),
        input: pretty_json(call.raw_input.as_ref()),
        diff: diff_content(&call.content),
    }
}

pub(crate) fn tool_kind(kind: loop_protocol::ToolKind) -> UiToolKind {
    match kind {
        loop_protocol::ToolKind::Read => UiToolKind::Read,
        loop_protocol::ToolKind::Edit => UiToolKind::Edit,
        loop_protocol::ToolKind::Delete => UiToolKind::Delete,
        loop_protocol::ToolKind::Move => UiToolKind::Move,
        loop_protocol::ToolKind::Search => UiToolKind::Search,
        loop_protocol::ToolKind::Execute => UiToolKind::Execute,
        loop_protocol::ToolKind::Fetch => UiToolKind::Fetch,
        loop_protocol::ToolKind::Think => UiToolKind::Think,
        loop_protocol::ToolKind::Other => UiToolKind::Other,
    }
}

pub(crate) fn tool_status(status: ToolCallStatus) -> UiToolStatus {
    match status {
        ToolCallStatus::Pending => UiToolStatus::Pending,
        ToolCallStatus::InProgress => UiToolStatus::InProgress,
        ToolCallStatus::Completed => UiToolStatus::Completed,
        ToolCallStatus::Failed => UiToolStatus::Failed,
    }
}

pub(crate) fn notice_level(level: loop_protocol::NoticeLevel) -> UiNotice {
    match level {
        loop_protocol::NoticeLevel::Info => UiNotice::Info,
        loop_protocol::NoticeLevel::Warning => UiNotice::Warning,
        loop_protocol::NoticeLevel::Error => UiNotice::Error,
        loop_protocol::NoticeLevel::Other => UiNotice::Info,
    }
}

/// Plain-text view of a tool call's output blocks.
pub(crate) fn text_content(content: &[ToolContent]) -> String {
    let mut out = String::new();
    for block in content {
        if let ToolContent::Content { content } = block {
            if let Some(text) = content.as_text() {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(text);
            }
        }
    }
    out
}

/// The first unified diff in a tool call's output, when there is one.
pub(crate) fn diff_content(content: &[ToolContent]) -> Option<crate::state::DiffContent> {
    content.iter().find_map(|block| match block {
        ToolContent::UnifiedDiff { path, diff } => Some(crate::state::DiffContent {
            path: path.clone(),
            diff: diff.clone(),
        }),
        _ => None,
    })
}

/// Pretty-print raw JSON arguments for the expanded card.
pub(crate) fn pretty_json(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_default()
}
