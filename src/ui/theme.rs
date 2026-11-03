//! Design tokens. Every colour, spacing and radius used by the UI lives here.

use egui::{Color32, CornerRadius, FontId, Margin, Stroke, Vec2};

/// Spacing scale in px.
pub const SPACE_4: f32 = 4.0;
/// 8 px.
pub const SPACE_8: f32 = 8.0;
/// 12 px.
pub const SPACE_12: f32 = 12.0;
/// 16 px.
pub const SPACE_16: f32 = 16.0;
/// 24 px.
pub const SPACE_24: f32 = 24.0;
/// 32 px.
pub const SPACE_32: f32 = 32.0;

/// Corner radius for chips and small buttons.
pub const RADIUS_SM: u8 = 6;
/// Corner radius for cards and inputs.
pub const RADIUS_MD: u8 = 10;
/// Corner radius for the composer and user bubbles.
pub const RADIUS_LG: u8 = 16;

/// Sidebar width in px.
pub const SIDEBAR_WIDTH: f32 = 260.0;
/// Conversation column max width in px.
pub const CHAT_MAX_WIDTH: f32 = 760.0;
/// Changes panel default width in px.
pub const CHANGES_WIDTH: f32 = 380.0;

/// Caption text (12 px).
pub const TEXT_CAPTION: f32 = 12.0;
/// UI text (13.5 px).
pub const TEXT_UI: f32 = 13.5;
/// Chat body text (15 px).
pub const TEXT_BODY: f32 = 15.0;
/// Section title (17 px).
pub const TEXT_TITLE: f32 = 17.0;
/// Empty-state greeting (22 px).
pub const TEXT_GREETING: f32 = 22.0;
/// Monospace code (13 px).
pub const TEXT_MONO: f32 = 13.0;
/// Chat line height multiplier.
pub const LINE_HEIGHT: f32 = 1.6;

/// One palette; light or dark.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// App background.
    pub bg: Color32,
    /// Cards and panels.
    pub surface: Color32,
    /// Subtle fill (chips, code blocks, user bubbles).
    pub surface_2: Color32,
    /// 1px lines.
    pub border: Color32,
    /// Primary text.
    pub text: Color32,
    /// Secondary text.
    pub muted: Color32,
    /// Accent (terracotta).
    pub accent: Color32,
    /// Accent tint for pills and focus rings.
    pub accent_soft: Color32,
    /// Success.
    pub success: Color32,
    /// Danger.
    pub danger: Color32,
    /// Added-line background.
    pub diff_add: Color32,
    /// Removed-line background.
    pub diff_del: Color32,
}

/// The light palette.
pub const LIGHT: Palette = Palette {
    bg: hex(0xF5F4EE),
    surface: hex(0xFFFFFF),
    surface_2: hex(0xEFEDE6),
    border: hex(0xE3E0D6),
    text: hex(0x1F1E1D),
    muted: hex(0x6B6A68),
    accent: hex(0x2F55D4),
    accent_soft: hex(0xDFE6F8),
    success: hex(0x4A7C59),
    danger: hex(0xB3402F),
    diff_add: hex(0xE6F2E8),
    diff_del: hex(0xF8E5E1),
};

/// The dark palette.
pub const DARK: Palette = Palette {
    bg: hex(0x1F1E1D),
    surface: hex(0x262624),
    surface_2: hex(0x2E2D2A),
    border: hex(0x3A3936),
    text: hex(0xECEBE6),
    muted: hex(0x9C9A92),
    accent: hex(0x6C8CF0),
    accent_soft: hex(0x232C4A),
    success: hex(0x7FB08A),
    danger: hex(0xE0705F),
    diff_add: hex(0x23352A),
    diff_del: hex(0x3A2523),
};

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb(
        ((rgb >> 16) & 0xFF) as u8,
        ((rgb >> 8) & 0xFF) as u8,
        (rgb & 0xFF) as u8,
    )
}

/// Which palette is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    /// Warm paper.
    Light,
    /// Soft charcoal.
    Dark,
}

impl ThemeMode {
    /// The palette for this mode.
    pub fn palette(self) -> Palette {
        match self {
            Self::Light => LIGHT,
            Self::Dark => DARK,
        }
    }
}

/// A slightly darker fill for hover states.
pub fn hover_fill(p: &Palette) -> Color32 {
    mix(p.surface_2, p.muted, 0.18)
}

/// Mix two colours; `t` is the weight of `b`.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let lerp = |x: u8, y: u8| x as f32 + (y as f32 - x as f32) * t;
    Color32::from_rgb(
        lerp(a.r(), b.r()) as u8,
        lerp(a.g(), b.g()) as u8,
        lerp(a.b(), b.b()) as u8,
    )
}

