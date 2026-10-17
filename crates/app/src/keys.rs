//! Translation from macroquad key codes to `shuyi-ui`'s hardware-agnostic
//! [`Key`], and collecting a frame's keyboard events as [`RawInputEvent`]s.

use macroquad::prelude::{
    KeyCode, get_char_pressed, get_keys_pressed, get_keys_released, is_key_down,
};
use shuyi_ui::RawInputEvent;
use shuyi_ui::input::{Chord, Key};

/// The game key for a macroquad key code; `None` for keys the game ignores
/// (bare modifiers, which only form `Shift+` chords, and rare keys).
///
/// On the web, miniquad 0.4 reports neither `'` nor `/`, and reports the
/// `` ` `` key as `'` (its JS key table); they work on native.
pub fn to_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::A => Key::A,
        KeyCode::B => Key::B,
        KeyCode::C => Key::C,
        KeyCode::D => Key::D,
        KeyCode::E => Key::E,
        KeyCode::F => Key::F,
        KeyCode::G => Key::G,
        KeyCode::H => Key::H,
        KeyCode::I => Key::I,
        KeyCode::J => Key::J,
        KeyCode::K => Key::K,
        KeyCode::L => Key::L,
        KeyCode::M => Key::M,
        KeyCode::N => Key::N,
        KeyCode::O => Key::O,
        KeyCode::P => Key::P,
        KeyCode::Q => Key::Q,
        KeyCode::R => Key::R,
        KeyCode::S => Key::S,
        KeyCode::T => Key::T,
        KeyCode::U => Key::U,
        KeyCode::V => Key::V,
        KeyCode::W => Key::W,
        KeyCode::X => Key::X,
        KeyCode::Y => Key::Y,
        KeyCode::Z => Key::Z,
        KeyCode::Key0 => Key::Digit0,
        KeyCode::Key1 => Key::Digit1,
        KeyCode::Key2 => Key::Digit2,
        KeyCode::Key3 => Key::Digit3,
        KeyCode::Key4 => Key::Digit4,
        KeyCode::Key5 => Key::Digit5,
        KeyCode::Key6 => Key::Digit6,
        KeyCode::Key7 => Key::Digit7,
        KeyCode::Key8 => Key::Digit8,
        KeyCode::Key9 => Key::Digit9,
        KeyCode::Semicolon => Key::Semicolon,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter | KeyCode::KpEnter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Space => Key::Space,
        KeyCode::Tab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::F1 => Key::F1,
        KeyCode::F2 => Key::F2,
        KeyCode::F3 => Key::F3,
        KeyCode::F4 => Key::F4,
        KeyCode::F5 => Key::F5,
        KeyCode::F6 => Key::F6,
        KeyCode::F7 => Key::F7,
        KeyCode::F8 => Key::F8,
        KeyCode::F9 => Key::F9,
        KeyCode::F10 => Key::F10,
        KeyCode::F11 => Key::F11,
        KeyCode::F12 => Key::F12,
        KeyCode::Comma => Key::Comma,
        KeyCode::Period => Key::Period,
        KeyCode::Slash => Key::Slash,
        KeyCode::Apostrophe => Key::Apostrophe,
        KeyCode::LeftBracket => Key::LeftBracket,
        KeyCode::RightBracket => Key::RightBracket,
        KeyCode::Backslash => Key::Backslash,
        KeyCode::Minus => Key::Minus,
        KeyCode::Equal => Key::Equal,
        KeyCode::GraveAccent => Key::Backquote,
        KeyCode::Insert => Key::Insert,
        KeyCode::Delete => Key::Delete,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Kp0 => Key::Kp0,
        KeyCode::Kp1 => Key::Kp1,
        KeyCode::Kp2 => Key::Kp2,
        KeyCode::Kp3 => Key::Kp3,
        KeyCode::Kp4 => Key::Kp4,
        KeyCode::Kp5 => Key::Kp5,
        KeyCode::Kp6 => Key::Kp6,
        KeyCode::Kp7 => Key::Kp7,
        KeyCode::Kp8 => Key::Kp8,
        KeyCode::Kp9 => Key::Kp9,
        KeyCode::KpAdd => Key::KpAdd,
        KeyCode::KpSubtract => Key::KpSubtract,
        KeyCode::KpMultiply => Key::KpMultiply,
        KeyCode::KpDivide => Key::KpDivide,
        KeyCode::KpDecimal => Key::KpDecimal,
        _ => return None,
    })
}

/// Whether the window's fullscreen shortcut was pressed this frame: `F11`,
/// or `Alt+Enter`. Like the window's close button these belong to the
/// platform, not to the game's keymap (ticket 0902).
pub fn fullscreen_shortcut() -> bool {
    let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
    get_keys_pressed().into_iter().any(|code| {
        code == KeyCode::F11 || (alt && matches!(code, KeyCode::Enter | KeyCode::KpEnter))
    })
}

