use std::fs;
use std::path::{Path, PathBuf};

use super::*;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const BASE: &str = "https://docs.invalid/engine/";

/// Every URL Zola builds for the fixture site's Learn.
const BUILT: [&str; 7] = [
    "learn/",
    "learn/book/",
    "learn/book/start/",
    "learn/book/later/",
    "learn/guide/",
    "learn/guide/map/",
    "learn/guide/tips/",
];

#[allow(clippy::unnecessary_wraps)]
fn fixed_date(_: &str) -> Result<String, String> {
    Ok("2026-10-28".to_owned())
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

fn put(dir: &Path, file: &str, text: &str) {
    let path = dir.join(file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest);
        } else {
            fs::copy(&path, &dest).unwrap();
        }
    }
}

/// The fixture site (`tests/fixtures/llms/`) in a folder of its own, with
/// the manifests (written here, so no other tool takes the fixture for
/// packages) and Zola's output for it, an `index.html` at each URL.
struct Fixture {
    root: PathBuf,
    public: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("xtask-llms-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        copy_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/llms"),
            &root,
        );
        put(
            &root,
            "Cargo.toml",
            "[workspace.package]\nversion = \"0.3.0\"\n",
        );
        put(
            &root,
            "crates/shuyi/Cargo.toml",
            "[package]\nname = \"shuyi\"\ndescription = \"The front door.\"\n",
        );
        put(
            &root,
            "crates/core/Cargo.toml",
            "[package]\nname = \"shuyi-core\"\ndescription = \"The core.\"\n",
        );
        put(
            &root,
            "crates/app/Cargo.toml",
            "[package]\nname = \"shuyi-app\"\ndescription = \"The backend.\"\n\n[lib]\nname = \"shuyi_window\"\n",
        );
        put(
            &root,
            "crates/helper/Cargo.toml",
            "[package]\nname = \"helper\"\n",
        );
        let public = root.join("public");
        for url in BUILT {
            put(&public, &format!("{url}index.html"), "<!doctype html>\n");
        }
        Self { root, public }
    }

    fn options(&self) -> Options {
        Options {
            dir: self.public.clone(),
            full_only: false,
            base_url: BASE.to_owned(),
            commit: COMMIT.to_owned(),
        }
    }

    fn run(&self) -> Outcome {
        self.run_with(&self.options(), FULL_BUDGET_BYTES)
    }

    fn run_with(&self, options: &Options, budget: usize) -> Outcome {
        run(
            &self.root,
            options,
            &Env {
                date: &fixed_date,
                budget,
            },
        )
    }

    fn passes(&self) -> Outcome {
        let outcome = self.run();
        assert!(outcome.errors.is_empty(), "{:#?}", outcome.errors);
        outcome
    }

    fn read(&self, file: &str) -> String {
        fs::read_to_string(self.public.join(file)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn has(errors: &[String], part: &str) -> bool {
    errors.iter().any(|e| e.contains(part))
}

// -- the options --

#[test]
fn options_read_the_site_and_end_the_base_with_a_slash() {
    let options = Options::parse(&args(&[
        "--public",
        "site/public",
        "--base-url",
        "https://zafnok.github.io/shuyi-engine",
        "--commit",
        "abc",
    ]))
    .unwrap();
    assert_eq!(
        options,
        Options {
            dir: PathBuf::from("site/public"),
            full_only: false,
            base_url: "https://zafnok.github.io/shuyi-engine/".to_owned(),
            commit: "abc".to_owned(),
        }
    );
}

#[test]
fn options_full_only_take_out_and_no_public() {
    let full = Options::parse(&args(&[
        "--full-only",
        "--out",
        "dist",
        "--base-url",
        "https://a.invalid/",
        "--commit",
        "abc",
    ]))
    .unwrap();
    assert!(full.full_only);
    assert_eq!(full.dir, PathBuf::from("dist"));
    for wrong in [
        &[
            "--full-only",
            "--public",
            "p",
            "--base-url",
            "u",
            "--commit",
            "c",
        ][..],
        &["--out", "o", "--base-url", "u", "--commit", "c"],
        &["--public", "p", "--base-url", "u"],
        &["--public", "p", "--commit", "c"],
        &[
            "--public",
            "p",
            "--base-url",
            "u",
            "--commit",
            "c",
            "--extra",
        ],
        &["--public"],
    ] {
        assert_eq!(Options::parse(&args(wrong)), None, "{wrong:?}");
    }
}

// -- the front matter --

#[test]
fn the_front_matter_gives_each_key() {
    let text = "+++\ntitle = \"A\"\ndescription = \"B\"\nweight = 30\ndraft = true\n\n[extra]\nchapter = 3\n+++\n\nBody\n";
    let front = front_matter("a.md", text).unwrap();
    assert_eq!(front.title, "A");
    assert_eq!(front.description.as_deref(), Some("B"));
    assert_eq!(front.weight, 30);
    assert!(front.draft);
    assert_eq!(&text[front.body..], "\nBody\n");
    assert_eq!(front.body_line, 10);
}

#[test]
fn the_front_matter_has_defaults_but_a_title() {
    let front = front_matter("a.md", "+++\ntitle = \"A\"\n+++\n").unwrap();
    assert_eq!(
        (front.description, front.weight, front.draft),
        (None, 0, false)
    );
    let e = front_matter("a.md", "+++\nweight = 1\n+++\n").unwrap_err();
    assert!(e.starts_with("a.md:1: no `title`"), "{e}");
}

#[test]
fn front_matter_that_isnt_toml_or_isnt_there_fails() {
    let e = front_matter("a.md", "+++\ntitle = \n+++\n").unwrap_err();
    assert!(e.contains("a.md: its front matter isn't TOML"), "{e}");
    let e = front_matter("a.md", "# A\n").unwrap_err();
    assert!(e.contains("a.md:1: no front matter"), "{e}");
    let e = front_matter("a.md", "+++\ntitle = \"A\"\n").unwrap_err();
    assert!(e.contains("no closing `+++`"), "{e}");
}

#[test]
fn a_slug_or_a_path_fails_on_its_line() {
    let e = front_matter("a.md", "+++\ntitle = \"A\"\nslug = \"b\"\n+++\n").unwrap_err();
    assert!(e.starts_with("a.md:3: sets `slug`"), "{e}");
    let e = front_matter("a.md", "+++\npath  = \"b\"\ntitle = \"A\"\n+++\n").unwrap_err();
    assert!(e.starts_with("a.md:2: sets `path`"), "{e}");
}

// -- the URL rule --

#[test]
fn a_url_is_the_file_s_name_or_its_folder_s() {
    assert_eq!(
        url_of("learn/book/quick-start.md").unwrap(),
        "learn/book/quick-start/"
    );
    assert_eq!(url_of("learn/book/_index.md").unwrap(), "learn/book/");
    assert_eq!(
        url_of("learn/guide/map/index.md").unwrap(),
        "learn/guide/map/"
    );
    assert_eq!(
        url_of("learn/errors/sh0001.md").unwrap(),
        "learn/errors/sh0001/"
    );
    assert_eq!(url_of("_index.md").unwrap(), "");
}

#[test]
fn a_name_not_in_slug_form_fails() {
    for source in [
        "learn/book/Quick-Start.md",
        "learn/book/quick_start.md",
        "learn/Book/a.md",
        "learn/2026-10-20-a.md",
        "learn/book/a.txt",
    ] {
        assert!(url_of(source).is_err(), "{source}");
    }
}

// -- the shortcodes --

fn snippets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/llms/site/snippets")
}

#[test]
fn a_snippet_is_its_anchor_s_lines_without_the_nested_anchors() {
    let source = "a\n// ANCHOR: all\nb\n    // ANCHOR: inner\n    c\n    // ANCHOR_END: inner\n// ANCHOR_END: all\nd\n";
    assert_eq!(anchored(source, "all").unwrap(), ["b", "    c"]);
    assert_eq!(anchored(source, "inner").unwrap(), ["    c"]);
    assert_eq!(anchored(source, "none"), None);
    assert_eq!(anchored("// ANCHOR: open\nb\n", "open"), None);
}

#[test]
fn a_snippet_is_the_same_text_as_its_anchored_lines() {
    let (text, errors) = expand(
        "a.md",
        "Before\n\n{{ snippet(file=\"src/garden.rs\", anchor=\"garden\") }}\n\nAfter\n",
        6,
        &snippets(),
    );
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        text,
        "Before\n\n```rust\n/// A garden of rows.\npub struct Garden {\n    pub rows: u8,\n}\n```\n\nAfter\n"
    );
}

