use std::collections::HashSet;

use super::*;

/// The fixture crate's index (`rustdoc_index/fixture`).
fn index() -> Index {
    Index::read(include_str!("../rustdoc_index/fixture/fixture.json")).unwrap()
}

/// The fixture crate's listing, as `cargo public-api --simplified` wrote it.
fn listing() -> Vec<String> {
    include_str!("../rustdoc_index/fixture/listing.txt")
        .lines()
        .map(str::to_owned)
        .collect()
}

fn ids(list: &[&str]) -> HashSet<String> {
    list.iter().map(|id| (*id).to_owned()).collect()
}

/// A row with a counterpart, naming `path` with `aliases`.
fn row(path: &str, aliases: &[&str]) -> Term {
    Term {
        shuyi: "a `Countdown`".to_owned(),
        paths: vec![path.to_owned()],
        aliases: aliases.iter().map(|a| (*a).to_owned()).collect(),
        godot: "a timer".to_owned(),
        unity: "—".to_owned(),
        bevy: "`Timer`".to_owned(),
        gamemaker: "an alarm".to_owned(),
        pico8: "—".to_owned(),
        ..Term::default()
    }
}

/// A "none yet" row planned by `ticket`.
fn none_yet(ticket: Option<&str>, planned: Option<bool>) -> Term {
    Term {
        shuyi: "none yet".to_owned(),
        paths: Vec::new(),
        aliases: Vec::new(),
        ticket: ticket.map(str::to_owned),
        planned,
        ..row("x", &[])
    }
}

/// The terms the fixture's aliases match: the row gives `Timer`, two guesses
/// `Alarm` and `Delay`.
fn matching() -> Terms {
    Terms {
        term: vec![row("fixture::timer::Countdown", &["Timer"])],
        guess: vec![
            Guess {
                name: "Alarm".to_owned(),
                path: "fixture::Countdown".to_owned(),
            },
            Guess {
                name: "Delay".to_owned(),
                path: "fixture::timer::Countdown".to_owned(),
            },
        ],
    }
}

#[test]
fn options_read_out_or_check() {
    let args = |list: &[&str]| list.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>();
    assert_eq!(Options::parse(&args(&["--check"])), Some(Options::Check));
    assert_eq!(
        Options::parse(&args(&["--out", "site/public/api"])),
        Some(Options::Out(PathBuf::from("site/public/api")))
    );
    assert_eq!(Options::parse(&args(&["--out"])), None);
    assert_eq!(Options::parse(&args(&["--check", "--out", "x"])), None);
}

#[test]
fn the_terms_in_the_repository_have_their_shape() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let terms = read_terms(&fs::read_to_string(root.join(TERMS)).unwrap()).unwrap();
    let open = ticket_ids(&root.join("tickets/open")).unwrap();
    let done = ticket_ids(&root.join("tickets/done")).unwrap();
    assert_eq!(shape(&terms, &open, &done), Vec::<String>::new());
    assert!(terms.term.len() >= 19, "{}", terms.term.len());
    // The tween row is no longer "none yet" (2109 shipped `Tween`).
    let tween = terms
        .term
        .iter()
        .find(|term| term.godot == "a tween")
        .unwrap();
    assert_eq!(tween.paths, ["shuyi_core::ease::Tween"]);
    assert!(tween.shuyi.contains("frame-and-skins.md#things-that-move"));
    for name in [
        "draw_sprite",
        "is_key_down",
        "just_pressed",
        "TileMap",
        "Vec2",
        "Timer",
    ] {
        assert!(terms.guess.iter().any(|guess| guess.name == name), "{name}");
    }
}

#[test]
fn read_terms_refuses_a_field_it_doesnt_know() {
    let error = read_terms("[[term]]\nshuyi = \"x\"\ncolour = \"red\"\n").unwrap_err();
    assert!(error.contains(TERMS) && error.contains("colour"), "{error}");
}

#[test]
fn a_none_yet_row_whose_ticket_is_done_is_out_of_date() {
    let terms = Terms {
        term: vec![none_yet(Some("2109"), None)],
        guess: Vec::new(),
    };
    let errors = shape(&terms, &ids(&[]), &ids(&["2109"]));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("its ticket 2109 is done: the row is out of date"),
        "{errors:?}"
    );
}

