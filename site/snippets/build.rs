//! The book's chapter 9 shows this file: a game's `crates/game/build.rs`,
//! which its web build needs. The web page's JS plugins (storage and
//! controllers) are functions the page provides at run time, not linked
//! in: on the web the binary links with `--allow-undefined`, so the
//! linker leaves them as imports.

// ANCHOR: build
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
// ANCHOR_END: build
