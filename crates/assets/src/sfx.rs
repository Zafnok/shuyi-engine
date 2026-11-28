//! Sound effects from a few numbers (ADR-0144): a `.sfx.ron`
//! file is a wave, a pitch that may slide, step or wobble, and how the
//! loudness rises, holds and fades. The game's sound hook renders it once,
//! when it loads, to a 16-bit mono WAV at [`SAMPLE_RATE`]
//! ([`audio::playable`](crate::audio::playable)), so the player and the
//! backends see a WAV like any other.
//!
//! ```ron
//! // A coin: a short high note that jumps up a fourth.
//! (
//!     wave: Square(duty: 50),
//!     hz: 988,
//!     hold_ms: 120,
//!     fade_ms: 200,
//!     volume: 45,
//!     step: (at_ms: 60, percent: 133),
//! )
//! ```
//!
//! The rendering is integers only, with no table: the same file gives the
//! same bytes on every platform and without `std`, so a sound's hash is a
//! golden row. It is behind the `sfx` feature of `shuyi-assets`, off by
//! default, which a game turns on in its own `Cargo.toml`.
//!
//! A file is two kinds, as an image is (ADR-0019 §6): [`SfxDef`], its
//! numbers, read by [`SfxDefRon`]; and the [`Sound`] it renders to, read by
//! [`SfxRon`], which the manifest's sounds are. Ten presets ([`PRESETS`],
//! [`preset`]) are a start for a game's own.

use crate::prelude::*;

use serde::{Deserialize, Serialize};
use shuyi_core::SimRng;

use crate::audio::{MAX_VOLUME, SAMPLE_RATE, Sound, pcm16_wav};
use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Encode, Format, TextFormat};
use crate::ron::parse_ron_implicit_some;

/// The longest a sound may be, in milliseconds: `attack_ms`, `hold_ms`
/// and `fade_ms` together.
pub const MAX_MS: u32 = 5_000;

/// The lowest pitch, in hertz.
pub const MIN_HZ: u32 = 20;

/// The highest pitch, in hertz, also once a [`Step`] has raised it.
pub const MAX_HZ: u32 = 12_000;

/// The end of a sound effect's file name.
pub const SUFFIX: &str = crate::audio::SFX_SUFFIX;

/// The shape of a sound's wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub enum Wave {
    /// A square, high for `duty` percent of each cycle (1 to 99): 50 is
    /// hollow, a small duty thin and buzzy.
    Square {
        /// The share of each cycle the wave is high, in percent.
        duty: u8,
    },
    /// A triangle: soft, like a flute.
    Triangle,
    /// A saw, rising each cycle: bright and harsh.
    Saw,
    /// A sine: the purest tone.
    Sine,
    /// Noise: a new random level every half cycle, so the pitch sets how
    /// hissy it is.
    Noise,
}

/// A pitch that wobbles: up and down by `percent` of itself, `hz` times
/// a second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vibrato {
    /// How fast it wobbles, 1 to 50 times a second.
    pub hz: u8,
    /// How far, 1 to 50 percent of the pitch.
    pub percent: u8,
}

/// A pitch that jumps: from `at_ms` on, it is multiplied by `percent` /
/// 100 (a coin's jump up is 133, a fourth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// When it jumps, in milliseconds from the start, inside the sound.
    pub at_ms: u32,
    /// What it multiplies the pitch by, 10 to 400 percent.
    pub percent: u16,
}

/// A sound effect, as a `.sfx.ron` file writes it ([module docs](self)).
/// Read one with [`from_source`] or [`preset`], or build one with
/// [`SfxDef::new`], its `with_` methods and its fields (ADR-0039); [`problems`] says
/// what is out of range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RawSfxDef")]
#[non_exhaustive]
pub struct SfxDef {
    /// The wave's shape.
    pub wave: Wave,
    /// The pitch at the start, in hertz ([`MIN_HZ`] to [`MAX_HZ`]).
    pub hz: u32,
    /// The pitch at the end, in hertz: it slides in a straight line from
    /// `hz`. A file that leaves it out has `hz`.
    pub to_hz: u32,
    /// How long the loudness rises from nothing, in milliseconds.
    #[serde(skip_serializing_if = "is_default")]
    pub attack_ms: u32,
    /// How long it then holds, in milliseconds.
    pub hold_ms: u32,
    /// How long it then falls to nothing, in milliseconds. The three
    /// lengths together are 1 to [`MAX_MS`].
    pub fade_ms: u32,
    /// How loud, 0 to 100 percent, as a cue's volume.
    pub volume: u8,
    /// The pitch's wobble, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vibrato: Option<Vibrato>,
    /// The pitch's jump, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<Step>,
    /// The noise's luck: two seeds are two noises.
    #[serde(skip_serializing_if = "is_default")]
    pub seed: u64,
}

