//! The web shell's JS plugins (`crates/cli/web/`: storage and
//! controllers) are functions the page provides at run time, not linked
//! in: on the web this package's own binaries, its `render_card` example
//! (ticket 2019), link with `--allow-undefined`, so `rust-lld` leaves them
//! as imports (the web shell's README says why it is set here). A game's
//! binary sets it in its own build script: a link argument reaches only
//! the package that prints it. Only on the web (`wasm32-unknown-unknown`).

fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH");
    let os = std::env::var("CARGO_CFG_TARGET_OS");
    if arch.as_deref() == Ok("wasm32") && os.as_deref() == Ok("unknown") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
