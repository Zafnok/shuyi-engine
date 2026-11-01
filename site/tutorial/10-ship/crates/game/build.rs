//! On the web the binary links with `--allow-undefined`: the page gives it
//! the shell's plugins (storage and controllers) when it runs.

// ANCHOR: build
fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH");
    let os = std::env::var("CARGO_CFG_TARGET_OS");
    if arch.as_deref() == Ok("wasm32") && os.as_deref() == Ok("unknown") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
// ANCHOR_END: build
