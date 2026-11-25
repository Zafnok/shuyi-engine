//! `sfx <preset | file.sfx.ron> [--seed N] [--out <file.sfx.ron>] [--wav
//! <file.wav>]`: a sound effect (`.sfx.ron`, ADR-0144) started from one of
//! the engine's presets or read from a file, its numbers checked, and what
//! it renders to measured. `--out` writes its text, a game's own sound to
//! change; `--wav` writes the sound itself, for a person to listen to. It
//! needs no game, like `doctor`: a game's `files/audio.ron` names the file
//! it writes.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::sfx::{self, SfxDef};

use super::{Answer, Op, Problem, codes};

/// What `sfx` makes or reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct SfxInput {
    /// A preset's name (`coin`; the answer lists them), or a `.sfx.ron`
    /// file, from the working folder.
    pub source: String,
    /// The preset's variation: 0, or none, is the preset as written;
    /// another seed moves its pitches by up to 10 % and its `hold_ms` by up
    /// to 20 %. Only for a preset.
    #[serde(default)]
    pub seed: Option<u64>,
    /// The `.sfx.ron` file to write the sound's text to, from the working
    /// folder; its folder is made if it is missing.
    #[serde(default)]
    pub out: Option<String>,
    /// The WAV file to write the rendered sound to, from the working
    /// folder, to listen to it.
    #[serde(default)]
    pub wav: Option<String>,
}

/// What `sfx` answers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SfxOutput {
    /// Every preset's name.
    pub presets: Vec<String>,
    /// The sound's text, as `out` writes it: a preset's with a line naming
    /// it, a file's in the form the engine writes. A file with problems
    /// gives its own text.
    pub text: String,
    /// How long it plays, in milliseconds; 0 with problems.
    pub length_ms: u64,
    /// How many samples it renders to, mono at 44 100 a second; 0 with
    /// problems.
    pub samples: u64,
    /// Its loudest sample, 0 to 32 767; 0 with problems.
    pub peak: u16,
    /// Every problem with a file's numbers, each with its line: none
    /// passes. Nothing is written when there is one.
    pub problems: Vec<Problem>,
    /// The files written (`out`, then `wav`), as given.
    pub written: Vec<String>,
}

/// Every preset's name.
fn presets() -> Vec<String> {
    sfx::PRESETS.iter().map(|&name| name.to_owned()).collect()
}

/// Whether `source` names a file rather than a preset: it ends in `.ron`.
fn is_file(source: &str) -> bool {
    Path::new(source)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ron"))
}

/// `sfx`: the sound `input` names, from the working folder `cwd`, measured
/// and written where it says.
pub fn sfx(input: &SfxInput, cwd: &Path) -> Answer<SfxOutput> {
    let failed = |code, error: String| Answer::failed(Op::Sfx, code, error);
    let source = input.source.as_str();
    let (def, text) = if is_file(source) {
        if input.seed.is_some() {
            let error = "--seed varies a preset; a file's own `seed` is its noise's";
            return failed(codes::WRONG_ARGUMENTS, error.to_owned());
        }
        let read = match fs::read_to_string(cwd.join(source)) {
            Ok(read) => read,
            Err(e) => return failed(codes::FILE, format!("{source}: {e}")),
        };
        match sfx::from_source(source, &read) {
            Ok(def) => {
                let text = sfx::to_text(&def);
                (def, text)
            }
            Err(errors) => {
                let output = SfxOutput {
                    presets: presets(),
                    text: read,
                    problems: errors.iter().map(Problem::from).collect(),
                    ..SfxOutput::default()
                };
                return Answer::new(Op::Sfx, false, output, Vec::new());
            }
        }
    } else {
        let seed = input.seed.unwrap_or(0);
        let Some(def) = sfx::preset(source, seed) else {
            let names = sfx::PRESETS.join(", ");
            let error = format!("no preset {source}: the presets are {names}");
            return failed(codes::WRONG_ARGUMENTS, error).with_help(
                "name a preset, or a .sfx.ron file (`sfx coin`, `sfx files/sfx/coin.sfx.ron`)",
            );
        };
        let made = if seed == 0 {
            format!("// The engine's preset `{source}` (`shuyi sfx {source}`).\n")
        } else {
            format!(
                "// The engine's preset `{source}`, seed {seed} (`shuyi sfx {source} --seed {seed}`).\n"
            )
        };
        (def.clone(), made + &sfx::to_text(&def))
    };
    measured(def, text, input, cwd)
}

