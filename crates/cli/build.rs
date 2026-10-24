//! `new`'s templates and the skill pack it copies into a game are embedded
//! from `templates/` and `skills/` at the repository's root, outside this
//! crate's folder: rebuild when they change.

fn main() {
    println!("cargo:rerun-if-changed=../../templates");
    println!("cargo:rerun-if-changed=../../skills");
    println!("cargo:rerun-if-changed=build.rs");
}
