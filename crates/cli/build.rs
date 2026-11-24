//! `new`'s template, the skill pack it copies into a game and the engine's
//! target profiles are embedded from `templates/shuyi-default`, `skills/`
//! and `targets/` at the repository's root, outside this crate's folder:
//! `shuyi-embed` writes their tables, and watches them, so a change there
//! rebuilds `shuyi`.

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    shuyi_embed::Table::new("templates")
        .dir("../../templates/shuyi-default")
        .write();
    shuyi_embed::Table::new("skills")
        .dir("../../skills")
        .write();
    shuyi_embed::Table::new("targets")
        .dir("../../targets")
        .write();
}
