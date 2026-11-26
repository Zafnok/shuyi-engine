//! Each kind of prompt answered by a script ([`scripted::Scripted`]), the
//! lists' planned answers, and the wizard asked in order.

use std::path::PathBuf;

use super::scripted::{Scripted, Step, picked, typed};
use super::*;
use crate::contract::new::wizard::{Asker, Offered};
use crate::template::Answers;

/// A list question of `takes`, offering `answers` (id, text, status,
/// tickets joined by commas), at `default`.
fn question(takes: Takes, answers: &[(&str, &str, Status, &str)], default: &[&str]) -> Question {
    let mut question = wizard::engine_list("q", "Which one?", takes, &[], Vec::new(), "--q");
    question.answers = answers
        .iter()
        .map(|(id, text, status, tickets)| Offered {
            id: (*id).to_owned(),
            text: (*text).to_owned(),
            status: *status,
            tickets: tickets
                .split(',')
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect(),
        })
        .collect();
    question.default = default.iter().map(|d| (*d).to_owned()).collect();
    question
}

fn rules() -> Question {
    question(
        Takes::One,
        &[
            ("none", "none: rules in Rust", Status::Ready, ""),
            ("rhai", "Rhai scripts", Status::Planned, "2101,2102"),
            ("lua", "Lua scripts", Status::Parked, ""),
        ],
        &["none"],
    )
}

#[test]
fn a_list_offers_the_ready_answers_and_shows_the_others_under_it() {
    let list = list(&rules());
    assert_eq!(list.question, "Which one?");
    assert_eq!(
        list.choices,
        [("none".to_owned(), "none: rules in Rust".to_owned())]
    );
    assert_eq!(
        list.coming,
        [
            "Rhai scripts: planned, tickets 2101, 2102",
            "Lua scripts: parked"
        ]
    );
}

#[test]
fn a_planned_answer_cant_be_picked() {
    let mut script = Scripted::new([picked(&["rhai"])]);
    let refused = pick(&mut script, &rules()).unwrap_err();
    assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(refused.to_string(), "`rhai` is no choice of: Which one?");
    // Enter takes the default, the one ready answer.
    assert_eq!(pick(&mut Scripted::enters(1), &rules()).unwrap(), ["none"]);
}

#[test]
fn a_one_question_picks_one_its_default_at_first() {
    let pick_of = |default: &[&str], step: Step| {
        let question = question(
            Takes::One,
            &[
                ("public", "public", Status::Ready, ""),
                ("private", "private", Status::Ready, ""),
            ],
            default,
        );
        pick(&mut Scripted::new([step]), &question).unwrap()
    };
    assert_eq!(pick_of(&["private"], Step::Enter), ["private"]);
    assert_eq!(pick_of(&["public"], picked(&["private"])), ["private"]);
    // A default that is no choice starts at the first.
    assert_eq!(pick_of(&["gone"], Step::Enter), ["public"]);
    let question = question(Takes::One, &[("a", "a", Status::Ready, "")], &[]);
    let two = pick(&mut Scripted::new([picked(&["a", "a"])]), &question);
    assert!(two.unwrap_err().to_string().starts_with("one pick for"));
    let line = pick(&mut Scripted::new([typed("a")]), &question);
    assert!(line.unwrap_err().to_string().starts_with("a line typed"));
}

#[test]
fn a_many_question_ticks_any_its_defaults_ticked_at_first() {
    let question = question(
        Takes::Many,
        &[
            ("desktop", "desktop", Status::Ready, ""),
            ("web", "web", Status::Ready, ""),
            ("ios", "ios", Status::Planned, ""),
        ],
        &["desktop", "web"],
    );
    let mut script = Scripted::enters(1);
    assert_eq!(pick(&mut script, &question).unwrap(), ["desktop", "web"]);
    assert_eq!(script.lists[0].coming, ["ios: planned"]);
    let mut script = Scripted::new([picked(&["web"])]);
    assert_eq!(pick(&mut script, &question).unwrap(), ["web"]);
    let mut script = Scripted::new([picked(&[])]);
    assert_eq!(pick(&mut script, &question).unwrap(), Vec::<String>::new());
    let mut script = Scripted::new([picked(&["ios"])]);
    assert!(pick(&mut script, &question).is_err());
    let mut script = Scripted::new([typed("web")]);
    assert!(pick(&mut script, &question).is_err());
}

