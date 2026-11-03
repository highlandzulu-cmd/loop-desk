//! Drawing code shared by the live app and the demo screenshot mode.

pub mod changes;
pub mod chat;
pub mod composer;
pub mod connect;
pub mod diff;
pub mod icons;
pub mod markdown;
pub mod sidebar;
pub mod terminal;
pub mod theme;
pub mod tool_card;

use crate::state::{AppState, PermissionMode};
use egui::{Align, Layout};

/// What the user asked for while drawing a frame.
#[derive(Debug, Clone, Default)]
pub enum ViewAction {
    /// Nothing to do.
    #[default]
    None,
    /// Start a new session.
    NewSession,
    /// Open this session.
    SelectSession(String),
    /// The composer acted.
    Composer(composer::ComposerAction),
    /// The user answered an approval.
    Approve {
        /// Which request.
        id: String,
        /// The decision.
        outcome: crate::state::PermissionOutcome,
    },
    /// Switch the permission mode.
    SetPermissionMode(PermissionMode),
    /// Show or hide the terminal dock.
    ToggleTerminal,
    /// Show or hide the changes panel.
    ToggleChanges,
    /// Switch between the light and dark theme.
    ToggleTheme,
    /// The model and sign-in dialog asked for something.
    Connect(connect::ConnectAction),
}

/// Everything needed to draw the whole window from [`AppState`].
#[derive(Debug, Clone)]
pub struct ChatView {
    /// Theme mode.
    pub mode: theme::ThemeMode,
    /// Window title shown in the sticky header.
    pub title: String,
    /// Sidebar visible.
    pub sidebar_open: bool,
    /// Right-hand "Changes" panel visible.
    pub changes_open: bool,
}

impl ChatView {
    /// A view with the given mode and default chrome.
    pub fn new(mode: theme::ThemeMode) -> Self {
        Self {
            mode,
            title: "New session".to_owned(),
            sidebar_open: true,
            changes_open: false,
        }
    }

