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
//!
//! **Memory tags** (ADR-0046 §6) say what the bytes are for. A
//! [`CountingAlloc::with_tags`] also counts each block under the tag
//! current on its thread when it was allocated: the frame's phase
//! (`update`, `draw`, …), the game's span, or the asset kind being
//! decoded, the innermost one entered winning ([`tags`]). Its
//! [`rows`](CountingAlloc::rows) give each tag's live bytes and blocks,
//! the bytes ever allocated under it and its peak ([`TagCounts`]). Tags
//! come from a process-wide table of 256: tag 0 is `other`, what is
//! allocated outside any tag, and tag 255 `overflow`, which every name
//! past the 254th shares ([`tags::overflowed`] counts them). A `realloc`
//! moves its block to the current tag, so a list that grows in `update`
//! counts in `update`.
//!
//! **The cost:** each block carries its tag in a header in front of it,
//! of its alignment or 8 bytes if that is more ([`Tagged`]), and each call
//! counts twice. The `global` counter has tags, in debug builds only; a
//! [`CountingAlloc::new`] has none, and no header.

#![cfg_attr(not(feature = "std"), no_std)]
// Counts are whole bytes (ADR-0032).
#![deny(clippy::float_arithmetic)]

extern crate alloc;

// The `GlobalAlloc` impls, the tags' table and the counts' cells: this
// crate's only `unsafe`.
#[allow(unsafe_code)]
mod counter;
#[cfg(feature = "global")]
pub mod global;

pub use counter::{CountingAlloc, Tagged, Untagged, tags};

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

/// One memory tag's counts in a [`CountingAlloc::with_tags`]
/// ([`CountingAlloc::rows`]), in bytes asked for. Build it with
/// [`TagCounts::new`] (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TagCounts {
    /// The tag ([`tags::tag`]).
    pub tag: u16,
    /// Its name's class, as [`tags::tag`] was given it;
    /// [`tags::OTHER_CLASS`] for `other` and `overflow`.
    pub class: u8,
    /// Its name: `other` for tag 0, `overflow` for tag 255.
    pub name: &'static str,
    /// The bytes live under it.
    pub live_bytes: u64,
    /// The blocks live under it.
    pub live_blocks: u32,
    /// The bytes ever allocated under it: a `realloc` adds its new size
    /// (a machine word: on a 32-bit target it wraps past 4 GiB).
    pub allocated_bytes: u64,
    /// The most bytes live under it at once.
    pub peak_bytes: u64,
}

impl TagCounts {
    /// Tag `tag`, named `name` of `class`, with nothing counted.
    pub const fn new(tag: u16, class: u8, name: &'static str) -> Self {
        Self {
            tag,
            class,
            name,
            live_bytes: 0,
            live_blocks: 0,
            allocated_bytes: 0,
            peak_bytes: 0,
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
