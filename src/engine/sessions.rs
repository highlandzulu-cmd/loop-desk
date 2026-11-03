//! Reading the session store: listing sessions and loading history.

use std::sync::Arc;

use loop_protocol::{Role, SessionUpdate};

use super::translate::{card_from_call, notice_level};
use crate::state::{ChatRow, SessionEntry};

/// Sessions for the runtime's project directory, newest first.
pub(crate) async fn list_sessions(rt: &loop_app_core::Runtime) -> Vec<SessionEntry> {
    let cwd = rt.cwd.to_str().map(str::to_owned);
    list_session_metas(&rt.sessions_db, cwd.as_deref())
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|meta| SessionEntry {
            id: meta.id,
            title: meta.name.unwrap_or_else(|| "Untitled session".to_owned()),
            created_at: meta.created_at,
            busy: false,
        })
        .collect()
}

/// Session metadata for `cwd`, newest first. Plain async: it runs on the engine's own runtime.
pub(crate) async fn list_session_metas(
    db: &std::path::Path,
    cwd: Option<&str>,
) -> anyhow::Result<Vec<loop_agent::harness::session::SessionMetadata>> {
    let store = loop_agent::harness::create_sqlite_session_store(db)
        .map_err(|e| anyhow::anyhow!("sqlite session store: {e}"))?;
    let mut metas = store.list(cwd).await?;
    metas.sort_by_key(|m| std::cmp::Reverse(m.created_at));
    Ok(metas)
}

/// Open a session's metadata reader from the store.
pub(crate) async fn open_store_reader(
    db: &std::path::Path,
    id: &str,
) -> anyhow::Result<Arc<dyn loop_agent::harness::session::SessionReader>> {
    let store = loop_agent::harness::create_sqlite_session_store(db)
        .map_err(|e| anyhow::anyhow!("sqlite session store: {e}"))?;
    store
        .load(id)
        .await
        .map_err(|e| anyhow::anyhow!("load session: {e}"))
}

/// The active session's stored history as chat rows.
pub(crate) async fn transcript_rows(rt: &loop_app_core::Runtime) -> Vec<ChatRow> {
    let session = rt.session_ref();
    let Ok(updates) = rt.backend.transcript(&session).await else {
        return Vec::new();
    };
    let mut rows: Vec<ChatRow> = Vec::new();
    for update in updates {
        match update {
            SessionUpdate::MessageChunk {
                message_id,
                role,
                content,
            } => {
                let Some(text) = content.as_text() else { continue };
                match role {
                    Role::User => push_unique(
                        &mut rows,
                        ChatRow::User {
                            id: message_id.0,
                            text: text.to_owned(),
                        },
                    ),
                    Role::Agent => push_unique(
                        &mut rows,
                        ChatRow::Agent {
                            id: message_id.0,
                            text: text.to_owned(),
                            done: true,
                        },
                    ),
                }
            }
            SessionUpdate::ThoughtChunk { message_id, content } => {
                if let Some(text) = content.as_text() {
                    push_unique(
                        &mut rows,
                        ChatRow::Thought {
                            id: message_id.0,
                            text: text.to_owned(),
                            seconds: 0.0,
                        },
                    );
                }
            }
            SessionUpdate::ToolCall(call) => {
                push_unique(&mut rows, ChatRow::Tool(card_from_call(&call)))
            }
            SessionUpdate::Notice { level, text } => {
                push_unique(&mut rows, ChatRow::Notice {
                    level: notice_level(level),
                    text,
                })
            }
            _ => {}
        }
    }
    rows
}

/// Add a row, appending replayed text blocks into a row with the same id.
///
/// Replay ids are fresh per message and each chunk carries a whole text
/// block, so repeated ids are consecutive blocks of one message: append them.
fn push_unique(rows: &mut Vec<ChatRow>, row: ChatRow) {
    // Stored streams arrive as many small chunks; consecutive chunks of the
    // same kind belong to one message whatever their ids say.
    match (rows.last_mut(), &row) {
        (Some(ChatRow::Agent { text: dst, .. }), ChatRow::Agent { text: src, .. }) => {
            dst.push_str(src);
            return;
        }
        (Some(ChatRow::Thought { text: dst, .. }), ChatRow::Thought { text: src, .. }) => {
            dst.push_str(src);
            return;
        }
        _ => {}
    }
    let key = row.key().to_owned();
    if let Some(slot) = rows.iter_mut().find(|r| r.key() == key) {
        match (slot, row) {
            (ChatRow::Agent { text: dst, .. }, ChatRow::Agent { text: src, .. }) => {
                dst.push_str(&src);
            }
            (
                ChatRow::Thought { text: dst, .. },
                ChatRow::Thought { text: src, .. },
            ) => {
                dst.push_str(&src);
            }
            (dst, src) => *dst = src,
        }
    } else {
        rows.push(row);
    }
}

