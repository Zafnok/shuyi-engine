//! `bots <n>`: the game's stage played `n` times by the engine's bots
//! (`shuyi_core::bots`), from seeds `seed`, `seed + 1`, …: how each play
//! ended, how long it took, the game's own measures of it, and its record
//! for `replay`. The game plays them ([`GameTools::bots`]); [`tally`]
//! sums them up.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Answer, Op, codes};
use crate::game::{GameArg, GameOption, GameTools, check_start, code_of};

pub use crate::game::game_arg;

/// The most games one `bots` plays.
pub const MAX_GAMES: u32 = 10_000;

/// What the outcome of a play that hit the turn limit is called.
pub const UNFINISHED: &str = "unfinished";

/// What `bots` plays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct BotsInput {
    /// How many games.
    pub games: u32,
    /// The first game's seed; each next game takes the next.
    pub seed: u64,
    /// The turn a play stops at if it hasn't ended.
    pub turns: u32,
    /// The game's own options, in the order given.
    pub start: Vec<GameArg>,
}

impl Default for BotsInput {
    fn default() -> Self {
        Self {
            games: 10,
            seed: 0,
            turns: 200,
            start: Vec::new(),
        }
    }
}

/// One play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer a game builds with `Play::new`: it may grow (ADR-0039).
#[non_exhaustive]
pub struct Play {
    /// Its seed.
    pub seed: u64,
    /// How it ended, in the game's words, or [`UNFINISHED`].
    pub outcome: String,
    /// The turn it ended on (past the limit if the limit stopped it).
    pub turns: u32,
    /// The commands applied.
    pub commands: u32,
    /// The game's own measures of it, by name.
    pub measures: BTreeMap<String, i64>,
    /// Its record, in the game's record format, for `replay`.
    pub record: Option<String>,
}

impl Play {
    /// The play seeded `seed` that ended `outcome` on turn `turns` after
    /// `commands` commands, with no measures and no record.
    pub fn new(seed: u64, outcome: impl Into<String>, turns: u32, commands: u32) -> Self {
        Self {
            seed,
            outcome: outcome.into(),
            turns,
            commands,
            measures: BTreeMap::new(),
            record: None,
        }
    }
}

/// How a number spread over the plays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spread {
    /// The smallest.
    pub min: i64,
    /// The largest.
    pub max: i64,
    /// The sum.
    pub total: i64,
    /// The mean (the sum over the plays that have it).
    pub mean: f64,
}

impl Spread {
    /// The spread of `values`, if there are any.
    pub fn of(values: &[i64]) -> Option<Self> {
        let (&min, &max) = (values.iter().min()?, values.iter().max()?);
        let total: i64 = values.iter().sum();
        #[allow(clippy::cast_precision_loss)] // a report, not a simulation
        let mean = total as f64 / values.len() as f64;
        Some(Self {
            min,
            max,
            total,
            mean,
        })
    }
}

/// What `bots` found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BotsOutput {
    /// How many games were played.
    pub games: u32,
    /// How many ended each way, by outcome.
    pub outcomes: BTreeMap<String, u32>,
    /// How many turns the plays took.
    pub turns: Option<Spread>,
    /// The game's measures over the plays, by name.
    pub measures: BTreeMap<String, Spread>,
    /// Every play, in seed order.
    pub plays: Vec<Play>,
}

/// `plays` summed up.
pub fn tally(plays: Vec<Play>) -> BotsOutput {
    let mut outcomes = BTreeMap::new();
    let mut measures: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for play in &plays {
        *outcomes.entry(play.outcome.clone()).or_insert(0) += 1;
        for (name, &value) in &play.measures {
            measures.entry(name.clone()).or_default().push(value);
        }
    }
    let turns: Vec<i64> = plays.iter().map(|p| i64::from(p.turns)).collect();
    BotsOutput {
        games: u32::try_from(plays.len()).unwrap_or(u32::MAX),
        outcomes,
        turns: Spread::of(&turns),
        measures: measures
            .into_iter()
            .filter_map(|(name, values)| Some((name, Spread::of(&values)?)))
            .collect(),
        plays,
    }
}

