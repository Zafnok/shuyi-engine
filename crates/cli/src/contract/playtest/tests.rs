//! `playtest`'s arguments, its choice of stages and entrants, the tries on
//! threads, and its answer, on the fake game.

use shuyi_core::bots::playtest::{Check, Found, Tried};

use super::*;
use crate::contract::fake::{Fake, FakePlaytest, fake_checks};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).into()).collect()
}

/// The fake with `playtest`.
fn fake(playtest: FakePlaytest) -> Fake {
    Fake {
        playtest: Ok(playtest),
        ..Fake::default()
    }
}

/// The fake with stages `stages`.
fn staged(stages: &[&str]) -> Fake {
    fake(FakePlaytest {
        stages: stages.iter().map(|s| (*s).to_owned()).collect(),
        ..FakePlaytest::default()
    })
}

/// An input of `tries` tries and the rest as `more` sets it.
fn input(tries: u32, more: impl FnOnce(&mut PlaytestInput)) -> PlaytestInput {
    let mut input = PlaytestInput {
        tries: Some(tries),
        ..PlaytestInput::default()
    };
    more(&mut input);
    input
}

#[test]
fn the_arguments_name_the_stage_the_entrants_the_tries_and_the_threads() {
    let options = [GameOption {
        flag: "--hard",
        value: None,
        help: "harder",
    }];
    let parsed = parse_args(
        &args(&[
            "--stage",
            "cave",
            "--entrant",
            "walk",
            "--entrant",
            "all",
            "--tries",
            "5",
            "--seed",
            "9",
            "--turns",
            "50",
            "--threads",
            "3",
            "--report",
            "out.json",
            "--history",
            "kept",
            "--brief",
            "--hard",
        ]),
        &options,
    )
    .unwrap();
    let want = PlaytestInput {
        stage: Some("cave".to_owned()),
        every_stage: false,
        entrants: args(&["walk", "all"]),
        tries: Some(5),
        seed: Some(9),
        turns: Some(50),
        threads: Some(3),
        start: vec![GameArg {
            flag: "--hard".to_owned(),
            value: None,
        }],
        history: Some("kept".to_owned()),
        no_history: false,
    };
    assert_eq!(parsed.input, want);
    assert_eq!(parsed.report, Some(PathBuf::from("out.json")));
    assert!(parsed.brief);
    assert!(
        parse_args(&args(&["--no-history"]), &[])
            .unwrap()
            .input
            .no_history
    );
    assert!(
        parse_args(&args(&["--every-stage"]), &[])
            .unwrap()
            .input
            .every_stage
    );
    // Nothing given: every number is the game's.
    assert_eq!(parse_args(&[], &[]).unwrap(), PlaytestArgs::default());
    assert_eq!(
        parse_args(&args(&["3"]), &options).unwrap_err(),
        "no <games> here, only flags: 3"
    );
    assert_eq!(
        parse_args(&args(&["--threads", "x"]), &[]).unwrap_err(),
        "--threads: not a whole number: x"
    );
    for bad in [
        &["3"][..],
        &["--tries"],
        &["--stage"],
        &["--entrant"],
        &["--report"],
        &["--history"],
        &["--seed", "-1"],
        &["--tries", "9999999999"],
        &["--threads", "9999999999"],
        &["--hard"],
    ] {
        assert!(parse_args(&args(bad), &[]).is_err(), "{bad:?}");
    }
}

