//! The web shell's JS plugins (`crates/cli/web/`: storage and
//! controllers) are functions the page provides at run time, not linked
//! in: on the web the binary links with `--allow-undefined`, so `rust-lld`
//! leaves them as imports (the web shell's README says why it is set
//! here).

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