#[test]
fn a_none_yet_row_names_an_open_ticket_or_says_unplanned() {
    let check = |term: Term| {
        shape(
            &Terms {
                term: vec![term],
                guess: Vec::new(),
            },
            &ids(&["2221"]),
            &ids(&[]),
        )
    };
    assert_eq!(check(none_yet(Some("2221"), None)), Vec::<String>::new());
    assert_eq!(check(none_yet(None, Some(false))), Vec::<String>::new());
    assert!(check(none_yet(Some("9999"), None))[0].contains("isn't in tickets/open/"));
    assert!(check(none_yet(None, None))[0].contains("`planned = false`"));
    assert!(check(none_yet(Some("2221"), Some(false)))[0].contains("`planned = false`"));
    let mut with_path = none_yet(Some("2221"), None);
    with_path.paths = vec!["fixture::Inner".to_owned()];
    assert!(check(with_path)[0].contains("names a path"));
    let mut counterpart = row("fixture::Inner", &[]);
    counterpart.ticket = Some("2221".to_owned());
    assert!(check(counterpart)[0].contains("are for a \"none yet\" row"));
}

#[test]
fn the_shape_refuses_an_alias_with_a_space_or_no_path_and_a_bad_guess() {
    let mut no_path = row("x", &["Timer"]);
    no_path.paths.clear();
    let mut blank = row("fixture::Inner", &[]);
    blank.pico8 = " ".to_owned();
    let guess = Guess {
        name: "draw sprite".to_owned(),
        path: "fixture::Inner".to_owned(),
    };
    let terms = Terms {
        term: vec![row("fixture::Inner", &["a timer"]), no_path, blank],
        guess: vec![guess.clone(), guess],
    };
    let errors = shape(&terms, &ids(&[]), &ids(&[]));
    assert_eq!(errors.len(), 6, "{errors:?}");
    assert!(errors[0].contains("`a timer` isn't an identifier"));
    assert!(errors[1].contains("has aliases and no path"));
    assert!(errors[2].contains("has no PICO-8 words"));
    assert!(errors[3].contains("the guess `draw sprite` isn't an identifier"));
    assert!(errors[5].contains("is there twice"));
}

#[test]
fn the_check_passes_when_the_terms_and_the_aliases_agree() {
    assert_eq!(
        check(&matching(), &[index()], &ids(&[]), &ids(&[])),
        Vec::<String>::new()
    );
}

#[test]
fn the_check_fails_on_a_path_that_names_no_item_or_no_crate() {
    let mut terms = matching();
    terms.term.push(row("fixture::timer::Stopwatch", &[]));
    terms.term.push(row("shuyi_nope::Thing", &[]));
    let errors = check(&terms, &[index()], &ids(&[]), &ids(&[]));
    assert_eq!(
        errors,
        [
            format!("{TERMS}: `fixture::timer::Stopwatch` names no public item of fixture"),
            format!("{TERMS}: `shuyi_nope::Thing` names no crate of {API_TOML}"),
        ]
    );
}

#[test]
fn the_check_fails_on_an_alias_the_item_lacks() {
    let mut terms = matching();
    terms.guess.push(Guess {
        name: "Cooldown".to_owned(),
        path: "fixture::Countdown".to_owned(),
    });
    let errors = check(&terms, &[index()], &ids(&[]), &ids(&[]));
    assert_eq!(
        errors,
        [format!(
            "`fixture::Countdown` lacks `#[doc(alias = \"Cooldown\")]`, which {TERMS} gives it"
        )]
    );
}

#[test]
fn the_check_fails_on_a_doc_alias_the_terms_lack() {
    let mut terms = matching();
    terms.guess.pop();
    let errors = check(&terms, &[index()], &ids(&[]), &ids(&[]));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with(
            "`fixture::Countdown` has `#[doc(alias = \"Delay\")]`, which site/data/engine-terms.toml \
             doesn't name"
        ),
        "{errors:?}"
    );
}

#[test]
fn the_check_fails_on_a_none_yet_row_whose_ticket_is_done() {
    let mut terms = matching();
    terms.term.push(none_yet(Some("2109"), None));
    let errors = check(&terms, &[index()], &ids(&[]), &ids(&["2109"]));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("is done"), "{errors:?}");
}