#[test]
fn a_list_with_nothing_ready_says_so_and_keeps_its_default() {
    let question = question(
        Takes::Many,
        &[("rhai", "Rhai scripts", Status::Planned, "2101")],
        &[],
    );
    assert_eq!(
        list(&question).coming,
        ["Rhai scripts: planned, ticket 2101"]
    );
    let mut script = Scripted::default();
    assert_eq!(pick(&mut script, &question).unwrap(), Vec::<String>::new());
    assert!(script.shown.is_empty(), "{:?}", script.shown);
}

#[test]
fn a_text_prompt_takes_its_default_or_a_line_and_asks_again_until_checked() {
    let check = |line: &str| {
        if line.contains(' ') {
            Err(format!("`{line}` holds a space"))
        } else {
            Ok(())
        }
    };
    let mut script = Scripted::enters(1);
    assert_eq!(script.text("Id?", "a.b", &check).unwrap(), "a.b");
    let mut script = Scripted::new([typed("a b"), typed("  c.d  ")]);
    assert_eq!(script.text("Id?", "a.b", &check).unwrap(), "c.d");
    assert_eq!(script.shown, ["Id?", "  `a b` holds a space"]);
    let mut script = Scripted::new([picked(&["x"])]);
    assert!(script.text("Id?", "", &check).is_err());
    let none = Scripted::default().text("Id?", "", &check).unwrap_err();
    assert_eq!(none.kind(), io::ErrorKind::UnexpectedEof);
    assert_eq!(none.to_string(), "no answer to: Id?");
}

#[test]
fn the_terminals_lists_show_the_coming_answers_and_the_help_under_the_last_choice() {
    let shown = list(&rules());
    let theme = Lists::new(&shown, ONE_HELP);
    let mut last = String::new();
    theme
        .format_select_prompt_item(&mut last, "none: rules in Rust", false)
        .unwrap();
    let plain = dialoguer::console::strip_ansi_codes(&last).into_owned();
    assert!(
        plain.ends_with(&format!(
            "none: rules in Rust\n    Rhai scripts: planned, tickets 2101, 2102 (not yet)\n    Lua \
         scripts: parked (not yet)\n  {ONE_HELP}"
        )),
        "{plain}"
    );
    let mut other = String::new();
    theme
        .format_multi_select_prompt_item(&mut other, "another", true, true)
        .unwrap();
    assert!(!other.contains('\n'), "{other}");
    // The prompts and their answers are the colourful theme's.
    let base = ColorfulTheme::default();
    let same = |a: fn(&dyn Theme, &mut String) -> fmt::Result| {
        let (mut ours, mut theirs) = (String::new(), String::new());
        a(&theme, &mut ours).unwrap();
        a(&base, &mut theirs).unwrap();
        assert_eq!(ours, theirs);
    };
    same(|t, f| t.format_prompt(f, "Q"));
    same(|t, f| t.format_error(f, "bad"));
    same(|t, f| t.format_input_prompt(f, "Q", Some("d")));
    same(|t, f| t.format_input_prompt_selection(f, "Q", "a"));
    same(|t, f| t.format_select_prompt(f, "Q"));
    same(|t, f| t.format_select_prompt_selection(f, "Q", "a"));
    same(|t, f| t.format_multi_select_prompt(f, "Q"));
    same(|t, f| t.format_multi_select_prompt_selection(f, "Q", &["a", "b"]));
}

