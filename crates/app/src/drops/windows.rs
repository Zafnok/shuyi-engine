//! Drops on Windows, read by the backend itself (ticket 3307, ADR-0104).
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
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, LRESULT, TRUE, UINT, WPARAM};
use winapi::shared::windef::HWND;
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

/// miniquad's main window, if this thread has made it.
fn miniquad_window() -> Option<HWND> {
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
    if String::from_utf16_lossy(&class[..len]) != MINIQUAD_CLASS {
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