#[test]
fn listed_path_reads_an_items_path_without_its_generics() {
    let cases = [
        (
            "pub fn shuyi_ui::input::frame::FrameInput<A>::held(&self) -> &[A]",
            Some("shuyi_ui::input::frame::FrameInput::held"),
        ),
        (
            "#[non_exhaustive] pub struct shuyi_core::ease::Tween<T, E>",
            Some("shuyi_core::ease::Tween"),
        ),
        (
            "pub const fn shuyi_core::timer::Countdown::new(u32) -> Self",
            Some("shuyi_core::timer::Countdown::new"),
        ),
        (
            "pub shuyi_core::grid::Pos::x: i16",
            Some("shuyi_core::grid::Pos::x"),
        ),
        (
            "pub fixture::Align::Centered",
            Some("fixture::Align::Centered"),
        ),
        (
            "pub const shuyi_core::rng::SimRng::SEED: u64",
            Some("shuyi_core::rng::SimRng::SEED"),
        ),
        ("pub mod fixture", None),
        ("impl core::clone::Clone for fixture::Countdown", None),
        (
            "pub fn shuyi_ui::screen::Transition<G>::pop_with<T: core::any::Any>(T) -> Self",
            Some("shuyi_ui::screen::Transition::pop_with"),
        ),
        ("pub macro shuyi::files!", Some("shuyi::files")),
    ];
    for (line, path) in cases {
        assert_eq!(listed_path(line).as_deref(), path, "{line}");
    }
}

#[test]
fn first_sentence_takes_the_first_paragraph_to_its_first_stop() {
    assert_eq!(
        first_sentence("A timer: it fires once.\n\nMore words."),
        "A timer: it fires once."
    );
    assert_eq!(
        first_sentence("A sprite: part of an image,\ndrawn at a rectangle. It is more."),
        "A sprite: part of an image, drawn at a rectangle."
    );
    assert_eq!(
        first_sentence("Reads [`Pos`](crate::Pos) and [the map][map]. Then [`x`]."),
        "Reads `Pos` and the map."
    );
    assert_eq!(
        first_sentence("Calls `a.b()` first. Then."),
        "Calls `a.b()` first."
    );
    assert_eq!(first_sentence("No stop at all"), "No stop at all");
    assert_eq!(first_sentence("\nStarts late?\n"), "Starts late?");
    assert_eq!(first_sentence("Ends at code\n```\nx\n```"), "Ends at code");
    assert_eq!(first_sentence(""), "");
}

#[test]
fn an_auto_traits_impl_is_left_out_and_another_kept() {
    assert!(is_auto_impl("impl core::marker::Send for fixture::Align"));
    assert!(is_auto_impl(
        "impl<T> core::panic::unwind_safe::UnwindSafe for fixture::Tween<T> where T: x"
    ));
    assert!(is_auto_impl("impl !core::marker::Sync for fixture::Cell"));
    assert!(!is_auto_impl(
        "impl core::clone::Clone for fixture::Countdown"
    ));
    assert!(!is_auto_impl("impl fixture::Countdown"));
    assert!(!is_auto_impl(
        "pub fn fixture::Countdown::restart(&mut self)"
    ));
}

#[test]
fn the_text_of_the_fixture_crate() {
    insta::assert_snapshot!(text(
        "fixture",
        "a crate for the tests",
        &listing(),
        &index()
    ));
}

#[test]
fn the_workflows_install_the_api_jobs_tools_and_run_api_text() {
    let pages = include_str!("../../../../.github/workflows/pages.yml");
    let ci = include_str!("../../../../.github/workflows/ci.yml");
    for pinned in [
        format!(
            "rustup toolchain install {} --profile minimal",
            api::NIGHTLY
        ),
        format!(
            "cargo install --locked cargo-public-api@{}",
            api::PUBLIC_API
        ),
        "cargo xtask api-text --out site/public/api".to_owned(),
    ] {
        assert!(pages.contains(&pinned), "pages.yml lacks `{pinned}`");
    }
    // After the API docs are copied in (they replace site/public/api), and
    // before llms.txt's links to the files are checked.
    let text = pages.find("cargo xtask api-text").unwrap();
    assert!(pages.find("cp -r target/doc site/public/api").unwrap() < text);
    assert!(text < pages.find("cargo xtask llms --public").unwrap());
    assert!(ci.contains("cargo xtask api-text --check"));
}

#[test]
fn site_crates_pairs_each_crate_of_api_toml_with_its_check() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let crates = site_crates(&root).unwrap();
    // Not a count: the `sound-give-up` job runs the tests with the sound
    // crates removed from api.toml.
    for (krate, entry) in &crates {
        assert_eq!(entry.name.replace('-', "_"), krate.name);
    }
    for lib in ["shuyi", "shuyi_core", "shuyi_ui", "shuyi_platform"] {
        assert!(crates.iter().any(|(krate, _)| krate.name == lib), "{lib}");
    }
    let (ui, entry) = crates
        .iter()
        .find(|(krate, _)| krate.name == "shuyi_ui")
        .unwrap();
    assert_eq!(entry.name, "shuyi-ui");
    assert!(ui.text.contains("frame"));
}