/// An empty working folder.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("shuyi-prompts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_wizard_asks_the_name_the_id_the_perspectives_then_the_rest_in_order() {
    let scratch = Scratch::new("order");
    let mut input = NewInput::new("demo");
    let mut script = Scripted::new([
        typed("Moon Garden"),
        Step::Enter,
        Step::Enter,
        Step::Enter,
        picked(&["none"]),
        picked(&["none"]),
        picked(&["no"]),
        picked(&["desktop"]),
    ]);
    ask(&scratch.0, &mut input, &mut script).unwrap();
    assert!(script.steps.is_empty());
    // Only the questions asked are shown: those with nothing to pick
    // between, and the private files the GitHub questions answer, aren't.
    let asked: Vec<&str> = script.shown.iter().map(String::as_str).collect();
    assert_eq!(
        asked,
        [
            "What is the game called?",
            "A unique id for the game, like io.github.yourname.moon-garden: app stores ask for \
             one.",
            STORAGE_QUESTION,
            "Which perspectives?",
            "How do players play online?",
            "How are the rules written?",
            "Include the engine skills' worked examples (bots.rs, playtest.ron…)?",
            "Which targets does it ship on?",
        ]
    );
    // The perspectives: the grid ticked, the parked ones under the list.
    let perspectives = &script.lists[0];
    assert_eq!(perspectives.choices[0].0, "top-down-grid");
    assert!(
        perspectives
            .coming
            .iter()
            .any(|line| line.starts_with("top-down free: parked, tickets 1120")),
        "{:?}",
        perspectives.coming
    );
    // The name typed sets the id's default: Enter took it, so no --id; and
    // the id the saves' folder's, so no --storage.
    assert_eq!(input.name.as_deref(), Some("Moon Garden"));
    assert_eq!(input.id, None);
    assert_eq!(input.storage, None);
    assert_eq!(
        input.set,
        Answers(
            [
                ("perspective", vec!["top-down-grid"]),
                ("online", vec!["none"]),
                ("scripting", vec!["none"]),
                ("examples", vec!["no"]),
                ("targets", vec!["desktop"]),
            ]
            .into_iter()
            .map(|(id, picked)| (
                id.to_owned(),
                picked.into_iter().map(str::to_owned).collect()
            ))
            .collect()
        )
    );
}

#[test]
fn the_wizard_asks_again_for_the_perspectives_picked_and_not_what_is_given() {
    let scratch = Scratch::new("given");
    let mut input = NewInput::new("demo");
    input.id = Some("org.example.demo".to_owned());
    input.set = Answers::parse("set", r#"{ "scripting": ["none"], "examples": ["yes"] }"#).unwrap();
    // No perspective: the questions are listed again for none. Enter
    // takes online play's default, then the targets'.
    let mut script = Scripted::new([
        typed("demo"),
        Step::Enter,
        picked(&[]),
        Step::Enter,
        Step::Enter,
    ]);
    ask(&scratch.0, &mut input, &mut script).unwrap();
    assert_eq!(
        script.shown[..3],
        [
            "What is the game called?",
            STORAGE_QUESTION,
            "Which perspectives?"
        ]
    );
    assert_eq!(input.storage, None);
    assert_eq!(
        script.shown.last().unwrap(),
        "Which targets does it ship on?"
    );
    // The folder's own name, typed, is no --name.
    assert_eq!(input.name, None);
    assert_eq!(input.set.0["perspective"], Vec::<String>::new());
    assert_eq!(input.set.0["targets"], ["desktop", "web"]);
}

#[test]
fn the_wizard_checks_the_name_and_the_id_as_new_does() {
    let scratch = Scratch::new("checks");
    let mut input = NewInput::new("9");
    // The folder's name makes no crate: Enter is refused, then a name.
    let mut script = Scripted::new([
        Step::Enter,
        typed("Moon"),
        typed("not an id"),
        typed("org.example.moon"),
    ]);
    let ran = ask(&scratch.0, &mut input, &mut script);
    // The script ends at the perspectives.
    assert_eq!(ran.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    assert_eq!(input.name.as_deref(), Some("Moon"));
    assert_eq!(input.id.as_deref(), Some("org.example.moon"));
    assert_eq!(
        script.shown[1],
        "  `9` makes no crate name: it needs a letter first, then letters, digits or spaces"
    );
    assert!(script.shown[3].starts_with("  "), "{:#?}", script.shown);
}

/// The saves' folder's question, as a terminal asks it.
const STORAGE_QUESTION: &str = "Which folder are the game's saves kept in, on the player's \
                                computer? The id is the default, since it is nearly sure \
                                to be unique.";

/// A [`Prompter`] that keeps each text prompt's default, then answers as
/// `inner` does.
struct Defaults<'a> {
    inner: &'a mut Scripted,
    defaults: Vec<(String, String)>,
}

impl Prompter for Defaults<'_> {
    fn text(
        &mut self,
        question: &str,
        default: &str,
        check: &dyn Fn(&str) -> Result<(), String>,
    ) -> io::Result<String> {
        self.defaults
            .push((question.to_owned(), default.to_owned()));
        self.inner.text(question, default, check)
    }

    fn one(&mut self, list: &List, default: usize) -> io::Result<usize> {
        self.inner.one(list, default)
    }

    fn many(&mut self, list: &List, ticked: &[bool]) -> io::Result<Vec<usize>> {
        self.inner.many(list, ticked)
    }

    fn say(&mut self, line: &str) -> io::Result<()> {
        self.inner.say(line)
    }
}

