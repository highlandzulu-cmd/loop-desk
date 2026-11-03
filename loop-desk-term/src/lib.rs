//! `loop-desk-term`: an embeddable terminal panel for egui.
//!
//! The crate is intentionally small and self-contained so that it can be
//! copied into another application (e.g. as `src/ui/terminal.rs`) or used as
//! a path dependency.  See [`TerminalPanel`] for the entry point.

mod grid;
mod input;
mod keys;
mod palette;
mod pty;
mod render;

pub use palette::TermTheme;
pub use pty::TermTab;

use anyhow::Result;

/// A multi-tab terminal panel.
///
/// Create one with [`TerminalPanel::new`], then call
/// [`TerminalPanel::ui`] every frame from inside your egui code.
pub struct TerminalPanel {
    tabs: Vec<TermTab>,
    active: usize,
    cwd: std::path::PathBuf,
    ctx: egui::Context,
}

impl TerminalPanel {
    /// Creates the panel and starts the first tab running the user's login
    /// shell (`$SHELL`, falling back to `/bin/zsh`) in `cwd`.
    pub fn new(cwd: std::path::PathBuf, ctx: egui::Context) -> Result<Self> {
        let mut panel = Self {
            tabs: Vec::new(),
            active: 0,
            cwd,
            ctx,
        };
        panel.new_tab()?;
        Ok(panel)
    }

    /// Spawns a new tab running the login shell and makes it active.
    pub fn new_tab(&mut self) -> Result<()> {
        let tab = TermTab::spawn(self.cwd.clone(), self.ctx.clone())?;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        Ok(())
    }

    /// Closes the active tab (killing its shell).  Closing the last tab
    /// opens a fresh one.
    pub fn close_active_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let idx = self.active.min(self.tabs.len() - 1);
        self.close_tab_at(idx);
    }

    /// Number of open tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Size (rows, cols) of the active tab's terminal.
    pub fn terminal_size(&self) -> (u16, u16) {
        self.tabs
            .get(self.active_tab_index())
            .map(|t| t.parser().lock().screen().size())
            .unwrap_or((0, 0))
    }

    /// Index of the currently visible tab.
    pub fn active_tab_index(&self) -> usize {
        self.active.min(self.tabs.len().saturating_sub(1))
    }

    // ---- internals used by the renderer ----

    pub(crate) fn select_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active = idx;
        }
    }

    pub(crate) fn close_tab_at(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            // Closing the last tab opens a new one (best effort).
            let _ = self.new_tab();
            return;
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if idx < self.active {
            self.active -= 1;
        }
    }

    pub(crate) fn tab_titles(&self) -> Vec<String> {
        self.tabs.iter().map(|t| t.title()).collect()
    }

    pub(crate) fn active_tab_mut(&mut self) -> Option<&mut TermTab> {
        let idx = self.active_tab_index();
        self.tabs.get_mut(idx)
    }

    /// Draws the tab strip and the active terminal, and handles input and
    /// resizing.
    pub fn ui(&mut self, ui: &mut egui::Ui, theme: &TermTheme) {
        render::tab_strip(self, ui, theme);
        render::terminal(self, ui, theme);
    }
}
