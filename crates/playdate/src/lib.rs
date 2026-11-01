//! Shuyi Engine's Playdate backend (ADR-0008 §2, ADR-0062): the
//! [`Platform`](shuyi_platform::Platform) on Panic's SDK.
//!
//! A game's `playdate` feature names this crate, and its library says how
//! it starts with [`main!`]; `shuyi export playdate` builds that library as
//! a static library with `--cfg shuyi_playdate_sdk`, links it with
//! `shim/shim.c` and the SDK's `setup.c`, and packs the `.pdx`. The rest of
//! the time the crate is the plain part, so the workspace's host builds and
//! tests never need the SDK:
//!
//! - [`sdk`]: the small trait of SDK calls the rest is written against, which
//!   the tests' fake and the device's real calls implement;
//! - [`backend`]: [`Playdate`], the [`Platform`](shuyi_platform::Platform)
//!   itself;
//! - [`driver`]: [`Driver`], the update callback's body around
//!   [`App::frame`](shuyi_platform::App::frame);
//! - [`input`]: the buttons and the crank; [`display`]: the frame buffer and
//!   its changed rows; [`storage`]: the game's saves on the SDK's files;
//! - [`audio`]: the shared player's backend, on the SDK's sample and file
//!   players;
//! - [`stack`]: the arithmetic of the game's own stack, which every call from
//!   the SDK into the game runs on.
#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod audio;
pub mod backend;
pub mod display;
pub mod driver;
pub mod heap;
pub mod input;
pub mod sdk;
pub mod stack;
pub mod storage;
pub mod text;

#[cfg(shuyi_playdate_sdk)]
#[doc(hidden)]
pub mod device;

#[cfg(test)]
mod fake;

pub use backend::Playdate;
pub use driver::Driver;

#[cfg(shuyi_playdate_sdk)]
pub use device::{pack, stack_high_water};

/// The game's baked pack: off the device there is none.
#[cfg(not(shuyi_playdate_sdk))]
pub fn pack() -> Result<&'static [u8], alloc::string::String> {
    use alloc::borrow::ToOwned;
    // check-text: not player text (a build problem for the log)
    Err("the pack is on the device alone".to_owned())
}

/// The bytes of the game's stack it has ever used: on the device alone, since
/// the Simulator runs on the computer's stack.
#[cfg(not(shuyi_playdate_sdk))]
pub fn stack_high_water() -> Option<usize> {
    None
}

/// The game's entry points, for the crate `shuyi export playdate` builds:
/// `main!(start)`, with `start` a `fn() -> Start<G>`, or `main!(start,
/// stack_bytes = 128 * 1024)` for a game's own stack of another size than
/// [`stack::DEFAULT_BYTES`]. Off the device it only checks `start` exists.
#[cfg(shuyi_playdate_sdk)]
#[macro_export]
macro_rules! main {
    ($start:path) => {
        $crate::main!($start, stack_bytes = $crate::stack::DEFAULT_BYTES);
    };
    ($start:path, stack_bytes = $bytes:expr) => {
        /// The SDK's first event.
        #[unsafe(no_mangle)]
        pub extern "C" fn shuyi_playdate_init() {
            $crate::device::init($start, $bytes);
        }

        /// The SDK's update callback; 1 when the screen changed.
        #[unsafe(no_mangle)]
        pub extern "C" fn shuyi_playdate_update() -> core::ffi::c_int {
            $crate::device::update()
        }

        /// The SDK's other events.
        #[unsafe(no_mangle)]
        pub extern "C" fn shuyi_playdate_event(event: core::ffi::c_int) {
            $crate::device::event(event);
        }

        /// The player chose the game's system menu item in `slot`.
        #[unsafe(no_mangle)]
        pub extern "C" fn shuyi_playdate_menu_chosen(slot: core::ffi::c_int) {
            $crate::device::menu_chosen(slot);
        }
    };
}

/// The game's entry points, for the crate `shuyi export playdate` builds:
/// `main!(start)`, with `start` a `fn() -> Start<G>`, or `main!(start,
/// stack_bytes = 128 * 1024)` for a game's own stack of another size than
/// [`stack::DEFAULT_BYTES`]. Off the device it only checks `start` exists.
#[cfg(not(shuyi_playdate_sdk))]
#[macro_export]
macro_rules! main {
    ($start:path) => {
        const _: () = {
            let _ = $start;
        };
    };
    ($start:path, stack_bytes = $bytes:expr) => {
        const _: () = {
            let _ = $start;
            let _: usize = $bytes;
        };
    };
}

#[cfg(test)]
mod tests {
    use alloc::borrow::ToOwned;
    use alloc::string::String;
    use alloc::vec::Vec;

    /// The lines of a TOML table in `text`, without comments.
    fn table(text: &str, name: &str) -> Vec<String> {
        text.lines()
            .skip_while(|l| l.trim() != name)
            .skip(1)
            .take_while(|l| !l.starts_with('['))
            .map(|l| l.split('#').next().unwrap_or("").trim().to_owned())
            .filter(|l| !l.is_empty())
            .collect()
    }

    #[test]
    fn off_the_device_there_is_no_pack() {
        assert!(crate::pack().is_err());
    }

    #[test]
    fn off_the_device_no_stack_is_measured() {
        assert_eq!(crate::stack_high_water(), None);
    }

    /// `[lints]` is copied from the workspace's because one lint differs
    /// (`unsafe_code`): the copy must not drift.
    #[test]
    fn the_lints_are_the_workspace_lints_except_unsafe_code() {
        let workspace = include_str!("../../../Cargo.toml");
        let manifest = include_str!("../Cargo.toml");
        let clippy = table(workspace, "[workspace.lints.clippy]");
        assert!(clippy.len() > 5, "{clippy:?}");
        let rust: Vec<String> = table(workspace, "[workspace.lints.rust]")
            .iter()
            .map(|l| l.replace("unsafe_code = \"forbid\"", "unsafe_code = \"deny\""))
            .collect();
        assert_eq!(table(manifest, "[lints.clippy]"), clippy);
        assert_eq!(table(manifest, "[lints.rust]"), rust);
    }
}
