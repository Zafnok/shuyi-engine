//! The web shell's JS plugins (storage and controllers) are functions the
//! page gives the game when it runs, not linked in: on the web the binary
//! links with `--allow-undefined`, so `rust-lld` leaves them as imports
//! (`cargo shuyi web` builds it). Only on the web
//! (`wasm32-unknown-unknown`): under WASI no page gives them.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH");
    let os = std::env::var("CARGO_CFG_TARGET_OS");
    if arch.as_deref() == Ok("wasm32") && os.as_deref() == Ok("unknown") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