/// This frame's key releases, then presses (with the current Shift state),
/// then the characters typed (for text boxes, with the keyboard's layout).
pub fn poll() -> Vec<RawInputEvent> {
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    let released = get_keys_released()
        .into_iter()
        .filter_map(to_key)
        .map(RawInputEvent::Up);
    let pressed = get_keys_pressed()
        .into_iter()
        .filter_map(to_key)
        .map(|key| RawInputEvent::Down(Chord { key, shift }));
    let typed = std::iter::from_fn(get_char_pressed).map(RawInputEvent::Text);
    released.chain(pressed).chain(typed).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key code `app` reads as `key`. Exhaustive, so a new [`Key`]
    /// without a key code doesn't compile.
    fn code_for(key: Key) -> KeyCode {
        match key {
            Key::A => KeyCode::A,
            Key::B => KeyCode::B,
            Key::C => KeyCode::C,
            Key::D => KeyCode::D,
            Key::E => KeyCode::E,
            Key::F => KeyCode::F,
            Key::G => KeyCode::G,
            Key::H => KeyCode::H,
            Key::I => KeyCode::I,
            Key::J => KeyCode::J,
            Key::K => KeyCode::K,
            Key::L => KeyCode::L,
            Key::M => KeyCode::M,
            Key::N => KeyCode::N,
            Key::O => KeyCode::O,
            Key::P => KeyCode::P,
            Key::Q => KeyCode::Q,
            Key::R => KeyCode::R,
            Key::S => KeyCode::S,
            Key::T => KeyCode::T,
            Key::U => KeyCode::U,
            Key::V => KeyCode::V,
            Key::W => KeyCode::W,
            Key::X => KeyCode::X,
            Key::Y => KeyCode::Y,
            Key::Z => KeyCode::Z,
            Key::Digit0 => KeyCode::Key0,
            Key::Digit1 => KeyCode::Key1,
            Key::Digit2 => KeyCode::Key2,
            Key::Digit3 => KeyCode::Key3,
            Key::Digit4 => KeyCode::Key4,
            Key::Digit5 => KeyCode::Key5,
            Key::Digit6 => KeyCode::Key6,
            Key::Digit7 => KeyCode::Key7,
            Key::Digit8 => KeyCode::Key8,
            Key::Digit9 => KeyCode::Key9,
            Key::Semicolon => KeyCode::Semicolon,
            Key::Up => KeyCode::Up,
            Key::Down => KeyCode::Down,
            Key::Left => KeyCode::Left,
            Key::Right => KeyCode::Right,
            Key::Enter => KeyCode::Enter,
            Key::Escape => KeyCode::Escape,
            Key::Space => KeyCode::Space,
            Key::Tab => KeyCode::Tab,
            Key::Backspace => KeyCode::Backspace,
            Key::F1 => KeyCode::F1,
            Key::F2 => KeyCode::F2,
            Key::F3 => KeyCode::F3,
            Key::F4 => KeyCode::F4,
            Key::F5 => KeyCode::F5,
            Key::F6 => KeyCode::F6,
            Key::F7 => KeyCode::F7,
            Key::F8 => KeyCode::F8,
            Key::F9 => KeyCode::F9,
            Key::F10 => KeyCode::F10,
            Key::F11 => KeyCode::F11,
            Key::F12 => KeyCode::F12,
            Key::Comma => KeyCode::Comma,
            Key::Period => KeyCode::Period,
            Key::Slash => KeyCode::Slash,
            Key::Apostrophe => KeyCode::Apostrophe,
            Key::LeftBracket => KeyCode::LeftBracket,
            Key::RightBracket => KeyCode::RightBracket,
            Key::Backslash => KeyCode::Backslash,
            Key::Minus => KeyCode::Minus,
            Key::Equal => KeyCode::Equal,
            Key::Backquote => KeyCode::GraveAccent,
            Key::Insert => KeyCode::Insert,
            Key::Delete => KeyCode::Delete,
            Key::Home => KeyCode::Home,
            Key::End => KeyCode::End,
            Key::PageUp => KeyCode::PageUp,
            Key::PageDown => KeyCode::PageDown,
            Key::Kp0 => KeyCode::Kp0,
            Key::Kp1 => KeyCode::Kp1,
            Key::Kp2 => KeyCode::Kp2,
            Key::Kp3 => KeyCode::Kp3,
            Key::Kp4 => KeyCode::Kp4,
            Key::Kp5 => KeyCode::Kp5,
            Key::Kp6 => KeyCode::Kp6,
            Key::Kp7 => KeyCode::Kp7,
            Key::Kp8 => KeyCode::Kp8,
            Key::Kp9 => KeyCode::Kp9,
            Key::KpAdd => KeyCode::KpAdd,
            Key::KpSubtract => KeyCode::KpSubtract,
            Key::KpMultiply => KeyCode::KpMultiply,
            Key::KpDivide => KeyCode::KpDivide,
            Key::KpDecimal => KeyCode::KpDecimal,
        }
    }

    #[test]
    fn every_key_is_mapped_from_a_key_code() {
        for &key in Key::ALL {
            assert_eq!(to_key(code_for(key)), Some(key), "{key}");
        }
    }

    #[test]
    fn every_game_key_has_a_key_code() {
        let codes = [
            KeyCode::A,
            KeyCode::Z,
            KeyCode::Key0,
            KeyCode::Key9,
            KeyCode::Semicolon,
            KeyCode::Up,
            KeyCode::Right,
            KeyCode::Enter,
            KeyCode::Escape,
            KeyCode::Space,
            KeyCode::Tab,
            KeyCode::Backspace,
            KeyCode::F1,
            KeyCode::F12,
        ];
        let keys: Vec<String> = codes
            .into_iter()
            .filter_map(to_key)
            .map(|k| k.to_string())
            .collect();
        assert_eq!(
            keys,
            [
                "a",
                "z",
                "0",
                "9",
                ";",
                "Up",
                "Right",
                "Enter",
                "Escape",
                "Space",
                "Tab",
                "Backspace",
                "F1",
                "F12"
            ]
        );
        assert_eq!(to_key(KeyCode::KpEnter), Some(Key::Enter));
    }

    #[test]
    fn other_key_codes_are_ignored() {
        for code in [
            KeyCode::LeftShift,
            KeyCode::RightShift,
            KeyCode::LeftControl,
            KeyCode::LeftAlt,
            KeyCode::LeftSuper,
            KeyCode::CapsLock,
            KeyCode::KpEqual,
            KeyCode::F13,
        ] {
            assert_eq!(to_key(code), None);
        }
    }
}
