//! `bake [--target <id>] [--out <file>]`: the game's sources baked into
//! one pack (`.shuyipak`, ADR-0023), with what went in, the sizes, and
//! how long baking and loading took. A source with errors is refused,
//! with every error. `check --baked` uses the same bake to prove the pack
//! equal to its sources ([`check_baked`]).
//!
//! The game says what it bakes for a target, from which bundle and with
//! which formats ([`GameTools::bake`]). A target must have a profile, the
//! engine's or the game's ([`target::profiles`]); the game picks its
//! layers and kinds by it (ticket 3007 bakes one-bit pictures so).

use std::fmt::Write as _;
use std::fs;
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::format::Formats;
use shuyi_assets::pack::{Bake, Pack};

use super::{Answer, Op, Problem, codes, target};
use crate::game::{GameBake, GameTools, code_of};
use crate::project::Project;

/// Where a pack goes when `--out` doesn't say, from the project's root:
/// `target/shuyi/game.shuyipak`, or `<target>.shuyipak` for a target.
pub const OUT_DIR: &str = "target/shuyi";

/// How many times each load is timed: the fastest counts.
pub const LOAD_RUNS: u32 = 5;

/// What `bake` bakes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BakeInput {
    /// The target to bake for, by its profile's id, passed to the game;
    /// none for its default bundle.
    #[serde(default)]
    pub target: Option<String>,
    /// The pack's file, from the project's root; `target/shuyi/game.shuyipak`
    /// (or `<target>.shuyipak`) if none.
    #[serde(default)]
    pub out: Option<String>,
}

/// What `bake` made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BakeOutput {
    /// Every error of the sources: the pack is made only when there is
    /// none.
    pub errors: Vec<Problem>,
    /// The pack, if it was made.
    pub pack: Option<PackMade>,
}

/// A pack `bake` wrote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackMade {
    /// Its file, from the project's root.
    pub file: String,
    /// The target it was baked for.
    pub target: Option<String>,
    /// The files in it.
    pub files: u32,
    /// How many of them hold baked assets; the others hold their own
    /// bytes.
    pub baked: u32,
    /// The kinds baked, by name.
    pub kinds: Vec<String>,
    /// The sources' bytes, added up.
    pub source_bytes: u64,
    /// The pack's bytes.
    pub pack_bytes: u64,
    /// How long baking took, in microseconds.
    pub bake_us: u64,
    /// How long reading every baked asset from its source took, in
    /// microseconds: the fastest of a few runs.
    pub load_sources_us: u64,
    /// How long opening the pack and reading every baked asset from it
    /// took, in microseconds: the fastest of a few runs.
    pub load_pack_us: u64,
}

/// `duration` in whole microseconds.
fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

/// The fastest of [`LOAD_RUNS`] runs of `run`.
fn fastest(mut run: impl FnMut()) -> u64 {
    (0..LOAD_RUNS)
        .map(|_| {
            let start = Instant::now();
            run();
            micros(start.elapsed())
        })
        .min()
        .unwrap_or_default()
}

/// The game's bake for `target`, its formats with the pack's.
fn prepared(game: &dyn GameTools, target: Option<&str>) -> Result<GameBake, String> {
    let mut prepared = game.bake(target)?;
    prepared
        .bake
        .register(&mut prepared.formats)
        .map_err(|e| e.to_string())?;
    Ok(prepared)
}

/// A bundle of pack `bytes`, opened with `stamp`.
fn opened(bytes: &[u8], stamp: &str) -> Result<Bundle, String> {
    let pack = Pack::open(bytes, stamp).map_err(|e| e.to_string())?;
    Ok(Bundle::new(vec![pack.into_owned().into_layer("pack")]))
}

/// How long reading every asset `bake` names from `bundle` takes, the
/// pack opened first if `pack` holds one.
fn load_time(bake: &Bake, formats: &Formats, bundle: &Bundle, pack: Option<&[u8]>) -> u64 {
    fastest(|| match pack {
        Some(bytes) => {
            if let Ok(bundle) = opened(bytes, bake.stamp()) {
                let _ = bake.load(formats, &bundle);
            }
        }
        None => {
            let _ = bake.load(formats, bundle);
        }
    })
}

/// The pack's file for `input`, from the project's root.
fn out_file(input: &BakeInput) -> String {
    input.out.clone().unwrap_or_else(|| {
        let name = input.target.as_deref().unwrap_or("game");
        format!("{OUT_DIR}/{name}.{}", shuyi_assets::pack::EXTENSION)
    })
}

