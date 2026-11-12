//! `bake [--target <id>] [--out <file>] [--sheet <file>]`: the game's
//! sources baked into one pack (`.shuyipak`, ADR-0023), with what went
//! in, the sizes, and how long baking and loading took. A source with
//! errors is refused, with every error. `check --baked` uses the same
//! bake to prove the pack equal to its sources ([`check_baked`]).
//!
//! The game says what it bakes for a target, from which bundle and with
//! which formats ([`GameTools::bake`]). A target must have a profile, the
//! engine's or the game's ([`target::profiles`]); the game picks its
//! layers and kinds by it. For a one-bit target it bakes its pictures as
//! one-bit picture files (`Bake::under_as`), from the target's own layer
//! (`Bundle::for_target`) where it has one: the answer names the pictures
//! dithered, for an artist to check, and those its layer replaced, and
//! `--sheet` draws each dithered one beside its source ([`sheet`]).
//!
//! `--content <folder>` bakes a folder of the project in place of the files
//! the game's tools embed, as an edition's export does (ADR-0100, ADR-0101):
//! the game's tool program is built with the folder as `SHUYI_CONTENT`, in a
//! Cargo profile of its own ([`CONTENT_PROFILE`]), and bakes in its own
//! process. The game's code says what the folder replaces, as it does for
//! the program it ships.
//!
//! `--check` bakes nothing: it reads the pack at `--out` and proves it equal
//! to the sources ([`Bake::check`]), every difference an error naming its
//! file; with `--content`, against the content root's files. A release export
//! that ships a pack runs it on the pack it ships (ADR-0109).
//!
//! `--audio <folder>` also writes the target's sounds and music there, baked
//! as the target plays them ([`GameTools::audio`]; the Playdate's form,
//! [`playdate::audio`]), by the same program: with a content root, the one
//! built with it (ADR-0102).

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::bitmap::OneBitPicture;
use shuyi_assets::bundle::{Bundle, Layer};
use shuyi_assets::format::{AssetKind, Formats};
use shuyi_assets::image::PNG_SIGNATURE;
use shuyi_assets::pack::{Bake, Baked, Pack};

use super::{Answer, Op, Problem, codes, target};
use crate::export::CONTENT;
use crate::export::playdate;
use crate::game::{GameBake, GameTools, code_of, is_unsupported};
use crate::project::{self, GameProgram, Project};

pub mod sheet;

/// Where a pack goes when `--out` doesn't say, from the project's root:
/// `target/shuyi/game.shuyipak`, or `<target>.shuyipak` for a target.
pub const OUT_DIR: &str = "target/shuyi";

/// How many times each load is timed: the fastest counts.
pub const LOAD_RUNS: u32 = 5;

/// The Cargo profile the game's tool program is built in to bake a content
/// root ([`BakeInput::content`]): the `dev` profile's settings in a folder of
/// its own, so the tool program running the bake isn't rebuilt under itself,
/// and its own build isn't undone for the next command.
pub const CONTENT_PROFILE: &str = "shuyi-content";

/// What `bake` bakes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct BakeInput {
    /// The target to bake for, by its profile's id, passed to the game;
    /// none for its default bundle.
    #[serde(default)]
    pub target: Option<String>,
    /// The pack's file, from the project's root; `target/shuyi/game.shuyipak`
    /// (or `<target>.shuyipak`) if none.
    #[serde(default)]
    pub out: Option<String>,
    /// A contact sheet to write, from the project's root: a PNG of every
    /// picture the bake dithered, each beside its source, for review.
    #[serde(default)]
    pub sheet: Option<String>,
    /// A folder of the project, from its root with `/`, to bake in place of
    /// the files the game's tools embed: an edition's content root. The
    /// game's tool program is built with it as `SHUYI_CONTENT` and bakes;
    /// none bakes the files of the tools running now.
    #[serde(default)]
    pub content: Option<String>,
    /// A folder to write the target's sounds and music into, from the
    /// project's root: each sound file and music track the game ships on
    /// `target` (`GameTools::audio`), baked as the target plays them, under
    /// `audio/` and `music/`. It is emptied first, and not made when the
    /// game's tools offer no audio. Needs a target. With `content`, the tool
    /// program built with that folder writes it.
    #[serde(default)]
    pub audio: Option<String>,
    /// Bake nothing: read the pack at `out` (its default file without one)
    /// and prove it equal to the sources, as `Bake::check` does. Every
    /// difference is an error of the answer, naming its file, and no pack is
    /// made. With `content`, the sources are that folder's.
    #[serde(default)]
    pub check: bool,
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
#[non_exhaustive]
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
    /// The pictures made one-bit by dithering a PNG of the game's own
    /// layers, as messages show them: what an artist checks.
    #[serde(default)]
    pub dithered: Vec<String>,
    /// The pictures read from the target's own layer
    /// (`assets@<target>/`) in place of the game's.
    #[serde(default)]
    pub replaced: Vec<String>,
    /// The contact sheet written, from the project's root, if one was
    /// asked for.
    #[serde(default)]
    pub sheet: Option<String>,
    /// How many sound and music files were written to the audio folder
    /// (`--audio`); none if it wasn't asked for, or the game's tools offer no
    /// audio.
    #[serde(default)]
    pub audio_files: Option<u32>,
}

