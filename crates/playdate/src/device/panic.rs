//! Panics stop the game with the SDK's error screen, showing the message
//! and logging it to the console. On the device that call never comes back;
//! on the Simulator `shim.c` leaves the game's call from inside it, so its
//! window stays alive.

use core::ffi::c_char;
use core::fmt::Write as _;
use core::panic::PanicInfo;

use crate::text::FixedText;

unsafe extern "C" {
    fn pd_error(text: *const c_char);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // No allocation here: running out of memory panics too.
    let mut message = FixedText::<256>::new();
    let _ = write!(message, "panic: {info}");
    unsafe { pd_error(message.as_c_bytes().as_ptr().cast()) };
    loop {
        core::hint::spin_loop();
    }
}

/// The Simulator build (`x86_64-pc-windows-gnu`) links the prebuilt `core`,
/// which is built to unwind; with `panic = "abort"` nothing calls this, but
/// the linker needs it.
#[cfg(not(target_os = "none"))]
#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

// The device build's unwind tables name the ARM unwinder's personality
// routines (an unoptimized build keeps those tables). With `panic = "abort"`
// nothing unwinds, so these are never called; without them the link pulls in
// libgcc's unwinder, which needs C library calls (`abort`, `raise`) the
// Playdate doesn't have.

/// Never called: nothing unwinds.
#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn __aeabi_unwind_cpp_pr0() {}

/// Never called: nothing unwinds.
#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn __aeabi_unwind_cpp_pr1() {}

/// Never called: nothing unwinds.
#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn __aeabi_unwind_cpp_pr2() {}