/// `bake`: `game`'s pack for `input`, written under `project`'s root.
pub fn bake(game: &dyn GameTools, project: &Project, input: &BakeInput) -> Answer<BakeOutput> {
    let target = input.target.as_deref();
    if let Some(Err((code, why))) =
        target.map(|target| target::profile(&project.config, &project.root, target))
    {
        return Answer::failed(Op::Bake, code, why);
    }
    let GameBake {
        bake,
        bundle,
        formats,
    } = match prepared(game, target) {
        Ok(prepared) => prepared,
        Err(e) => return Answer::failed(Op::Bake, code_of(&e), e),
    };
    let start = Instant::now();
    let baked = match bake.bake(&formats, &bundle, target) {
        Ok(baked) => baked,
        Err(errors) => {
            let errors = errors.iter().map(Problem::from).collect();
            return Answer::new(
                Op::Bake,
                false,
                BakeOutput { errors, pack: None },
                Vec::new(),
            );
        }
    };
    let bake_us = micros(start.elapsed());
    let file = out_file(input);
    let path = project.root.join(&file);
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&path, &baked.bytes));
    if let Err(e) = written {
        return Answer::failed(Op::Bake, codes::FILE, format!("{file}: {e}"));
    }
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let pack = PackMade {
        file,
        target: input.target.clone(),
        files: count(baked.files.len()),
        baked: count(baked.files.iter().filter(|f| !f.kinds.is_empty()).count()),
        kinds: bake.kinds().iter().map(|&k| k.to_owned()).collect(),
        source_bytes: baked.source_bytes() as u64,
        pack_bytes: baked.bytes.len() as u64,
        bake_us,
        load_sources_us: load_time(&bake, &formats, &bundle, None),
        load_pack_us: load_time(&bake, &formats, &bundle, Some(&baked.bytes)),
    };
    let output = BakeOutput {
        errors: Vec::new(),
        pack: Some(pack),
    };
    let limit = "times are of this machine and this build: a release build, or the web build, \
                 loads at its own speed";
    Answer::new(Op::Bake, true, output, vec![limit.to_owned()])
}

/// `check --baked`'s part: every way the pack `game` bakes (for no
/// target) differs from its sources ([`Bake::check`]), or why it could
/// not be baked.
pub fn check_baked(game: &dyn GameTools) -> Result<Vec<ContentError>, String> {
    let GameBake {
        bake,
        bundle,
        formats,
    } = prepared(game, None)?;
    let baked = match bake.bake(&formats, &bundle, None) {
        Ok(baked) => baked,
        Err(errors) => return Ok(errors),
    };
    let pack = opened(&baked.bytes, bake.stamp())?;
    Ok(bake.check(&formats, &bundle, &pack))
}

/// Reads `bake`'s arguments (not `--json`).
pub fn parse_args(args: &[String]) -> Result<BakeInput, String> {
    let mut input = BakeInput::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--target" => input.target = Some(value()?),
            "--out" => input.out = Some(value()?),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(input)
}

/// `bake`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!("usage: {command} bake [--target <id>] [--out <file>] [--json]")
}

/// `n` bytes, with a thousands separator.
fn bytes(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `us` microseconds, as milliseconds with a decimal.
fn ms(us: u64) -> String {
    format!("{}.{} ms", us / 1000, us % 1000 / 100)
}

/// What `bake` made, for people.
pub fn text(output: &BakeOutput) -> String {
    let mut out = String::new();
    if !output.errors.is_empty() {
        let _ = writeln!(
            out,
            "bake: {} error(s) in the sources; no pack made",
            output.errors.len()
        );
        for error in &output.errors {
            let _ = writeln!(out, "  {error}");
        }
        out.push_str(&codes::help_lines(&output.errors));
    }
    if let Some(pack) = &output.pack {
        let target = pack
            .target
            .as_ref()
            .map(|t| format!(" for {t}"))
            .unwrap_or_default();
        let percent = (pack.pack_bytes * 100)
            .checked_div(pack.source_bytes)
            .unwrap_or(100);
        // Writing to a `String` can't fail.
        let _ = writeln!(
            out,
            "bake: {}{target}: {} files, {} baked ({})",
            pack.file,
            pack.files,
            pack.baked,
            pack.kinds.join(", ")
        );
        let _ = writeln!(
            out,
            "  size: sources {} bytes, pack {} bytes ({percent}%)",
            bytes(pack.source_bytes),
            bytes(pack.pack_bytes)
        );
        let _ = writeln!(out, "  baked in {}", ms(pack.bake_us));
        let _ = writeln!(
            out,
            "  load: sources {}, pack {}",
            ms(pack.load_sources_us),
            ms(pack.load_pack_us)
        );
    }
    out
}

#[cfg(test)]
mod tests;