/// Whether `value` is its default (0), which a file leaves out.
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

impl SfxDef {
    /// A `wave` at `hz` at full volume, of no length until
    /// [`with_hold_ms`](Self::with_hold_ms) or
    /// [`with_fade_ms`](Self::with_fade_ms): no slide, no attack, no
    /// vibrato, no step, seed 0. No constructor takes two numbers, which a
    /// call could swap (ADR-0151).
    #[must_use]
    pub const fn new(wave: Wave, hz: u32) -> Self {
        Self {
            wave,
            hz,
            to_hz: hz,
            attack_ms: 0,
            hold_ms: 0,
            fade_ms: 0,
            volume: 100,
            vibrato: None,
            step: None,
            seed: 0,
        }
    }

    /// The same sound, holding `hold_ms` milliseconds.
    #[must_use]
    pub const fn with_hold_ms(mut self, hold_ms: u32) -> Self {
        self.hold_ms = hold_ms;
        self
    }

    /// The same sound, fading over `fade_ms` milliseconds.
    #[must_use]
    pub const fn with_fade_ms(mut self, fade_ms: u32) -> Self {
        self.fade_ms = fade_ms;
        self
    }

    /// The same sound, `volume` percent loud.
    #[must_use]
    pub const fn with_volume(mut self, volume: u8) -> Self {
        self.volume = volume;
        self
    }

    /// How long it plays, in milliseconds: its three lengths together.
    pub fn length_ms(&self) -> u64 {
        u64::from(self.attack_ms) + u64::from(self.hold_ms) + u64::from(self.fade_ms)
    }

    /// The highest pitch it reaches before a vibrato, in hertz: its step
    /// applied to the higher of `hz` and `to_hz`.
    fn top_hz(&self) -> u64 {
        let top = u64::from(self.hz.max(self.to_hz));
        match self.step {
            Some(step) => top * u64::from(step.percent) / 100,
            None => top,
        }
    }
}

impl AssetKind for SfxDef {
    const NAME: &str = "sound effect";
}

/// A `.sfx.ron` file as written: `to_hz`, `attack_ms`, `vibrato`, `step`
/// and `seed` may be left out. Its errors name it `SfxDef`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "SfxDef")]
struct RawSfxDef {
    wave: Wave,
    hz: u32,
    #[serde(default)]
    to_hz: Option<u32>,
    #[serde(default)]
    attack_ms: u32,
    hold_ms: u32,
    fade_ms: u32,
    volume: u8,
    #[serde(default)]
    vibrato: Option<Vibrato>,
    #[serde(default)]
    step: Option<Step>,
    #[serde(default)]
    seed: u64,
}

impl From<RawSfxDef> for SfxDef {
    fn from(raw: RawSfxDef) -> Self {
        Self {
            wave: raw.wave,
            hz: raw.hz,
            to_hz: raw.to_hz.unwrap_or(raw.hz),
            attack_ms: raw.attack_ms,
            hold_ms: raw.hold_ms,
            fade_ms: raw.fade_ms,
            volume: raw.volume,
            vibrato: raw.vibrato,
            step: raw.step,
            seed: raw.seed,
        }
    }
}

