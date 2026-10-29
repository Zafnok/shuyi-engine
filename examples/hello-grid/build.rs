//! The web shell's JS plugins (`crates/cli/web/`: storage and
//! controllers) are functions the page provides at run time, not linked
//! in: on the web the binary links with `--allow-undefined`, so `rust-lld`
//! leaves them as imports (the web shell's README says why it is set
//! here). Only on the web (`wasm32-unknown-unknown`): under WASI, where
//! CI's `determinism` job runs the golden test (ticket 2116), no page
//! provides them.

fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH");
    let os = std::env::var("CARGO_CFG_TARGET_OS");
    if arch.as_deref() == Ok("wasm32") && os.as_deref() == Ok("unknown") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
