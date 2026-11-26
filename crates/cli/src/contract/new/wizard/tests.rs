//! The wizard's questions, its flags and targets, and the command that
//! makes a game again.

use std::path::PathBuf;

use super::*;
use crate::skills::Pack;
use crate::template::built_in;

#[test]
fn the_engines_concerns_each_have_a_flag_and_time_has_none() {
    assert_eq!(
        concern_flag("--perspective").as_deref(),
        Some("perspective")
    );
    assert_eq!(concern_flag("--bodies").as_deref(), Some("bodies"));
    assert_eq!(concern_flag("--kind").as_deref(), Some("kind"));
    assert_eq!(concern_flag("--time"), None);
    assert_eq!(concern_flag("--nope"), None);
    assert_eq!(concern_flag("bodies"), None);
}

#[test]
fn the_targets_are_the_engines_profiles_and_default_to_the_templates() {
    assert_eq!(default_targets(&built_in().unwrap()), ["desktop", "web"]);
    let targets = engine_targets().unwrap();
    let ready = |id: &str| targets.iter().find(|t| t.id == id).map(|t| t.ready);
    assert_eq!(ready("desktop"), Some(true));
    assert_eq!(ready("playdate"), Some(true));
    assert_eq!(ready("switch"), Some(false));
}

/// A catalog of `concerns`, a `docs/modules.ron` text, with no module.
fn catalog(concerns: &str) -> Catalog {
    Catalog {
        root: PathBuf::new(),
        concerns: Concerns::parse(concerns).unwrap_or_else(|e| panic!("{e}")),
        modules: Vec::new(),
        skills: Pack::default(),
    }
}

/// A catalog where a free perspective is ready, and bodies and motion has
/// a ready answer and a planned one.
const FREE: &str = r#"(concerns: [
    (id: "time", name: "Time", alternatives: true, answers: [
        (id: "turns", name: "turns", by: "default", status: ready, tickets: []),
        (id: "fixed-ticks", name: "fixed ticks", by: "default", status: ready, tickets: []),
    ]),
    (id: "perspective", name: "Perspective", alternatives: true, pick: Many,
     question: "Which perspectives?", answers: [
        (id: "top-down-grid", name: "top-down grid", by: "default", status: ready, tickets: []),
        (id: "top-down", name: "top-down free", by: "default", status: ready, tickets: []),
        (id: "side-view", name: "side view", by: "default", status: parked, tickets: ["1127"]),
    ]),
    (id: "bodies", name: "Bodies and motion", alternatives: true, pick: Many,
     question: "How do bodies move?", serves: ["top-down", "side-view"], answers: [
        (id: "movers", name: "movers", by: "default", status: ready, tickets: ["1122"]),
        (id: "rigid-body", name: "rigid-body physics", by: "shuyi-physics", status: planned, tickets: ["1128"]),
    ]),
    (id: "online", name: "Online play", alternatives: true, pick: One,
     question: "How do players play online?", answers: [
        (id: "none", name: "none", by: "default", status: ready, tickets: []),
        (id: "lockstep", name: "lockstep", by: "shuyi-net", status: parked, tickets: ["1140", "1141"]),
    ]),
])"#;

/// The targets the fixtures offer.
fn targets() -> Vec<Target> {
    vec![
        Target {
            id: "desktop".into(),
            ready: true,
        },
        Target {
            id: "switch".into(),
            ready: false,
        },
    ]
}

fn named() -> Named {
    Named::new(
        "moon".into(),
        "Moon".into(),
        "com.example.moon".into(),
        None,
    )
}

fn answers(ron: &str) -> Answers {
    Answers::parse("given", ron).unwrap()
}

/// The questions of the fixture catalog `concerns` for `given`.
fn asked(concerns: &str, given: &str) -> Vec<Question> {
    questions(
        &catalog(concerns),
        &built_in().unwrap(),
        &targets(),
        &named(),
        &answers(given),
    )
}

fn question<'a>(questions: &'a [Question], id: &str) -> &'a Question {
    questions
        .iter()
        .find(|q| q.id == id)
        .unwrap_or_else(|| panic!("no question {id}"))
}

