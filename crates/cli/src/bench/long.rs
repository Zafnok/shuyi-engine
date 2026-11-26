//! `bench --long`: plays a game's long scenarios for many frames and
//! checks that its memory and the sounds it keeps alive stay flat, and,
//! with `--twice`, that a second run hashes every sampled frame the same
//! (ADR-0146).
//!
//! A **long scenario** is a `*.long.ron` file anywhere in the game's
//! folder, found as `bench` finds its scenarios ([`LongScenario`]). Its
//! script plays `repeat` times, each followed by `idle` frames with no
//! input. At the end of the first step that reaches each `sample_every`
//! frames, the run records a [`Sample`]: the live heap, each memory tag's
//! live bytes, the sounds alive and the frame's hash. The driver keeps no log of the frames
//! ([`Session::set_long`]), and the samples go to the file as they are
//! taken, so neither grows the heap measured.
//!
//! **The flatness rule** ([`judge`]): past `warm_up`, the samples are
//! split in quarters (each a quarter of their count, rounded down, at
//! least one), and the run fails when the most live heap of the last
//! quarter is over the first quarter's most by more than
//! `slack.heap_bytes`, or the most sounds alive of the last quarter are
//! over the first quarter's most by more than `slack.sounds`.
//!
//! **The determinism rule** ([`first_difference`]): with `--twice`, the
//! scenario plays again in a second process, and the run fails at the
//! first sample whose frame or hash differs.

use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead as _, BufReader, BufWriter, Write as _};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::contract::codes;
use crate::contract::session::{Session, measured_start};
use crate::frame_png::Script;
use crate::game::{GameArg, GameTools, check_start, code_of};

use super::Failed;

#[cfg(test)]
mod tests;

/// The end of a long scenario's file name.
pub(crate) const SUFFIX: &str = ".long.ron";

/// The long scenario format this `shuyi` reads.
pub(crate) const FORMAT_VERSION: u32 = 1;

/// The long report's file names, in `--out`.
pub(crate) const REPORT_JSON: &str = "long-report.json";
/// The long report's table.
pub(crate) const REPORT_MD: &str = "long-report.md";

/// A long scenario: what a `*.long.ron` file holds.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LongScenario {
    /// The file's format: [`FORMAT_VERSION`].
    pub format_version: u32,
    /// What it plays, for the report.
    #[serde(default)]
    pub about: String,
    /// The game's start options, as a bench scenario's.
    #[serde(default)]
    pub start: Vec<GameArg>,
    /// The steps played each time, in `run`'s script form.
    #[serde(default)]
    pub script: Script,
    /// How many times the script plays, 1 or more.
    pub repeat: u32,
    /// Frames with no input after each time.
    #[serde(default)]
    pub idle: u32,
    /// The first frames, not judged.
    #[serde(default)]
    pub warm_up: u32,
    /// Frames between samples, 1 or more.
    pub sample_every: u32,
    /// How much the last quarter may be over the first.
    #[serde(default)]
    pub slack: Slack,
}

/// How much the last quarter's most may be over the first's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct Slack {
    /// Live heap bytes: 64 KiB unless given.
    pub heap_bytes: u64,
    /// Sounds alive: none unless given.
    pub sounds: u32,
}

impl Default for Slack {
    fn default() -> Self {
        Self {
            heap_bytes: 64 * 1024,
            sounds: 0,
        }
    }
}

impl LongScenario {
    /// Reads `text`, the long scenario shown as `file`; a file that doesn't
    /// read is an error naming the file and what is wrong.
    pub(crate) fn read(file: &str, text: &str) -> Result<Self, String> {
        let options = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        let scenario: Self = options.from_str(text).map_err(|e| format!("{file}:{e}"))?;
        if scenario.format_version != FORMAT_VERSION {
            return Err(format!(
                "{file}: format_version {}: this shuyi reads {FORMAT_VERSION}",
                scenario.format_version
            ));
        }
        scenario
            .script
            .check()
            .map_err(|e| format!("{file}: script: {e}"))?;
        if scenario.repeat == 0 {
            return Err(format!("{file}: repeat: 1 or more"));
        }
        if scenario.sample_every == 0 {
            return Err(format!("{file}: sample_every: 1 or more"));
        }
        Ok(scenario)
    }
}

