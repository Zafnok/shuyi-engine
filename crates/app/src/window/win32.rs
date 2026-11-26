//! The window's size on Windows, read from the system.
//!
//! miniquad 0.4.11 keeps the framebuffer's size as the client area divided
//! by the display's scale, cut to a whole number, then multiplied back
//! (`update_dimensions`): at 175 % a client area 1550 pixels wide reads as
//! 1548. A record of that size would shrink the window a little at every
//! launch, so the record reads the client area itself.

// The backend's `unsafe` (game ADR-0034, ADR-0104): one Win32 call that
// reads the window's client area.
#![allow(unsafe_code)]

use winapi::shared::windef::RECT;
use winapi::um::winuser::GetClientRect;

/// The size of miniquad's window's client area in device pixels, if the
/// window exists and the system answers.
pub(crate) fn client_size() -> Option<(i32, i32)> {
    let hwnd = crate::drops::miniquad_window()?;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: `hwnd` is a live window of this thread, and `rect` is a
    // `RECT` it may write.
    let read = unsafe { GetClientRect(hwnd, &raw mut rect) };
    (read != 0).then_some((rect.right - rect.left, rect.bottom - rect.top))
}
