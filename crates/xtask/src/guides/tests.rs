//! The guides' check and their copy into the book, on fixtures.

use super::*;
use crate::modules::parse_concerns;

const CONCERNS_RON: &str = r#"(concerns: [
    (id: "perspective", name: "Perspective", alternatives: true, answers: [
        (id: "top-down-grid", name: "grid", by: "default", status: ready, tickets: []),
        (id: "side-view", name: "side", by: "default", status: parked, tickets: ["1127"]),
    ]),
    (id: "kind", name: "Kind of game", alternatives: false, answers: [
        (id: "tactics", name: "tactics", by: "default", status: ready, tickets: []),
    ]),
    (id: "world-drawing", name: "World drawing", alternatives: true, answers: [
        (id: "flat", name: "flat", by: "default", status: ready, tickets: []),
    ]),
    (id: "time", name: "Time", alternatives: true, answers: [
        (id: "turns", name: "turns", by: "default", status: ready, tickets: []),
        (id: "fixed-ticks", name: "ticks", by: "default", status: ready, tickets: []),
    ]),
])"#;

const README_MD: &str = "# Guides\n\nWhich parts go with which game. And more.\n\n\
                         - [Grid](grid.md)\n- [Side view](side-view.md)\n";

const GRID_MD: &str = "# Top-down grid\n\n\
                       - **Covers:** `perspective/top-down-grid`, `kind/tactics`, `time/turns`\n\
                       - **Status:** ready\n\n\
                       A board of [tiles](../../crates/core/). The rest.\n\n\
                       ```rust\nlet x = [1](2);\n```\n\n\
                       See [side view](side-view.md#until-then) and `[not](a/link)`.\n";

const SIDE_MD: &str = "# Side view\n\n\
                       - **Covers:** `perspective/side-view`\n\
                       - **Status:** planned\n\
                       - **Tickets:** 1127, 1122\n\n\
                       Platformers.\n";

fn concerns() -> Concerns {
    parse_concerns(CONCERNS_RON).unwrap()
}

fn tickets() -> Tickets {
    Tickets {
        open: ["1127".to_owned()].into(),
        done: ["1122".to_owned(), "2003".to_owned()].into(),
    }
}

fn guides(pages: &[(&str, &str)]) -> Vec<(String, String)> {
    pages
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect()
}

fn clean() -> Vec<(String, String)> {
    guides(&[("grid.md", GRID_MD), ("side-view.md", SIDE_MD)])
}

/// Every path a fixture's links name is there.
fn there(path: &str) -> bool {
    [
        "crates/core",
        "docs/guides/side-view.md",
        "docs/guides/grid.md",
        "docs/guides/move.md",
    ]
    .contains(&path)
}

/// The doc tests of the fixture: they include `grid.md`.
const DOC_TESTS_RS: &str = "#[cfg(doctest)]
#[doc = include_str!(\"../../../docs/guides/grid.md\")]
pub struct Grid;
";

fn errors_of(readme: &str, pages: &[(String, String)]) -> Vec<String> {
    check(&concerns(), readme, pages, &tickets(), DOC_TESTS_RS, there)
}

#[test]
fn clean_guides_pass() {
    assert_eq!(errors_of(README_MD, &clean()), Vec::<String>::new());
}

#[test]
fn an_answer_without_a_guide_fails() {
    let pages = guides(&[("grid.md", GRID_MD)]);
    let readme = "# Guides\n\n[Grid](grid.md)\n";
    let errors = errors_of(readme, &pages);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`perspective/side-view`"), "{errors:?}");
}

#[test]
fn flat_and_the_other_concerns_need_no_guide() {
    // `world-drawing/flat` and `time/fixed-ticks` are covered by no guide.
    let errors = errors_of(README_MD, &clean());
    assert!(
        !errors
            .iter()
            .any(|e| e.contains("flat") || e.contains("fixed-ticks"))
    );
}

