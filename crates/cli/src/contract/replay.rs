//! `replay`: a record of a game re-run by the game's own rules, with a
//! state hash per step ([`shuyi_core::state_hash`]). One record is run
//! twice, which finds luck or state that a run doesn't own; two records
//! are run side by side. The answer says the first step where the two
//! runs' hashes differ, and passes if there is none.
//!
//! What a record is belongs to the game (for a turn game: its seed and
//! its commands, ADR-0003 §2). [`turns`] re-runs one over any
//! [`TurnRules`].

use std::fmt::{Debug, Display, Write as _};
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_core::{TurnRules, state_hash};

use super::codes::{self, Code};
use super::{Answer, Op};
use crate::game::{GameTools, code_of};

/// What `replay` re-runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayInput {
    /// The record's file, from the project's root.
    pub record: String,
    /// A second record to run beside it; without one, the record is run
    /// twice.
    #[serde(default)]
    pub other: Option<String>,
}

/// A record handed to the game: its file's name and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The file, as given.
    pub file: String,
    /// Its text.
    pub text: String,
}

/// One run of a record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayRun {
    /// The record's file.
    pub record: String,
    /// Step 0 (the state it starts in), then one step per command.
    pub steps: Vec<ReplayStep>,
    /// How the game ended, in the game's words, if it did.
    pub outcome: Option<String>,
}

/// One step of a run: the state after a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayStep {
    /// The step: 0 before any command, then the command's place from 1.
    pub step: u32,
    /// The command, in the game's words; none at step 0.
    pub command: Option<String>,
    /// The state's hash after it (sixteen hex digits).
    pub hash: String,
    /// What it did, in the game's words.
    pub events: Vec<String>,
    /// Why the rules refused it, if they did (the state didn't change).
    pub refused: Option<String>,
}

/// Where two runs part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Difference {
    /// The first step whose hashes differ, or that only one run has.
    pub step: u32,
    /// Each run's hash at that step; none for a run that has no such
    /// step.
    pub hashes: Vec<Option<String>>,
}

/// What `replay` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayOutput {
    /// The two runs.
    pub runs: Vec<ReplayRun>,
    /// The first step where they differ, if they do.
    pub first_difference: Option<Difference>,
}