#[test]
fn a_snippet_of_a_step_reads_the_tutorial_and_no_anchor_is_the_whole_file() {
    let root = std::env::temp_dir().join(format!("llms-step-{}", std::process::id()));
    let step = root.join("tutorial/01-new");
    fs::create_dir_all(&step).unwrap();
    let text = "first\n// ANCHOR: a\nsecond\n// ANCHOR_END: a\n";
    fs::write(step.join("f.ron"), text).unwrap();
    let snippets = root.join("snippets");
    let whole = snippet(
        "step=\"01-new\", file=\"f.ron\", language=\"ron\"",
        &snippets,
    );
    assert_eq!(whole.unwrap(), "```ron\nfirst\nsecond\n\n```");
    let one = snippet("step=\"01-new\", file=\"f.ron\", anchor=\"a\"", &snippets);
    assert_eq!(one.unwrap(), "```rust\nsecond\n```");
    let none = snippet("step=\"02-board\", file=\"f.ron\"", &snippets).unwrap_err();
    assert!(none.contains("site/tutorial/02-board/f.ron"), "{none}");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_snippet_takes_its_language_and_any_quotes() {
    let (text, errors) = expand(
        "a.md",
        "{{ snippet(language='ron',\n  file=`files/garden.ron`, anchor=\"rows\") }}",
        1,
        &snippets(),
    );
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(text, "```ron\n(rows: 3)\n```");
}

#[test]
fn a_snippet_s_unknown_argument_missing_anchor_or_no_file_fails_on_its_line() {
    let (_, errors) = expand(
        "a.md",
        "a\n{{ snippet(file=\"src/garden.rs\", anchor=\"garden\", lang=\"en\") }}\n",
        4,
        &snippets(),
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("a.md:5: snippet: no argument `lang`"),
        "{errors:?}"
    );
    let (_, errors) = expand(
        "a.md",
        "{{ snippet(file=\"src/garden.rs\", anchor=\"gone\") }}",
        1,
        &snippets(),
    );
    assert!(
        has(&errors, "a.md:1: snippet: no `// ANCHOR: gone`"),
        "{errors:?}"
    );
    let (_, errors) = expand("a.md", "{{ snippet(anchor=\"garden\") }}", 1, &snippets());
    assert!(has(&errors, "snippet: no `file`"), "{errors:?}");
}

#[test]
fn an_escaped_shortcode_becomes_plain() {
    let (text, errors) = expand(
        "a.md",
        "```text\n{{/* snippet(file=\"x\", anchor=\"y\") */}} and {%/* note() */%}\n```\n",
        1,
        &snippets(),
    );
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        text,
        "```text\n{{ snippet(file=\"x\", anchor=\"y\") }} and {% note() %}\n```\n"
    );
}