#[test]
fn playtest_checks_the_tries_the_seeds_and_the_threads_then_asks_the_game() {
    let game = Fake::default();
    let answer = playtest(&game, &input(1, |_| {}));
    assert!(answer.ok, "{:?}", answer.error);
    assert_eq!(answer.op, Op::Playtest);
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
    assert!(playtest(&game, &input(MAX_GAMES, |_| {})).ok);
    for tries in [0, MAX_GAMES + 1] {
        let answer = playtest(&game, &input(tries, |_| {}));
        assert_eq!(answer.exit_code(), 2);
        let error = answer.error.unwrap();
        assert!(error.starts_with("tries: 1 to 10000"), "{error}");
    }
    assert_eq!(
        playtest(&game, &input(2, |i| i.seed = Some(u64::MAX - 1)))
            .error
            .as_deref(),
        Some("seed: too big for that many tries")
    );
    let none = playtest(&game, &input(2, |i| i.threads = Some(0)));
    assert_eq!(none.error.as_deref(), Some("threads: at least 1"));
    assert_eq!(none.exit_code(), 2);
    // The game's plan doesn't load, or the game doesn't offer one.
    let broken = Fake {
        playtest: Err("no stage".into()),
        ..Fake::default()
    };
    let answer = playtest(&broken, &input(1, |_| {}));
    assert_eq!(answer.error.as_deref(), Some("no stage"));
    assert_eq!(answer.code.as_deref(), Some("SH3002"));
    let left_out = Fake {
        playtest: Err(crate::game::unsupported("playtest", "playtest_plan")),
        ..Fake::default()
    };
    let answer = playtest(&left_out, &input(1, |_| {}));
    assert_eq!(answer.code.as_deref(), Some("SH3001"));
}

#[test]
fn a_number_not_given_is_the_games_default() {
    let game = fake(FakePlaytest {
        defaults: Tries::new(3, 1, 60),
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &PlaytestInput::default());
    let report = &answer.result.unwrap()[0].report;
    for entrant in &report.entrants {
        let seeds: Vec<u64> = entrant.tries.iter().map(|t| t.seed).collect();
        assert_eq!(seeds, [1, 2, 3], "{}", entrant.name);
    }
    // The fake's walker stops at the turn limit.
    assert_eq!(report.entrants[0].tries[0].turns, 60);
    // The game's plan is asked for with the numbers filled in, the rest as
    // given.
    let planned = game.planned.lock().unwrap().clone();
    let want = PlaytestInput {
        tries: Some(3),
        seed: Some(1),
        turns: Some(60),
        ..PlaytestInput::default()
    };
    assert_eq!(planned, [want]);
    // A number given wins over the game's.
    let answer = playtest(&game, &input(2, |i| i.turns = Some(9)));
    let report = &answer.result.unwrap()[0].report;
    assert_eq!(report.entrants[0].tries.len(), 2);
    assert_eq!(report.entrants[0].tries[1].seed, 2);
    assert_eq!(report.entrants[0].tries[0].turns, 9);
    // And the engine's, when the game has none of its own.
    assert_eq!(
        PlaytestInput::default().tries(&Fake::default().playtest_defaults()),
        DEFAULT_TRIES
    );
}

#[test]
fn the_entrants_asked_for_play_with_only_their_checks() {
    let game = Fake::default();
    let played = |entrants: &[&str]| {
        let answer = playtest(&game, &input(2, |i| i.entrants = args(entrants)));
        answer.result.unwrap().remove(0)
    };
    let names = |output: &PlaytestOutput| -> Vec<String> {
        output
            .report
            .entrants
            .iter()
            .map(|e| e.name.clone())
            .collect()
    };
    let walk = played(&["walk"]);
    assert_eq!(names(&walk), ["walk"]);
    let checks: Vec<&Check> = walk.report.checks.iter().map(|j| &j.check).collect();
    assert_eq!(checks, fake_checks(&["walk"]).iter().collect::<Vec<_>>());
    assert_eq!(walk.entrants.len(), 1);
    // A group, a name twice, and names out of order: the plan's order.
    for asked in [&["all"][..], &["run", "walk"], &["run", "all", "run"]] {
        let output = played(asked);
        assert_eq!(names(&output), ["walk", "run"], "{asked:?}");
        assert_eq!(output.report.checks.len(), 3, "{asked:?}");
    }
    assert_eq!(names(&played(&[])), ["walk", "run"]);
    // An entrant who isn't there.
    let answer = playtest(&game, &input(2, |i| i.entrants = args(&["nobody"])));
    assert_eq!(answer.exit_code(), 2);
    assert_eq!(
        answer.error.as_deref(),
        Some("entrant: no nobody; the entrants: walk, run; the groups: all")
    );
    // The game's default entrants, when none is asked for.
    let game = fake(FakePlaytest {
        default_entrants: args(&["run"]),
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &input(2, |_| {}));
    assert_eq!(names(&answer.result.unwrap()[0]), ["run"]);
    let wrong = fake(FakePlaytest {
        default_entrants: args(&["ghost"]),
        ..FakePlaytest::default()
    });
    let answer = playtest(&wrong, &input(2, |_| {}));
    assert_eq!(answer.code.as_deref(), Some("SH3002"));
    assert!(answer.error.unwrap().starts_with("entrant: no ghost;"));
}

#[test]
fn a_game_of_several_stages_plays_the_one_named_or_every_one() {
    let game = staged(&["cave", "hard hill"]);
    let answer = playtest(&game, &input(2, |_| {}));
    assert_eq!(answer.exit_code(), 2);
    assert_eq!(
        answer.error.as_deref(),
        Some("stage: the game has 2 stages, name one or play every stage: cave, hard hill")
    );
    let cave = playtest(&game, &input(2, |i| i.stage = Some("cave".into())));
    assert!(cave.ok);
    let outputs = cave.result.unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].stage.as_deref(), Some("cave"));
    assert_eq!(outputs[0].title, "the fake on cave");
    let hill = playtest(&game, &input(2, |i| i.stage = Some("hard hill".into())));
    assert!(!hill.ok);
    assert_eq!(hill.exit_code(), 1);
    assert!(hill.result.unwrap()[0].report.blocked);
    // Every stage: a report each, failing if either is blocked.
    let every = playtest(&game, &input(2, |i| i.every_stage = true));
    assert_eq!(every.exit_code(), 1);
    let outputs = every.result.unwrap();
    let stages: Vec<Option<&str>> = outputs.iter().map(|o| o.stage.as_deref()).collect();
    assert_eq!(stages, [Some("cave"), Some("hard hill")]);
    let blocked: Vec<bool> = outputs.iter().map(|o| o.report.blocked).collect();
    assert_eq!(blocked, [false, true]);
    assert!(
        playtest(
            &staged(&["cave", "lake"]),
            &input(2, |i| i.every_stage = true)
        )
        .ok
    );
    // Wrong stages.
    let error = |game: &Fake, more: fn(&mut PlaytestInput)| {
        let answer = playtest(game, &input(2, more));
        assert_eq!(answer.exit_code(), 2);
        answer.error.unwrap()
    };
    assert_eq!(
        error(&game, |i| i.stage = Some("moon".into())),
        "stage: no moon; the stages: cave, hard hill"
    );
    assert_eq!(
        error(&game, |i| {
            i.stage = Some("cave".into());
            i.every_stage = true;
        }),
        "stage or every stage, not both"
    );
    assert_eq!(
        error(&Fake::default(), |i| i.stage = Some("cave".into())),
        "stage: the game has no stages, not cave"
    );
    // One stage is played without naming it; no stages, the unnamed one.
    let one = playtest(&staged(&["cave"]), &input(2, |_| {}));
    assert_eq!(one.result.unwrap()[0].stage.as_deref(), Some("cave"));
    let unnamed = playtest(&Fake::default(), &input(2, |i| i.every_stage = true));
    let outputs = unnamed.result.unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].stage, None);
}