/// A long scenario's name: its file's, without [`SUFFIX`].
pub(crate) fn name_of(file: &str) -> &str {
    let base = file.rsplit('/').next().unwrap_or(file);
    base.strip_suffix(SUFFIX).unwrap_or(base)
}

/// What a long run read at one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sample {
    /// The frame it was read after: the first frame the game runs is 1.
    pub frame: u32,
    /// The heap's live bytes.
    pub live_heap_bytes: u64,
    /// Each memory tag's live bytes, by name.
    pub tags: Vec<(String, u64)>,
    /// The sounds started with a handle and alive.
    pub sounds_alive: u32,
    /// The frame's hash, if frames are hashed.
    pub hash: Option<u64>,
}

/// One line of what `bench-play --long` writes: a sample, or the end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Line {
    /// A sample, as it was taken.
    Sample(Sample),
    /// The run's end.
    End {
        /// The frames it ran.
        frames: u32,
        /// Whether the heap read is this program's own counter's.
        counts_heap: bool,
        /// What the run can't promise.
        limits: Vec<String>,
    },
}

/// What one long run played: its samples and how it ended.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LongPlayed {
    /// The samples, in order.
    pub samples: Vec<Sample>,
    /// The frames it ran.
    pub frames: u32,
    /// Whether the heap read is the program's own counter's.
    pub counts_heap: bool,
    /// What the run can't promise.
    pub limits: Vec<String>,
}

/// Reads what `bench-play --long` wrote to `path`.
pub(crate) fn read_played(path: &Path) -> Result<LongPlayed, String> {
    let file = fs::File::open(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let mut played = LongPlayed::default();
    let mut ended = false;
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| format!("reading {}: {e}", path.display()))?;
        match serde_json::from_str(&line)
            .map_err(|e| format!("{}:{}: {e}", path.display(), i + 1))?
        {
            Line::Sample(sample) => played.samples.push(sample),
            Line::End {
                frames,
                counts_heap,
                limits,
            } => {
                played.frames = frames;
                played.counts_heap = counts_heap;
                played.limits = limits;
                ended = true;
            }
        }
    }
    if !ended {
        return Err(format!("{}: the run didn't end", path.display()));
    }
    Ok(played)
}

/// Plays long scenario `file` (from `root`) in `game` and writes each
/// sample to `out` as it is taken, then the end.
pub(crate) fn play(
    root: &Path,
    file: &str,
    game: &dyn GameTools,
    out: &Path,
) -> Result<(), Failed> {
    let text =
        fs::read_to_string(root.join(file)).map_err(|e| (codes::FILE, format!("{file}: {e}")))?;
    let scenario = LongScenario::read(file, &text).map_err(|e| (codes::FILE, e))?;
    check_start(game.frame_options(), &scenario.start)
        .map_err(|e| (codes::FILE, format!("{file}: start: {e}")))?;
    let mut session =
        measured_start(|| game.session(&scenario.start)).map_err(|e| (code_of(&e), e))?;
    session.set_perf(true);
    session.set_long();
    let written = fs::File::create(out)
        .map_err(|e| (codes::FILE, format!("writing {}: {e}", out.display())))?;
    let mut sampler = Sampler {
        out: BufWriter::new(written),
        every: scenario.sample_every,
        next: scenario.sample_every,
    };
    let write = |e: std::io::Error| (codes::FILE, format!("writing {}: {e}", out.display()));
    // One step at a time, so the samples fall all through the script, not
    // only at its end: made once, before the frames measured.
    let steps: Vec<Script> = scenario
        .script
        .steps
        .iter()
        .map(|step| Script::new(vec![step.clone()]))
        .collect();
    for _ in 0..scenario.repeat {
        for step in &steps {
            session.play(step);
            sampler.take(session.as_mut()).map_err(write)?;
        }
        for _ in 0..scenario.idle {
            session.idle(1);
            sampler.take(session.as_mut()).map_err(write)?;
        }
    }
    let mut limits = Vec::new();
    let counts_heap = session.counts_process_heap();
    if !counts_heap {
        limits.push(
            "the heap read is the game's own (a fake, or none): not this program's".to_owned(),
        );
    }
    let end = Line::End {
        frames: session.frame_count(),
        counts_heap,
        limits,
    };
    sampler.write(&end).map_err(write)?;
    sampler.out.flush().map_err(write)
}