/// `bots`: `input`'s games played by `game`'s bots.
pub fn bots(game: &dyn GameTools, input: &BotsInput) -> Answer<BotsOutput> {
    if input.games == 0 || input.games > MAX_GAMES {
        return Answer::failed(
            Op::Bots,
            codes::WRONG_ARGUMENTS,
            format!("games: 1 to {MAX_GAMES}, not {}", input.games),
        );
    }
    if input.seed.checked_add(u64::from(input.games)).is_none() {
        let error = "seed: too big for that many games";
        return Answer::failed(Op::Bots, codes::WRONG_ARGUMENTS, error);
    }
    if let Err(e) = check_start(game.frame_options(), &input.start) {
        return Answer::failed(Op::Bots, codes::WRONG_ARGUMENTS, e);
    }
    match game.bots(input) {
        Ok(output) => {
            let mut limits = Vec::new();
            let unfinished = output.outcomes.get(UNFINISHED).copied().unwrap_or(0);
            if unfinished > 0 {
                limits.push(format!(
                    "{unfinished} play(s) reached turn {} without an end: their outcome is unknown",
                    input.turns
                ));
            }
            if output.plays.iter().any(|p| p.record.is_none()) {
                limits.push("the game's tools give no record of some plays".to_owned());
            }
            Answer::new(Op::Bots, true, output, limits)
        }
        Err(e) => Answer::failed(Op::Bots, code_of(&e), e),
    }
}

/// `bots`'s usage, with the game's start `options`.
pub fn usage(command: &str, options: &[GameOption]) -> String {
    let mut out = format!(
        "usage: {command} bots <games> [--seed N] [--turns N] [options] [--json]\n\n\
         The game's bots play <games> games (1 to {MAX_GAMES}), seeded N, N + 1, …\n\
         (0 first by default), each stopped at turn --turns (200 by default).\n"
    );
    out.push_str(&options_text(options));
    out
}

/// The game's start `options`, a line each, for a usage.
pub fn options_text(options: &[GameOption]) -> String {
    let mut out = String::new();
    for option in options {
        let name = match option.value {
            Some(value) => format!("{} {value}", option.flag),
            None => option.flag.to_owned(),
        };
        // Writing to a `String` can't fail.
        let _ = writeln!(
            out,
            "  {name:<20} {}",
            option.help.lines().next().unwrap_or_default()
        );
    }
    out
}

/// Reads `bots`'s arguments (not `--json`).
pub fn parse_args(args: &[String], options: &[GameOption]) -> Result<BotsInput, String> {
    let mut input = BotsInput::default();
    let mut games = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        let number = |text: &str| {
            text.parse::<u64>()
                .map_err(|_| format!("{arg}: not a whole number: {text}"))
        };
        match arg.as_str() {
            "--seed" => input.seed = number(&value()?)?,
            "--turns" => {
                input.turns =
                    u32::try_from(number(&value()?)?).map_err(|_| format!("{arg}: too big"))?;
            }
            flag if flag.starts_with("--") => input.start.push(game_arg(flag, options, value)?),
            text if games.is_none() => {
                games = Some(
                    text.parse::<u32>()
                        .map_err(|_| format!("<games>: not a whole number: {text}"))?,
                );
            }
            extra => return Err(format!("one <games> only, not also {extra}")),
        }
    }
    input.games = games.ok_or("missing <games>")?;
    Ok(input)
}