#[test]
fn an_answer_not_in_the_concerns_fails() {
    let grid = GRID_MD.replace("`time/turns`", "`time/seasons`");
    let pages = guides(&[("grid.md", &grid), ("side-view.md", SIDE_MD)]);
    let errors = errors_of(README_MD, &pages);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`time/seasons`"), "{errors:?}");
}

#[test]
fn a_move_names_two_answers_that_exist() {
    let good = "# A to B\n\n- **Move:** `perspective/side-view` to `perspective/top-down-grid`\n\
                - **Status:** planned\n- **Tickets:** 1127\n\nText.\n";
    let bad = good.replace("`perspective/top-down-grid`", "`perspective/hex`");
    let readme = format!("{README_MD}- [Move](move.md)\n");
    let mut pages = clean();
    pages.push(("move.md".to_owned(), good.to_owned()));
    assert_eq!(errors_of(&readme, &pages), Vec::<String>::new());
    pages.pop();
    pages.push(("move.md".to_owned(), bad));
    let errors = errors_of(&readme, &pages);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`perspective/hex`"), "{errors:?}");
}

#[test]
fn a_link_to_nothing_fails() {
    let grid = GRID_MD.replace("../../crates/core/", "../../crates/gone/README.md");
    let readme =
        format!("{README_MD}\n[Out](../../../x.md) [Web](https://example.com) [Here](#guides)\n");
    let pages = guides(&[("grid.md", &grid), ("side-view.md", SIDE_MD)]);
    let errors = errors_of(&readme, &pages);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors.iter().any(|e| e.contains("climbs out")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("crates/gone/README.md isn't there")),
        "{errors:?}"
    );
}

#[test]
fn a_ticket_that_doesnt_exist_fails() {
    let side = SIDE_MD.replace("1127, 1122", "1127, 9999");
    let pages = guides(&[("grid.md", GRID_MD), ("side-view.md", &side)]);
    let errors = errors_of(README_MD, &pages);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("ticket 9999"), "{errors:?}");
}

#[test]
fn a_planned_guide_names_an_open_ticket() {
    let side = SIDE_MD.replace("1127, 1122", "1122");
    let pages = guides(&[("grid.md", GRID_MD), ("side-view.md", &side)]);
    let errors = errors_of(README_MD, &pages);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("names no open ticket"), "{errors:?}");
}

#[test]
fn a_guide_the_readme_doesnt_link_fails() {
    let readme = "# Guides\n\n[Grid](grid.md)\n";
    let errors = errors_of(readme, &clean());
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("side-view.md isn't linked"),
        "{errors:?}"
    );
}

#[test]
fn rust_no_doc_test_runs_fails() {
    let side = format!(
        "{SIDE_MD}
```rust
let y = 1;
```
"
    );
    let pages = guides(&[("grid.md", GRID_MD), ("side-view.md", &side)]);
    let errors = errors_of(README_MD, &pages);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("side-view.md has Rust"), "{errors:?}");
    // Other blocks need no doc test.
    let side = format!(
        "{SIDE_MD}
```bash
cargo run
```
"
    );
    let pages = guides(&[("grid.md", GRID_MD), ("side-view.md", &side)]);
    assert_eq!(errors_of(README_MD, &pages), Vec::<String>::new());
}

#[test]
fn facts_that_dont_read_fail() {
    for (text, says) in [
        ("No title\n", "first line"),
        (
            "# T\n\n- **Covers:** `kind/tactics`\n\nText.\n",
            "no `- **Status:**",
        ),
        (
            "# T\n\n- **Covers:** `kind/tactics`\n- **Status:** soon\n",
            "`soon`",
        ),
        (
            "# T\n\n- **Covers:** `kind/tactics`\n- **Owner:** me\n",
            "`Owner`",
        ),
        ("# T\n\n- **Status:** ready\n", "say what it covers"),
        (
            "# T\n\n- **Move:** `kind/tactics`\n- **Status:** ready\n",
            "two answers",
        ),
        ("# T\n\n- **Covers** `kind/tactics`\n", "isn't a fact"),
    ] {
        let err = facts("t.md", text).unwrap_err();
        assert!(err.contains(says), "{text:?}: {err}");
    }
}

