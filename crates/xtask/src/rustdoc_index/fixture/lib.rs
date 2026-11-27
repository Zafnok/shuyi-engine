//! The crate the tests of `rustdoc_index` and `api_text` read. Its files
//! were made by the pinned nightly (`api::NIGHTLY`) from a package of this
//! file alone (`[package] name = "fixture"`, edition 2024, `[lib] path =
//! "lib.rs"`, and an empty `[workspace]`): `fixture.json` by `cargo
//! +nightly-2026-08-16 rustdoc --lib -- -Z unstable-options
//! --output-format json`, and `listing.txt` by `cargo +nightly-2026-08-16
//! public-api --simplified`.

#![no_std]

/// The clock.
pub mod timer {
    /// A timer: it fires once, after its length.
    ///
    /// More words.
    #[doc(alias = "Timer")]
    #[doc(alias("Alarm", "Delay"))]
    pub struct Countdown {
        /// Its length.
        pub length: u32,
        left: u32,
    }

    impl Countdown {
        /// Starts it again.
        pub fn restart(&mut self) {
            self.left = self.length;
        }

        /// The old name of [`restart`](Self::restart).
        #[deprecated(note = "renamed `restart`")]
        pub fn reset(&mut self) {
            self.restart();
        }

        #[allow(dead_code)]
        fn hidden(&self) {}
    }

    impl Clone for Countdown {
        fn clone(&self) -> Self {
            Self { length: self.length, left: self.left }
        }
    }

    /// Where it is.
    pub enum Align {
        /// In the middle.
        Centered,
    }

    /// A thing that ticks.
    pub trait Tick {
        /// One tick.
        fn tick(&mut self);
    }
}

mod hidden {
    /// Reached only through the re-export.
    pub struct Inner;
}

pub use hidden::Inner;
pub use timer::Countdown;
pub use timer::*;
