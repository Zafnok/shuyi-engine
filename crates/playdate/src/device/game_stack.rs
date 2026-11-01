//! The game's own stack ([`crate::stack`]): a block from the heap, entered
//! with a few lines of assembly around each call into the game ([`run`]).
//!
//! Only the device switches stacks. The Simulator runs the game natively on
//! the computer's stack, which is big, so there [`run`] is a plain call and
//! nothing is measured.

use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use crate::stack;

/// The stack's lowest byte; null until [`init`] (and always off the device).
static BASE: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());

/// The stack's size in bytes, set before [`BASE`].
static BYTES: AtomicUsize = AtomicUsize::new(0);

/// Whether the game is running on the stack now.
static INSIDE: AtomicBool = AtomicBool::new(false);

/// Takes `bytes` bytes for the stack from the heap, painted, for the rest of
/// the game. Called once, at start, on the C stack.
#[cfg(target_os = "none")]
pub fn init(bytes: usize) {
    assert!(
        bytes > stack::RED_ZONE,
        // check-text: not player text (a build problem for the console)
        "a stack of {bytes} bytes is no bigger than its red zone"
    );
    let block = alloc::vec![stack::PAINT; bytes].leak();
    BYTES.store(bytes, Ordering::Relaxed);
    BASE.store(block.as_mut_ptr(), Ordering::Relaxed);
}

/// Nothing to take off the device.
#[cfg(not(target_os = "none"))]
pub fn init(_bytes: usize) {}

/// The stack's bytes, lowest address first, read one at a time: the game may
/// be running on them, so no slice can hold them.
fn bytes(base: *const u8, size: usize) -> impl Iterator<Item = u8> {
    // SAFETY: `init` took these bytes for the stack and never frees them.
    (0..size).map(move |i| unsafe { base.add(i).read_volatile() })
}

/// The bytes of the stack the game has ever used, or `None` where it is not
/// measured (the Simulator).
pub fn high_water() -> Option<usize> {
    let base = BASE.load(Ordering::Relaxed);
    if base.is_null() {
        return None;
    }
    let size = BYTES.load(Ordering::Relaxed);
    Some(stack::high_water(size, bytes(base, size)))
}

/// Runs `f` on the game's stack and returns what it returns. Every call from
/// the SDK into the game goes through here.
pub fn run<R>(f: impl FnOnce() -> R) -> R {
    let base = BASE.load(Ordering::Relaxed);
    if base.is_null() {
        return f();
    }
    let size = BYTES.load(Ordering::Relaxed);
    let out = stack::run(&INSIDE, f, |mut call| {
        // SAFETY: the stack is `size` bytes from `init`, and unused:
        // `stack::run` makes this call only from outside it.
        unsafe { switch(stack::top(base.addr(), size), (&raw mut call).cast()) };
    });
    assert!(
        !stack::overflowed(bytes(base, size)),
        // check-text: not player text (a build problem for the console)
        "the game's stack of {size} bytes overflowed: give main! more"
    );
    out
}

/// Calls the `&mut dyn FnMut()` behind `call`: the first function on the
/// game's stack.
extern "C" fn trampoline(call: *mut c_void) {
    // SAFETY: `run` passes a pointer to its `&mut dyn FnMut()`, alive for the
    // whole switch.
    let call = unsafe { &mut *call.cast::<&mut dyn FnMut()>() };
    call();
}

/// Moves the stack pointer to `top`, calls `trampoline(call)`, and moves it
/// back: Thumb, on the device.
#[cfg(target_os = "none")]
unsafe fn switch(top: usize, call: *mut c_void) {
    // The registers a C call may change, written out: `clobber_abi("C")` would
    // also name d16 to d31, which the single-precision build (`-fp64`) doesn't
    // have.
    let first: extern "C" fn(*mut c_void) = trampoline;
    unsafe {
        core::arch::asm!(
            // r4 is callee-saved: `trampoline` gives it back unchanged.
            "mov r4, sp",
            "mov sp, r1",
            "blx r2",
            "mov sp, r4",
            inout("r0") call => _,
            inout("r1") top => _,
            inout("r2") first => _,
            out("r3") _,
            out("r4") _,
            out("r12") _,
            out("lr") _,
            out("s0") _, out("s1") _, out("s2") _, out("s3") _,
            out("s4") _, out("s5") _, out("s6") _, out("s7") _,
            out("s8") _, out("s9") _, out("s10") _, out("s11") _,
            out("s12") _, out("s13") _, out("s14") _, out("s15") _,
        );
    }
}

/// Off the device [`BASE`] stays null, so [`run`] never gets here; a plain
/// call keeps the build whole.
#[cfg(not(target_os = "none"))]
unsafe fn switch(_top: usize, call: *mut c_void) {
    trampoline(call);
}
