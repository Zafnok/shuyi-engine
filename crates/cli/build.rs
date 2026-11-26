//! `new`'s template, the skill pack it copies into a game and the engine's
//! target profiles are embedded from `templates/shuyi-default`, `skills/`
//! and `targets/` at the repository's root, outside this crate's folder:
//! `shuyi-embed` writes their tables, and watches them, so a change there
//! rebuilds `shuyi`.
//!
//! So are the corpus `docs` searches (ADR-0153): the Learn pages, the
//! book's snippets, the tutorial's files the snippets show, and the
//! examples' `.rs` and `.md` files.

/// What the examples' and the tutorial's tables leave out: everything but
/// the text `docs` reads (`.rs` and `.md`, and the tutorial's files a
/// snippet shows), and the builds' and tools' folders.
const NOT_TEXT: [&str; 18] = [
    "**/target/**",
    "**/.sentry-native/**",
    "**/*.png",
    "**/*.snap",
    "**/*.lock",
    "**/.gitignore",
    "**/*.map",
    "**/*.meta",
    "**/*.json",
    "**/*.dlg",
    "**/*.dat",
    "**/*.bin",
    "**/*.bdf",
    "**/*.txt",
    "**/*.wav",
    "**/*.ogg",
    "**/*.ttf",
    "**/*.toml",
];

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
    // Without the pages the site's build generates (ignored by git), so
    // the corpus doesn't depend on whether the site was built first: the
    // problem codes' pages are in it from their table already.
    shuyi_embed::Table::new("learn")
        .dir("../../site/content/learn")
        .skip("book/guides/**")
        .skip("errors/**")
        .write();
    shuyi_embed::Table::new("snippets")
        .dir("../../site/snippets")
        .skip("**/target/**")
        .write();
    let mut examples = shuyi_embed::Table::new("examples").dir("../../examples");
    for pattern in NOT_TEXT {
        examples = examples.skip(pattern);
    }
    examples.skip("**/*.ron").write();
    // The tutorial's steps: only the files a snippet shows (`step=`), its
    // crates' code, data and tests; not the builds, the art or the maps.
    let mut tutorial = shuyi_embed::Table::new("tutorial").dir("../../site/tutorial");
    for pattern in NOT_TEXT {
        // A snippet shows a level's map and a test's frame.
        if !matches!(pattern, "**/*.txt" | "**/*.map") {
            tutorial = tutorial.skip(pattern);
        }
    }
    tutorial
        .skip("**/font/**")
        .skip("**/examples/**")
        .skip("**/.cargo/**")
        .write();
}
