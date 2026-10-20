//! `new`'s templates are embedded from `templates/` at the repository's
//! root, outside this crate's folder: rebuild when they change.

fn main() {
    println!("cargo:rerun-if-changed=../../templates");
    println!("cargo:rerun-if-changed=build.rs");
}
