//! Three things the build needs from outside the code.
//!
//! The folder of the game's files the program embeds (`src/lib.rs`): `files`,
//! unless `cargo shuyi export --edition <name>` builds an edition with a
//! content root of its own (`shuyi.ron`'s `editions`), which it hands over as
//! `SHUYI_CONTENT`. A public demo built from its own folder holds none of the
//! full game's files.
//!
//! The folder of its private files, laid over those (the engine's ADR-0111):
//! `private/files/` of the game's folder whenever it is there (a checkout of
//! the private repository, `cargo shuyi private-assets`), else an empty
//! folder. An edition with its own content root gets the empty one too.
//!
//! The web shell's JS plugins (storage and controllers) are functions the
//! page gives the game when it runs, not linked in: on the web the binary
//! links with `--allow-undefined`, so `rust-lld` leaves them as imports
//! (`cargo shuyi web` builds it). Not in `.cargo/config.toml`'s
//! `rustflags`: a `RUSTFLAGS` variable, as CI sets, replaces those. Only
//! on the web (`wasm32-unknown-unknown`): under WASI no page gives them.

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SHUYI_CONTENT");
    let here = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let content = std::env::var("SHUYI_CONTENT").ok();
    let files = content.clone().unwrap_or_else(|| format!("{here}/files"));
    println!("cargo:rustc-env=SHUYI_FILES={files}");
    let root = Path::new(&here).join("../..");
    let private = match content {
        Some(_) => None,
        None => private_files(&root),
    };
    let private = private.unwrap_or_else(no_private_files);
    println!("cargo:rustc-env=SHUYI_PRIVATE_FILES={}", private.display());
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH");
    let os = std::env::var("CARGO_CFG_TARGET_OS");
    if arch.as_deref() == Ok("wasm32") && os.as_deref() == Ok("unknown") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}

/// The checkout's `private/files/`, watched, if it is there. Else it watches
/// what a checkout made later changes: the checkout's index, or the folder
/// `private/` (made empty when the game has a pin, for `cargo shuyi
/// private-assets` to clone into). A watched path that doesn't exist would
/// rebuild the crate on every build.
fn private_files(root: &Path) -> Option<PathBuf> {
    let checkout = root.join("private");
    let files = checkout.join("files");
    if files.is_dir() {
        println!("cargo:rerun-if-changed={}", files.display());
        return Some(files);
    }
    let index = checkout.join(".git/index");
    if index.is_file() {
        println!("cargo:rerun-if-changed={}", index.display());
    } else if checkout.is_dir()
        || (root.join("private.rev").is_file() && std::fs::create_dir_all(&checkout).is_ok())
    {
        println!("cargo:rerun-if-changed={}", checkout.display());
    }
    None
}

/// An empty folder of the build's own: an `include_dir!` path must exist.
fn no_private_files() -> PathBuf {
    let out = std::env::var("OUT_DIR").unwrap_or_default();
    let empty = Path::new(&out).join("no-private-files");
    let _ = std::fs::create_dir_all(&empty);
    empty
}