#[test]
fn the_questions_come_in_order_and_time_is_never_asked() {
    let questions = asked(FREE, "{}");
    let ids: Vec<&str> = questions.iter().map(|q| q.id.as_str()).collect();
    // The engine's, the perspectives, the concerns in the catalog's order,
    // the template's, then the targets.
    assert_eq!(
        ids,
        [
            "dir",
            "name",
            "id",
            "storage",
            "perspective",
            "bodies",
            "online",
            "private-files",
            "examples",
            "targets"
        ]
    );
    let askers: Vec<Asker> = questions.iter().map(|q| q.asker).collect();
    assert_eq!(askers[..5], [Asker::Engine; 5]);
    assert_eq!(askers[5..7], [Asker::Catalog; 2]);
    assert_eq!(askers[7..9], [Asker::Template; 2]);
    assert_eq!(askers[9], Asker::Engine);
    // The text questions' defaults are the folder, its name, the id and
    // the id again, for the saves' folder.
    let defaults: Vec<&[String]> = questions[..4].iter().map(|q| &q.default[..]).collect();
    assert_eq!(
        defaults,
        [
            &["moon".to_owned()][..],
            &["Moon".to_owned()],
            &["com.example.moon".to_owned()],
            &["com.example.moon".to_owned()]
        ]
    );
    assert!(questions[..4].iter().all(|q| q.takes == Takes::Text));
    let flags: Vec<Option<&str>> = questions.iter().map(|q| q.flag.as_deref()).collect();
    assert_eq!(
        flags,
        [
            None,
            Some("--name"),
            Some("--id"),
            Some("--storage"),
            Some("--perspective"),
            Some("--bodies"),
            Some("--online"),
            Some("--set private-files="),
            Some("--set examples="),
            Some("--targets")
        ]
    );
    // The engine's catalog asks every concern but `time` too.
    let engine = questions_of_the_engine("{}");
    assert!(engine.iter().all(|q| q.id != "time"));
    let concerns = Concerns::embedded().unwrap();
    for concern in concerns.concerns.iter().filter(|c| c.id != "time") {
        assert!(engine.iter().any(|q| q.id == concern.id), "{}", concern.id);
    }
}

/// The questions of the engine's own catalog and template for `given`.
fn questions_of_the_engine(given: &str) -> Vec<Question> {
    questions(
        &Catalog::embedded().unwrap(),
        &built_in().unwrap(),
        &engine_targets().unwrap(),
        &named(),
        &answers(given),
    )
}