/// Takes the samples of a long run as its frames pass.
struct Sampler<W> {
    /// Where they go, a line each.
    out: W,
    /// Frames between samples.
    every: u32,
    /// The frame the next is due at.
    next: u32,
}

impl<W: std::io::Write> Sampler<W> {
    /// Takes a sample of `session` if one is due: once for each run of
    /// frames that passed the frame due, at the frame it has reached.
    fn take(&mut self, session: &mut dyn Session) -> std::io::Result<()> {
        let frame = session.frame_count();
        if frame < self.next {
            return Ok(());
        }
        while self.next <= frame {
            self.next = self.next.saturating_add(self.every);
        }
        let snapshot = session.heap_snapshot().unwrap_or_default();
        let sample = Sample {
            frame,
            live_heap_bytes: snapshot.live_bytes,
            tags: snapshot
                .tags
                .iter()
                .map(|row| (row.name.to_string(), row.live_bytes))
                .collect(),
            sounds_alive: session.sounds_alive(),
            hash: session.frame_hash().map(|hash| hash.0),
        };
        self.write(&Line::Sample(sample))
    }

    /// Writes `line`.
    fn write(&mut self, line: &Line) -> std::io::Result<()> {
        serde_json::to_writer(&mut self.out, line)?;
        self.out.write_all(b"\n")
    }
}

/// Why a run broke the flatness rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Verdict {
    /// Samples judged, past the warm-up.
    pub judged: usize,
    /// The most live heap of the first quarter and of the last.
    pub heap: (u64, u64),
    /// The most sounds alive of the first quarter and of the last.
    pub sounds: (u32, u32),
    /// Whether the heap grew past its slack.
    pub heap_grew: bool,
    /// Whether the sounds alive grew past their slack.
    pub sounds_grew: bool,
    /// The three memory tags whose most live bytes grew most from the
    /// first quarter to the last, with how much.
    pub grew_most: Vec<(String, u64)>,
}

/// The fewest samples past the warm-up a run is judged on.
pub(crate) const FEWEST: usize = 4;