/// What `bots` found, for people.
pub fn text(output: &BotsOutput) -> String {
    let mut out = format!("bots: {} game(s)\n", output.games);
    let outcomes: Vec<String> = output
        .outcomes
        .iter()
        .map(|(name, n)| format!("{name} {n}"))
        .collect();
    let _ = writeln!(out, "outcomes: {}", outcomes.join(", "));
    let spread = |s: &Spread| format!("{}–{}, mean {:.2}", s.min, s.max, s.mean);
    if let Some(turns) = &output.turns {
        let _ = writeln!(out, "turns: {}", spread(turns));
    }
    for (name, s) in &output.measures {
        let _ = writeln!(out, "{name}: {}, total {}", spread(s), s.total);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::fake::Fake;

    fn play(seed: u64, outcome: &str, turns: u32, trips: i64) -> Play {
        Play {
            seed,
            outcome: outcome.to_owned(),
            turns,
            commands: turns * 2,
            measures: [("trips".to_owned(), trips)].into(),
            record: None,
        }
    }

    #[test]
    fn plays_are_counted_by_outcome_and_their_numbers_spread() {
        let out = tally(vec![
            play(0, "won", 3, 1),
            play(1, "caught", 5, 0),
            play(2, "won", 7, 2),
        ]);
        assert_eq!(out.games, 3);
        assert_eq!(
            out.outcomes,
            [("caught".to_owned(), 1), ("won".to_owned(), 2)].into()
        );
        let turns = out.turns.unwrap();
        assert_eq!((turns.min, turns.max, turns.total), (3, 7, 15));
        assert!((turns.mean - 5.0).abs() < f64::EPSILON);
        let trips = &out.measures["trips"];
        assert_eq!((trips.min, trips.max, trips.total), (0, 2, 3));
        assert!((trips.mean - 1.0).abs() < f64::EPSILON);
        assert!(tally(Vec::new()).turns.is_none());
        assert!(Spread::of(&[]).is_none());
    }

    #[test]
    fn the_arguments_name_the_games_and_the_seeds() {
        let args = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).into()).collect() };
        let options = [GameOption {
            flag: "--hard",
            value: None,
            help: "harder",
        }];
        let input = parse_args(
            &args(&["5", "--seed", "9", "--turns", "50", "--hard"]),
            &options,
        )
        .unwrap();
        assert_eq!((input.games, input.seed, input.turns), (5, 9, 50));
        assert_eq!(input.start[0].flag, "--hard");
        assert_eq!(parse_args(&args(&["3"]), &[]).unwrap().turns, 200);
        for bad in [
            &[][..],
            &["x"],
            &["3", "4"],
            &["3", "--seed"],
            &["3", "--seed", "-1"],
            &["3", "--turns", "9999999999"],
            &["3", "--hard"],
        ] {
            assert!(parse_args(&args(bad), &[]).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn bots_checks_the_games_and_the_seeds_then_asks_the_game() {
        let input = |games: u32, seed: u64| BotsInput {
            games,
            seed,
            ..BotsInput::default()
        };
        let game = Fake {
            bots: Ok(tally(vec![
                play(0, "won", 3, 0),
                play(1, UNFINISHED, 201, 0),
            ])),
            ..Fake::default()
        };
        let answer = bots(&game, &input(2, 0));
        assert!(answer.ok);
        assert_eq!(answer.op, Op::Bots);
        assert_eq!(
            answer.limits,
            [
                "1 play(s) reached turn 200 without an end: their outcome is unknown",
                "the game's tools give no record of some plays",
            ]
        );
        let recorded = Play {
            record: Some("(seed: 0)".into()),
            ..play(0, "won", 3, 0)
        };
        let game = Fake {
            bots: Ok(tally(vec![recorded])),
            ..Fake::default()
        };
        let got = bots(&game, &input(1, 0)).limits;
        assert!(got.is_empty(), "{got:?}");
        assert!(bots(&game, &input(MAX_GAMES, 0)).ok);
        for (games, seed) in [(0, 0), (MAX_GAMES + 1, 0)] {
            let answer = bots(&game, &input(games, seed));
            assert_eq!(answer.exit_code(), 2);
            let error = answer.error.unwrap();
            assert!(error.starts_with("games: 1 to 10000"), "{error}");
        }
        assert_eq!(
            bots(&game, &input(2, u64::MAX - 1)).error.as_deref(),
            Some("seed: too big for that many games")
        );
        assert!(bots(&game, &input(1, u64::MAX - 1)).ok);
        let broken = Fake {
            bots: Err("no stage".into()),
            ..Fake::default()
        };
        let answer = bots(&broken, &input(1, 0));
        assert_eq!(answer.error.as_deref(), Some("no stage"));
        assert_eq!(answer.code.as_deref(), Some("SH3002"));
        // A game whose tools leave bots out.
        let left_out = Fake {
            bots: Err(crate::game::unsupported("bots", "bots")),
            ..Fake::default()
        };
        let answer = bots(&left_out, &input(1, 0));
        assert_eq!(answer.code.as_deref(), Some("SH3001"));
    }

    #[test]
    fn what_the_bots_found_reads_for_people() {
        let out = tally(vec![play(0, "won", 3, 1), play(1, "caught", 5, 0)]);
        assert_eq!(
            text(&out),
            "bots: 2 game(s)\noutcomes: caught 1, won 1\nturns: 3–5, mean 4.00\n\
             trips: 0–1, mean 0.50, total 1\n"
        );
        let usage = usage(
            "tools",
            &[GameOption {
                flag: "--seed",
                value: Some("N"),
                help: "the seed\nof it",
            }],
        );
        assert!(usage.starts_with("usage: tools bots <games> [--seed N] [--turns N]"));
        assert!(
            usage.ends_with("\n  --seed N             the seed\n"),
            "{usage}"
        );
    }

    #[test]
    fn a_new_play_has_no_measures_and_no_record() {
        let literal = Play {
            seed: 4,
            outcome: "won".to_owned(),
            turns: 9,
            commands: 18,
            measures: BTreeMap::new(),
            record: None,
        };
        assert_eq!(Play::new(4, "won", 9, 18), literal);
    }
}
