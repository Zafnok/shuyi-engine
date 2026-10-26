//! Shuyi Engine's heap counter (ADR-0046 §9): [`CountingAlloc`], a global
//! allocator around another that counts what is asked of it, and with the
//! `global` feature, the `global` module: the one counting allocator of
//! every debug build of a game on `shuyi-app` and of its tool program.
//!
//! What it counts:
//!
//! - **the bytes asked for**, each [`Layout`](core::alloc::Layout)'s
//!   size: never the inner allocator's own overhead, so a reading is what
//!   the program asked for, the same on every machine;
//! - **live bytes**, allocated and not yet freed, and **their peak**, the
//!   most at once since the start or a [`CountingAlloc::reset_peak`];
//! - **calls** to `alloc`, `alloc_zeroed` and `realloc`, taken: each read
//!   ([`CountingAlloc::take_allocs`]) gives the calls since the last.
//!
//! A global allocator counts the **whole process**: every thread's
//! allocations count, not only the game's.
//!
//! **Debug builds count; release builds don't.** `shuyi-app` and
//! `shuyi-cli` turn on `global`, so a game's debug build and its tool
//! program's run on `global`'s counter, and `App` puts its counts in
//! each frame's record (`shuyi_ui::perf::FrameRecord::heap`). A game with
//! a `#[global_allocator]` of its own opts out, since a binary has one:
//! in its `.cargo/config.toml`,
//!
//! ```toml
//! [build]
//! rustflags = ["--cfg", "shuyi_no_heap_count"]
//! ```
//!
//! Without `std` the crate is `no_std`: a console's backend wraps the
//! SDK's own allocator in a [`CountingAlloc`].

#![cfg_attr(not(feature = "std"), no_std)]
// Counts are whole bytes (ADR-0032).
#![deny(clippy::float_arithmetic)]

// The `GlobalAlloc` impl and the counts' cells: this crate's only `unsafe`.
#[allow(unsafe_code)]
mod counter;
#[cfg(feature = "global")]
pub mod global;

pub use counter::CountingAlloc;

/// What a [`CountingAlloc`] counted, read at once
/// ([`CountingAlloc::counts`]). Build it with [`Counts::new`] (ADR-0039).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Counts {
    /// The bytes allocated and not yet freed.
    pub live_bytes: u64,
    /// The most bytes live at once since the start or the last
    /// [`CountingAlloc::reset_peak`].
    pub peak_bytes: u64,
    /// The calls to `alloc`, `alloc_zeroed` and `realloc` since the last
    /// reading.
    pub allocs: u32,
}

impl Counts {
    /// `live_bytes` live, `peak_bytes` at the most, and `allocs` calls.
    pub const fn new(live_bytes: u64, peak_bytes: u64, allocs: u32) -> Self {
        Self {
            live_bytes,
            peak_bytes,
            allocs,
        }
    }
}

#[cfg(test)]
mod tests {
    /// The lines of `manifest`'s `[header]` table, without comments and
    /// blank lines.
    fn table<'a>(manifest: &'a str, header: &str) -> Vec<&'a str> {
        let lines = manifest.lines().skip_while(|l| l.trim() != header).skip(1);
        lines
            .take_while(|l| !l.starts_with('['))
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .collect()
    }

    /// This crate copies the workspace lints because one of them differs
    /// (`unsafe_code`, ADR-0046 §9); the copy must not drift.
    #[test]
    fn the_lints_are_the_workspace_lints_except_unsafe_code() {
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |path: &std::path::Path| std::fs::read_to_string(path).unwrap();
        let root = here.parent().unwrap().parent().unwrap();
        let workspace = read(&root.join("Cargo.toml"));
        let clippy = table(&workspace, "[workspace.lints.clippy]");
        assert!(clippy.len() > 5, "{clippy:?}");
        let rust: Vec<String> = table(&workspace, "[workspace.lints.rust]")
            .iter()
            .map(|l| l.replace("unsafe_code = \"forbid\"", "unsafe_code = \"deny\""))
            .collect();
        assert!(
            rust.contains(&"unsafe_code = \"deny\"".to_owned()),
            "{rust:?}"
        );
        let manifest = read(&here.join("Cargo.toml"));
        assert_eq!(table(&manifest, "[lints.clippy]"), clippy);
        assert_eq!(table(&manifest, "[lints.rust]"), rust);
    }

    /// One module may use `unsafe`, the counter's: the crate's other files
    /// never name it.
    #[test]
    fn only_the_counter_allows_unsafe_code() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap();
        let lines: Vec<&str> = lib.lines().collect();
        let allows: Vec<usize> = (0..lines.len())
            .filter(|&i| lines[i] == "#[allow(unsafe_code)]")
            .collect();
        assert_eq!(allows.len(), 1, "{allows:?}");
        assert_eq!(lines[allows[0] + 1], "mod counter;");
        let global = std::fs::read_to_string(src.join("global.rs")).unwrap();
        assert!(!global.contains("unsafe"), "global.rs");
    }
}