#[test]
fn facts_read() {
    let facts = facts("side-view.md", SIDE_MD).unwrap();
    assert_eq!(facts.title, "Side view");
    assert_eq!(facts.covers, ["perspective/side-view"]);
    assert_eq!(facts.moves, Vec::<String>::new());
    assert_eq!(facts.status, Status::Planned);
    assert_eq!(facts.tickets, ["1127", "1122"]);
}

#[test]
fn links_skip_code() {
    assert_eq!(
        links(GRID_MD),
        ["../../crates/core/", "side-view.md#until-then"]
    );
}

#[test]
fn links_resolve_from_the_guides() {
    assert_eq!(resolve("x.md#a").as_deref(), Some("docs/guides/x.md"));
    assert_eq!(
        resolve("../modules.ron").as_deref(),
        Some("docs/modules.ron")
    );
    assert_eq!(
        resolve("../../crates/core/").as_deref(),
        Some("crates/core")
    );
    assert_eq!(resolve("../../../x"), None);
}

#[test]
fn links_on_the_site() {
    assert_eq!(site_target("x.md"), "@/learn/book/guides/x.md");
    assert_eq!(
        site_target("README.md#each"),
        "@/learn/book/guides/_index.md#each"
    );
    assert_eq!(
        site_target("../modules.md#how"),
        "https://github.com/Zafnok/shuyi-engine/blob/main/docs/modules.md#how"
    );
    assert_eq!(site_target("#here"), "#here");
    assert_eq!(site_target("https://example.com"), "https://example.com");
}

#[test]
fn a_page_on_the_site_keeps_its_code() {
    let body = site_body(GRID_MD);
    assert!(!body.contains("# Top-down grid"), "{body}");
    assert!(body.starts_with("- **Covers:**"), "{body}");
    assert!(body.contains("[tiles](https://github.com/Zafnok/shuyi-engine/blob/main/crates/core)"));
    assert!(body.contains("let x = [1](2);"), "{body}");
    assert!(body.contains("`[not](a/link)`"), "{body}");
    assert!(body.contains("[side view](@/learn/book/guides/side-view.md#until-then)"));
}

#[test]
fn a_description_is_the_first_sentence() {
    assert_eq!(description(GRID_MD), "A board of tiles.");
    assert_eq!(description(SIDE_MD), "Platformers.");
    assert_eq!(description(README_MD), "Which parts go with which game.");
}

#[test]
fn the_book_s_pages_follow_the_readme() {
    let pages = site_pages(
        README_MD,
        &guides(&[
            ("side-view.md", SIDE_MD),
            ("extra.md", SIDE_MD),
            ("grid.md", GRID_MD),
        ]),
    );
    let names: Vec<&str> = pages.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["_index.md", "grid.md", "side-view.md", "extra.md"]);
    let index = &pages[0].1;
    assert!(index.starts_with("+++\ntitle = \"Guides\"\n"), "{index}");
    assert!(index.contains("template = \"guides.html\""), "{index}");
    assert!(
        index.contains("[Grid](@/learn/book/guides/grid.md)"),
        "{index}"
    );
    let grid = &pages[1].1;
    assert!(
        grid.starts_with(
            "+++\ntitle = \"Top-down grid\"\ndescription = \"A board of tiles.\"\nweight = 10\n+++\n"
        ),
        "{grid}"
    );
    assert!(pages[2].1.contains("weight = 20"));
}

#[test]
fn toml_strings_escape() {
    assert_eq!(toml_string(r#"a "b" \c"#), r#""a \"b\" \\c""#);
}

#[test]
fn the_repository_s_guides_pass() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let concerns = parse_concerns(&fs::read_to_string(root.join(CONCERNS)).unwrap()).unwrap();
    assert_eq!(run(&root, &concerns), Vec::<String>::new());
}
