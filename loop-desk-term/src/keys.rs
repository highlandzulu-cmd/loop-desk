//! Keyboard mapping: pure functions turning egui key events into the byte
//! sequences a terminal expects.

use egui::{Key, Modifiers};

/// Maps an egui key press (plus modifiers) to the bytes to send to the pty.
///
/// `app_cursor` selects application cursor mode (`ESC O A` instead of
/// `ESC [ A`).  Returns `None` for keys we do not handle (the caller then
/// ignores the event).
pub fn key_to_bytes(key: Key, modifiers: Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
    // Alt is handled as an ESC prefix below; Command/Ctrl combos that are not
    // terminal shortcuts are ignored entirely.
    if modifiers.command || modifiers.mac_cmd {
        return None;
    }

    if modifiers.ctrl && !modifiers.alt {
        return ctrl_key(key).map(|b| vec![b]);
    }

    // Letters normally arrive as Event::Text, but with Alt held egui reports
    // them as Key events; map them here so Alt+x sends ESC x.
    if modifiers.alt {
        if let Some(c) = letter_for(key) {
            return Some(vec![0x1b, c]);
        }
    }

    let mut seq = plain_key(key, app_cursor)?;

    if modifiers.alt {
        let mut out = Vec::with_capacity(seq.len() + 1);
        out.push(0x1b);
        out.extend_from_slice(&seq);
        seq = out;
    }
    Some(seq)
}

/// Text input, possibly with Alt (which becomes an ESC prefix).
pub fn text_to_bytes(text: &str, modifiers: Modifiers) -> Option<Vec<u8>> {
    if text.is_empty() || modifiers.ctrl || modifiers.command || modifiers.mac_cmd {
        return None;
    }
    let mut bytes = text.as_bytes().to_vec();
    if modifiers.alt {
        let mut out = Vec::with_capacity(bytes.len() + 1);
        out.push(0x1b);
        out.extend_from_slice(&bytes);
        bytes = out;
    }
    Some(bytes)
}

/// Wraps pasted text for bracketed paste when the application asked for it.
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        let mut out = Vec::with_capacity(text.len() + 12);
        out.extend_from_slice(b"\x1b[200~");
        out.extend_from_slice(text.as_bytes());
        out.extend_from_slice(b"\x1b[201~");
        out
    } else {
        text.as_bytes().to_vec()
    }
}

/// Lowercase ASCII letter for a letter key, if `key` is one.
fn letter_for(key: Key) -> Option<u8> {
    match key {
        Key::A => Some(b'a'),
        Key::B => Some(b'b'),
        Key::C => Some(b'c'),
        Key::D => Some(b'd'),
        Key::E => Some(b'e'),
        Key::F => Some(b'f'),
        Key::G => Some(b'g'),
        Key::H => Some(b'h'),
        Key::I => Some(b'i'),
        Key::J => Some(b'j'),
        Key::K => Some(b'k'),
        Key::L => Some(b'l'),
        Key::M => Some(b'm'),
        Key::N => Some(b'n'),
        Key::O => Some(b'o'),
        Key::P => Some(b'p'),
        Key::Q => Some(b'q'),
        Key::R => Some(b'r'),
        Key::S => Some(b's'),
        Key::T => Some(b't'),
        Key::U => Some(b'u'),
        Key::V => Some(b'v'),
        Key::W => Some(b'w'),
        Key::X => Some(b'x'),
        Key::Y => Some(b'y'),
        Key::Z => Some(b'z'),
        _ => None,
    }
}

fn ctrl_key(key: Key) -> Option<u8> {
    let code = match key {
        Key::A => b'a',
        Key::B => b'b',
        Key::C => b'c',
        Key::D => b'd',
        Key::E => b'e',
        Key::F => b'f',
        Key::G => b'g',
        Key::H => b'h',
        Key::I => b'i',
        Key::J => b'j',
        Key::K => b'k',
        Key::L => b'l',
        Key::M => b'm',
        Key::N => b'n',
        Key::O => b'o',
        Key::P => b'p',
        Key::Q => b'q',
        Key::R => b'r',
        Key::S => b's',
        Key::T => b't',
        Key::U => b'u',
        Key::V => b'v',
        Key::W => b'w',
        Key::X => b'x',
        Key::Y => b'y',
        Key::Z => b'z',
        // Ctrl+Space / Ctrl+@ are NUL.
        Key::Space => b' ',
        // Ctrl+[ ^[ , Ctrl+\ ^\ , Ctrl+] ^] , Ctrl+Enter ^M etc.
        Key::OpenBracket => b'[',
        Key::Backslash => b'\\',
        Key::CloseBracket => b']',
        _ => return None,
    };
    // Control codes are 0x00-0x1f; letters map to 1-26.
    Some(code & 0x1f)
}