#[test]
fn an_unknown_shortcode_fails_with_its_file_and_line_even_in_code() {
    let (_, errors) = expand(
        "a.md",
        "a\n\n{{ video(id=\"x\") }}\n\n```\n{% note(kind=\"tip\") %}\n```\n",
        5,
        &snippets(),
    );
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors[0].starts_with("a.md:7: the shortcode `video`"),
        "{errors:?}"
    );
    assert!(
        errors[1].starts_with("a.md:10: the shortcode `note`"),
        "{errors:?}"
    );
}

#[test]
fn text_that_isnt_a_call_stays_as_it_is() {
    let body = "`{{name}}` and `{{ engine.docs }}` and {% if %}\n";
    let (text, errors) = expand("a.md", body, 1, &snippets());
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(text, body);
    let (_, errors) = expand("a.md", "{{ snippet(file=\"a\"\n", 1, &snippets());
    assert!(has(&errors, "has no closing `)`"), "{errors:?}");
}

#[test]
fn a_shortcode_left_outside_code_is_found() {
    let text =
        "# A\n\na {{ x }} b, `{{ inline }}`\n\n```\n{{ y }}\n```\n\n    {% z %}\n\n{% w %}\n";
    let errors = left_shortcodes("a/index.md", text);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors[0].starts_with("a/index.md:3:"), "{errors:?}");
    assert!(errors[1].starts_with("a/index.md:11:"), "{errors:?}");
}

