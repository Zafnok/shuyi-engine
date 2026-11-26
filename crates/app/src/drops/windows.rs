//! Drops on Windows, read by the backend itself (ADR-0104).
//!
//! miniquad 0.4.11 reads a `WM_DROPFILES` with `std::fs::read(..).unwrap()`,
//! so a dropped folder panics inside its window procedure, and it never
//! calls `files_dropped_event`, so macroquad never sees a drop there at
//! all. This module subclasses miniquad's window once it exists: its own
//! procedure answers `WM_DROPFILES` (miniquad never sees it), reads each
//! path with [`super::read`], which leaves out a folder or a file it can't
//! read, and keeps the files until the next [`take`]. Every other message
//! goes on to miniquad's procedure.

// The backend's `unsafe` (game ADR-0034, ADR-0104): the Win32 calls that
// find the window, swap its procedure and read the drop's paths.
#![allow(unsafe_code)]

use std::path::PathBuf;
use std::sync::Mutex;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicPtr, Ordering};

use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, LRESULT, TRUE, UINT, WPARAM};
use winapi::shared::windef::{HWND, HWND__};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::shellapi::{DragFinish, DragQueryFileW, HDROP};
use winapi::um::winuser::{
    CallWindowProcW, EnumThreadWindows, GWLP_WNDPROC, GetClassNameW, SetWindowLongPtrW,
    WM_DROPFILES, WNDPROC,
};

use super::Raw;

/// The class miniquad registers its main window under
/// (`src/native/windows.rs`, `create_window`).
const MINIQUAD_CLASS: &str = "MINIQUADAPP";
/// [`MINIQUAD_CLASS`] in UTF-16, as `GetClassNameW` writes it.
const MINIQUAD_CLASS_UTF16: [u16; MINIQUAD_CLASS.len()] = ascii_to_utf16(MINIQUAD_CLASS);

/// An ASCII string's UTF-16 units, one per byte.
const fn ascii_to_utf16<const N: usize>(text: &str) -> [u16; N] {
    let bytes = text.as_bytes();
    assert!(bytes.len() == N && text.is_ascii());
    let mut units = [0u16; N];
    let mut i = 0;
    while i < N {
        units[i] = bytes[i] as u16;
        i += 1;
    }
    units
}

/// Whether a window class's name, in UTF-16, is miniquad's: no `String`
/// made, since this runs for each window until miniquad's is found.
fn is_miniquad_class(units: &[u16]) -> bool {
    units == MINIQUAD_CLASS_UTF16
}

/// Whether miniquad's window has this module's procedure.
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// miniquad's own procedure, which every other message goes on to.
static PREVIOUS: AtomicIsize = AtomicIsize::new(0);
/// The files dropped since the last [`take`].
static PENDING: Mutex<Vec<Raw>> = Mutex::new(Vec::new());

/// The files dropped since the last call, subclassing miniquad's window
/// first if it isn't yet.
pub(crate) fn take() -> Vec<Raw> {
    install();
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::mem::take(&mut *pending)
}

/// Gives miniquad's window this module's procedure, once: the window is
/// made on this thread, before the first frame.
fn install() {
    if INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    let Some(hwnd) = miniquad_window() else {
        return;
    };
    // SAFETY: `hwnd` is a live window of this thread, and `procedure` has
    // the signature of a window procedure.
    let mine: unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT = procedure;
    let previous = unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, (mine as usize).cast_signed()) };
    PREVIOUS.store(previous, Ordering::Relaxed);
    INSTALLED.store(true, Ordering::Relaxed);
}

/// miniquad's main window, null until [`miniquad_window`] first finds it.
///
/// miniquad makes one window, on the game's thread, before the first
/// frame, and it lives for the whole run, so once found it is kept rather than looked for again every
/// frame. A miss is not kept: before miniquad has made its
/// window, the next call looks again.
static WINDOW: AtomicPtr<HWND__> = AtomicPtr::new(std::ptr::null_mut());

/// How many times [`miniquad_window`] has listed the thread's windows.
#[cfg(test)]
static ENUMERATIONS: AtomicUsize = AtomicUsize::new(0);

/// miniquad's main window, if this thread has made it: also the window
/// whose size the window's record reads (`crate::window`). The thread's
/// windows are listed only until it is found.
pub(crate) fn miniquad_window() -> Option<HWND> {
    cached(&WINDOW, enumerate)
}

/// The window `slot` holds, or else the one `search` finds, kept in `slot`
/// when it finds one.
fn cached(slot: &AtomicPtr<HWND__>, search: impl FnOnce() -> Option<HWND>) -> Option<HWND> {
    let kept = slot.load(Ordering::Relaxed);
    if !kept.is_null() {
        return Some(kept);
    }
    let found = search()?;
    slot.store(found, Ordering::Relaxed);
    Some(found)
}

/// Lists this thread's windows for miniquad's.
fn enumerate() -> Option<HWND> {
    #[cfg(test)]
    ENUMERATIONS.fetch_add(1, Ordering::Relaxed);
    let mut found: HWND = std::ptr::null_mut();
    // SAFETY: `find` reads `found` through the pointer it is handed, which
    // lives until `EnumThreadWindows` returns.
    unsafe {
        EnumThreadWindows(
            GetCurrentThreadId(),
            Some(find),
            std::ptr::from_mut(&mut found) as LPARAM,
        );
    }
    (!found.is_null()).then_some(found)
}

