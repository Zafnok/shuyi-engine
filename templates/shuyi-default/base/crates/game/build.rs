//! Two things the build needs from outside the code.
//!
//! The folder of the game's files the program embeds (`src/lib.rs`): `files`,
//! unless `cargo shuyi export --edition <name>` builds an edition with a
//! content root of its own (`shuyi.ron`'s `editions`), which it hands over as
//! `SHUYI_CONTENT`. A public demo built from its own folder holds none of the
//! full game's files.
//!
//! The web shell's JS plugins (storage and controllers) are functions the
//! page gives the game when it runs, not linked in: on the web the binary
//! links with `--allow-undefined`, so `rust-lld` leaves them as imports
//! (`cargo shuyi web` builds it). Not in `.cargo/config.toml`'s
//! `rustflags`: a `RUSTFLAGS` variable, as CI sets, replaces those. Only
//! on the web (`wasm32-unknown-unknown`): under WASI no page gives them.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SHUYI_CONTENT");
    let files = std::env::var("SHUYI_CONTENT").unwrap_or_else(|_| {
        let here = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        format!("{here}/files")
    });
    println!("cargo:rustc-env=SHUYI_FILES={files}");
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH");
    let os = std::env::var("CARGO_CFG_TARGET_OS");
    if arch.as_deref() == Ok("wasm32") && os.as_deref() == Ok("unknown") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
