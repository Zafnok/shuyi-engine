//! The frames of a crash report's stack (ticket 2598): each return address
//! of the panicking thread as an offset into the executable, which a debug
//! file of the same build names with `addr2line`.
//!
//! No crate walks the stack: the system's own unwinder does, by the unwind
//! tables a stripped build keeps.
//!
//! - **Windows:** `RtlCaptureStackBackTrace`, by the `.pdata` tables that
//!   MSVC and the GNU toolchain both write. `GetModuleHandleExW` says which
//!   module a frame is in, and the executable's base is
//!   `GetModuleHandleW(null)`.
//! - **Linux (glibc):** `backtrace()`, by `.eh_frame`. The executable's
//!   place is the range of its lines in `/proc/self/maps`, read once by
//!   [`note_executable`] (from `crash::install`), so the hook reads no file.
//! - **Anything else** (macOS, the web): no frames.
//!
//! `backtrace` 0.3 would do both, but brings `addr2line`, `gimli`, `object`
//! and `rustc-demangle` into every game for a hook that never names a frame.

// The backend's `unsafe` (game ADR-0034): the system calls that walk the
// stack and find a frame's module.
#![allow(unsafe_code)]

use shuyi_platform::crash::{CodeFrame, FRAMES};

/// The frames of the calling thread's stack, the innermost first, up to
/// [`FRAMES`].
pub(crate) fn capture() -> Vec<CodeFrame> {
    os::return_addresses()
        .into_iter()
        .take(FRAMES)
        .map(|address| CodeFrame::new(os::offset(address)))
        .collect()
}

/// Finds where the executable is in memory, for [`capture`] on Linux;
/// nothing elsewhere. Called by `crash::install`, outside the hook.
pub(crate) fn note_executable() {
    os::note_executable();
}

/// `address - base` as an offset, when `inside` says the address is in
/// the executable.
#[cfg_attr(
    not(any(windows, all(target_os = "linux", target_env = "gnu"))),
    allow(dead_code)
)]
fn offset_from(address: usize, base: usize, inside: bool) -> Option<u64> {
    let offset = address.checked_sub(base).filter(|_| inside)?;
    u64::try_from(offset).ok()
}

/// The range `[start, end)` that `exe`'s lines cover in `maps`, the text of
/// `/proc/self/maps`: from the lowest start to the highest end. `None` when
/// no line names it.
#[cfg_attr(not(all(target_os = "linux", target_env = "gnu")), allow(dead_code))]
fn executable_range(maps: &str, exe: &str) -> Option<(u64, u64)> {
    let mut range: Option<(u64, u64)> = None;
    for line in maps.lines() {
        // `start-end perms offset dev inode path`; the path may hold spaces.
        let mut fields = line.splitn(6, ' ');
        let Some(span) = fields.next() else { continue };
        let path = fields.nth(4).map(str::trim_start);
        if path != Some(exe) {
            continue;
        }
        let Some((start, end)) = span.split_once('-') else {
            continue;
        };
        let (Ok(start), Ok(end)) = (u64::from_str_radix(start, 16), u64::from_str_radix(end, 16))
        else {
            continue;
        };
        range = Some(range.map_or((start, end), |(low, high)| (low.min(start), high.max(end))));
    }
    range
}

#[cfg(windows)]
mod os {
    use std::ptr;

    use shuyi_platform::crash::FRAMES;
    use winapi::ctypes::c_void;
    use winapi::shared::minwindef::HMODULE;
    use winapi::um::libloaderapi::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleHandleExW, GetModuleHandleW,
    };
    use winapi::um::winnt::RtlCaptureStackBackTrace;

    pub(super) fn return_addresses() -> Vec<usize> {
        let mut addresses = [ptr::null_mut::<c_void>(); FRAMES];
        let most = u32::try_from(FRAMES).unwrap_or(u32::MAX);
        // SAFETY: the buffer holds `most` pointers, and the hash may be null.
        let taken =
            unsafe { RtlCaptureStackBackTrace(0, most, addresses.as_mut_ptr(), ptr::null_mut()) };
        addresses[..usize::from(taken).min(FRAMES)]
            .iter()
            .map(|&address| address as usize)
            .collect()
    }

    pub(super) fn offset(address: usize) -> Option<u64> {
        // SAFETY: a null name asks for the executable's own module, which
        // lives as long as the process.
        let exe = unsafe { GetModuleHandleW(ptr::null()) };
        let flags =
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
        let mut module: HMODULE = ptr::null_mut();
        // SAFETY: with FROM_ADDRESS the name is an address, never read as
        // text; UNCHANGED_REFCOUNT takes no reference to release.
        let found = unsafe { GetModuleHandleExW(flags, address as *const u16, &raw mut module) };
        let inside = found != 0 && !exe.is_null() && module == exe;
        super::offset_from(address, exe as usize, inside)
    }

    pub(super) const fn note_executable() {}
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod os {
    use std::ffi::{c_int, c_void};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use shuyi_platform::crash::FRAMES;

    unsafe extern "C" {
        /// glibc's `execinfo.h`: the calling thread's return addresses.
        fn backtrace(buffer: *mut *mut c_void, size: c_int) -> c_int;
    }

    /// The executable's range in memory, `[START, END)`
    /// ([`note_executable`]): empty until found, and then no frame is in it.
    static START: AtomicUsize = AtomicUsize::new(0);
    static END: AtomicUsize = AtomicUsize::new(0);

    pub(super) fn return_addresses() -> Vec<usize> {
        let mut addresses = [std::ptr::null_mut::<c_void>(); FRAMES];
        let most = c_int::try_from(FRAMES).unwrap_or(c_int::MAX);
        // SAFETY: the buffer holds `most` pointers.
        let taken = unsafe { backtrace(addresses.as_mut_ptr(), most) };
        let taken = usize::try_from(taken).unwrap_or(0).min(FRAMES);
        addresses[..taken]
            .iter()
            .map(|&address| address as usize)
            .collect()
    }

    pub(super) fn offset(address: usize) -> Option<u64> {
        let (start, end) = (START.load(Ordering::Relaxed), END.load(Ordering::Relaxed));
        super::offset_from(address, start, (start..end).contains(&address))
    }

    pub(super) fn note_executable() {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let Ok(maps) = std::fs::read_to_string("/proc/self/maps") else {
            return;
        };
        let range = super::executable_range(&maps, &exe.to_string_lossy());
        if let Some((start, end)) = range {
            let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
                return;
            };
            START.store(start, Ordering::Relaxed);
            END.store(end, Ordering::Relaxed);
        }
    }
}

#[cfg(not(any(windows, all(target_os = "linux", target_env = "gnu"))))]
mod os {
    pub(super) const fn return_addresses() -> Vec<usize> {
        Vec::new()
    }

    pub(super) const fn offset(_address: usize) -> Option<u64> {
        None
    }

    pub(super) const fn note_executable() {}
}

#[cfg(test)]
mod tests;
