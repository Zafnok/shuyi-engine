//! A line-oriented JSON oracle and deterministic batch runner for the pilot.

use std::io::{self, BufRead, Write};

use cloning_fixture::{courier, workshop};
use serde::Serialize;
use shuyi_core::{TurnRules, state_hash};

#[derive(Clone, Serialize)]
struct Record {
    ok: bool,
    action: String,
    events: Vec<String>,
    state: serde_json::Value,
    frame: String,
    hash: String,
    outcome: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct Batch {
    game: String,
    seed: u64,
    trace: Vec<Record>,
    r#final: Record,
}

enum Session {
    Courier(courier::Courier),
    Workshop(workshop::Workshop),
}

impl Session {
    fn new(game: &str, seed: u64) -> Result<Self, String> {
        match game {
            "courier" => Ok(Self::Courier(courier::Courier::new(seed))),
            "workshop" => Ok(Self::Workshop(workshop::Workshop::new(seed))),
            _ => Err(format!("unknown game: {game}")),
        }
    }

    fn record(
        &self,
        action: &str,
        events: Vec<String>,
        error: Option<String>,
    ) -> Result<Record, String> {
        let (state, frame, hash, outcome) = match self {
            Self::Courier(game) => (
                game.visible(),
                game.frame(),
                state_hash(game)
                    .map_err(|error| error.to_string())?
                    .to_string(),
                game.outcome().map(|result| format!("{result:?}")),
            ),
            Self::Workshop(game) => (
                game.visible(),
                game.frame(),
                state_hash(game)
                    .map_err(|error| error.to_string())?
                    .to_string(),
                game.outcome().map(|result| format!("{result:?}")),
            ),
        };
        Ok(Record {
            ok: error.is_none(),
            action: action.to_owned(),
            events,
            state,
            frame,
            hash,
            outcome,
            error,
        })
    }

    fn step(&mut self, action: &str) -> Result<Record, String> {
        if action == "state" {
            return self.record(action, Vec::new(), None);
        }
        let result = match self {
            Self::Courier(game) => courier::Action::parse(action)
                .ok_or_else(|| format!("unknown action: {action}"))
                .and_then(|parsed| {
                    game.apply(&parsed)
                        .map(|events| {
                            events
                                .into_iter()
                                .map(|event| format!("{event:?}"))
                                .collect()
                        })
                        .map_err(|error| format!("{error:?}"))
                }),
            Self::Workshop(game) => workshop::Action::parse(action)
                .ok_or_else(|| format!("unknown action: {action}"))
                .and_then(|parsed| {
                    game.apply(&parsed)
                        .map(|events| {
                            events
                                .into_iter()
                                .map(|event| format!("{event:?}"))
                                .collect()
                        })
                        .map_err(|error| format!("{error:?}"))
                }),
        };
        match result {
            Ok(events) => self.record(action, events, None),
            Err(error) => self.record(action, Vec::new(), Some(error)),
        }
    }
}

fn write_json<T: Serialize>(out: &mut impl Write, value: &T) -> Result<(), String> {
    serde_json::to_writer(&mut *out, value).map_err(|error| error.to_string())?;
    writeln!(out).map_err(|error| error.to_string())?;
    out.flush().map_err(|error| error.to_string())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: cloning-fixture <courier|workshop> <u64-seed> <comma-separated-actions|--oracle>".to_owned());
    }
    let seed = args[1]
        .parse::<u64>()
        .map_err(|error| format!("seed: {error}"))?;
    let mut session = Session::new(&args[0], seed)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if args[2] == "--oracle" {
        for line in io::stdin().lock().lines() {
            let line = line.map_err(|error| error.to_string())?;
            let action = line.trim();
            write_json(&mut out, &session.step(action)?)?;
        }
        return Ok(());
    }
    let mut trace = vec![session.record("state", Vec::new(), None)?];
    if args[2] != "-" {
        for action in args[2].split(',') {
            trace.push(session.step(action.trim())?);
        }
    }
    let r#final = trace.last().cloned().ok_or("missing initial record")?;
    write_json(
        &mut out,
        &Batch {
            game: args[0].clone(),
            seed,
            trace,
            r#final,
        },
    )
}

fn main() {
    if let Err(error) = run() {
        let _ = writeln!(io::stderr().lock(), "{error}");
        std::process::exit(2);
    }
}