/// The pictures of `baked`, baked from `bundle` for `target` as one-bit
/// pictures: those dithered from a PNG of a layer that isn't the
/// target's, and those read from the target's layer, each by path.
fn one_bit_pictures(
    baked: &Baked,
    bundle: &Bundle,
    target: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let mut dithered = Vec::new();
    let mut replaced = Vec::new();
    for file in &baked.files {
        if !file.kinds.contains(&OneBitPicture::NAME) {
            continue;
        }
        let layer = bundle.layer_of(&file.path).and_then(Layer::target);
        if target.is_some() && layer == target {
            replaced.push(file.path.clone());
        } else if bundle
            .bytes(&file.path)
            .is_some_and(|bytes| bytes.starts_with(PNG_SIGNATURE))
        {
            dithered.push(file.path.clone());
        }
    }
    (dithered, replaced)
}

/// `duration` in whole microseconds.
fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

/// The fastest of [`LOAD_RUNS`] runs of `run`.
fn fastest(mut run: impl FnMut()) -> u64 {
    fastest_of(|| {
        let start = Instant::now();
        run();
        start.elapsed()
    })
}

/// The smallest of [`LOAD_RUNS`] durations `measure` gives, in whole
/// microseconds.
fn fastest_of(mut measure: impl FnMut() -> Duration) -> u64 {
    (0..LOAD_RUNS)
        .map(|_| micros(measure()))
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

/// The absolute path of `content`, a folder of the project at `root` written
/// from it with `/`; or why it isn't one.
pub fn content_folder(root: &Path, content: &str) -> Result<PathBuf, String> {
    let outside = content.starts_with('/')
        || content.contains([':', '\\'])
        || content
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."));
    if outside {
        return Err(format!(
            "the content root `{content}`: write it from the project's root, with `/`, and no \
             empty part, `.` or `..`"
        ));
    }
    let folder = root.join(content);
    if !folder.is_dir() {
        return Err(format!(
            "the content root `{content}` isn't a folder of the project"
        ));
    }
    Ok(std::path::absolute(&folder).unwrap_or(folder))
}

/// The arguments of the game's tool program's own `bake` for `input`, the
/// pack written to `out`: all of `input` but the content root, which the
/// program is built with.
fn content_args(input: &BakeInput, out: &str) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(target) = &input.target {
        args.extend(["--target".to_owned(), target.clone()]);
    }
    args.extend(["--out".to_owned(), out.to_owned()]);
    if let Some(sheet) = &input.sheet {
        args.extend(["--sheet".to_owned(), sheet.clone()]);
    }
    if let Some(audio) = &input.audio {
        args.extend(["--audio".to_owned(), audio.clone()]);
    }
    if input.check {
        args.push("--check".to_owned());
    }
    args.push("--json".to_owned());
    args
}

/// The `cargo` arguments that run `program`, the game's tool program, built
/// in [`CONTENT_PROFILE`], for its `bake` of `input` into `out`.
fn content_cargo_args(program: &GameProgram, input: &BakeInput, out: &str) -> Vec<String> {
    let mut cargo = program.cargo_args("bake", &content_args(input, out));
    let profile = [
        "--profile".to_owned(),
        CONTENT_PROFILE.to_owned(),
        "--config".to_owned(),
        format!("profile.{CONTENT_PROFILE}.inherits=\"dev\""),
    ];
    cargo.splice(1..1, profile);
    cargo
}