/// Judges `samples` by the flatness rule, those of frame `warm_up` or
/// before left out, with `slack`. Fewer than [`FEWEST`] judged is an
/// error.
pub(crate) fn judge(samples: &[Sample], warm_up: u32, slack: Slack) -> Result<Verdict, String> {
    let judged: Vec<&Sample> = samples.iter().filter(|s| s.frame > warm_up).collect();
    if judged.len() < FEWEST {
        return Err(format!(
            "{} sample(s) after the warm-up of {warm_up} frames: it judges {FEWEST} or more \
             (more repeats, or a smaller sample_every)",
            judged.len()
        ));
    }
    let quarter = judged.len() / 4;
    let (first, last) = (&judged[..quarter], &judged[judged.len() - quarter..]);
    let most_heap = |part: &[&Sample]| part.iter().map(|s| s.live_heap_bytes).max().unwrap_or(0);
    let most_sounds = |part: &[&Sample]| part.iter().map(|s| s.sounds_alive).max().unwrap_or(0);
    let heap = (most_heap(first), most_heap(last));
    let sounds = (most_sounds(first), most_sounds(last));
    let most_of = |part: &[&Sample], tag: &str| {
        part.iter()
            .flat_map(|s| &s.tags)
            .filter(|(name, _)| name == tag)
            .map(|(_, bytes)| *bytes)
            .max()
            .unwrap_or(0)
    };
    let mut names: Vec<&str> = last
        .iter()
        .flat_map(|s| &s.tags)
        .map(|(name, _)| name.as_str())
        .collect();
    names.sort_unstable();
    names.dedup();
    let mut grew: Vec<(String, u64)> = names
        .into_iter()
        .map(|name| {
            let growth = most_of(last, name).saturating_sub(most_of(first, name));
            (name.to_owned(), growth)
        })
        .filter(|(_, growth)| *growth > 0)
        .collect();
    grew.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    grew.truncate(3);
    Ok(Verdict {
        judged: judged.len(),
        heap,
        sounds,
        heap_grew: heap.1 > heap.0.saturating_add(slack.heap_bytes),
        sounds_grew: sounds.1 > sounds.0.saturating_add(slack.sounds),
        grew_most: grew,
    })
}

/// The first sample at which runs `a` and `b` differ, by its frame in
/// `a` (or in `b`, past `a`'s last): a frame, a hash, or one run's samples
/// ending before the other's.
pub(crate) fn first_difference(a: &[Sample], b: &[Sample]) -> Option<u32> {
    let differs = a
        .iter()
        .zip(b)
        .find(|(x, y)| x.frame != y.frame || x.hash != y.hash)
        .map(|(x, _)| x.frame);
    differs.or_else(|| {
        let shared = a.len().min(b.len());
        a.get(shared).or_else(|| b.get(shared)).map(|s| s.frame)
    })
}

/// One long scenario's result, in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LongRun {
    /// Its name.
    pub scenario: String,
    /// Its file, from the game's folder.
    pub file: String,
    /// What it plays.
    pub about: String,
    /// The frames it ran.
    pub frames: u32,
    /// Its slack.
    pub slack: Slack,
    /// The flatness rule's numbers, if there were samples enough.
    pub verdict: Option<Verdict>,
    /// Whether a second run was played and compared.
    pub twice: bool,
    /// The frame of the first sample at which the second run differed
    /// ([`first_difference`]), if one did.
    pub differs_at: Option<u32>,
    /// Why it fails, each a sentence; none when it passes.
    pub failures: Vec<String>,
    /// What the run can't promise.
    pub limits: Vec<String>,
    /// The first run's samples.
    pub samples: Vec<Sample>,
}

/// What `bench --long` writes as `long-report.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LongReport {
    /// The report's format: [`FORMAT_VERSION`].
    pub format_version: u32,
    /// Each scenario's run.
    pub runs: Vec<LongRun>,
}

impl LongReport {
    /// Whether any run failed.
    pub(crate) fn failed(&self) -> bool {
        self.runs.iter().any(|run| !run.failures.is_empty())
    }
}

/// The run of `scenario` from `file`: `first` judged, and `second`
/// compared with it if played.
pub(crate) fn run(
    file: &str,
    scenario: &LongScenario,
    first: LongPlayed,
    second: Option<&LongPlayed>,
) -> LongRun {
    let name = name_of(file);
    let mut failures = Vec::new();
    let verdict = match judge(&first.samples, scenario.warm_up, scenario.slack) {
        Ok(verdict) => Some(verdict),
        Err(e) => {
            failures.push(format!("{name}: {e}"));
            None
        }
    };
    if let Some(verdict) = &verdict {
        failures.extend(flatness(name, verdict, scenario.slack));
    }
    let differs_at = second.and_then(|second| first_difference(&first.samples, &second.samples));
    if let Some(frame) = differs_at {
        failures.push(format!(
            "{name}: the second run differs from the first at frame {frame}: the game isn't \
             deterministic over a long run"
        ));
    }
    if second.is_some() && first.samples.iter().any(|s| s.hash.is_none()) {
        failures.push(format!(
            "{name}: its frames weren't hashed, so the runs can't be compared"
        ));
    }
    LongRun {
        scenario: name.to_owned(),
        file: file.to_owned(),
        about: scenario.about.clone(),
        frames: first.frames,
        slack: scenario.slack,
        verdict,
        twice: second.is_some(),
        differs_at,
        failures,
        limits: first.limits,
        samples: first.samples,
    }
}