#[test]
fn bodies_and_motion_is_skipped_for_a_grid_only_game_and_asked_of_a_free_one() {
    let grid = asked(FREE, "{}");
    let bodies = question(&grid, "bodies");
    assert_eq!(
        bodies.skipped.as_deref(),
        Some(
            "its answers are for the perspectives top-down, side-view, and the game's are \
             top-down-grid"
        )
    );
    let none = asked(FREE, r#"{ "perspective": [] }"#);
    assert!(
        question(&none, "bodies")
            .skipped
            .as_deref()
            .unwrap()
            .ends_with("and none is picked")
    );
    let free = asked(FREE, r#"{ "perspective": ["top-down-grid", "top-down"] }"#);
    let bodies = question(&free, "bodies");
    assert_eq!(bodies.skipped, None);
    assert_eq!(bodies.takes, Takes::Many);
    assert_eq!(bodies.default, ["movers"]);
    assert_eq!(bodies.text, "How do bodies move?");
    // In the engine's catalog no free perspective is ready yet.
    let engine = questions_of_the_engine("{}");
    assert!(question(&engine, "bodies").skipped.is_some());
}

#[test]
fn planned_and_parked_answers_are_marked_with_their_tickets() {
    let free = asked(FREE, r#"{ "perspective": ["top-down"] }"#);
    let bodies = question(&free, "bodies");
    assert_eq!(
        bodies.answers,
        [
            Offered {
                id: "movers".into(),
                text: "movers".into(),
                status: Status::Ready,
                tickets: Vec::new(),
            },
            Offered {
                id: "rigid-body".into(),
                text: "rigid-body physics".into(),
                status: Status::Planned,
                tickets: vec!["1128".into()],
            }
        ]
    );
    let perspective = question(&free, "perspective");
    assert_eq!(perspective.answers[2].status, Status::Parked);
    assert_eq!(perspective.answers[2].tickets, ["1127"]);
    // A planned target is offered as one, and never by default.
    let targets = question(&free, "targets");
    assert_eq!(targets.answers[1].status, Status::Planned);
    assert_eq!(
        targets.answers[1].text,
        "switch: the engine doesn't run on it yet"
    );
    assert_eq!(targets.default, ["desktop", "web"]);
}

#[test]
fn a_concern_with_fewer_than_two_ready_or_planned_answers_isnt_asked() {
    let free = asked(FREE, r#"{ "perspective": ["top-down"] }"#);
    let online = question(&free, "online");
    assert_eq!(
        online.skipped.as_deref(),
        Some("only `none` of its answers is ready or planned: there is nothing to pick between")
    );
    assert_eq!(online.default, ["none"]);
    // The perspectives are always asked, even with one ready answer.
    let engine = questions_of_the_engine("{}");
    assert_eq!(question(&engine, "perspective").skipped, None);
    // The engine's scripting has a ready answer and a planned one.
    assert_eq!(question(&engine, "scripting").skipped, None);
}

#[test]
fn private_files_is_answered_by_the_github_questions() {
    let questions = asked(FREE, "{}");
    let private = question(&questions, "private-files");
    assert!(
        private
            .skipped
            .as_deref()
            .unwrap()
            .starts_with("the GitHub questions answer it"),
        "{private:?}"
    );
    assert_eq!(private.default, ["no"]);
    assert_eq!(question(&questions, "examples").skipped, None);
}

#[test]
fn the_answers_given_so_far_are_the_defaults() {
    let given = asked(
        FREE,
        r#"{ "online": ["lockstep"], "examples": ["no"], "targets": ["desktop"] }"#,
    );
    assert_eq!(question(&given, "online").default, ["lockstep"]);
    assert_eq!(question(&given, "examples").default, ["no"]);
    assert_eq!(question(&given, "targets").default, ["desktop"]);
    assert_eq!(question(&given, "perspective").default, ["top-down-grid"]);
}

#[test]
fn the_questions_read_for_people() {
    let listed = Questions {
        template: "shuyi-default".into(),
        questions: asked(FREE, "{}"),
    };
    let text = text(&listed);
    assert!(
        text.starts_with("new: 10 question(s), from shuyi-default\n"),
        "{text}"
    );
    assert!(text.contains("Which folder is the game made in? (<dir>, now moon)\n"));
    assert!(text.contains(
        "Which folder are the game's saves kept in, on the player's computer? The id is the \
         default, since it is nearly sure to be unique. (--storage, now com.example.moon)\n"
    ));
    assert!(text.contains(
        "Which perspectives? (--perspective, now top-down-grid)\n  any of: top-down-grid, \
         top-down, side-view (parked: 1127)\n"
    ));
    assert!(text.contains("How do bodies move? (--bodies, now movers)\n  not asked: "));
    assert!(
        text.contains("  any of: desktop, switch (planned)\n"),
        "{text}"
    );
}

#[test]
fn a_word_is_quoted_for_powershell_and_a_posix_shell_alike() {
    assert_eq!(quote("--perspective").as_deref(), Some("--perspective"));
    assert_eq!(quote("desktop,web").as_deref(), Some("desktop,web"));
    assert_eq!(quote("D:/games/moon").as_deref(), Some("D:/games/moon"));
    assert_eq!(quote("perspective=").as_deref(), Some("perspective="));
    assert_eq!(quote("Moon Garden").as_deref(), Some("'Moon Garden'"));
    assert_eq!(quote("").as_deref(), Some("''"));
    assert_eq!(quote("$HOME").as_deref(), Some("'$HOME'"));
    assert_eq!(quote("@splat").as_deref(), Some("'@splat'"));
    assert_eq!(quote("~/moon").as_deref(), Some("'~/moon'"));
    assert_eq!(quote("Nick's Moon").as_deref(), Some("\"Nick's Moon\""));
    assert_eq!(quote("Nick's $5 Moon"), None);
    assert_eq!(quote("Nick's `Moon`"), None);
}

/// The command `new` prints for `input`, named `name`, with `given` and the
/// engine's template's defaults.
fn command_for(input: &NewInput, name: &str, given: &str) -> (String, bool) {
    let template = built_in().unwrap();
    let concerns = Concerns::embedded().unwrap();
    let mut answers = template.with_defaults(&answers(given));
    for concern in concerns.concerns.iter().filter(|c| c.pick.is_some()) {
        if let Some(default) = concern.default_answer() {
            answers
                .0
                .entry(concern.id.clone())
                .or_insert_with(|| vec![default.id.clone()]);
        }
    }
    command(input, name, &answers, &concerns, &template)
}

#[test]
fn the_command_names_every_answer_by_its_flag() {
    let input = NewInput::new("moon");
    let (line, alike) = command_for(
        &input,
        "Moon Garden",
        r#"{ "targets": ["desktop", "web"] }"#,
    );
    assert!(alike);
    assert_eq!(
        line,
        "shuyi new moon --no-input --name 'Moon Garden' --perspective top-down-grid --online none \
         --world-drawing flat --kind tactics --scripting none --targets desktop,web --set \
         private-files=no --examples"
    );
    // No perspective is `--set perspective=`; `--id`, `--no-examples`, the
    // engine and the private repository follow.
    let mut input = NewInput::new("games/moon");
    input.id = Some("org.example.moon".into());
    input.engine = Engine::Rev {
        rev: "abc1234".into(),
    };
    input.private_url = Some("https://github.com/o/moon-assets.git".into());
    let (line, _) = command_for(
        &input,
        "Moon",
        r#"{ "perspective": [], "examples": ["no"], "private-files": ["yes"], "targets": ["web"] }"#,
    );
    assert_eq!(
        line,
        "shuyi new games/moon --no-input --name Moon --id org.example.moon --set perspective= \
         --online none --world-drawing flat --kind tactics --scripting none --targets web --set \
         private-files=yes --no-examples --engine-rev abc1234 --private-url \
         https://github.com/o/moon-assets.git"
    );
    let mut input = NewInput::new("moon");
    input.engine = Engine::Path {
        path: "../shuyi engine".into(),
    };
    let (line, _) = command_for(&input, "Moon", "{}");
    assert!(
        line.ends_with(" --examples --engine-path '../shuyi engine'"),
        "{line}"
    );
}

#[test]
fn the_command_names_the_saves_folder_after_the_id_unless_it_is_the_id() {
    let mut input = NewInput::new("moon");
    input.id = Some("org.example.moon".into());
    input.storage = Some("Moon Saves".into());
    let (line, alike) = command_for(&input, "Moon", "{}");
    assert!(alike);
    assert!(
        line.starts_with(
            "shuyi new moon --no-input --name Moon --id org.example.moon --storage 'Moon Saves' \
             --perspective "
        ),
        "{line}"
    );
    // Without --id, after the name; the same as the id (given or not), not
    // at all.
    input.id = None;
    let (line, _) = command_for(&input, "Moon", "{}");
    assert!(
        line.starts_with("shuyi new moon --no-input --name Moon --storage 'Moon Saves' "),
        "{line}"
    );
    input.storage = Some("com.example.moon".into());
    let (line, _) = command_for(&input, "Moon", "{}");
    assert!(!line.contains("--storage"), "{line}");
    input.id = Some("org.example.moon".into());
    input.storage = Some("org.example.moon".into());
    let (line, _) = command_for(&input, "Moon", "{}");
    assert!(!line.contains("--storage"), "{line}");
}

#[test]
fn the_saves_folder_follows_the_id_unless_given() {
    let questions = asked(FREE, "{}");
    let storage = question(&questions, "storage");
    assert_eq!(storage.default, ["com.example.moon"]);
    assert_eq!(storage.takes, Takes::Text);
    assert_eq!(storage.flag.as_deref(), Some("--storage"));
    let named = Named::new(
        "moon".into(),
        "Moon".into(),
        "org.example.moon".into(),
        Some("Moon Saves".into()),
    );
    assert_eq!(named.storage, "Moon Saves");
    let named = Named::new(
        "moon".into(),
        "Moon".into(),
        "org.example.moon".into(),
        None,
    );
    assert_eq!(named.storage, "org.example.moon");
}

#[test]
fn a_word_no_quote_keeps_in_both_is_quoted_for_a_posix_shell() {
    let (line, alike) = command_for(&NewInput::new("moon"), "Nick's $5 Moon", "{}");
    assert!(!alike);
    assert!(line.contains(" --name 'Nick'\\''s $5 Moon' "), "{line}");
}