/// The answer for `def`, whose text is `text`: measured, and written where
/// `input` says, from `cwd`.
fn measured(def: SfxDef, text: String, input: &SfxInput, cwd: &Path) -> Answer<SfxOutput> {
    let samples = sfx::render(&def);
    let mut output = SfxOutput {
        presets: presets(),
        length_ms: def.length_ms(),
        samples: u64::try_from(samples.len()).unwrap_or(u64::MAX),
        peak: samples.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0),
        text,
        ..SfxOutput::default()
    };
    let wav = input.wav.as_ref().map(|wav| (wav, sfx::to_wav(&samples)));
    let files = input
        .out
        .as_ref()
        .map(|out| (out, output.text.clone().into_bytes()))
        .into_iter()
        .chain(wav);
    for (file, bytes) in files {
        let path = cwd.join(file);
        let wrote = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&path, bytes));
        if let Err(e) = wrote {
            let failed = Answer::new(Op::Sfx, false, output, Vec::new());
            let mut failed = failed.with_code(codes::FILE);
            failed.error = Some(format!("{file}: {e}"));
            return failed;
        }
        output.written.push(file.clone());
    }
    Answer::new(Op::Sfx, true, output, Vec::new())
}

/// Reads `sfx`'s arguments (not `--json`): a preset or a file, then
/// `--seed`, `--out` and `--wav`, each once, in any order.
pub fn parse_args(args: &[String]) -> Result<SfxInput, String> {
    let mut input = SfxInput::default();
    let mut source = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| {
            args.next()
                .filter(|value| !value.starts_with("--"))
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        let twice = |flag: &str| format!("{flag} given twice");
        match arg.as_str() {
            "--seed" => {
                let seed = value("--seed")?;
                let seed = seed
                    .parse()
                    .map_err(|_| format!("--seed: a whole number, not {seed}"))?;
                if input.seed.replace(seed).is_some() {
                    return Err(twice("--seed"));
                }
            }
            "--out" => {
                if input.out.replace(value("--out")?).is_some() {
                    return Err(twice("--out"));
                }
            }
            "--wav" => {
                if input.wav.replace(value("--wav")?).is_some() {
                    return Err(twice("--wav"));
                }
            }
            flag if flag.starts_with("--") => return Err(format!("no flag {flag}")),
            name => {
                if source.replace(name.to_owned()).is_some() {
                    return Err("one preset or file".to_owned());
                }
            }
        }
    }
    input.source = source.ok_or("a preset or a .sfx.ron file")?;
    Ok(input)
}

/// `sfx`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    let presets = sfx::PRESETS.join(", ");
    format!(
        "usage: {command} sfx <preset | file.sfx.ron> [--seed N] [--out <file.sfx.ron>] \
         [--wav <file.wav>] [--json]\n\n\
         A sound effect from a preset or a file: its text, how long it plays and how loud.\n\
         --seed varies a preset; --out writes its text, a sound of the game's own to change\n\
         (files/audio.ron names it); --wav writes the sound, to listen to it.\n\
         The presets: {presets}."
    )
}

/// What `sfx` found, for people: the problems, or the text and its
/// measures, then the files written.
pub fn text(output: &SfxOutput) -> String {
    let mut out = String::new();
    if output.problems.is_empty() {
        out.push_str(&output.text);
        // Writing to a `String` can't fail.
        let _ = writeln!(
            out,
            "{} ms, {} samples, peak {}",
            output.length_ms, output.samples, output.peak
        );
    } else {
        for problem in &output.problems {
            let _ = writeln!(out, "{problem}");
        }
    }
    for file in &output.written {
        let _ = writeln!(out, "wrote {file}");
    }
    out
}

#[cfg(test)]
mod tests;