/// The sentences saying how `verdict` of scenario `name` broke the
/// flatness rule with `slack`.
fn flatness(name: &str, verdict: &Verdict, slack: Slack) -> Vec<String> {
    let mut said = Vec::new();
    if verdict.heap_grew {
        let tags: Vec<String> = verdict
            .grew_most
            .iter()
            .map(|(tag, bytes)| format!("{tag} +{}", super::report::grouped(*bytes)))
            .collect();
        let tags = if tags.is_empty() {
            String::new()
        } else {
            format!(": grew most under {}", tags.join(", "))
        };
        said.push(format!(
            "{name}: the live heap grew from {} bytes (the most of the first quarter) to {} \
             (the most of the last), over the slack of {}{tags}",
            super::report::grouped(verdict.heap.0),
            super::report::grouped(verdict.heap.1),
            super::report::grouped(slack.heap_bytes),
        ));
    }
    if verdict.sounds_grew {
        said.push(format!(
            "{name}: the sounds alive grew from {} (the most of the first quarter) to {} (the \
             most of the last), over the slack of {}",
            verdict.sounds.0, verdict.sounds.1, slack.sounds
        ));
    }
    said
}

/// The report's table, for a pull request or a job's summary.
pub(crate) fn markdown(report: &LongReport) -> String {
    let grouped = super::report::grouped;
    let mut out = String::from(
        "## Long runs\n\n\
         | Scenario | Frames | Samples | Live heap, first quarter | Live heap, last quarter \
         | Sounds alive | Second run | Result |\n\
         |---|---:|---:|---:|---:|---:|---|---|\n",
    );
    for run in &report.runs {
        let (judged, heap0, heap1, sounds) = run.verdict.as_ref().map_or_else(
            || {
                (
                    "-".to_owned(),
                    "-".to_owned(),
                    "-".to_owned(),
                    "-".to_owned(),
                )
            },
            |v| {
                (
                    v.judged.to_string(),
                    grouped(v.heap.0),
                    grouped(v.heap.1),
                    format!("{} → {}", v.sounds.0, v.sounds.1),
                )
            },
        );
        let twice = match (run.twice, run.differs_at) {
            (false, _) => "not played".to_owned(),
            (true, None) => "same hashes".to_owned(),
            (true, Some(frame)) => format!("differs at frame {frame}"),
        };
        let result = if run.failures.is_empty() {
            "flat"
        } else {
            "**fails**"
        };
        let _ = writeln!(
            out,
            "| {} | {} | {judged} | {heap0} | {heap1} | {sounds} | {twice} | {result} |",
            run.scenario,
            grouped(u64::from(run.frames)),
        );
    }
    let failures: Vec<&String> = report.runs.iter().flat_map(|r| &r.failures).collect();
    if !failures.is_empty() {
        out.push_str("\n### Failures\n\n");
        for failure in failures {
            let _ = writeln!(out, "- {failure}");
        }
    }
    let limits: Vec<String> = report
        .runs
        .iter()
        .flat_map(|r| r.limits.iter().map(move |l| format!("{}: {l}", r.scenario)))
        .collect();
    if !limits.is_empty() {
        out.push_str("\n### Not measured\n\n");
        for limit in limits {
            let _ = writeln!(out, "- {limit}");
        }
    }
    out
}