/// Re-runs `commands` from `initial` by the rules, hashing the state
/// before them and after each: the run of a turn game's record `file`. A
/// refused command changes nothing and says why.
pub fn turns<R>(file: &str, initial: R, commands: &[R::Command]) -> Result<ReplayRun, String>
where
    R: TurnRules + Serialize,
    R::Command: Debug,
    R::Event: Debug,
    R::Error: Display,
    R::Outcome: Debug,
{
    let mut state = initial;
    let hash = |state: &R| {
        state_hash(state)
            .map(|h| h.to_string())
            .map_err(|e| e.to_string())
    };
    let mut steps = vec![ReplayStep {
        step: 0,
        command: None,
        hash: hash(&state)?,
        events: Vec::new(),
        refused: None,
    }];
    for (step, command) in (1..).zip(commands) {
        let (events, refused) = match state.apply(command) {
            Ok(events) => (events.iter().map(|e| format!("{e:?}")).collect(), None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        };
        steps.push(ReplayStep {
            step,
            command: Some(format!("{command:?}")),
            hash: hash(&state)?,
            events,
            refused,
        });
    }
    Ok(ReplayRun {
        record: file.to_owned(),
        steps,
        outcome: state.outcome().map(|o| format!("{o:?}")),
    })
}

/// The first step where `a` and `b` differ, if they do.
pub fn first_difference(a: &ReplayRun, b: &ReplayRun) -> Option<Difference> {
    let longest = a.steps.len().max(b.steps.len());
    (0..longest).find_map(|i| {
        let hashes = [a, b].map(|run| run.steps.get(i).map(|s| s.hash.clone()));
        (hashes[0] != hashes[1]).then(|| Difference {
            step: u32::try_from(i).unwrap_or(u32::MAX),
            hashes: hashes.to_vec(),
        })
    })
}

/// The record at `path` under `root`.
fn read(root: &Path, path: &str) -> Result<Record, String> {
    let text = fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
    Ok(Record {
        file: path.to_owned(),
        text,
    })
}

/// `replay`: `input`'s records, under `root`, re-run by `game`.
pub fn replay(game: &dyn GameTools, root: &Path, input: &ReplayInput) -> Answer<ReplayOutput> {
    let runs = (|| {
        let read = |path| read(root, path).map_err(|e| (codes::FILE, e));
        let first = read(&input.record)?;
        let second = match &input.other {
            Some(other) => read(other)?,
            None => first.clone(),
        };
        let replay = |record| game.replay(record).map_err(|e| (code_of(&e), e));
        Ok::<_, (Code, String)>([replay(&first)?, replay(&second)?])
    })();
    let runs = match runs {
        Ok(runs) => runs,
        Err((code, e)) => return Answer::failed(Op::Replay, code, e),
    };
    let difference = first_difference(&runs[0], &runs[1]);
    let mut limits = Vec::new();
    if input.other.is_none() {
        limits.push(
            "the record ran twice in one process: what differs from one machine or build to \
             another needs a second record made there"
                .to_owned(),
        );
    }
    let refused = runs[0].steps.iter().filter(|s| s.refused.is_some()).count();
    if refused > 0 {
        limits.push(format!(
            "the rules refused {refused} command(s) of {}: the record doesn't play as it was made",
            runs[0].record
        ));
    }
    let output = ReplayOutput {
        runs: runs.to_vec(),
        first_difference: difference,
    };
    Answer::new(
        Op::Replay,
        output.first_difference.is_none(),
        output,
        limits,
    )
}

/// What `replay` found, for people.
pub fn text(output: &ReplayOutput) -> String {
    let mut out = String::new();
    let Some(first) = output.runs.first() else {
        return out;
    };
    let _ = writeln!(
        out,
        "replay: {} ({} step(s)), outcome {}",
        first.record,
        first.steps.len().saturating_sub(1),
        first.outcome.as_deref().unwrap_or("none")
    );
    for step in &first.steps {
        let command = step.command.as_deref().unwrap_or("start");
        let _ = write!(out, "  {:>4} {} {command}", step.step, step.hash);
        if let Some(refused) = &step.refused {
            let _ = write!(out, " (refused: {refused})");
        } else if !step.events.is_empty() {
            let _ = write!(out, " → {}", step.events.join(", "));
        }
        out.push('\n');
    }
    match &output.first_difference {
        None => out.push_str("the two runs agree at every step\n"),
        Some(d) => {
            let shown: Vec<&str> = d
                .hashes
                .iter()
                .map(|h| h.as_deref().unwrap_or("(none)"))
                .collect();
            let _ = writeln!(
                out,
                "the runs differ first at step {}: {}",
                d.step,
                shown.join(" vs ")
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde::Serialize;
    use shuyi_core::TurnRules;

    use super::*;
    use crate::contract::fake::Fake;

    /// A counter that ends at 3; an `Add` of more than 1 is refused.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    struct Counter(u32);

    impl TurnRules for Counter {
        type Command = u32;
        type Event = u32;
        type Error = String;
        type Outcome = &'static str;

        fn check(&self, add: &u32) -> Result<(), String> {
            if *add > 1 {
                Err(format!("{add} is too many"))
            } else {
                Ok(())
            }
        }

        fn apply(&mut self, add: &u32) -> Result<Vec<u32>, String> {
            self.check(add)?;
            self.0 += add;
            Ok(vec![self.0])
        }

        fn legal_commands(&self) -> Vec<u32> {
            vec![0, 1]
        }

        fn outcome(&self) -> Option<&'static str> {
            (self.0 >= 3).then_some("Done")
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shuyi-replay-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn turns_hashes_the_state_before_and_after_each_command() {
        let run = turns("r.ron", Counter(0), &[1, 5, 1, 1]).unwrap();
        assert_eq!(run.record, "r.ron");
        assert_eq!(run.steps.len(), 5);
        let hash = |n: u32| state_hash(&Counter(n)).unwrap().to_string();
        assert_eq!(run.steps[0].hash, hash(0));
        assert_eq!(
            (run.steps[0].step, run.steps[0].command.as_deref()),
            (0, None)
        );
        assert_eq!(run.steps[1].command.as_deref(), Some("1"));
        assert_eq!(run.steps[1].events, ["1"]);
        assert_eq!(run.steps[1].hash, hash(1));
        // Refused: the state, and its hash, stay.
        assert_eq!(run.steps[2].refused.as_deref(), Some("5 is too many"));
        let got = &run.steps[2].events;
        assert!(got.is_empty(), "{got:?}");
        assert_eq!(run.steps[2].hash, hash(1));
        assert_eq!(run.steps[4].step, 4);
        assert_eq!(run.steps[4].hash, hash(3));
        assert_eq!(run.outcome.as_deref(), Some("\"Done\""));
        assert_eq!(turns("r", Counter(0), &[]).unwrap().outcome, None);
    }

    fn run_of(hashes: &[&str]) -> ReplayRun {
        ReplayRun {
            record: "r".into(),
            steps: (0..)
                .zip(hashes)
                .map(|(step, hash)| ReplayStep {
                    step,
                    command: None,
                    hash: (*hash).to_owned(),
                    events: Vec::new(),
                    refused: None,
                })
                .collect(),
            outcome: None,
        }
    }

    #[test]
    fn the_first_difference_is_the_first_step_that_differs_or_only_one_has() {
        let a = run_of(&["a", "b", "c"]);
        assert_eq!(first_difference(&a, &a), None);
        let b = run_of(&["a", "x", "c"]);
        assert_eq!(
            first_difference(&a, &b),
            Some(Difference {
                step: 1,
                hashes: vec![Some("b".into()), Some("x".into())]
            })
        );
        let short = run_of(&["a", "b"]);
        assert_eq!(
            first_difference(&a, &short),
            Some(Difference {
                step: 2,
                hashes: vec![Some("c".into()), None]
            })
        );
        assert_eq!(first_difference(&short, &a).unwrap().hashes[0], None);
    }

    #[test]
    fn replay_runs_one_record_twice_or_two_side_by_side() {
        let dir = scratch("runs");
        fs::write(dir.join("a.ron"), "h0\nh1\nh2").unwrap();
        fs::write(dir.join("b.ron"), "h0\nhx").unwrap();
        fs::write(dir.join("c.ron"), "h0\nrefused").unwrap();
        let input = |record: &str, other: Option<&str>| ReplayInput {
            record: record.into(),
            other: other.map(Into::into),
        };
        let answer = replay(&Fake::default(), &dir, &input("a.ron", None));
        assert!(answer.ok, "{answer:?}");
        assert_eq!(answer.op, Op::Replay);
        assert_eq!(answer.limits.len(), 1);
        assert!(answer.limits[0].starts_with("the record ran twice in one process"));
        let out = answer.result.unwrap();
        assert_eq!(out.runs.len(), 2);
        assert_eq!(out.runs[0], out.runs[1]);
        let answer = replay(&Fake::default(), &dir, &input("a.ron", Some("b.ron")));
        assert!(!answer.ok);
        assert!(answer.limits.is_empty(), "{:?}", answer.limits);
        let out = answer.result.unwrap();
        assert_eq!(out.runs[1].record, "b.ron");
        assert_eq!(out.first_difference.unwrap().step, 1);
        let refused = replay(&Fake::default(), &dir, &input("c.ron", Some("c.ron")));
        assert_eq!(
            refused.limits,
            ["the rules refused 1 command(s) of c.ron: the record doesn't play as it was made"]
        );
        // What can't be read or run says why.
        let missing = replay(&Fake::default(), &dir, &input("a.ron", Some("no.ron")));
        assert_eq!(missing.code.as_deref(), Some("SH3004"));
        assert_eq!(missing.exit_code(), 1);
        assert!(missing.error.unwrap().starts_with("no.ron: "));
        let game = Fake {
            replay: Err("bad record".into()),
            ..Fake::default()
        };
        let bad = replay(&game, &dir, &input("a.ron", None));
        assert_eq!(bad.code.as_deref(), Some("SH3002"));
        assert_eq!(bad.error.as_deref(), Some("bad record"));
        let unsupported = Fake {
            replay: Err(crate::game::unsupported("replay", "replay")),
            ..Fake::default()
        };
        let answer = replay(&unsupported, &dir, &input("a.ron", None));
        assert_eq!(answer.code.as_deref(), Some("SH3001"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_replay_found_reads_for_people() {
        let mut run = run_of(&["h0", "h1", "h2"]);
        run.outcome = Some("Won".into());
        run.steps[1].command = Some("Move".into());
        run.steps[1].events = vec!["Moved".into(), "Reached".into()];
        run.steps[2].command = Some("Stay".into());
        run.steps[2].refused = Some("over".into());
        let agreed = ReplayOutput {
            runs: vec![run.clone(), run.clone()],
            first_difference: None,
        };
        assert_eq!(
            text(&agreed),
            "replay: r (2 step(s)), outcome Won\n     0 h0 start\n     1 h1 Move → Moved, \
             Reached\n     2 h2 Stay (refused: over)\nthe two runs agree at every step\n"
        );
        let parted = ReplayOutput {
            runs: vec![run_of(&["a"])],
            first_difference: Some(Difference {
                step: 1,
                hashes: vec![Some("b".into()), None],
            }),
        };
        assert_eq!(
            text(&parted),
            "replay: r (0 step(s)), outcome none\n     0 a start\nthe runs differ first at \
             step 1: b vs (none)\n"
        );
        let none = ReplayOutput {
            runs: Vec::new(),
            first_difference: None,
        };
        assert_eq!(text(&none), "");
    }
}