// -- the links --

fn twins() -> BTreeMap<String, String> {
    [
        ("learn/book/x.md", "learn/book/x/"),
        ("learn/_index.md", "learn/"),
    ]
    .into_iter()
    .map(|(source, url)| (source.to_owned(), url.to_owned()))
    .collect()
}

fn rewritten(text: &str) -> (String, Vec<String>) {
    let twins = twins();
    let site = Site {
        base: BASE,
        twins: &twins,
    };
    rewrite_links("a.md", text, "learn/book/y/", &site)
}

#[test]
fn a_zola_link_goes_to_the_twin_with_its_anchor() {
    let (text, errors) =
        rewritten("See [x](@/learn/book/x.md#the-rows) and [Learn](@/learn/_index.md).");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        text,
        "See [x](https://docs.invalid/engine/learn/book/x/index.md#the-rows) and \
         [Learn](https://docs.invalid/engine/learn/index.md)."
    );
}

#[test]
fn a_zola_link_outside_learn_goes_to_its_html_page() {
    let (text, errors) =
        rewritten("[News](@/news/_index.md \"News\"), [card](@/examples/hello-grid.md#play)");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        text,
        "[News](https://docs.invalid/engine/news/ \"News\"), \
         [card](https://docs.invalid/engine/examples/hello-grid/#play)"
    );
}

#[test]
fn a_relative_link_or_image_is_made_absolute_against_the_page() {
    let (text, errors) =
        rewritten("![a](a.png) [up](../x/) [z](<z file.txt>) [api](/api/shuyi/index.html)");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        text,
        "![a](https://docs.invalid/engine/learn/book/y/a.png) \
         [up](https://docs.invalid/engine/learn/book/x/) \
         [z](<https://docs.invalid/engine/learn/book/y/z file.txt>) \
         [api](https://docs.invalid/engine/api/shuyi/index.html)"
    );
}

#[test]
fn a_reference_definition_is_rewritten() {
    let (text, errors) =
        rewritten("The [log][l] and [x].\n\n[l]: @/changelog.md\n[x]:\n  @/learn/book/x.md\n");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        text,
        "The [log][l] and [x].\n\n[l]: https://docs.invalid/engine/changelog/\n[x]:\n  \
         https://docs.invalid/engine/learn/book/x/index.md\n"
    );
}

#[test]
fn web_links_anchors_and_code_stay_as_they_are() {
    let text = "[web](https://example.com/a.md) [here](#a) [mail](mailto:a@b.invalid) `[c](c.md)`\n\n```\n[d](d.md)\n```\n";
    let (out, errors) = rewritten(text);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(out, text);
}

#[test]
fn a_zola_link_the_url_rule_doesnt_know_fails() {
    let (_, errors) = rewritten("[a](@/news/2026-10-20-hello.md)");
    assert!(
        has(&errors, "a.md: the link to `@/news/2026-10-20-hello.md`"),
        "{errors:?}"
    );
}

// -- the fixture site --

#[test]
fn the_fixture_s_llms_txt() {
    let fixture = Fixture::new("llms-txt");
    fixture.passes();
    insta::assert_snapshot!("llms-txt", fixture.read("llms.txt"));
}

#[test]
fn the_fixture_s_twin() {
    let fixture = Fixture::new("twin");
    fixture.passes();
    insta::assert_snapshot!("twin-start", fixture.read("learn/book/start/index.md"));
}

#[test]
fn the_fixture_s_llms_full_txt() {
    let fixture = Fixture::new("full");
    fixture.passes();
    insta::assert_snapshot!("llms-full-txt", fixture.read("llms-full.txt"));
}

#[test]
fn every_page_but_the_draft_has_its_twin() {
    let fixture = Fixture::new("twins");
    fixture.passes();
    for url in BUILT {
        let twin = fixture.read(&format!("{url}index.md"));
        assert!(twin.starts_with("# "), "{url}: {twin}");
        assert!(
            twin.contains(&format!("\nSource: {BASE}{url}\n")),
            "{url}: {twin}"
        );
    }
    assert!(!fixture.public.join("learn/book/draft/index.md").exists());
    assert!(!fixture.read("llms-full.txt").contains("A draft"));
}

