//! Two small value types that grew out of `state.rs`: the permission mode
//! and the per-file change record the Changes panel lists.

/// How much the agent may do without asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// Reading, searching and the web only.
    ReadOnly,
    /// Ask before edits and commands.
    #[default]
    Ask,
    /// Edit files in the workspace without asking; ask for commands.
    AcceptEdits,
    /// Run every tool call without asking.
    FullAccess,
}

impl PermissionMode {
    /// Every mode, strictest first.
    pub const ALL: [Self; 4] = [
        Self::ReadOnly,
        Self::Ask,
        Self::AcceptEdits,
        Self::FullAccess,
    ];

    /// Short label for the chip.
    pub fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::Ask => "ask before edits",
            Self::AcceptEdits => "accept edits",
            Self::FullAccess => "full access",
        }
    }

    /// One line of explanation for the picker.
    pub fn description(self) -> &'static str {
        match self {
            Self::ReadOnly => "read, search and the web only; no edits or commands",
            Self::Ask => "ask before edits and commands",
            Self::AcceptEdits => "edit files in this workspace without asking; ask for commands",
            Self::FullAccess => "run every tool call without asking",
        }
    }

    /// The settings/flag name of this mode.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::Ask => "ask",
            Self::AcceptEdits => "accept-edits",
            Self::FullAccess => "full-access",
        }
    }

    /// Parse a settings or engine value.
    pub fn from_str(value: &str) -> Option<Self> {
        Some(
            match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
                "read-only" | "readonly" => Self::ReadOnly,
                "ask" | "default" => Self::Ask,
                "accept-edits" | "acceptedits" => Self::AcceptEdits,
                "full-access" | "fullaccess" => Self::FullAccess,
                _ => return None,
            },
        )
    }
}

/// One file the agent changed, newest change last.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangedFile {
    /// File path as the engine reported it.
    pub path: String,
    /// Unified diff text of the latest change to this file.
    pub diff: String,
    /// Added lines in the latest diff.
    pub added: usize,
    /// Removed lines in the latest diff.
    pub removed: usize,
}

impl ChangedFile {
    /// Build from a unified diff, counting its added and removed lines.
    pub fn from_diff(path: String, diff: String) -> Self {
        let mut added = 0;
        let mut removed = 0;
        for line in diff.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                added += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                removed += 1;
            }
        }
        Self {
            path,
            diff,
            added,
            removed,
        }
    }
}
