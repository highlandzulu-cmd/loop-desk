//! Colour handling: the 16 ANSI base colours, the 256-colour cube and
//! truecolor, plus the [`TermTheme`] public type.

use egui::Color32;
use vt100::Color as VtColor;

/// Visual theme for a terminal panel.
#[derive(Clone, Debug)]
pub struct TermTheme {
    /// Default background colour.
    pub bg: Color32,
    /// Default foreground (text) colour.
    pub fg: Color32,
    /// Cursor colour.
    pub cursor: Color32,
    /// Selection highlight colour.
    pub selection: Color32,
    /// The 16 base ANSI colours (0-7 normal, 8-15 bright).
    pub ansi: [Color32; 16],
    /// Font size in points.
    pub font_size: f32,
}

impl TermTheme {
    /// A light-on-dark theme similar to macOS Terminal's "Pro" preset.
    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(0x1e, 0x1e, 0x1e),
            fg: Color32::from_rgb(0xd6, 0xd6, 0xd6),
            cursor: Color32::from_rgb(0xd6, 0xd6, 0xd6),
            selection: Color32::from_rgba_premultiplied(60, 90, 150, 120),
            ansi: [
                Color32::from_rgb(0x00, 0x00, 0x00), // 0 black
                Color32::from_rgb(0xc2, 0x35, 0x35), // 1 red
                Color32::from_rgb(0x2f, 0xb1, 0x4d), // 2 green
                Color32::from_rgb(0xc5, 0x8a, 0x1f), // 3 yellow
                Color32::from_rgb(0x2f, 0x5c, 0xd0), // 4 blue
                Color32::from_rgb(0xa1, 0x3f, 0xd0), // 5 magenta
                Color32::from_rgb(0x1f, 0x9e, 0xb8), // 6 cyan
                Color32::from_rgb(0xc0, 0xc0, 0xc0), // 7 white
                Color32::from_rgb(0x66, 0x66, 0x66), // 8 bright black
                Color32::from_rgb(0xf0, 0x50, 0x50), // 9 bright red
                Color32::from_rgb(0x54, 0xd4, 0x6e), // 10 bright green
                Color32::from_rgb(0xf0, 0xb0, 0x30), // 11 bright yellow
                Color32::from_rgb(0x50, 0x80, 0xff), // 12 bright blue
                Color32::from_rgb(0xc7, 0x72, 0xf0), // 13 bright magenta
                Color32::from_rgb(0x40, 0xd0, 0xf0), // 14 bright cyan
                Color32::from_rgb(0xff, 0xff, 0xff), // 15 bright white
            ],
            font_size: 14.0,
        }
    }

    /// A dark-on-light theme similar to macOS Terminal's "Basic" preset.
    pub fn light() -> Self {
        Self {
            bg: Color32::from_rgb(0xff, 0xff, 0xff),
            fg: Color32::from_rgb(0x1a, 0x1a, 0x1a),
            cursor: Color32::from_rgb(0x1a, 0x1a, 0x1a),
            selection: Color32::from_rgba_premultiplied(150, 180, 240, 110),
            ansi: [
                Color32::from_rgb(0x00, 0x00, 0x00),
                Color32::from_rgb(0xa1, 0x27, 0x27),
                Color32::from_rgb(0x1d, 0x7f, 0x33),
                Color32::from_rgb(0xa5, 0x70, 0x10),
                Color32::from_rgb(0x1b, 0x3f, 0x9e),
                Color32::from_rgb(0x77, 0x2f, 0x9e),
                Color32::from_rgb(0x11, 0x73, 0x87),
                Color32::from_rgb(0x55, 0x55, 0x55),
                Color32::from_rgb(0x44, 0x44, 0x44),
                Color32::from_rgb(0xd1, 0x40, 0x40),
                Color32::from_rgb(0x30, 0xb5, 0x4a),
                Color32::from_rgb(0xd7, 0x99, 0x2a),
                Color32::from_rgb(0x37, 0x62, 0xe0),
                Color32::from_rgb(0x9b, 0x52, 0xe0),
                Color32::from_rgb(0x28, 0xb9, 0xd7),
                Color32::from_rgb(0x88, 0x88, 0x88),
            ],
            font_size: 14.0,
        }
    }
}

impl Default for TermTheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Maps a vt100 foreground/background colour to an actual [`Color32`].
///
/// * `Default` uses `fallback` (the theme's `fg` or `bg`).
/// * `Idx(0..=15)` picks from the theme's ANSI palette.  For the default
///   background (index 7 used as bg by some programs is still mapped
///   literally) we keep the literal palette entry, except that index 0 used
///   as a background maps to the theme background so that inverse video of
///   plain text looks right.
/// * `Idx(16..=255)` uses the xterm 256-colour cube.
/// * `Rgb` is truecolor.
pub fn resolve_color(color: VtColor, theme: &TermTheme, fallback: Color32) -> Color32 {
    match color {
        VtColor::Default => fallback,
        VtColor::Idx(idx) => idx_to_color(idx, theme),
        VtColor::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

/// The xterm 256-colour palette: 0-15 are the theme's base colours, 16-231 a
/// 6x6x6 colour cube and 232-255 a grayscale ramp.
pub fn idx_to_color(idx: u8, theme: &TermTheme) -> Color32 {
    match idx {
        0..=15 => theme.ansi[usize::from(idx)],
        16..=231 => {
            let n = u32::from(idx) - 16;
            let r = cube_level(n / 36);
            let g = cube_level((n / 6) % 6);
            let b = cube_level(n % 6);
            Color32::from_rgb(r, g, b)
        }
        232..=255 => {
            let level = 8 + 10 * (u32::from(idx) - 232);
            let l = level.clamp(0, 255) as u8;
            Color32::from_gray(l)
        }
    }
}

fn cube_level(level: u32) -> u8 {
    match level {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> TermTheme {
        TermTheme::dark()
    }

    #[test]
    fn base_colors_come_from_theme() {
        assert_eq!(idx_to_color(0, &theme()), theme().ansi[0]);
        assert_eq!(idx_to_color(15, &theme()), theme().ansi[15]);
    }

    #[test]
    fn color_cube_corners() {
        // 16 is black, 231 is white, 21 is pure blue.
        assert_eq!(idx_to_color(16, &theme()), Color32::from_rgb(0, 0, 0));
        assert_eq!(idx_to_color(21, &theme()), Color32::from_rgb(0, 0, 255));
        assert_eq!(
            idx_to_color(196, &theme()),
            Color32::from_rgb(255, 0, 0)
        );
        assert_eq!(
            idx_to_color(231, &theme()),
            Color32::from_rgb(255, 255, 255)
        );
        // A middle cube entry: 16 + 36*2 + 6*3 + 1 = 107 -> (135,175,95)
        assert_eq!(
            idx_to_color(107, &theme()),
            Color32::from_rgb(135, 175, 95)
        );
    }

    #[test]
    fn grayscale_ramp() {
        assert_eq!(
            idx_to_color(232, &theme()),
            Color32::from_gray(8)
        );
        assert_eq!(
            idx_to_color(255, &theme()),
            Color32::from_gray(238)
        );
    }

    #[test]
    fn default_and_truecolor() {
        let t = theme();
        let fg = Color32::from_rgb(1, 2, 3);
        assert_eq!(resolve_color(VtColor::Default, &t, fg), fg);
        assert_eq!(
            resolve_color(VtColor::Rgb(10, 20, 30), &t, fg),
            Color32::from_rgb(10, 20, 30)
        );
        assert_eq!(
            resolve_color(VtColor::Idx(9), &t, fg),
            t.ansi[9]
        );
    }
}