/// `EnumThreadWindows`'s callback: stops at miniquad's window and writes it
/// to the `HWND` that `found` points at.
unsafe extern "system" fn find(hwnd: HWND, found: LPARAM) -> BOOL {
    let mut class = [0u16; 64];
    // SAFETY: `class` holds as many units as it says.
    let len = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), 64) };
    let len = usize::try_from(len).unwrap_or(0);
    if !is_miniquad_class(&class[..len]) {
        return TRUE;
    }
    // SAFETY: `miniquad_window` hands a pointer to its `HWND`.
    unsafe { *(found as *mut HWND) = hwnd };
    FALSE
}

/// The window's procedure: a drop is this module's, every other message
/// miniquad's.
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_DROPFILES {
        let hdrop = wparam as HDROP;
        // SAFETY: Windows hands a `WM_DROPFILES` its drop in `wparam`.
        let files = super::read(unsafe { paths(hdrop) });
        // SAFETY: the drop is read, and `DragFinish` frees it once.
        unsafe { DragFinish(hdrop) };
        PENDING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend(files);
        return 0;
    }
    // SAFETY: `PREVIOUS` holds the procedure `SetWindowLongPtrW` replaced,
    // a window procedure (or none, which `CallWindowProcW` refuses).
    unsafe {
        let previous: WNDPROC = std::mem::transmute(PREVIOUS.load(Ordering::Relaxed));
        CallWindowProcW(previous, hwnd, message, wparam, lparam)
    }
}

/// Each path of the drop, however long.
///
/// # Safety
///
/// `hdrop` is the drop of a `WM_DROPFILES` not yet finished.
unsafe fn paths(hdrop: HDROP) -> Vec<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    // SAFETY: `u32::MAX` asks the drop for its count.
    let count = unsafe { DragQueryFileW(hdrop, u32::MAX, std::ptr::null_mut(), 0) };
    (0..count)
        .filter_map(|i| {
            // SAFETY: a null buffer asks the path's length, without its nul.
            let len = unsafe { DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0) };
            let mut path = vec![0u16; usize::try_from(len).ok()? + 1];
            let size = u32::try_from(path.len()).ok()?;
            // SAFETY: `path` holds `size` units.
            let written = unsafe { DragQueryFileW(hdrop, i, path.as_mut_ptr(), size) };
            path.truncate(usize::try_from(written).ok()?);
            (!path.is_empty()).then(|| PathBuf::from(std::ffi::OsString::from_wide(&path)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::um::winuser::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, WNDCLASSW,
        WS_OVERLAPPEDWINDOW,
    };

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn the_class_comparison_matches_miniquads_name_only() {
        assert!(is_miniquad_class(&utf16(MINIQUAD_CLASS)));
        assert!(!is_miniquad_class(&utf16("SHUYIAPP___")));
        assert!(!is_miniquad_class(&utf16("MINIQUAD")));
        assert!(!is_miniquad_class(&utf16("MINIQUADAPPX")));
        assert!(!is_miniquad_class(&[]));
    }

    #[test]
    fn a_found_window_is_kept_and_a_miss_is_looked_for_again() {
        let slot = AtomicPtr::new(std::ptr::null_mut());
        let mut searches = 0;
        for _ in 0..3 {
            let found = cached(&slot, || {
                searches += 1;
                None
            });
            assert_eq!(found, None);
        }
        assert_eq!(searches, 3, "a miss is looked for again");
        let window: HWND = std::ptr::NonNull::dangling().as_ptr();
        for _ in 0..100 {
            let found = cached(&slot, || {
                searches += 1;
                Some(window)
            });
            assert_eq!(found, Some(window));
        }
        assert_eq!(searches, 4, "once found, the window is kept");
    }

    /// The real lookup on a window of miniquad's class made on this thread:
    /// the thread's windows are listed while none is found, and once only
    /// after (the only test that calls [`miniquad_window`], since the
    /// window it finds is kept for the process).
    #[test]
    fn the_thread_s_windows_are_listed_until_miniquads_is_found() {
        let before = ENUMERATIONS.load(Ordering::Relaxed);
        assert_eq!(miniquad_window(), None);
        assert_eq!(miniquad_window(), None);
        assert_eq!(ENUMERATIONS.load(Ordering::Relaxed) - before, 2);

        let class: Vec<u16> = utf16(MINIQUAD_CLASS).into_iter().chain([0]).collect();
        // SAFETY: a null name asks for this program's module; the class's
        // name is nul-terminated and outlives the class and its window,
        // and `DefWindowProcW` is a window procedure.
        let window = unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let mut description: WNDCLASSW = std::mem::zeroed();
            description.lpfnWndProc = Some(DefWindowProcW);
            description.hInstance = instance;
            description.lpszClassName = class.as_ptr();
            assert_ne!(RegisterClassW(&raw const description), 0);
            CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                100,
                100,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null_mut(),
            )
        };
        assert!(!window.is_null());

        let before = ENUMERATIONS.load(Ordering::Relaxed);
        for _ in 0..100 {
            assert_eq!(miniquad_window(), Some(window));
        }
        assert_eq!(ENUMERATIONS.load(Ordering::Relaxed) - before, 1);
        // SAFETY: the window is this thread's, and nothing uses it after.
        unsafe { DestroyWindow(window) };
    }
}