#[test]
fn sections_and_pages_go_by_weight_then_by_name() {
    let fixture = Fixture::new("order");
    fixture.passes();
    let llms = fixture.read("llms.txt");
    let at = |part: &str| {
        llms.find(part)
            .unwrap_or_else(|| panic!("no {part} in {llms}"))
    };
    assert!(at("## Learn") < at("## The book"));
    assert!(at("## The book") < at("## The guide"));
    assert!(at("[Start]") < at("[Later]"));
    assert!(at("[Tips]") < at("[The map]"));
    let full = fixture.read("llms-full.txt");
    let titles: Vec<&str> = full.lines().filter(|line| line.starts_with("# ")).collect();
    assert_eq!(
        titles,
        [
            "# Shuyi Engine 0.3.0: every page of Learn",
            "# Learn",
            "# The book",
            "# Start",
            "# Later",
            "# The guide",
            "# Tips",
            "# The map",
        ]
    );
}

#[test]
fn the_schemas_are_copied_with_an_index() {
    let fixture = Fixture::new("schemas");
    fixture.passes();
    for name in [
        "versions.json",
        "hello.input.json",
        "hello.output.json",
        "wave.input.json",
    ] {
        let copied = fs::read(fixture.public.join("contract/schemas/v2").join(name)).unwrap();
        let source = fs::read(fixture.root.join(SCHEMAS).join(name)).unwrap();
        assert_eq!(copied, source, "{name}");
    }
    let index = fixture.read("contract/schemas/v2/index.md");
    assert!(
        index.contains(&format!(
            "| `hello` | [hello.input.json]({BASE}contract/schemas/v2/hello.input.json) | \
         [hello.output.json]({BASE}contract/schemas/v2/hello.output.json) |"
        )),
        "{index}"
    );
    assert!(index.contains("| `wave` | [wave.input.json]"), "{index}");
    assert!(index.contains(" | none |"), "{index}");
    assert!(
        fixture
            .read("llms.txt")
            .contains(&format!("({BASE}contract/schemas/v2/index.md)"))
    );
}

#[test]
fn the_api_links_name_each_shuyi_crate_by_rustdoc_s_folder() {
    let fixture = Fixture::new("api");
    let outcome = fixture.passes();
    assert!(
        has(&outcome.notes, "the API links weren't checked"),
        "{:?}",
        outcome.notes
    );
    let llms = fixture.read("llms.txt");
    for lib in ["shuyi", "shuyi_core", "shuyi_window"] {
        assert!(
            llms.contains(&format!("({BASE}api/{lib}/index.html)")),
            "{lib}: {llms}"
        );
    }
    assert!(!llms.contains("helper"), "{llms}");

    // With the API docs built, a missing crate is a broken link.
    put(&fixture.public, "api/shuyi/index.html", "");
    put(&fixture.public, "api/shuyi_core/index.html", "");
    let outcome = fixture.run();
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        has(&outcome.errors, "api/shuyi_window/index.html names no file"),
        "{:?}",
        outcome.errors
    );
    assert!(
        !has(&outcome.notes, "the API links weren't checked"),
        "{:?}",
        outcome.notes
    );
}

#[test]
fn llms_txt_says_none_yet_without_a_released_copy() {
    let fixture = Fixture::new("none-yet");
    fixture.passes();
    assert!(
        fixture
            .read("llms.txt")
            .contains("## Released versions\n\nNone yet.\n")
    );
}

#[test]
fn llms_txt_lists_the_released_copies_newest_first() {
    let fixture = Fixture::new("released");
    for version in ["v0.2", "v0.10", "v1.0"] {
        put(
            &fixture.public,
            &format!("{version}/llms-full.txt"),
            "frozen\n",
        );
    }
    put(&fixture.public, "v0.3/index.html", "no copy here\n");
    put(&fixture.public, "vnext/llms-full.txt", "not a version\n");
    fixture.passes();
    let llms = fixture.read("llms.txt");
    let listed: Vec<&str> = llms
        .lines()
        .filter(|line| line.starts_with("- [Shuyi Engine "))
        .collect();
    assert_eq!(
        listed,
        [
            format!(
                "- [Shuyi Engine 1.0]({BASE}v1.0/llms-full.txt): every page of Learn, as released"
            ),
            format!(
                "- [Shuyi Engine 0.10]({BASE}v0.10/llms-full.txt): every page of Learn, as released"
            ),
            format!(
                "- [Shuyi Engine 0.2]({BASE}v0.2/llms-full.txt): every page of Learn, as released"
            ),
        ]
    );
    assert!(!llms.contains("None yet"));
}

