//! `playtest`: the game's named bots each play its stage K times from
//! seeds `seed`, `seed + 1`, …, the game measures every try, and its
//! checks say per bot whether it succeeded as often as wanted
//! (`shuyi_core::bots::playtest`). The game plays and judges them
//! ([`GameTools::playtest`]); the answer passes unless a rate fell under
//! its floor.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_core::bots::playtest::PlaytestReport;

use super::bots::{MAX_GAMES, game_arg, options_text};
use super::{Answer, Op, codes};
use crate::game::{GameArg, GameOption, GameTools, code_of};

/// What `playtest` plays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, default)]
pub struct PlaytestInput {
    /// How many tries of each bot.
    pub tries: u32,
    /// The first try's seed; each next try takes the next.
    pub seed: u64,
    /// The turn a try stops at if it hasn't ended.
    pub turns: u32,
    /// The game's own options, in the order given.
    pub start: Vec<GameArg>,
}

impl Default for PlaytestInput {
    fn default() -> Self {
        Self {
            tries: 100,
            seed: 0,
            turns: 200,
            start: Vec::new(),
        }
    }
}

/// `playtest`: `input`'s tries played and judged by `game`.
pub fn playtest(game: &dyn GameTools, input: &PlaytestInput) -> Answer<PlaytestReport> {
    if input.tries == 0 || input.tries > MAX_GAMES {
        return Answer::failed(
            Op::Playtest,
            codes::WRONG_ARGUMENTS,
            format!("tries: 1 to {MAX_GAMES}, not {}", input.tries),
        );
    }
    if input.seed.checked_add(u64::from(input.tries)).is_none() {
        let error = "seed: too big for that many tries";
        return Answer::failed(Op::Playtest, codes::WRONG_ARGUMENTS, error);
    }
    match game.playtest(input) {
        Ok(report) => {
            let unfinished = report
                .entrants
                .iter()
                .flat_map(|e| &e.tries)
                .filter(|t| t.outcome.is_none())
                .count();
            let limits = if unfinished > 0 {
                vec![format!(
                    "{unfinished} of the tries reached turn {} without an end: their outcome is unknown",
                    input.turns
                )]
            } else {
                Vec::new()
            };
            Answer::new(Op::Playtest, !report.blocked, report, limits)
        }
        Err(e) => Answer::failed(Op::Playtest, code_of(&e), e),
    }
}

/// `playtest`'s usage, with the game's start `options`.
pub fn usage(command: &str, options: &[GameOption]) -> String {
    let mut out = format!(
        "usage: {command} playtest [--tries N] [--seed N] [--turns N] [options] [--json]\n\n\
         Each of the game's bots plays --tries tries (1 to {MAX_GAMES}, 100 by default),\n\
         seeded N, N + 1, … (0 first by default), each stopped at turn --turns\n\
         (200 by default), and the game's checks judge them. It fails only when a\n\
         bot succeeds less often than a check's floor.\n"
    );
    out.push_str(&options_text(options));
    out
}

/// Reads `playtest`'s arguments (not `--json`).
pub fn parse_args(args: &[String], options: &[GameOption]) -> Result<PlaytestInput, String> {
    let mut input = PlaytestInput::default();
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
        let small = |n: u64| u32::try_from(n).map_err(|_| format!("{arg}: too big"));
        match arg.as_str() {
            "--seed" => input.seed = number(&value()?)?,
            "--tries" => input.tries = small(number(&value()?)?)?,
            "--turns" => input.turns = small(number(&value()?)?)?,
            flag if flag.starts_with("--") => input.start.push(game_arg(flag, options, value)?),
            extra => return Err(format!("no <games> here, only flags: {extra}")),
        }
    }
    Ok(input)
}

/// What `playtest` found, for people: the report's table.
pub fn text(report: &PlaytestReport) -> String {
    report.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::fake::{Fake, report};

    #[test]
    fn the_arguments_name_the_tries_and_the_seeds() {
        let args = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).into()).collect() };
        let options = [GameOption {
            flag: "--hard",
            value: None,
            help: "harder",
        }];
        let input = parse_args(
            &args(&["--tries", "5", "--seed", "9", "--turns", "50", "--hard"]),
            &options,
        )
        .unwrap();
        assert_eq!((input.tries, input.seed, input.turns), (5, 9, 50));
        assert_eq!(input.start[0].flag, "--hard");
        assert_eq!(parse_args(&[], &[]).unwrap(), PlaytestInput::default());
        assert_eq!(
            parse_args(&args(&["3"]), &options).unwrap_err(),
            "no <games> here, only flags: 3"
        );
        for bad in [
            &["3"][..],
            &["--tries"],
            &["--seed", "-1"],
            &["--tries", "9999999999"],
            &["--hard"],
        ] {
            assert!(parse_args(&args(bad), &[]).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn playtest_checks_the_tries_and_the_seeds_then_asks_the_game() {
        let input = |tries: u32, seed: u64| PlaytestInput {
            tries,
            seed,
            ..PlaytestInput::default()
        };
        let game = Fake::default();
        let answer = playtest(&game, &input(1, 0));
        assert!(answer.ok);
        assert_eq!(answer.op, Op::Playtest);
        assert!(answer.limits.is_empty());
        assert!(playtest(&game, &input(MAX_GAMES, 0)).ok);
        for tries in [0, MAX_GAMES + 1] {
            let error = playtest(&game, &input(tries, 0)).error.unwrap();
            assert!(error.starts_with("tries: 1 to 10000"), "{error}");
        }
        assert_eq!(
            playtest(&game, &input(2, u64::MAX - 1)).error.as_deref(),
            Some("seed: too big for that many tries")
        );
        let blocked = Fake {
            playtest: Ok(report(None, true)),
            ..Fake::default()
        };
        let answer = playtest(&blocked, &input(1, 0));
        assert!(!answer.ok);
        assert_eq!(
            answer.limits,
            ["1 of the tries reached turn 200 without an end: their outcome is unknown"]
        );
        assert_eq!(answer.exit_code(), 1);
        let broken = Fake {
            playtest: Err("no stage".into()),
            ..Fake::default()
        };
        assert_eq!(
            playtest(&broken, &input(1, 0)).error.as_deref(),
            Some("no stage")
        );
    }

    #[test]
    fn what_the_playtest_found_reads_for_people() {
        let table = text(&report(Some("Won"), false));
        assert!(table.starts_with("1 tries (seeds 0–0)\n"), "{table}");
        assert!(
            table.ends_with("try 0 · walk · Won · turns 3 · won 1\n"),
            "{table}"
        );
        let usage = usage(
            "tools",
            &[GameOption {
                flag: "--seed",
                value: Some("N"),
                help: "the seed\nof it",
            }],
        );
        assert!(usage.starts_with("usage: tools playtest [--tries N] [--seed N]"));
        assert!(
            usage.ends_with("\n  --seed N             the seed\n"),
            "{usage}"
        );
    }
}