#[test]
fn the_report_is_the_same_on_any_number_of_threads() {
    let game = Fake::default();
    let report = |threads: u32| {
        let answer = playtest(&game, &input(25, |i| i.threads = Some(threads)));
        answer.result.unwrap().remove(0)
    };
    let one = report(1);
    for threads in [2, 4, 7, 64] {
        assert_eq!(report(threads).report, one.report, "{threads} threads");
    }
    assert!(!one.report.blocked);
    assert!(matches!(
        one.report.checks[2].found,
        Found::Median {
            median: Some(199),
            than: Some(200),
            ..
        }
    ));
    // Every entrant is timed, with its label.
    let played: Vec<(&str, Option<&str>)> = one
        .entrants
        .iter()
        .map(|p| (p.name.as_str(), p.label.as_deref()))
        .collect();
    assert_eq!(played, [("walk", None), ("run", Some("fast"))]);
    assert!(one.entrants.iter().all(|p| p.seconds_per_try >= 0.0));
}

#[test]
fn an_entrants_time_is_the_mean_of_its_tries() {
    // Four tries of 10 ms each, on one thread: a try takes 10 ms or more
    // (a sleep overshoots by as much as the machine is busy), and four of
    // them one after another fit in the playtest's own time. Their sum,
    // 40 ms or more, wouldn't four times over.
    let game = fake(FakePlaytest {
        sleep: Duration::from_millis(10),
        ..FakePlaytest::default()
    });
    let started = std::time::Instant::now();
    let answer = playtest(
        &game,
        &input(4, |i| {
            i.entrants = args(&["walk"]);
            i.threads = Some(1);
        }),
    );
    let wall = started.elapsed().as_secs_f64();
    let seconds = answer.result.unwrap()[0].entrants[0].seconds_per_try;
    assert!(seconds >= 0.01, "{seconds}");
    assert!(seconds * 4.0 <= wall, "{seconds} a try, {wall} in all");
}