/// `bake` of `input`, whose content root is `content`: the answer of the
/// game's tool program of `project` built with the folder as [`CONTENT`] (in
/// [`CONTENT_PROFILE`]), which names the folder in its limits.
fn bake_content(project: &Project, content: &str, input: &BakeInput) -> Answer<BakeOutput> {
    let folder = match content_folder(&project.root, content) {
        Ok(folder) => folder,
        Err(e) => return Answer::failed(Op::Bake, codes::FILE, e),
    };
    let Some(program) = &project.config.game else {
        return Answer::failed(
            Op::Bake,
            codes::PROJECT_FILE,
            format!(
                "{} names no game tool program (`game`): a content root is baked by it, \
                 built with that folder",
                project::FILE
            ),
        );
    };
    let out = out_file(input);
    let ran = Command::new(crate::cargo())
        .args(content_cargo_args(program, input, &out))
        .current_dir(&project.root)
        .env(CONTENT, &folder)
        .stderr(Stdio::inherit())
        .output();
    let output = match ran {
        Ok(output) => output,
        Err(e) => {
            return Answer::failed(Op::Bake, codes::TOOLCHAIN, format!("cannot run cargo: {e}"));
        }
    };
    let said = String::from_utf8_lossy(&output.stdout);
    match serde_json::from_str::<Answer<BakeOutput>>(said.trim()) {
        Ok(mut answer) => {
            answer.limits.push(format!(
                "baked by the game's tool program built with `{content}` as its files \
                 ({CONTENT})"
            ));
            answer
        }
        Err(_) => Answer::failed(
            Op::Bake,
            codes::GAME_PROGRAM,
            format!(
                "the game's tool program, built with `{content}` as its files, gave no answer \
                 of a bake ({}): {}",
                output.status,
                said.trim()
            ),
        ),
    }
}

/// Where the audio bake keeps each file it made, from the project's root,
/// so a source it has baked before isn't decoded again.
fn audio_cache(project: &Project) -> PathBuf {
    playdate::audio::cache_dir(&project.root.join(OUT_DIR))
}

/// Writes the sounds and music `game` ships on `target` into `folder`, a
/// folder of `project` from its root: how many files, or none if the game's
/// tools offer no audio.
fn bake_audio(
    game: &dyn GameTools,
    project: &Project,
    target: &str,
    folder: &str,
) -> Result<Option<u32>, (codes::Code, String)> {
    let shipped = match game.audio(target) {
        Ok(shipped) => shipped,
        Err(e) if is_unsupported(&e) => return Ok(None),
        Err(e) => return Err((codes::GAME_ERROR, format!("the game's audio: {e}"))),
    };
    let file_error = |e: String| (codes::FILE, format!("the game's audio: {e}"));
    let jobs = playdate::audio::jobs(&shipped, &project.root).map_err(file_error)?;
    let into = project.root.join(folder);
    fs::create_dir_all(&into).map_err(|e| file_error(format!("{folder}: {e}")))?;
    playdate::audio::bake(&jobs, &audio_cache(project), &into).map_err(file_error)?;
    Ok(Some(u32::try_from(jobs.len()).unwrap_or(u32::MAX)))
}

/// Empties `input`'s audio folder, so what an earlier bake wrote there isn't
/// taken for this one's; or why it can't be, or `input` has no target for it.
fn clear_audio(project: &Project, input: &BakeInput) -> Result<(), (codes::Code, String)> {
    let Some(folder) = &input.audio else {
        return Ok(());
    };
    if input.target.is_none() {
        return Err((
            codes::WRONG_ARGUMENTS,
            "--audio bakes the sounds and music of a target: give --target too".to_owned(),
        ));
    }
    let path = project.root.join(folder);
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| (codes::FILE, format!("clear {folder}: {e}")))?;
    }
    Ok(())
}