    /// Draw one full frame, reporting what the user asked for.
    pub fn show(
        self,
        ctx: &egui::Context,
        state: &mut AppState,
        mut dock: Option<&mut terminal::TerminalDock>,
    ) -> ViewAction {
        let mut view = self;
        let changes_before = view.changes_open;
        theme::apply(ctx, view.mode);
        let mut action = ViewAction::None;

        // Global shortcuts.
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::D) {
                view.changes_open = !view.changes_open;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Backtick) {
                if let Some(dock) = dock.as_deref_mut() {
                    dock.toggle(ctx);
                }
            }
        });

        if view.sidebar_open {
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .default_width(theme::SIDEBAR_WIDTH)
                .width_range(200.0..=340.0)
                .frame(
                    egui::Frame::new()
                        .fill(view.mode.palette().bg)
                        .inner_margin(theme::margin_sides(
                            theme::SPACE_12,
                            theme::SPACE_12,
                            theme::SPACE_8,
                            theme::SPACE_8,
                        ))
                        .stroke(theme::hairline(&view.mode.palette())),
                )
                .show(ctx, |ui| {
                    match sidebar::show(
                        ui,
                        &state.sessions,
                        &state.active_session,
                        &project_name(&state.project_dir),
                    ) {
                        sidebar::SidebarAction::None => {}
                        sidebar::SidebarAction::NewSession => action = ViewAction::NewSession,
                        sidebar::SidebarAction::Select(id) => {
                            state.active_session = id.clone();
                            action = ViewAction::SelectSession(id);
                        }
                    }
                });
        }

        // Terminal dock on the right edge, outside the conversation column.
        if let Some(dock) = dock.as_deref_mut() {
            if dock.open && dock.side == terminal::DockSide::Right {
                let p = view.mode.palette();
                let panel = egui::SidePanel::right("terminal-right")
                    .resizable(true)
                    .default_width(dock.size.max(dock.default_size()))
                    .width_range(260.0..=900.0)
                    .frame(
                        egui::Frame::new()
                            .fill(p.bg)
                            .inner_margin(theme::margin(theme::SPACE_8))
                            .stroke(theme::hairline(&p)),
                    )
                    .show(ctx, |ui| {
                        if dock.toolbar(ui, &p) {
                            dock.open = false;
                        }
                        ui.add_space(theme::SPACE_4);
                        dock.body(ui, view.mode);
                    });
                dock.size = panel.response.rect.width();
            }
        }

        // Changes panel, also on the right but inside the app's own panels.
        if view.changes_open {
            let p = view.mode.palette();
            egui::SidePanel::right("changes")
                .resizable(true)
                .default_width(theme::CHANGES_WIDTH)
                .width_range(280.0..=560.0)
                .frame(
                    egui::Frame::new()
                        .fill(p.bg)
                        .inner_margin(theme::margin(theme::SPACE_12))
                        .stroke(theme::hairline(&p)),
                )
                .show(ctx, |ui| {
                    if changes::show(ui, state) {
                        view.changes_open = false;
                    }
                });
        }

        // Terminal dock at the bottom.
        if let Some(dock) = dock.as_mut() {
            if dock.open && dock.side == terminal::DockSide::Bottom {
                let p = view.mode.palette();
                let panel = egui::TopBottomPanel::bottom("terminal-bottom")
                    .resizable(true)
                    .default_height(dock.size.max(dock.default_size()))
                    .height_range(140.0..=720.0)
                    .frame(
                        egui::Frame::new()
                            .fill(p.bg)
                            .inner_margin(theme::margin(theme::SPACE_8))
                            .stroke(theme::hairline(&p)),
                    )
                    .show(ctx, |ui| {
                        if dock.toolbar(ui, &p) {
                            dock.open = false;
                        }
                        ui.add_space(theme::SPACE_4);
                        dock.body(ui, view.mode);
                    });
                dock.size = panel.response.rect.height();
            }
        }

        let p = view.mode.palette();

        // Slim header (44 px), then the composer pinned at the bottom, then the conversation fills the rest.
        egui::TopBottomPanel::top("header")
            .exact_height(44.0)
            .frame(egui::Frame::new().fill(p.bg).stroke(egui::Stroke::new(1.0_f32, p.border)))
            .show(ctx, |ui| {
                if let Some(header_action) = header(ui, &view, state) {
                    match header_action {
                        HeaderAction::SetPermissionMode(mode) => {
                            action = ViewAction::SetPermissionMode(mode);
                        }
                        HeaderAction::ToggleTerminal => action = ViewAction::ToggleTerminal,
                        HeaderAction::ToggleChanges => action = ViewAction::ToggleChanges,
                        HeaderAction::ToggleTheme => action = ViewAction::ToggleTheme,
                    }
                }
            });

        // Engine failures (bad key, provider errors) show as a dismissible banner under the header.
        if let Some(message) = state.error.clone() {
            let mut dismiss = false;
            egui::TopBottomPanel::top("error-banner")
                .frame(
                    egui::Frame::new()
                        .fill(p.bg.lerp_to_gamma(p.danger, 0.16))
                        .inner_margin(theme::margin_sides(
                            theme::SPACE_16,
                            theme::SPACE_16,
                            theme::SPACE_8,
                            theme::SPACE_8,
                        )),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&message).font(theme::muted_font()).color(p.text),
                            )
                            .wrap(),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icons::button(ui, icons::Icon::Cross, p.text).clicked() {
                                dismiss = true;
                            }
                        });
                    });
                });
            if dismiss {
                state.error = None;
            }
        }

        egui::TopBottomPanel::bottom("composer")
            // A bottom panel is sized from the previous frame; reserving room up front keeps the first frame whole.
            .min_height(112.0)
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(theme::margin_sides(
                        theme::SPACE_16,
                        theme::SPACE_16,
                        theme::SPACE_8,
                        theme::SPACE_16,
                    )),
            )
            .show(ctx, |ui| {
                centered_column(ui, |ui| {
                    if let Some(composer_action) = composer::show(ui, state) {
                        action = ViewAction::Composer(composer_action);
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg))
            .show(ctx, |ui| {
                let output = egui::ScrollArea::vertical()
                    .id_salt("chat-scroll")
                    .auto_shrink([false, false])
                    .stick_to_bottom(state.auto_scroll)
                    .show(ui, |ui| {
                        ui.add_space(theme::SPACE_16);
                        centered_column(ui, |ui| {
                            if let Some(chat_action) = chat::show(ui, state) {
                                match chat_action {
                                    chat::ChatAction::Approve { id, outcome } => {
                                        action = ViewAction::Approve { id, outcome };
                                    }
                                }
                            }
                        });
                        ui.add_space(theme::SPACE_24);
                    });
                // Following the stream pauses when the user scrolls up and resumes at the bottom.
                let max_offset = (output.content_size.y - output.inner_rect.height()).max(0.0);
                state.auto_scroll = output.state.offset.y >= max_offset - 2.0;
            });

        if view.changes_open != changes_before {
            state.pending_changes_open = Some(view.changes_open);
        }
        if let Some(dialog) = connect::show(ctx, state, &p) {
            action = ViewAction::Connect(dialog);
        }

        action
    }
}

/// Run `add` inside a column of at most `CHAT_MAX_WIDTH`, centred in the available width.
fn centered_column(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    let column = (width - theme::SPACE_32).clamp(240.0, theme::CHAT_MAX_WIDTH);
    let pad = ((width - column) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.set_width(column);
            add(ui);
        });
    });
}

