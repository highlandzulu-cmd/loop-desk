//! The approval flow: engine prompts in, UI decisions out.

use loop_app_core::tool_approval::{ApprovalDecision, ApprovalKind};

use super::Actor;
use crate::protocol::UiEvent;
use crate::state::{
    ChatRow, PermissionOutcome, PermissionPrompt,
};

impl Actor {
    /// A tool call is waiting on the user.
    pub(super) fn on_approval(
        &mut self,
        prompt: loop_app_core::tool_approval::ApprovalPrompt,
        send: &impl Fn(UiEvent),
    ) {
        // The reply channel lives here until the UI answers; dropping it
        // would reject the call.
        let id = format!("approval-{}", self.approval_seq);
        self.approval_seq += 1;
        self.pending_response_tx = Some(prompt.response_tx);
        self.pending_approval_id = Some(id.clone());
        let row = ChatRow::Permission(PermissionPrompt {
            id,
            kind: approval_kind_label(prompt.kind).to_owned(),
            tool_name: prompt.tool_name,
            summary: prompt.summary,
            detail: prompt.detail,
            resolved: false,
            outcome: None,
        });
        self.pending_approval_row = Some(row.clone());
        self.pending_approval_session = Some(self.rt.session_id.clone());
        send(UiEvent::ApprovalRequested(row));
    }

    /// Answer the pending approval, if it is still the one being asked.
    pub(super) fn approve(&mut self, id: String, outcome: PermissionOutcome) {
        let matches_current = self
            .pending_approval_id
            .as_deref()
            .is_some_and(|current| current == id);
        if !matches_current {
            return;
        }
        self.pending_approval_id = None;
        self.pending_approval_row = None;
        self.pending_approval_session = None;
        let Some(tx) = self.pending_response_tx.take() else {
            return;
        };
        let decision = match outcome {
            PermissionOutcome::AllowOnce => ApprovalDecision::Accept,
            PermissionOutcome::AllowSession => ApprovalDecision::AcceptSession,
            PermissionOutcome::Deny => ApprovalDecision::Reject { reason: None },
        };
        let _ = tx.send(decision);
    }
}

/// UI label for an approval kind.
pub(crate) fn approval_kind_label(kind: ApprovalKind) -> &'static str {
    match kind {
        ApprovalKind::FileEdit => "file edit",
        ApprovalKind::Bash => "bash",
        ApprovalKind::Web => "web",
    }
}
