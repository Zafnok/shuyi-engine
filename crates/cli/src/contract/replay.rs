//! `replay`: a record of a game re-run by the game's own rules, with a
//! state hash per step ([`shuyi_core::state_hash`]). One record is run
//! twice, which finds luck or state that a run doesn't own; two records
//! are run side by side. The answer says the first step where the two
//! runs' hashes differ, and passes if there is none.
//!
//! What a record is belongs to the game (for a turn game: its seed and
//! its commands, ADR-0003 §2). [`turns`] re-runs one over any
//! [`TurnRules`].
//!
//! A record stamped with what made it ([`ReplayRun::made`]) is checked
//! against this build's stamp ([`GameTools::made`]) before anything is
//! compared: one made by another engine, another build of the game or
//! other content is refused, since its runs may differ with no desync,
//! unless the input says [`any_stamp`](ReplayInput::any_stamp).

use std::fmt::{Debug, Display, Write as _};
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_core::{Made, TurnRules, state_hash};

use super::codes::{self, Code};
use super::{Answer, Op};
use crate::game::{GameTools, code_of};

/// What `replay` re-runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `ReplayInput::new` (or `default()`) and its fields
// assigned, it may grow (ADR-0039).
#[non_exhaustive]
pub struct ReplayInput {
    /// The record's file, from the project's root.
    pub record: String,
    /// A second record to run beside it; without one, the record is run
    /// twice.
    #[serde(default)]
    pub other: Option<String>,
    /// Run a record stamped by another build or with other content too
    /// (`--any-stamp`), which is otherwise refused; its runs may then
    /// differ for that reason alone.
    #[serde(default)]
    pub any_stamp: bool,
}