#[test]
fn the_saves_folder_is_asked_after_the_id_with_the_id_typed_in() {
    let scratch = Scratch::new("storage");
    // The id typed is the saves' folder's default: Enter keeps it, and no
    // --storage is set.
    let mut input = NewInput::new("demo");
    let mut script = Scripted::new([Step::Enter, typed("org.example.moon"), Step::Enter]);
    let mut prompter = Defaults {
        inner: &mut script,
        defaults: Vec::new(),
    };
    let ran = ask(&scratch.0, &mut input, &mut prompter);
    assert_eq!(ran.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    assert_eq!(
        prompter.defaults[2],
        (STORAGE_QUESTION.to_owned(), "org.example.moon".to_owned())
    );
    assert_eq!(input.id.as_deref(), Some("org.example.moon"));
    assert_eq!(input.storage, None);

    // Without an id typed, its default: the folder's crate's.
    let mut input = NewInput::new("demo");
    let mut script = Scripted::new([Step::Enter, Step::Enter, Step::Enter]);
    let mut prompter = Defaults {
        inner: &mut script,
        defaults: Vec::new(),
    };
    let _ = ask(&scratch.0, &mut input, &mut prompter);
    assert_eq!(prompter.defaults[2].1, "com.example.demo");

    // A name check_storage refuses is asked again; another name is
    // --storage.
    let mut input = NewInput::new("demo");
    let mut script = Scripted::new([Step::Enter, Step::Enter, typed(".bad"), typed("Demo Saves")]);
    let ran = ask(&scratch.0, &mut input, &mut script);
    assert_eq!(ran.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    assert_eq!(input.storage.as_deref(), Some("Demo Saves"));
    assert_eq!(script.shown[2], STORAGE_QUESTION);
    assert_eq!(
        script.shown[3],
        format!("  {}", crate::app_info::check_storage(".bad").unwrap_err())
    );

    // --storage given: it isn't asked.
    let mut input = NewInput::new("demo");
    input.storage = Some("Demo Saves".to_owned());
    let mut script = Scripted::new([Step::Enter, Step::Enter]);
    let _ = ask(&scratch.0, &mut input, &mut script);
    assert!(!script.shown.iter().any(|s| s == STORAGE_QUESTION));
}

#[test]
fn the_wizard_asks_nothing_of_an_input_that_makes_no_game() {
    let scratch = Scratch::new("no-game");
    for input in [
        NewInput::new("demo").template("nope"),
        {
            let mut input = NewInput::new("demo");
            input.name = Some("9".to_owned());
            input
        },
        {
            let mut input = NewInput::new("demo");
            input.engine = crate::contract::new::Engine::Path {
                path: "nowhere".to_owned(),
            };
            input
        },
    ] {
        let mut input = input;
        let mut script = Scripted::default();
        ask(&scratch.0, &mut input, &mut script).unwrap();
        assert!(script.shown.is_empty(), "{:?}", script.shown);
    }
    assert_eq!(
        Asker::Engine,
        wizard::engine_text("x", "X?", "", "--x").asker
    );
}

#[test]
fn the_terminal_prompts_on_stderr() {
    let terminal = Terminal::default();
    assert!(matches!(
        terminal.term.target(),
        dialoguer::console::TermTarget::Stderr
    ));
}