/// Standard 1px border.
pub fn hairline(p: &Palette) -> Stroke {
    Stroke::new(1.0_f32, p.border)
}

/// Card padding.
pub fn card_margin() -> Margin {
    Margin {
        left: SPACE_12 as i8,
        right: SPACE_12 as i8,
        top: SPACE_8 as i8,
        bottom: SPACE_8 as i8,
    }
}

/// Symmetric margin from the spacing scale, in px.
pub fn margin(px: f32) -> Margin {
    Margin::same(px as i8)
}

/// A margin with each side set separately, in px.
pub fn margin_sides(left: f32, right: f32, top: f32, bottom: f32) -> Margin {
    Margin {
        left: left as i8,
        right: right as i8,
        top: top as i8,
        bottom: bottom as i8,
    }
}

/// Chat line height for a given font size.
pub fn line_height(font_size: f32) -> Option<f32> {
    Some(font_size * LINE_HEIGHT)
}

/// Chip corner radius.
pub fn chip_radius() -> CornerRadius {
    CornerRadius::same(RADIUS_SM)
}

/// Card corner radius.
pub fn card_radius() -> CornerRadius {
    CornerRadius::same(RADIUS_MD)
}

/// Composer / bubble corner radius.
pub fn soft_radius() -> CornerRadius {
    CornerRadius::same(RADIUS_LG)
}

/// Chat body font.
pub fn body_font() -> FontId {
    FontId::proportional(TEXT_BODY)
}

/// Muted chat font.
pub fn muted_font() -> FontId {
    FontId::proportional(TEXT_UI)
}

/// Caption font.
pub fn caption_font() -> FontId {
    FontId::proportional(TEXT_CAPTION)
}

/// Code font.
pub fn mono_font() -> FontId {
    FontId::monospace(TEXT_MONO)
}

/// Icon size used next to tool names.
pub const ICON_SIZE: f32 = 14.0;

/// A little wider than the smallest bubble, so one-word messages look sane.
pub const SPACE_64: f32 = 64.0;

/// Height of one sidebar session row.
pub const SESSION_ROW_HEIGHT: f32 = 36.0;

/// Tallest an expanded tool card body may grow before it scrolls.
pub const TOOL_OUTPUT_MAX_HEIGHT: f32 = 240.0;

/// Install the system sans font when it loads, else keep egui's default.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let sans = std::fs::read("/System/Library/Fonts/SFNS.ttf").ok();
    let mono = std::fs::read("/System/Library/Fonts/Menlo.ttc").ok();
    if let Some(bytes) = sans {
        fonts.font_data.insert(
            "loop-sans".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            list.insert(0, "loop-sans".to_owned());
        }
    }
    if let Some(bytes) = mono {
        fonts.font_data.insert(
            "loop-mono".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            list.insert(0, "loop-mono".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

/// Apply the whole visual style for `mode`.
pub fn apply(ctx: &egui::Context, mode: ThemeMode) {
    let p = mode.palette();
    let mut style = (*ctx.style()).clone();
    style.visuals = mode.egui_visuals();
    style.spacing.item_spacing = Vec2::new(SPACE_8, SPACE_8);
    style.spacing.button_padding = Vec2::new(SPACE_8, SPACE_4);
    style.spacing.window_margin = margin(SPACE_12);
    ctx.set_style(style);
    let _ = p;
}

impl ThemeMode {
    fn egui_visuals(self) -> egui::Visuals {
        let p = self.palette();
        let mut v = match self {
            Self::Light => egui::Visuals::light(),
            Self::Dark => egui::Visuals::dark(),
        };
        v.panel_fill = p.bg;
        v.window_fill = p.surface;
        v.extreme_bg_color = p.surface_2;
        v.faint_bg_color = p.surface_2;
        v.code_bg_color = p.surface_2;
        v.hyperlink_color = p.accent;
        v.override_text_color = Some(p.text);
        v.widgets.noninteractive.bg_fill = p.surface;
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
        v.widgets.inactive.bg_fill = p.surface_2;
        v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
        v.widgets.hovered.bg_fill = hover_fill(&p);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text);
        v.widgets.active.bg_fill = hover_fill(&p);
        v.widgets.active.fg_stroke = Stroke::new(1.2_f32, p.text);
        v.selection.bg_fill = p.accent_soft;
        v.selection.stroke = Stroke::new(1.0_f32, p.text);
        v.window_stroke = hairline(&p);
        v.widgets.noninteractive.bg_stroke = hairline(&p);
        v.widgets.inactive.bg_stroke = hairline(&p);
        v.widgets.hovered.bg_stroke = hairline(&p);
        v.widgets.active.bg_stroke = hairline(&p);
        v
    }
}