impl ReplayInput {
    /// The record in the file `record`, run twice (no
    /// [`other`](Self::other)), refused if stamped by another build.
    pub fn new(record: impl Into<String>) -> Self {
        Self {
            record: record.into(),
            other: None,
            any_stamp: false,
        }
    }
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
    /// What made the record, if it says: the game's `replay` passes on
    /// its record's stamp ([`turns`] leaves it `None`).
    #[serde(default)]
    pub made: Option<Made>,
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
        made: None,
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
    // A record run twice is checked once.
    let records = if input.other.is_some() {
        &runs[..]
    } else {
        &runs[..1]
    };
    let mut limits = match stamps(game.made().as_ref(), records, input.any_stamp) {
        Ok(limits) => limits,
        Err(e) => return Answer::failed(Op::Replay, codes::OTHER_STAMP, e),
    };
    let difference = first_difference(&runs[0], &runs[1]);
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

/// The limits the stamps of `runs` put on what they show, checked against
/// this build's, `now`; the refusal of the first stamped by another build,
/// unless `any_stamp`.
fn stamps(now: Option<&Made>, runs: &[ReplayRun], any_stamp: bool) -> Result<Vec<String>, String> {
    let Some(now) = now else {
        return Ok(vec![
            "the game's tools give no stamp of this build (GameTools::made), so no record was \
             checked against it: a difference may come from another build or other content"
                .to_owned(),
        ]);
    };
    let mut limits = Vec::new();
    for run in runs {
        let record = &run.record;
        match run.made.as_ref().map(|made| made.check(now)) {
            None => limits.push(format!(
                "{record} has no stamp: a difference may come from another build or other content"
            )),
            Some(Ok(())) => {}
            Some(Err(mismatch)) if any_stamp => limits.push(format!(
                "{record} was {mismatch}: a difference may come from that, not a desync"
            )),
            Some(Err(mismatch)) => {
                return Err(format!(
                    "{record} was {mismatch}: make it again with this build, or replay it with \
                     --any-stamp"
                ));
            }
        }
    }
    Ok(limits)
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
    for (i, run) in output.runs.iter().enumerate() {
        if i > 0 && run.record == first.record {
            continue;
        }
        match &run.made {
            Some(made) => {
                let _ = writeln!(out, "  {} was made with {made}", run.record);
            }
            None => {
                let _ = writeln!(out, "  {} has no stamp", run.record);
            }
        }
    }
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
    use crate::contract::fake::{Fake, stamp};
    use shuyi_core::ContentHash;

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
            made: None,
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
        let input = |record: &str, other: Option<&str>| {
            let mut input = ReplayInput::new(record);
            input.other = other.map(Into::into);
            input
        };
        let answer = replay(&Fake::default(), &dir, &input("a.ron", None));
        assert!(answer.ok, "{answer:?}");
        assert_eq!(answer.op, Op::Replay);
        assert_eq!(answer.limits.len(), 2);
        assert!(answer.limits[0].starts_with("the game's tools give no stamp of this build"));
        assert!(answer.limits[1].starts_with("the record ran twice in one process"));
        let out = answer.result.unwrap();
        assert_eq!(out.runs.len(), 2);
        assert_eq!(out.runs[0], out.runs[1]);
        let answer = replay(&Fake::default(), &dir, &input("a.ron", Some("b.ron")));
        assert!(!answer.ok);
        assert_eq!(answer.limits.len(), 1, "{:?}", answer.limits);
        let out = answer.result.unwrap();
        assert_eq!(out.runs[1].record, "b.ron");
        assert_eq!(out.first_difference.unwrap().step, 1);
        let refused = replay(&Fake::default(), &dir, &input("c.ron", Some("c.ron")));
        assert_eq!(
            refused.limits[1..],
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
    fn a_record_stamped_by_another_build_is_refused_unless_any_stamp() {
        let dir = scratch("stamps");
        let now = stamp("0.2.0", "demo", ContentHash(2));
        let game = Fake {
            made: Some(now.clone()),
            ..Fake::default()
        };
        fs::write(
            dir.join("same.ron"),
            "made 0.2.0 demo 0000000000000002\nh0\nh1",
        )
        .unwrap();
        fs::write(
            dir.join("other.ron"),
            "made 0.2.0 demo 0000000000000001\nh0\nh1",
        )
        .unwrap();
        fs::write(dir.join("none.ron"), "h0\nh1").unwrap();
        // Stamped by this build: it runs, with its stamp, and nothing to say.
        let same = replay(&game, &dir, &ReplayInput::new("same.ron"));
        assert!(same.ok, "{same:?}");
        assert_eq!(same.limits.len(), 1, "{:?}", same.limits);
        let out = same.result.unwrap();
        assert_eq!(out.runs[0].made.as_ref(), Some(&now));
        assert_eq!(out.runs[0].steps.len(), 2);
        // Other content: refused before anything is compared, naming the
        // record and both stamps' contents.
        let mut input = ReplayInput::new("same.ron");
        input.other = Some("other.ron".into());
        let other = replay(&game, &dir, &input);
        assert!(!other.ok);
        assert_eq!(other.code.as_deref(), Some("SH3007"));
        assert_eq!(other.exit_code(), 1);
        assert_eq!(
            other.error.as_deref(),
            Some(
                "other.ron was made with content 0000000000000001; this build: content \
                 0000000000000002: make it again with this build, or replay it with --any-stamp"
            )
        );
        assert!(other.result.is_none());
        // With `any_stamp`, it runs and says why a difference may show.
        input.any_stamp = true;
        let anyway = replay(&game, &dir, &input);
        assert!(anyway.ok, "{anyway:?}");
        assert_eq!(
            anyway.limits,
            [
                "other.ron was made with content 0000000000000001; this build: content \
              0000000000000002: a difference may come from that, not a desync"
            ]
        );
        // A record with no stamp runs, and says so.
        let none = replay(&game, &dir, &ReplayInput::new("none.ron"));
        assert!(none.ok, "{none:?}");
        assert_eq!(
            none.limits[0],
            "none.ron has no stamp: a difference may come from another build or other content"
        );
        // A game that gives no stamp checks none.
        let unstamped = replay(&Fake::default(), &dir, &ReplayInput::new("other.ron"));
        assert!(unstamped.ok, "{unstamped:?}");
        let got = &unstamped.limits[0];
        assert!(got.contains("GameTools::made"), "{got}");
        // The stamp goes out as text, and reads back.
        let json = serde_json::to_value(&out.runs[0]).unwrap();
        assert_eq!(
            json["made"],
            serde_json::json!({"engine": "0.2.0", "game": "demo", "content": "0000000000000002"})
        );
        let back: ReplayRun = serde_json::from_value(json).unwrap();
        assert_eq!(back, out.runs[0]);
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
            "replay: r (2 step(s)), outcome Won\n  r has no stamp\n     0 h0 start\n     1 h1 \
             Move → Moved, Reached\n     2 h2 Stay (refused: over)\nthe two runs agree at every \
             step\n"
        );
        let mut stamped = run_of(&["h0"]);
        stamped.made = Some(stamp("0.1.0", "demo", ContentHash(1)));
        let mut other = run_of(&["h0"]);
        other.record = "o".into();
        let two = ReplayOutput {
            runs: vec![stamped, other],
            first_difference: None,
        };
        assert_eq!(
            text(&two),
            "replay: r (0 step(s)), outcome none\n  r was made with engine 0.1.0, game \"demo\", \
             content 0000000000000001\n  o has no stamp\n     0 h0 start\nthe two runs agree at \
             every step\n"
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
            "replay: r (0 step(s)), outcome none\n  r has no stamp\n     0 a start\nthe runs \
             differ first at step 1: b vs (none)\n"
        );
        let none = ReplayOutput {
            runs: Vec::new(),
            first_difference: None,
        };
        assert_eq!(text(&none), "");
    }

    #[test]
    fn a_new_input_runs_its_record_twice() {
        let literal = ReplayInput {
            record: "a.ron".to_owned(),
            other: None,
            any_stamp: false,
        };
        assert_eq!(ReplayInput::new("a.ron"), literal);
    }
}