/// What the user clicked in the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderAction {
    /// A permission mode was picked.
    SetPermissionMode(PermissionMode),
    /// The terminal button.
    ToggleTerminal,
    /// The changes button.
    ToggleChanges,
    /// The light / dark button.
    ToggleTheme,
}

/// The slim top bar: session title on the left, usage, the permission mode
/// picker and the panel toggles on the right.
fn header(ui: &mut egui::Ui, view: &ChatView, state: &mut AppState) -> Option<HeaderAction> {
    let p = view.mode.palette();
    let mut action = None;
    ui.horizontal_centered(|ui| {
        ui.add_space(theme::SPACE_16);
        let title = if state.session_title.is_empty() {
            view.title.as_str()
        } else {
            state.session_title.as_str()
        };
        ui.label(egui::RichText::new(title).size(theme::TEXT_UI).strong().color(p.text));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(theme::SPACE_16);

            // Light / dark toggle, then the panel toggles: terminal and changes.
            let dark = matches!(view.mode, theme::ThemeMode::Dark);
            if icon_toggle(
                ui,
                &p,
                if dark { icons::Icon::Sun } else { icons::Icon::Moon },
                "Switch to light / dark",
                false,
            ) {
                action = Some(HeaderAction::ToggleTheme);
            }

            if icon_toggle(
                ui,
                &p,
                icons::Icon::Execute,
                "Terminal (Ctrl+`)",
                false,
            ) {
                action = Some(HeaderAction::ToggleTerminal);
            }
            if icon_toggle(
                ui,
                &p,
                icons::Icon::Read,
                "Changes (Cmd+D)",
                view.changes_open,
            ) {
                action = Some(HeaderAction::ToggleChanges);
            }

            let menu = icons::button(ui, icons::Icon::Dots, p.muted);
            let _ = menu;

            // Permission mode chip with a popup picker.
            let chip = ui.add(
                egui::Button::new(
                    egui::RichText::new(state.permission_mode.label())
                        .font(theme::caption_font())
                        .color(p.text),
                )
                .fill(p.surface_2)
                .corner_radius(theme::chip_radius()),
            );
            let popup_id = ui.make_persistent_id("permission-mode-popup");
            if chip.clicked() {
                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
            }
            egui::popup::popup_below_widget(
                ui,
                popup_id,
                &chip,
                egui::popup::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.set_min_width(300.0);
                    ui.label(
                        egui::RichText::new("Permission mode")
                            .font(theme::caption_font())
                            .color(p.muted),
                    );

                    ui.add_space(theme::SPACE_4);
                    for mode in PermissionMode::ALL {
                        let selected = mode == state.permission_mode;
                        let row = ui.add(
                            egui::Button::new(
                                egui::RichText::new(format!(
                                    "{} — {}",
                                    mode.label(),
                                    mode.description()
                                ))
                                .font(theme::muted_font())
                                .color(if selected { p.accent } else { p.text }),
                            )
                            .fill(if selected {
                                p.accent_soft
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .corner_radius(theme::chip_radius()),
                        );
                        let picked_mode = row.clicked();
                        row.on_hover_cursor(egui::CursorIcon::PointingHand);
                        if picked_mode {
                            action = Some(HeaderAction::SetPermissionMode(mode));
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                    }
                },
            );
            chip.on_hover_cursor(egui::CursorIcon::PointingHand);

            let usage = &state.usage;
            if usage.total_tokens > 0 {
                let label = match usage.context_window {
                    Some(window) => format!("{} / {} tokens", usage.total_tokens, window),
                    None => format!("{} tokens", usage.total_tokens),
                };
                ui.label(egui::RichText::new(label).font(theme::caption_font()).color(p.muted));
                ui.add_space(theme::SPACE_8);
            }
        });
    });
    action
}

/// A small square icon button that looks pressed when `active`. Returns true
/// when clicked.
fn icon_toggle(
    ui: &mut egui::Ui,
    p: &theme::Palette,
    icon: icons::Icon,
    tooltip: &str,
    active: bool,
) -> bool {
    let size = theme::ICON_SIZE + 6.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    if active {
        ui.painter().rect_filled(
            rect.expand(1.0),
            theme::chip_radius(),
            p.accent_soft,
        );
    }
    let color = if response.hovered() || active {
        p.accent
    } else {
        p.muted
    };
    icons::draw(
        ui.painter(),
        rect,
        icon,
        color,
    );
    let clicked = response.clicked();
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let _ = response.on_hover_text(tooltip);
    clicked
}

/// Last path component of the project directory.
pub fn project_name(dir: &str) -> String {
    std::path::Path::new(dir)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.to_owned())
}