fn plain_key(key: Key, app_cursor: bool) -> Option<Vec<u8>> {
    let seq: Vec<u8> = match key {
        Key::Enter => b"\r".to_vec(),
        Key::Backspace => b"\x7f".to_vec(),
        Key::Tab => b"\t".to_vec(),
        Key::Escape => b"\x1b".to_vec(),
        Key::Delete => b"\x1b[3~".to_vec(),
        Key::Home => if app_cursor {
            b"\x1bOH"
        } else {
            b"\x1b[H"
        }
        .to_vec(),
        Key::End => if app_cursor {
            b"\x1bOF"
        } else {
            b"\x1b[F"
        }
        .to_vec(),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
        Key::Insert => b"\x1b[2~".to_vec(),
        Key::ArrowUp => if app_cursor {
            b"\x1bOA"
        } else {
            b"\x1b[A"
        }
        .to_vec(),
        Key::ArrowDown => if app_cursor {
            b"\x1bOB"
        } else {
            b"\x1b[B"
        }
        .to_vec(),
        Key::ArrowRight => if app_cursor {
            b"\x1bOC"
        } else {
            b"\x1b[C"
        }
        .to_vec(),
        Key::ArrowLeft => if app_cursor {
            b"\x1bOD"
        } else {
            b"\x1b[D"
        }
        .to_vec(),
        Key::F1 => b"\x1bOP".to_vec(),
        Key::F2 => b"\x1bOQ".to_vec(),
        Key::F3 => b"\x1bOR".to_vec(),
        Key::F4 => b"\x1bOS".to_vec(),
        Key::F5 => b"\x1b[15~".to_vec(),
        Key::F6 => b"\x1b[17~".to_vec(),
        Key::F7 => b"\x1b[18~".to_vec(),
        Key::F8 => b"\x1b[19~".to_vec(),
        Key::F9 => b"\x1b[20~".to_vec(),
        Key::F10 => b"\x1b[21~".to_vec(),
        Key::F11 => b"\x1b[23~".to_vec(),
        Key::F12 => b"\x1b[24~".to_vec(),
        _ => return None,
    };
    Some(seq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_backspace_tab_escape() {
        assert_eq!(key_to_bytes(Key::Enter, Modifiers::NONE, false), Some(b"\r".to_vec()));
        assert_eq!(
            key_to_bytes(Key::Backspace, Modifiers::NONE, false),
            Some(b"\x7f".to_vec())
        );
        assert_eq!(key_to_bytes(Key::Tab, Modifiers::NONE, false), Some(b"\t".to_vec()));
        assert_eq!(
            key_to_bytes(Key::Escape, Modifiers::NONE, false),
            Some(b"\x1b".to_vec())
        );
    }

    #[test]
    fn arrows_normal_and_application_mode() {
        assert_eq!(
            key_to_bytes(Key::ArrowUp, Modifiers::NONE, false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::ArrowDown, Modifiers::NONE, false),
            Some(b"\x1b[B".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::ArrowLeft, Modifiers::NONE, false),
            Some(b"\x1b[D".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::ArrowRight, Modifiers::NONE, false),
            Some(b"\x1b[C".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::ArrowUp, Modifiers::NONE, true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::ArrowDown, Modifiers::NONE, true),
            Some(b"\x1bOB".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::ArrowLeft, Modifiers::NONE, true),
            Some(b"\x1bOD".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::ArrowRight, Modifiers::NONE, true),
            Some(b"\x1bOC".to_vec())
        );
    }

    #[test]
    fn ctrl_letters() {
        assert_eq!(
            key_to_bytes(Key::C, Modifiers::CTRL, false),
            Some(vec![0x03])
        );
        assert_eq!(
            key_to_bytes(Key::A, Modifiers::CTRL, false),
            Some(vec![0x01])
        );
        assert_eq!(
            key_to_bytes(Key::Z, Modifiers::CTRL, false),
            Some(vec![0x1a])
        );
        assert_eq!(
            key_to_bytes(Key::D, Modifiers::CTRL, false),
            Some(vec![0x04])
        );
    }

    #[test]
    fn alt_keys_get_esc_prefix() {
        assert_eq!(
            key_to_bytes(Key::X, Modifiers::ALT, false),
            Some(b"\x1bx".to_vec())
        );
        // Alt+arrow is ESC followed by the arrow sequence.
        assert_eq!(
            key_to_bytes(Key::ArrowLeft, Modifiers::ALT, false),
            Some(b"\x1b\x1b[D".to_vec())
        );
        assert_eq!(text_to_bytes("x", Modifiers::ALT), Some(b"\x1bx".to_vec()));
    }

    #[test]
    fn function_keys() {
        assert_eq!(
            key_to_bytes(Key::F5, Modifiers::NONE, false),
            Some(b"\x1b[15~".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::F1, Modifiers::NONE, false),
            Some(b"\x1bOP".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::F12, Modifiers::NONE, false),
            Some(b"\x1b[24~".to_vec())
        );
    }

    #[test]
    fn home_end_delete_paging() {
        assert_eq!(
            key_to_bytes(Key::Home, Modifiers::NONE, false),
            Some(b"\x1b[H".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::End, Modifiers::NONE, false),
            Some(b"\x1b[F".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::Delete, Modifiers::NONE, false),
            Some(b"\x1b[3~".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::PageUp, Modifiers::NONE, false),
            Some(b"\x1b[5~".to_vec())
        );
        assert_eq!(
            key_to_bytes(Key::PageDown, Modifiers::NONE, false),
            Some(b"\x1b[6~".to_vec())
        );
    }

    #[test]
    fn paste_with_and_without_brackets() {
        assert_eq!(paste_bytes("hi", false), b"hi".to_vec());
        assert_eq!(
            paste_bytes("hi", true),
            b"\x1b[200~hi\x1b[201~".to_vec()
        );
    }

    #[test]
    fn command_shortcuts_are_ignored() {
        assert_eq!(key_to_bytes(Key::T, Modifiers::MAC_CMD, false), None);
        assert_eq!(text_to_bytes("t", Modifiers::MAC_CMD), None);
        let mut cmd = Modifiers::NONE;
        cmd.command = true;
        assert_eq!(key_to_bytes(Key::W, cmd, false), None);
    }
}