/// A problem with a sound effect: the field whose line it is reported on,
/// and what is wrong.
pub type Problem = (&'static str, String);

/// Every number of `def` out of its range ([`SfxDef`]'s fields say each
/// one), with the field whose line it is reported on.
pub fn problems(def: &SfxDef) -> Vec<Problem> {
    let mut found = Vec::new();
    if let Wave::Square { duty } = def.wave
        && !(1..=99).contains(&duty)
    {
        found.push(("wave", format!("duty {duty} is outside 1 to 99 (percent)")));
    }
    for (field, hz) in [("hz", def.hz), ("to_hz", def.to_hz)] {
        if hz < MIN_HZ {
            found.push((field, format!("{field} {hz} is under {MIN_HZ}")));
        } else if hz > MAX_HZ {
            found.push((field, format!("{field} {hz} is over {MAX_HZ}")));
        }
    }
    let length = def.length_ms();
    if length == 0 || length > u64::from(MAX_MS) {
        found.push((
            "hold_ms",
            format!(
                "the sound is {length} ms long (attack_ms, hold_ms and fade_ms together); \
                 it must be 1 to {MAX_MS}"
            ),
        ));
    }
    if def.volume > MAX_VOLUME {
        let volume = def.volume;
        found.push((
            "volume",
            format!("volume {volume} is over {MAX_VOLUME} (percent)"),
        ));
    }
    if let Some(Vibrato { hz, percent }) = def.vibrato {
        for (name, value) in [("hz", hz), ("percent", percent)] {
            if !(1..=50).contains(&value) {
                let why = format!("the vibrato's {name} {value} is outside 1 to 50");
                found.push(("vibrato", why));
            }
        }
    }
    if let Some(Step { at_ms, percent }) = def.step {
        if u64::from(at_ms) >= length {
            let why = format!("the step's at_ms {at_ms} is not inside the sound's {length} ms");
            found.push(("at_ms", why));
        }
        if !(10..=400).contains(&percent) {
            let why = format!("the step's percent {percent} is outside 10 to 400");
            found.push(("step", why));
        } else if def.top_hz() > u64::from(MAX_HZ) {
            let top = def.top_hz();
            let why = format!("the step takes the pitch to {top} hz, over {MAX_HZ}");
            found.push(("step", why));
        }
    }
    found
}

/// The first line of `source` where `field` is written as a field
/// (`hz:`, not `to_hz:`), if any.
fn field_line(source: &str, field: &str) -> Option<u32> {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let index = source.lines().position(|line| {
        line.match_indices(field).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = line[at + field.len()..].trim_start();
            !before.is_some_and(is_word) && after.starts_with(':')
        })
    })?;
    u32::try_from(index + 1).ok()
}

/// The sound effect in `source`, the text of the file `file`, or every
/// problem with it, each on the line of its field.
pub fn from_source(file: &str, source: &str) -> Result<SfxDef, Vec<ContentError>> {
    let def: SfxDef = parse_ron_implicit_some(file, source).map_err(|e| vec![e])?;
    let found = problems(&def);
    if found.is_empty() {
        return Ok(def);
    }
    Err(found
        .into_iter()
        .map(|(field, message)| {
            let error = ContentError::new(file, message);
            match field_line(source, field) {
                Some(line) => error.at(line, None),
                None => error,
            }
        })
        .collect())
}

/// `def` in the `.sfx.ron` text [`from_source`] reads back: RON's pretty
/// form, one field a line, and the fields a file may leave out left out
/// when they are their default. Its lines end in `\n` on every platform,
/// so a file written on Windows has the bytes of one written on Linux.
pub fn to_text(def: &SfxDef) -> String {
    let config = ::ron::ser::PrettyConfig::new()
        .new_line("\n")
        .depth_limit(1)
        .extensions(::ron::extensions::Extensions::IMPLICIT_SOME);
    // Numbers, enums and options always serialise.
    let text = ::ron::ser::to_string_pretty(def, config).unwrap_or_default();
    // [`from_source`] reads with `implicit_some` on: the line that turns
    // it on is not needed.
    let text = text
        .strip_prefix("#![enable(implicit_some)]")
        .map_or(text.as_str(), str::trim_start);
    format!("{text}\n")
}

/// Full scale: the loudest sample.
const FULL: i64 = 32_767;

/// The envelope's full gain.
const GAIN_ONE: i64 = 1 << 16;

/// A pitch in 1/256 hertz, so a slide moves smoothly.
const PITCH_ONE: i64 = 256;

/// The highest pitch a render plays, in 1/256 hertz: under half the
/// sample rate, so every half cycle has a sample.
const MAX_PITCH: i64 = 20_000 * PITCH_ONE;

/// Half a cycle of the phase.
const HALF: u32 = 1 << 31;

/// A quarter of a cycle of the phase.
const QUARTER: u32 = 1 << 30;

/// 1.0 in the sine's fixed point.
const SINE_ONE: i64 = 1 << 30;

/// The sine's refining weight, 0.225, in its fixed point: what brings the
/// parabola within a thousandth of a sine.
const SINE_REFINE: i64 = 241_591_910;

/// How many samples `ms` milliseconds are at [`SAMPLE_RATE`].
fn samples_in(ms: u64) -> i64 {
    i64::try_from(u64::from(SAMPLE_RATE) * ms / 1000).unwrap_or(i64::MAX)
}