/// `bake`: `game`'s pack for `input`, written under `project`'s root, and its
/// sounds and music if `input` asks; with a content root, those of the game's
/// tool program built with it.
pub fn bake(game: &dyn GameTools, project: &Project, input: &BakeInput) -> Answer<BakeOutput> {
    if let Err((code, why)) = clear_audio(project, input) {
        return Answer::failed(Op::Bake, code, why);
    }
    if let Some(content) = &input.content {
        return bake_content(project, content, input);
    }
    let target = input.target.as_deref();
    if let Some(Err((code, why))) =
        target.map(|target| target::profile(&project.config, &project.root, target))
    {
        return Answer::failed(Op::Bake, code, why);
    }
    if input.check {
        return check_file(game, project, input);
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
    let (dithered, replaced) = one_bit_pictures(&baked, &bundle, target);
    if let Some(sheet) = &input.sheet {
        let drawn = opened(&baked.bytes, bake.stamp())
            .and_then(|pack| sheet::draw(&bundle, &pack, &dithered))
            .and_then(|png| {
                let path = project.root.join(sheet);
                path.parent()
                    .map_or(Ok(()), fs::create_dir_all)
                    .and_then(|()| fs::write(&path, png))
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = drawn {
            return Answer::failed(Op::Bake, codes::FILE, format!("{sheet}: {e}"));
        }
    }
    let audio_files = match (&input.audio, target) {
        (Some(folder), Some(target)) => match bake_audio(game, project, target, folder) {
            Ok(files) => files,
            Err((code, why)) => return Answer::failed(Op::Bake, code, why),
        },
        _ => None,
    };
    let shown = |paths: Vec<String>| paths.iter().map(|p| bundle.display_path(p)).collect();
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
        dithered: shown(dithered),
        replaced: shown(replaced),
        sheet: input.sheet.clone(),
        audio_files,
    };
    let output = BakeOutput {
        errors: Vec::new(),
        pack: Some(pack),
    };
    let limit = "times are of this machine and this build: a release build, or the web build, \
                 loads at its own speed";
    Answer::new(Op::Bake, true, output, vec![limit.to_owned()])
}

/// `bake --check`: the pack at `input`'s file, read and proven equal to the
/// sources `game` bakes for `input`'s target ([`Bake::check`]). Every
/// difference is an error of the answer, which makes no pack.
fn check_file(game: &dyn GameTools, project: &Project, input: &BakeInput) -> Answer<BakeOutput> {
    let GameBake {
        bake,
        bundle,
        formats,
    } = match prepared(game, input.target.as_deref()) {
        Ok(prepared) => prepared,
        Err(e) => return Answer::failed(Op::Bake, code_of(&e), e),
    };
    let file = out_file(input);
    let pack = fs::read(project.root.join(&file))
        .map_err(|e| e.to_string())
        .and_then(|bytes| opened(&bytes, bake.stamp()));
    let pack = match pack {
        Ok(pack) => pack,
        Err(e) => return Answer::failed(Op::Bake, codes::FILE, format!("{file}: {e}")),
    };
    let errors: Vec<Problem> = bake
        .check(&formats, &bundle, &pack)
        .iter()
        .map(Problem::from)
        .collect();
    let ok = errors.is_empty();
    let output = BakeOutput { errors, pack: None };
    let limit = format!("{file} was checked against the sources, not baked");
    Answer::new(Op::Bake, ok, output, vec![limit])
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
            "--sheet" => input.sheet = Some(value()?),
            "--content" => input.content = Some(value()?),
            "--audio" => input.audio = Some(value()?),
            "--check" => input.check = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(input)
}

/// `bake`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} bake [--target <id>] [--out <file>] [--sheet <file>] \
         [--content <folder>] [--audio <folder>] [--check] [--json]"
    )
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
    if output.errors.is_empty() && output.pack.is_none() {
        out.push_str(
            "bake --check: the pack reads as its sources
",
        );
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
        if !pack.dithered.is_empty() {
            let _ = writeln!(
                out,
                "  dithered {} picture(s), to check: {}",
                pack.dithered.len(),
                pack.dithered.join(", ")
            );
        }
        if !pack.replaced.is_empty() {
            let _ = writeln!(
                out,
                "  from the target's layer: {}",
                pack.replaced.join(", ")
            );
        }
        if let Some(sheet) = &pack.sheet {
            let _ = writeln!(out, "  contact sheet: {sheet}");
        }
        if let Some(files) = pack.audio_files {
            let _ = writeln!(out, "  sounds and music: {files} file(s) baked");
        }
    }
    out
}

#[cfg(test)]
mod tests;
