//! `cargo xtask skill-examples [<skill>...] [--shard <i>/<n>]`: the worked
//! example of each skill of the engine's skill pack (`skills/`), run on a
//! fresh `shuyi new` game (ticket 1134, ADR-0043). If an example fails, its
//! skill is wrong.
//!
//! A skill's example is `skills/<skill>/example.ron`, an [`Example`]: the
//! template its game is made from, and the steps of a script
//! ([`crate::script`]), whose `Write` steps take files from the skill's
//! folder. Each game is made in `target/skill-examples/<skill>`, named
//! after its skill (`shuyi-gates`' crates are `shuyi-gates` and
//! `shuyi-gates-rules`), on this checkout of the engine, and builds in this
//! repository's target folder unless `CARGO_TARGET_DIR` names another. Two
//! games never share a crate's name: in one target folder, cargo could take
//! one's build for the other's. An example marked `manual` (it needs a
//! window) is listed and not run.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::script::{self, Runner, Step};

/// The skill pack, from the repository's root.
pub const SKILLS: &str = "skills";
/// A skill's worked example, in its folder.
pub const EXAMPLE: &str = "example.ron";
/// Where the games are made, from the repository's root.
const GAMES: &str = "target/skill-examples";

/// A skill's worked example.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    /// The template its game is made from, a short name of `shuyi new
    /// --template` (`grid`, `blank`).
    pub template: String,
    /// Why CI can't run it (it needs a window), if it can't.
    #[serde(default)]
    pub manual: Option<String>,
    /// The steps, run in the game's folder.
    pub steps: Vec<Step>,
}

/// `skill-examples`' options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The skills to run, by name; every one if empty.
    pub only: Vec<String>,
    /// `--shard <i>/<n>`: of the examples in name order, every `n`th from
    /// the `i`th (from 1).
    pub shard: Option<(usize, usize)>,
}

impl Options {
    /// The options `args` give, or why they are wrong.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            if arg == "--shard" {
                let value = rest.next().ok_or("--shard needs <i>/<n>")?;
                options.shard = Some(shard_of(value)?);
            } else if arg.starts_with("--") {
                return Err(format!("unknown option {arg}"));
            } else {
                options.only.push(arg.clone());
            }
        }
        Ok(options)
    }
}

/// `<i>/<n>`, with `1 <= i <= n`.
fn shard_of(value: &str) -> Result<(usize, usize), String> {
    let wrong = || format!("--shard {value}: not <i>/<n> with 1 <= i <= n");
    let (i, n) = value.split_once('/').ok_or_else(wrong)?;
    let (i, n): (usize, usize) = (
        i.parse().map_err(|_| wrong())?,
        n.parse().map_err(|_| wrong())?,
    );
    if i == 0 || i > n {
        return Err(wrong());
    }
    Ok((i, n))
}

/// The skills of the pack at `root` with a worked example, by name, with
/// their folder; or the errors of reading them.
pub fn examples(root: &Path) -> (Vec<(String, PathBuf, Example)>, Vec<String>) {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    let pack = root.join(SKILLS);
    let entries = match fs::read_dir(&pack) {
        Ok(entries) => entries,
        Err(e) => return (found, vec![format!("{SKILLS}: {e}")]),
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        let file = dir.join(EXAMPLE);
        if !file.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let shown = format!("{SKILLS}/{name}/{EXAMPLE}");
        match fs::read_to_string(&file) {
            Ok(text) => match ron::from_str::<Example>(&text) {
                Ok(example) => found.push((name, dir, example)),
                Err(e) => errors.push(format!("{shown}: {e}")),
            },
            Err(e) => errors.push(format!("{shown}: {e}")),
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    (found, errors)
}

/// What `skill-examples` found: the errors, and the report for people.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Each example that failed, or couldn't run.
    pub errors: Vec<String>,
    /// Each example's steps and verdicts.
    pub report: String,
}

/// Runs the worked examples `options` pick from the pack at `root`, each on
/// a game made in `games`, with `runner`'s commands.
pub fn run(root: &Path, games: &Path, options: &Options, runner: &dyn Runner) -> Outcome {
    let (found, mut errors) = examples(root);
    let mut report = String::new();
    for name in &options.only {
        if !found.iter().any(|(n, _, _)| n == name) {
            errors.push(format!(
                "{name}: no skill of {SKILLS}/ with a worked example ({EXAMPLE})"
            ));
        }
    }
    let picked = found
        .iter()
        .filter(|(name, _, _)| options.only.is_empty() || options.only.contains(name))
        .enumerate()
        .filter(|(i, _)| options.shard.is_none_or(|(at, of)| i % of == at - 1))
        .map(|(_, found)| found);
    for (name, dir, example) in picked {
        // Writing to a `String` can't fail.
        if let Some(why) = &example.manual {
            let _ = writeln!(report, "{name}: manual, not run: {why}");
            continue;
        }
        let game = games.join(name);
        let _ = writeln!(
            report,
            "{name}: {} step(s) on a new `{}` game in {}",
            example.steps.len(),
            example.template,
            game.display()
        );
        if let Err(e) = make_game(root, &game, &example.template, name, runner) {
            let _ = writeln!(report, "  the game wasn't made: {e}");
            errors.push(format!("{name}: the game wasn't made: {e}"));
            continue;
        }
        let verdicts = script::run(&example.steps, dir, &game, runner);
        report.push_str(&script::report(&verdicts));
        for verdict in verdicts.iter().filter(|v| !v.passed) {
            errors.push(format!(
                "{name}: step {} ({}) failed{}",
                verdict.step,
                verdict.what,
                verdict
                    .detail
                    .as_deref()
                    .and_then(|d| d.lines().next())
                    .map(|first| format!(": {first}"))
                    .unwrap_or_default()
            ));
        }
    }
    Outcome { errors, report }
}

/// Makes a game named `name` in `game` from `template` with the engine
/// checkout at `root` (`shuyi new`), over any game made there before.
fn make_game(
    root: &Path,
    game: &Path,
    template: &str,
    name: &str,
    runner: &dyn Runner,
) -> Result<(), String> {
    if game.exists() {
        fs::remove_dir_all(game).map_err(|e| format!("{}: {e}", game.display()))?;
    }
    let path = |p: &Path| p.to_string_lossy().into_owned();
    let args: Vec<String> = [
        "run",
        "-q",
        "-p",
        "shuyi-cli",
        "--locked",
        "--",
        "new",
        &path(game),
        "--template",
        template,
        "--name",
        name,
        "--engine-path",
        &path(root),
        "--json",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let ran = runner.run(root, "cargo", &args)?;
    let answer: Value = serde_json::from_str(ran.stdout.trim()).map_err(|e| {
        format!(
            "`shuyi new` gave no JSON answer ({e}): {}",
            ran.stderr.trim()
        )
    })?;
    if answer.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(format!(
            "`shuyi new` failed: {}",
            answer.get("error").unwrap_or(&Value::Null)
        ))
    }
}

/// Where the games are made, under the repository's root.
pub fn games_dir(root: &Path) -> PathBuf {
    root.join(GAMES)
}

#[cfg(test)]
mod tests;