#[test]
fn a_try_that_fails_stops_the_playtest_naming_it() {
    let game = fake(FakePlaytest {
        fail: Some(3),
        ..FakePlaytest::default()
    });
    for threads in [1, 4] {
        let answer = playtest(&game, &input(10, |i| i.threads = Some(threads)));
        assert_eq!(answer.error.as_deref(), Some("walk: try 3: refused"));
        assert_eq!(answer.code.as_deref(), Some("SH3002"));
    }
}

#[test]
fn unfinished_tries_are_a_limit() {
    let game = fake(FakePlaytest {
        outcome: None,
        stages: vec!["cave".to_owned()],
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &input(1, |_| {}));
    assert_eq!(
        answer.limits,
        ["cave: 2 of the tries reached turn 200 without an end: their outcome is unknown"]
    );
    let game = fake(FakePlaytest {
        outcome: None,
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &input(1, |_| {}));
    assert_eq!(
        answer.limits,
        ["2 of the tries reached turn 200 without an end: their outcome is unknown"]
    );
}

#[test]
fn what_the_playtest_found_reads_for_people() {
    let answer = playtest(
        &staged(&["cave", "lake"]),
        &input(1, |i| i.every_stage = true),
    );
    let mut outputs = answer.result.unwrap();
    outputs[0].entrants[0].seconds_per_try = 2.5;
    outputs[0].entrants[1].seconds_per_try = 0.001;
    assert_eq!(seconds(0.01), "0.01");
    assert_eq!(seconds(0.009), "< 0.01");
    let text = text(&outputs);
    let cave = format!(
        "the fake on cave\n\n{}\nwalk · won 1/1 · turns median 200 · 2.50 s per try\n\
         run  · fast · < 0.01 s per try\n",
        outputs[0].report
    );
    assert!(text.starts_with(&cave), "{text}");
    assert!(
        text[cave.len()..].starts_with("\nthe fake on lake\n\n"),
        "{text}"
    );
    let usage = usage(
        "tools",
        &[GameOption {
            flag: "--hard",
            value: None,
            help: "harder\nstill",
        }],
    );
    assert!(usage.starts_with("usage: tools playtest [--stage ID | --every-stage]"));
    assert!(usage.contains("\n  --threads N          the threads to play on"));
    assert!(
        usage.ends_with("\n  --hard               harder\n"),
        "{usage}"
    );
}

/// A try of seed `seed` with `measures`.
fn tried(seed: u64, measures: &[(&str, i64)]) -> Tried {
    Tried {
        seed,
        outcome: Some("Won".to_owned()),
        turns: 1,
        measures: measures.iter().map(|&(n, v)| (n.to_owned(), v)).collect(),
    }
}

/// Four tries: `won` in two (and missing in one), `deaths` 0, 1, 5 and
/// 0, `items` 1, 3, 0 and 2, archer and scout each fallen in two, the
/// knight in none, and `fell archer turn` in three.
fn four_tries() -> Vec<Tried> {
    vec![
        tried(
            0,
            &[
                ("won", 1),
                ("deaths", 0),
                ("items", 1),
                ("fell archer turn", 1),
            ],
        ),
        tried(
            1,
            &[
                ("won", 1),
                ("deaths", 1),
                ("items", 3),
                ("fell archer", 1),
                ("fell archer turn", 4),
            ],
        ),
        tried(
            2,
            &[
                ("won", 0),
                ("deaths", 5),
                ("items", 0),
                ("fell scout", 1),
                ("fell archer", 1),
                ("fell archer turn", 6),
            ],
        ),
        tried(
            3,
            &[
                ("deaths", 0),
                ("items", 2),
                ("fell scout", 1),
                ("fell knight", 0),
            ],
        ),
    ]
}

#[test]
fn each_summary_item_reads_as_its_doc_comment_says() {
    let tries = four_tries();
    let text = |item: Summary| item.text(&tries);
    assert_eq!(text(Summary::sum("won")), "won 2/4");
    assert_eq!(
        text(Summary::sum("won").with_label(Some("wins".to_owned()))),
        "wins 2/4"
    );
    assert_eq!(
        text(Summary::spread("deaths", 3)),
        "deaths per try 0: 2  1: 1  2: 0  3+: 1"
    );
    assert_eq!(
        text(Summary::spread("deaths", 1).with_label(Some("died".to_owned()))),
        "died per try 0: 2  1+: 2"
    );
    assert_eq!(text(Summary::median("items")), "items median 1");
    assert_eq!(text(Summary::max("items")), "items max 3");
    // A missing measure is 0, and no tries give 0.
    assert_eq!(text(Summary::median("lost")), "lost median 0");
    assert_eq!(Summary::max("items").text(&[]), "items max 0");
    assert_eq!(Summary::median("items").text(&[]), "items median 0");
    assert_eq!(Summary::sum("won").text(&[]), "won 0/0");
    assert_eq!(text(Summary::most("lost ")), "most: nobody");
    // A suffix only a `Most` takes.
    assert_eq!(
        Summary::sum("won").with_skip_suffix(Some("x".to_owned())),
        Summary::sum("won")
    );
}

#[test]
fn most_leaves_out_the_suffix_and_breaks_ties_by_name() {
    let tries = four_tries();
    // `fell archer turn`, in three tries, would be the most.
    assert_eq!(Summary::most("fell ").text(&tries), "most: archer turn 3×");
    // Without it, archer and scout fell in two tries each, the knight's 0
    // isn't a fall, and archer comes first by name.
    let most = Summary::most("fell ")
        .with_skip_suffix(Some(" turn".to_owned()))
        .with_label(Some("fell most".to_owned()));
    assert_eq!(most.text(&tries), "fell most: archer 2×");
    let scout_more = [tries.clone(), vec![tried(4, &[("fell scout", 2)])]].concat();
    assert_eq!(most.text(&scout_more), "fell most: scout 3×");
}

#[test]
fn summary_items_read_from_data() {
    let items: Vec<Summary> = ron::from_str(
        r#"[
            Sum(measure: "won"),
            Spread(measure: "deaths", up_to: 3, label: Some("died")),
            Most(prefix: "fell ", skip_suffix: Some(" turn")),
            Median(measure: "items"),
            Max(measure: "items"),
        ]"#,
    )
    .unwrap();
    assert_eq!(
        items,
        [
            Summary::sum("won"),
            Summary::spread("deaths", 3).with_label(Some("died".to_owned())),
            Summary::most("fell ").with_skip_suffix(Some(" turn".to_owned())),
            Summary::median("items"),
            Summary::max("items"),
        ]
    );
    assert!(ron::from_str::<Summary>("Sum(measure: \"won\", bogus: 1)").is_err());
}

/// An entrant named `name` that played with `summary`, at 0.5 s a try.
fn played(name: &str, summary: &[&str]) -> Played {
    Played {
        name: name.to_owned(),
        label: None,
        seconds_per_try: 0.5,
        summary: summary.iter().map(|s| (*s).to_owned()).collect(),
        was: None,
    }
}

#[test]
fn a_long_line_wraps_under_its_first_item() {
    let mut long = played(
        "hardcore",
        &[
            "won 83/100",
            "deaths per try 0: 30  1: 21  2: 25  3+: 24",
            "most: scout 41×",
            "items median 1",
            "turns median 14",
        ],
    );
    long.label = Some("Classic".to_owned());
    long.was = Some("was 47/100 succeed".to_owned());
    assert_eq!(
        entrant_line(&long, 9),
        "hardcore  · Classic · won 83/100 · deaths per try 0: 30  1: 21  2: 25  3+: 24 · most: scout 41×\n\
         \x20                     items median 1 · turns median 14 · 0.50 s per try · was 47/100 succeed\n"
    );
    assert!(
        entrant_line(&long, 9)
            .lines()
            .all(|l| l.chars().count() <= 100)
    );
    assert_eq!(
        entrant_line(&played("walk", &[]), 4),
        "walk · 0.50 s per try\n"
    );
}

#[test]
fn the_history_file_is_named_by_stage_entrants_and_options() {
    let names = |list: &[&str]| -> Vec<String> { list.iter().map(|s| (*s).to_owned()).collect() };
    assert_eq!(
        history::file_name(None, &names(&["random", "walk"]), &[]),
        "stage-random+walk.jsonl"
    );
    let options: Vec<String> = [
        GameArg {
            flag: "--mode".to_owned(),
            value: Some("ca sual/2".to_owned()),
        },
        GameArg {
            flag: "--hard".to_owned(),
            value: None,
        },
    ]
    .iter()
    .map(history::option)
    .collect();
    assert_eq!(options, ["mode=ca sual/2", "hard"]);
    assert_eq!(
        history::file_name(Some("cave 2"), &names(&["a/b", "c.d"]), &options),
        "cave_2-a_b+c.d-mode=ca_sual_2-hard.jsonl"
    );
}

#[test]
fn the_utc_stamp_gives_the_date_and_time() {
    let day = 86_400;
    let stamp = history::utc_stamp;
    assert_eq!(stamp(0), "1970-01-01 00:00 UTC");
    assert_eq!(stamp(59), "1970-01-01 00:00 UTC");
    assert_eq!(stamp(60), "1970-01-01 00:01 UTC");
    assert_eq!(stamp(day - 1), "1970-01-01 23:59 UTC");
    assert_eq!(stamp(day), "1970-01-02 00:00 UTC");
    assert_eq!(stamp(30 * day), "1970-01-31 00:00 UTC");
    assert_eq!(stamp(31 * day), "1970-02-01 00:00 UTC");
    assert_eq!(stamp(364 * day), "1970-12-31 00:00 UTC");
    assert_eq!(stamp(365 * day), "1971-01-01 00:00 UTC");
    // 1972 is a leap year.
    assert_eq!(stamp(789 * day), "1972-02-29 00:00 UTC");
    assert_eq!(stamp(790 * day), "1972-03-01 00:00 UTC");
    assert_eq!(stamp(1095 * day), "1972-12-31 00:00 UTC");
    assert_eq!(stamp(1096 * day), "1973-01-01 00:00 UTC");
    // 2000 is one (by 400), 2100 isn't (by 100).
    assert_eq!(stamp(951_782_400), "2000-02-29 00:00 UTC");
    assert_eq!(stamp(4_107_456_000), "2100-02-28 00:00 UTC");
    assert_eq!(stamp(4_107_542_400), "2100-03-01 00:00 UTC");
    assert_eq!(stamp(1_790_863_500), "2026-10-01 14:05 UTC");
    assert_eq!(stamp(1_798_761_540), "2026-12-31 23:59 UTC");
    assert_eq!(stamp(1_798_761_600), "2027-01-01 00:00 UTC");
    assert!(history::now() > 1_700_000_000);
}

#[test]
fn the_commit_is_the_short_hash_or_unknown() {
    let here = history::git_commit(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(
        here.len() >= 7 && here.chars().all(|c| c.is_ascii_hexdigit()),
        "{here}"
    );
    assert_eq!(
        history::git_commit(Path::new("no/such/folder/anywhere")),
        "unknown"
    );
}

/// An empty folder for `name`'s files.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-playtest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_history_keeps_every_run_and_the_next_says_what_it_was() {
    let root = scratch("kept");
    let game = fake(FakePlaytest::default());
    let both = input(2, |i| i.history = Some("kept".to_owned()));
    let first = kept(&game, &root, &both);
    assert!(first.ok, "{:?}", first.error);
    let first = first.result.unwrap();
    assert_eq!(first[0].previous, None);
    assert!(first[0].entrants.iter().all(|p| p.was.is_none()));
    let file = root.join("kept").join("stage-walk+run.jsonl");
    assert!(file.is_file());

    let second = kept(&game, &root, &both).result.unwrap();
    let previous = second[0].previous.clone().unwrap();
    let last = history::last(&file).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 2);
    assert_eq!(last.output.previous.as_ref(), Some(&previous));
    assert_eq!(last.entrants, ["walk", "run"]);
    assert_eq!(last.title, "the fake on its stage");
    assert!(previous.date.ends_with(" UTC"), "{}", previous.date);
    // Each has a rate check: its successes then.
    let was: Vec<Option<&str>> = second[0]
        .entrants
        .iter()
        .map(|p| p.was.as_deref())
        .collect();
    assert_eq!(was, [Some("was 2/2 succeed"), Some("was 2/2 succeed")]);
    let mut outputs = second.clone();
    for p in &mut outputs[0].entrants {
        p.seconds_per_try = 0.5;
    }
    let text = text(&outputs);
    assert!(
        text.contains(
            "\nwalk · won 2/2 · turns median 200 · 0.50 s per try · was 2/2 succeed\n\
             run  · fast · 0.50 s per try · was 2/2 succeed\n\
             previous "
        ),
        "{text}"
    );
    assert!(
        text.ends_with(&format!(
            "previous {} · commit {}\n",
            previous.date, previous.commit
        )),
        "{text}"
    );
    // Brief: no line per try, and the rest.
    let brief = brief(&outputs);
    assert!(text.contains("\ntry 0 · walk"), "{text}");
    assert!(!brief.contains("try 0"), "{brief}");
    assert!(brief.contains("walk · won 2/2"), "{brief}");
    assert!(brief.contains("\nprevious "), "{brief}");

    // Other entrants: another file, and nothing to compare with.
    let walk = input(2, |i| {
        i.history = Some("kept".to_owned());
        i.entrants = vec!["walk".to_owned()];
    });
    let alone = kept(&game, &root, &walk).result.unwrap();
    assert_eq!(alone[0].previous, None);
    assert_eq!(alone[0].entrants[0].was, None);
    assert!(root.join("kept").join("stage-walk.jsonl").is_file());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_entrant_with_no_rate_check_was_its_first_summary_item() {
    let game = fake(FakePlaytest::default());
    let mut now = playtest(&game, &input(3, |_| {})).result.unwrap().remove(0);
    let mut before = now.clone();
    before.entrants[0].summary = vec!["won 2/3".to_owned()];
    // With rate checks now, the rate then.
    assert_eq!(
        was(&now, &before, "walk").as_deref(),
        Some("was 3/3 succeed")
    );
    // With none, the first summary item then; none for an entrant with
    // none, or who didn't play then.
    now.report.checks.clear();
    assert_eq!(was(&now, &before, "walk").as_deref(), Some("was won 2/3"));
    assert_eq!(was(&now, &before, "run"), None);
    before.entrants.remove(0);
    assert_eq!(was(&now, &before, "walk"), None);
    // A rate check now and none then: nothing to say.
    let rated = playtest(&game, &input(3, |_| {})).result.unwrap().remove(0);
    let mut unrated = rated.clone();
    unrated.report.checks.clear();
    assert_eq!(was(&rated, &unrated, "walk"), None);
}

#[test]
fn no_history_writes_nothing_and_a_history_that_cant_be_written_fails() {
    let root = scratch("unkept");
    let game = fake(FakePlaytest::default());
    let none = kept(&game, &root, &input(1, |i| i.no_history = true));
    assert!(none.ok);
    assert!(std::fs::read_dir(&root).unwrap().next().is_none());
    // The history's folder is a file.
    std::fs::write(root.join("blocked"), "").unwrap();
    let answer = kept(
        &game,
        &root,
        &input(1, |i| i.history = Some("blocked".to_owned())),
    );
    assert!(!answer.ok);
    assert_eq!(answer.code.as_deref(), Some(codes::FILE.0));
    let error = answer.error.unwrap();
    let file = root.join("blocked").join("stage-walk+run.jsonl");
    assert!(
        error.starts_with(&format!("history {}: ", file.display())),
        "{error}"
    );
    assert!(answer.result.is_some());
    // A failed playtest keeps nothing.
    let failed = kept(&game, &root, &input(0, |_| {}));
    assert!(failed.result.is_none());
    std::fs::remove_dir_all(&root).unwrap();
}