/// A saw at `phase`: from -[`FULL`] at 0 up to [`FULL`] at the end of the
/// cycle.
fn ramp(phase: u32) -> i64 {
    i64::from(phase) * (2 * FULL) / (1_i64 << 32) - FULL
}

/// A triangle at `phase`: from -[`FULL`] at 0 up to [`FULL`] at half the
/// cycle, and down again.
fn triangle(phase: u32) -> i64 {
    let half = i64::from(HALF);
    FULL - 2 * FULL * (i64::from(phase) - half).abs() / half
}

/// A square at `phase`: [`FULL`] for the first `duty` percent of the
/// cycle, -[`FULL`] for the rest.
fn square(phase: u32, duty: u8) -> i64 {
    if u64::from(phase) * 100 < u64::from(duty) << 32 {
        FULL
    } else {
        -FULL
    }
}

/// A sine at `phase`, from a parabola and one refining step, in integers:
/// within 66 of `32 767 × sin` (a host test checks).
fn sine(phase: u32) -> i64 {
    // The phase as a signed angle: -1 to 1 (half cycles) in Q30.
    let mut at = i64::from(phase);
    if at >= i64::from(HALF) {
        at -= 1_i64 << 32;
    }
    let t = at >> 1;
    // 4t(1 - |t|), then y + 0.225 (y|y| - y).
    let y = (t * (SINE_ONE - t.abs())) >> 28;
    let y = y + ((SINE_REFINE * (((y * y.abs()) >> 30) - y)) >> 30);
    ((y * FULL) >> 30).clamp(-FULL, FULL)
}

/// A random level, -[`FULL`] to [`FULL`].
fn noise(rng: &mut SimRng) -> i64 {
    (i64::from(rng.next_u32() >> 16) - 32_768).max(-FULL)
}

/// The vibrato's triangle at sample `i`, -[`SAMPLE_RATE`] to
/// [`SAMPLE_RATE`]: `hz` times a second, starting at 0 and rising.
fn wobble(i: i64, hz: u8) -> i64 {
    let rate = i64::from(SAMPLE_RATE);
    // Four times the place in the cycle, a quarter cycle ahead: a
    // triangle of it starts at 0 and rises.
    let at = (4 * i * i64::from(hz) + 3 * rate) % (4 * rate);
    (at - 2 * rate).abs() - rate
}

/// `def` with every number in its range ([`problems`]), so a render of
/// any definition is bounded and never overflows: a definition that is in
/// range is unchanged.
fn clamped(def: &SfxDef) -> SfxDef {
    let mut def = def.clone();
    if let Wave::Square { duty } = &mut def.wave {
        *duty = (*duty).clamp(1, 99);
    }
    def.hz = def.hz.clamp(MIN_HZ, MAX_HZ);
    def.to_hz = def.to_hz.clamp(MIN_HZ, MAX_HZ);
    def.attack_ms = def.attack_ms.min(MAX_MS);
    def.hold_ms = def.hold_ms.min(MAX_MS - def.attack_ms);
    def.fade_ms = def.fade_ms.min(MAX_MS - def.attack_ms - def.hold_ms);
    def.volume = def.volume.min(MAX_VOLUME);
    if let Some(vibrato) = &mut def.vibrato {
        vibrato.hz = vibrato.hz.clamp(1, 50);
        vibrato.percent = vibrato.percent.clamp(1, 50);
    }
    if let Some(step) = &mut def.step {
        step.percent = step.percent.clamp(10, 400);
    }
    def
}