#[test]
fn the_full_file_over_its_budget_fails() {
    let fixture = Fixture::new("budget");
    let outcome = fixture.run_with(&fixture.options(), 100);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        has(&outcome.errors, "over its budget of 100"),
        "{:?}",
        outcome.errors
    );
    let size = fixture.read("llms-full.txt").len();
    assert!(has(
        &outcome.notes,
        &format!("llms-full.txt: {size} bytes, about {} tokens", size / 4)
    ));
    let errors = fixture.run_with(&fixture.options(), size).errors;
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_page_zola_didnt_build_fails() {
    let fixture = Fixture::new("unbuilt");
    fs::remove_file(fixture.public.join("learn/guide/tips/index.html")).unwrap();
    let outcome = fixture.run();
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        outcome.errors[0].starts_with(
            "site/content/learn/guide/tips.md:1: Zola built no /learn/guide/tips/index.html"
        ),
        "{:?}",
        outcome.errors
    );
}

#[test]
fn an_unknown_shortcode_in_a_page_fails_the_command() {
    let fixture = Fixture::new("unknown");
    put(
        &fixture.root,
        "site/content/learn/guide/tips.md",
        "+++\ntitle = \"Tips\"\n+++\n\nWatch:\n\n{{ video(id=\"x\") }}\n",
    );
    let outcome = fixture.run();
    assert!(
        has(
            &outcome.errors,
            "site/content/learn/guide/tips.md:7: the shortcode `video`"
        ),
        "{:?}",
        outcome.errors
    );
}

#[test]
fn a_page_with_a_slug_fails_the_command() {
    let fixture = Fixture::new("slug");
    put(
        &fixture.root,
        "site/content/learn/guide/tips.md",
        "+++\ntitle = \"Tips\"\nslug = \"hints\"\n+++\n",
    );
    let outcome = fixture.run();
    assert!(
        has(
            &outcome.errors,
            "site/content/learn/guide/tips.md:3: sets `slug`"
        ),
        "{:?}",
        outcome.errors
    );
}

#[test]
fn a_broken_llms_txt_link_fails_on_its_line() {
    let fixture = Fixture::new("links");
    let facts = Facts {
        version: "0.3.0".to_owned(),
        commit: COMMIT.to_owned(),
        date: "2026-10-28".to_owned(),
        base: BASE.to_owned(),
    };
    let llms = format!(
        "# A\n\n- [built]({BASE}learn/#top)\n- [gone]({BASE}learn/gone/index.md)\n\
         - [api]({BASE}api/shuyi/index.html)\n\
         - [here](https://raw.githubusercontent.com/{REPO}/{COMMIT}/docs/contract.md)\n\
         - [not here](https://github.com/{REPO}/blob/{COMMIT}/docs/gone.md)\n\
         - [elsewhere](https://example.com/gone)\n"
    );
    let errors = check_links(&llms, &facts, &fixture.root, &fixture.public, false);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors[0].starts_with(&format!(
            "llms.txt:4: {BASE}learn/gone/index.md names no file"
        )),
        "{errors:?}"
    );
    assert!(
        errors[1].starts_with("llms.txt:7: ")
            && errors[1].contains("names docs/gone.md, which isn't in the checkout"),
        "{errors:?}"
    );
    let errors = check_links(&llms, &facts, &fixture.root, &fixture.public, true);
    assert!(has(&errors, "llms.txt:5: "), "{errors:?}");
}

#[test]
fn full_only_writes_llms_full_txt_alone() {
    let fixture = Fixture::new("full-only");
    let out = fixture.root.join("out");
    let options = Options {
        dir: out.clone(),
        full_only: true,
        ..fixture.options()
    };
    // No Zola output: nothing reads it.
    fs::remove_dir_all(&fixture.public).unwrap();
    let outcome = fixture.run_with(&options, FULL_BUDGET_BYTES);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    let written: Vec<String> = fs::read_dir(&out)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(written, ["llms-full.txt"]);
    assert!(
        fs::read_to_string(out.join("llms-full.txt"))
            .unwrap()
            .contains("# Start")
    );
}

