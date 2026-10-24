//! The web shell's JS plugins (storage and controllers) are functions the
//! page gives the game when it runs, not linked in: on the web the binary
//! links with `--allow-undefined`, so the linker leaves them as imports.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