/// The samples of `def`, mono at [`SAMPLE_RATE`]: `SAMPLE_RATE ×
/// length_ms / 1000` of them, none louder than `32 767 × volume / 100`. A
/// number out of its range is brought into it first ([`problems`] reports
/// it). Integers only, so the same definition gives the same samples on
/// every platform.
pub fn render(def: &SfxDef) -> Vec<i16> {
    let def = clamped(def);
    let n = samples_in(def.length_ms());
    let attack = samples_in(u64::from(def.attack_ms));
    let held = attack + samples_in(u64::from(def.hold_ms));
    let fade = n - held;
    let from = i64::from(def.hz) * PITCH_ONE;
    let to = i64::from(def.to_hz) * PITCH_ONE;
    let step = def
        .step
        .map(|step| (samples_in(u64::from(step.at_ms)), i64::from(step.percent)));
    let per_cycle = i128::from(SAMPLE_RATE) * i128::from(PITCH_ONE);
    let mut rng = SimRng::new(def.seed);
    let mut level = noise(&mut rng);
    let mut half = false;
    let mut phase: u32 = 0;
    let mut samples = Vec::with_capacity(usize::try_from(n).unwrap_or_default());
    for i in 0..n {
        let mut pitch = from + (to - from) * i / n;
        if let Some((at, percent)) = step
            && i >= at
        {
            pitch = pitch * percent / 100;
        }
        if let Some(Vibrato { hz, percent }) = def.vibrato {
            let swing = pitch * i64::from(percent) * wobble(i, hz);
            pitch += swing / (100 * i64::from(SAMPLE_RATE));
        }
        let pitch = pitch.clamp(0, MAX_PITCH);
        let wave = match def.wave {
            Wave::Square { duty } => square(phase, duty),
            Wave::Triangle => triangle(phase.wrapping_add(QUARTER)),
            Wave::Saw => ramp(phase.wrapping_add(HALF)),
            Wave::Sine => sine(phase),
            Wave::Noise => {
                if (phase >= HALF) != half {
                    half = !half;
                    level = noise(&mut rng);
                }
                level
            }
        };
        let gain = if i < attack {
            GAIN_ONE * i / attack
        } else if i < held {
            GAIN_ONE
        } else {
            GAIN_ONE * (fade - 1 - (i - held)) / fade
        };
        let sample = wave * gain / GAIN_ONE * i64::from(def.volume) / 100;
        samples.push(i16::try_from(sample.clamp(-FULL, FULL)).unwrap_or_default());
        // A cycle is 2^32 of the phase: under half a cycle a sample.
        let step = (i128::from(pitch) << 32) / per_cycle;
        phase = phase.wrapping_add(u32::try_from(step).unwrap_or(HALF));
    }
    samples
}

/// `samples`, mono at [`SAMPLE_RATE`], as a 16-bit PCM WAV: the 44-byte
/// header, then the samples, the form
/// [`format_problem`](crate::audio::format_problem) accepts.
pub fn to_wav(samples: &[i16]) -> Vec<u8> {
    pcm16_wav(1, SAMPLE_RATE, samples)
}

/// What a problem of a `.sfx.ron` file is told in one line: with its line
/// when it has one (`line 3: hz 30000 is over 12000`).
fn one_line(error: &ContentError) -> String {
    match error.line {
        Some(line) => format!("line {line}: {}", error.message),
        None => error.message.clone(),
    }
}

/// The sound effect in a `.sfx.ron` file's `bytes`, or its first problem
/// in one line.
fn read(bytes: &[u8]) -> Result<SfxDef, String> {
    let source = core::str::from_utf8(bytes).map_err(|_| "not valid UTF-8".to_owned())?;
    from_source("", source).map_err(|errors| errors.first().map_or_default(one_line))
}

/// A `.sfx.ron` file's `bytes` rendered to a WAV ([`render`], [`to_wav`]),
/// or its first problem in one line: what the sound hook plays.
pub(crate) fn wav_of(bytes: &[u8]) -> Result<Vec<u8>, String> {
    read(bytes).map(|def| to_wav(&render(&def)))
}

/// How long a `.sfx.ron` file's `bytes` play once rendered, in milliseconds
/// (to the nearest): [`render`]'s samples over [`SAMPLE_RATE`]. `None` if
/// its numbers don't read.
pub(crate) fn length_ms_of(bytes: &[u8]) -> Option<u32> {
    read(bytes).ok().and_then(|def| rendered_ms(&def))
}

/// How long `def` plays once rendered, in milliseconds (to the nearest).
fn rendered_ms(def: &SfxDef) -> Option<u32> {
    let samples = u64::try_from(samples_in(clamped(def).length_ms())).ok()?;
    crate::audio::samples_ms(samples, SAMPLE_RATE)
}

/// A `.sfx.ron` file's first problem in one line, if it has one.
pub(crate) fn problem_of(bytes: &[u8]) -> Option<String> {
    read(bytes).err()
}

/// Whether `name` is a sound effect's: it ends in `.sfx.ron`, after a
/// name.
pub fn is_sfx_name(name: &str) -> bool {
    crate::audio::is_sfx(name)
}

/// A `.sfx.ron` file read for its numbers (`sfx`, `.ron`): [`from_source`].
/// It writes them back too ([`to_text`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SfxDefRon;

impl TextFormat for SfxDefRon {
    type Asset = SfxDef;

    fn id(&self) -> &'static str {
        "sfx"
    }

    fn read(
        &self,
        file: &AssetPath,
        text: &str,
        _cx: &DecodeCx<'_>,
    ) -> Result<SfxDef, Vec<ContentError>> {
        from_source(file.shown(), text)
    }
}