#[test]
fn a_commit_with_no_date_fails() {
    let fixture = Fixture::new("no-date");
    let outcome = run(
        &fixture.root,
        &fixture.options(),
        &Env {
            date: &|commit: &str| Err(format!("no commit {commit}")),
            budget: FULL_BUDGET_BYTES,
        },
    );
    assert_eq!(outcome.errors, [format!("no commit {COMMIT}")]);
}

#[test]
fn git_dates_a_commit_of_this_repository() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let date = git_date(&root)("HEAD").unwrap();
    assert_eq!(date.len(), 10, "{date}");
    assert!(git_date(&root)("not-a-commit").is_err());
}

// -- the tutorial's pictures, and the file it stays out of (ticket 2818) --

/// A site with the list of tries, `tries`, and the pictures in `pictures`.
fn tries_site(name: &str, tries: &str, pictures: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("llms-tries-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    put(&root, "tutorial/tries.toml", tries);
    for picture in pictures {
        put(&root, &format!("static/tutorial-tries/{picture}"), "png");
    }
    root
}

const BLUE: &str = "[blue]\nstep = \"01-new\"\nalt = \"Yellow\"\nafter = \"Blue\"\n\
    [[blue.edits]]\nfile = \"files/palette.ron\"\nfind = \"one\\ntwo\"\nreplace = \"three\"\n";

#[test]
fn a_try_is_its_edits_as_a_diff_and_its_pictures_as_images() {
    let root = tries_site("both", BLUE, &["blue/before.png", "blue/after.png"]);
    let text = try_it("name=\"blue\"", &root.join("snippets")).unwrap();
    assert_eq!(
        text,
        "In `files/palette.ron`:\n\n```diff\n- one\n- two\n+ three\n```\n\n\
         ![Yellow](/tutorial-tries/blue/before.png)\n\n![Blue](/tutorial-tries/blue/after.png)"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_try_with_no_edits_is_one_picture() {
    let list = "[title]\nstep = \"01-new\"\nalt = \"The title\"\n";
    let root = tries_site("one", list, &["title/before.png"]);
    let text = try_it("name=\"title\"", &root.join("snippets")).unwrap();
    assert_eq!(text, "![The title](/tutorial-tries/title/before.png)");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_try_s_missing_picture_unknown_name_or_argument_fails() {
    let root = tries_site("missing", BLUE, &["blue/before.png"]);
    let snippets = root.join("snippets");
    let picture = try_it("name=\"blue\"", &snippets).unwrap_err();
    assert!(picture.contains("blue/after.png"), "{picture}");
    assert!(
        picture.contains("cargo xtask tutorial-tries blue"),
        "{picture}"
    );
    let name = try_it("name=\"red\"", &snippets).unwrap_err();
    assert!(name.contains("no try `red`"), "{name}");
    let argument = try_it("color=\"red\"", &snippets).unwrap_err();
    assert!(argument.contains("no argument `color`"), "{argument}");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_tutorial_has_its_twin_and_its_line_but_stays_out_of_the_full_file() {
    let fixture = Fixture::new("for-people");
    put(
        &fixture.root,
        "site/content/learn/tutorial/_index.md",
        "+++\ntitle = \"The tutorial\"\n+++\n\nFrom nothing.\n",
    );
    put(
        &fixture.root,
        "site/content/learn/tutorial/01-new.md",
        "+++\ntitle = \"A new game\"\nweight = 1\n+++\n\nFirst steps.\n",
    );
    put(&fixture.public, "learn/tutorial/index.html", "");
    put(&fixture.public, "learn/tutorial/01-new/index.html", "");
    fixture.passes();
    assert!(
        fixture
            .read("learn/tutorial/01-new/index.md")
            .contains("First steps.")
    );
    assert!(
        fixture
            .read("llms.txt")
            .contains("learn/tutorial/01-new/index.md")
    );
    let full = fixture.read("llms-full.txt");
    assert!(!full.contains("First steps."), "{full}");
    assert!(!full.contains("From nothing."), "{full}");
}