impl Encode for SfxDefRon {
    fn encode(&self, asset: &SfxDef) -> Vec<u8> {
        to_text(asset).into_bytes()
    }
}

/// A `.sfx.ron` file read as the sound it renders to (`sfx`, `.ron`): mono
/// at [`SAMPLE_RATE`] once its numbers read. Only a name that ends in
/// `.sfx.ron` is one, so another RON file among the sounds is an error that
/// says so, not a sound.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SfxRon;

impl Format for SfxRon {
    type Asset = Sound;

    fn id(&self) -> &'static str {
        "sfx"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ron"]
    }

    fn max_bytes(&self) -> usize {
        Format::max_bytes(&SfxDefRon)
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<Sound, Vec<ContentError>> {
        if !is_sfx_name(file.name()) {
            let why = format!("a sound in RON is a sound effect: its name must end in {SUFFIX}");
            return Err(vec![ContentError::new(file.shown(), why)]);
        }
        SfxDefRon
            .decode(file, bytes, cx)
            .map(|def| Sound::mono(SAMPLE_RATE).with_length_ms(rendered_ms(&def)))
    }
}

/// The presets' names: a start for a game's own sounds ([`preset`]).
pub const PRESETS: [&str; 10] = [
    "blip",
    "tick",
    "select",
    "cancel",
    "coin",
    "jump",
    "hit",
    "explosion",
    "power_up",
    "laser",
];

/// The text of the preset `name`, as its file writes it.
pub fn preset_source(name: &str) -> Option<&'static str> {
    Some(match name {
        "blip" => include_str!("sfx/presets/blip.sfx.ron"),
        "tick" => include_str!("sfx/presets/tick.sfx.ron"),
        "select" => include_str!("sfx/presets/select.sfx.ron"),
        "cancel" => include_str!("sfx/presets/cancel.sfx.ron"),
        "coin" => include_str!("sfx/presets/coin.sfx.ron"),
        "jump" => include_str!("sfx/presets/jump.sfx.ron"),
        "hit" => include_str!("sfx/presets/hit.sfx.ron"),
        "explosion" => include_str!("sfx/presets/explosion.sfx.ron"),
        "power_up" => include_str!("sfx/presets/power_up.sfx.ron"),
        "laser" => include_str!("sfx/presets/laser.sfx.ron"),
        _ => return None,
    })
}

/// The preset `name` ([`PRESETS`]), or `None` if there is no such preset.
/// Seed 0 is its file as written. Another seed moves its pitches (`hz` and
/// `to_hz` together) by up to 10 % and its `hold_ms` by up to 20 %, with
/// `SimRng::new(seed)`, and is its `seed`: so a cue's variants can be a few
/// seeds of one preset. Every seed stays in range.
pub fn preset(name: &str, seed: u64) -> Option<SfxDef> {
    let mut def = from_source(name, preset_source(name)?).ok()?;
    if seed != 0 {
        vary(&mut def, seed);
    }
    Some(def)
}

/// `value` times `percent` / 100, rounded down.
fn scaled(value: u32, percent: u32) -> u32 {
    u32::try_from(u64::from(value) * u64::from(percent) / 100).unwrap_or(u32::MAX)
}

/// [`preset`]'s variation of `def` by `seed`, kept in range.
fn vary(def: &mut SfxDef, seed: u64) {
    let mut rng = SimRng::new(seed);
    let pitch = 90 + rng.roll_below(21);
    let hold = 80 + rng.roll_below(41);
    let top = def.step.map_or(MAX_HZ, |step| {
        let top = u64::from(MAX_HZ) * 100 / u64::from(step.percent.max(1));
        u32::try_from(top).unwrap_or(MAX_HZ).min(MAX_HZ)
    });
    def.hz = scaled(def.hz, pitch).clamp(MIN_HZ, top);
    def.to_hz = scaled(def.to_hz, pitch).clamp(MIN_HZ, top);
    let others = def.attack_ms.saturating_add(def.fade_ms);
    def.hold_ms = scaled(def.hold_ms, hold).min(MAX_MS.saturating_sub(others));
    if def.length_ms() == 0 {
        def.hold_ms = 1;
    }
    let last = u32::try_from(def.length_ms() - 1).unwrap_or(0);
    if let Some(step) = &mut def.step {
        step.at_ms = step.at_ms.min(last);
    }
    def.seed = seed;
}

#[cfg(test)]
mod tests;
