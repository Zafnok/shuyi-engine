//! The sound probe (ticket 2573, ADR-0066): the shared player on the real
//! backend, driven by `tools/sound-parity/scenarios.ron`, with nothing else
//! running. `tools/sound-parity/capture.py` records what it sends to the
//! sound device, and `cargo xtask sound-parity gate` judges the recording.
//!
//! ```text
//! cargo build -p shuyi-app --example sound_probe --features sound-probe --release
//! ```
//!
//! It runs in a folder holding `scenarios.ron`, `corpus/` and `probe.txt`,
//! whose one line says what to do: `scenario <name>`, `perf <burn>` for
//! the timed workload, `<burn>` being the slowdown it injects in percent (0
//! for none), or `decoders <pair>` for the old path's whole decodes against
//! `shuyi-sound`'s, the side first going by `<pair>`, or `mixer <pair>` for the
//! mixer's offline render, or, on the in-house backend natively, `latency` for how
//! long a play takes to reach the device's buffer. Every line it says starts
//! `shuyi-sound-probe: ` and ends in a JSON object: on stderr natively, on the console
//! on the web.
//!
//! On the web it waits for a click, the gesture a browser asks for before it
//! sounds anything. Natively it opens no window: the backend needs none.

use std::borrow::Cow;
use std::collections::BTreeMap;

use macroquad::miniquad::date;
use quad_snd::AudioContext;
#[cfg(not(feature = "in-house-sound"))]
use quad_snd::Sound;
use serde::Deserialize;
use shuyi_app::SoundProbeBackend;
use shuyi_assets::audio::{AudioManifest, CreditRef, Files, MusicCue, SoundCue};
use shuyi_platform::audio::SoundRequest;
use shuyi_platform::audio::player::{Player, RATE_ONE};
use shuyi_sound::mix::{Config, Mixer, Params, SharedPcm, UNITY};
use shuyi_sound::vorbis::VorbisSource;
use shuyi_sound::wav::WavSource;
use shuyi_sound::{Channels, Limits, SharedSource, read_all};
use shuyi_ui::audio::{AudioQueue, MusicCommand, SoundId};

/// How long one pass of the main loop waits natively, in seconds. The web's
/// is its frame.
#[cfg(not(target_arch = "wasm32"))]
const TICK: f64 = 0.004;
/// How long the perf workload plays, in seconds.
const PERF_PLAY: f64 = 3.0;
/// The copies of a looped sound the perf workload mixes.
const PERF_COPIES: usize = 4;
/// How many whole decodes a decode's time is the sum of: on the web each is
/// seen to end only at a frame, which one alone can't rise above.
const PERF_DECODES: usize = 8;
/// How long the perf workload waits for the music's load, in seconds, as a
/// scenario's `settle` does.
const PERF_SETTLE: f64 = 1.5;
/// How many whole decodes one decoder measure is the sum of, the slowdown it
/// injects into a third measure of the new side, in percent, and how long
/// `corpus/long`, which they decode, plays, in milliseconds.
const DECODER_DECODES: u32 = 3;
const DECODER_BURN: u32 = 100;
const DECODER_AUDIO_MS: u32 = 10_000;
/// The mixer's measure: the voices it mixes (as the most a backend makes when a
/// manifest doesn't say), how long a sound it renders, in milliseconds, in blocks of
/// how many frames, on an output of what rate, and the slowdown a third measure
/// injects, in percent.
const MIXER_VOICES: usize = 32;
const MIXER_AUDIO_MS: u32 = 10_000;
const MIXER_BLOCK: usize = 512;
const MIXER_RATE: u32 = 48_000;
const MIXER_BURN: u32 = 100;
/// How long the sound plays on after the measure, in seconds, so that what is
/// still on its way to the sink isn't cut off with the process.
const PERF_DRAIN: f64 = 0.5;

/// The part of `scenarios.ron` the probe reads; the gate reads the rest.
#[derive(Deserialize)]
struct Plan {
    /// Sound cues: their file in `corpus/`, and whether they loop.
    sounds: BTreeMap<String, (String, bool)>,
    /// Music cues: their file in `corpus/`, and their length in milliseconds.
    music: BTreeMap<String, (String, u32)>,
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    /// Done before the clock starts, with `settle` milliseconds to finish.
    #[serde(default)]
    setup: Vec<Action>,
    #[serde(default)]
    settle: u32,
    steps: Vec<Step>,
    /// Where the clock stops, in milliseconds.
    end: u32,
}

#[derive(Deserialize)]
struct Step {
    at: u32,
    #[serde(rename = "do")]
    action: Action,
}

#[derive(Clone, Deserialize)]
enum Action {
    Sound(String, f32),
    Start(u32, String, f32),
    Stop(u32),
    Volume(u32, f32),
    /// `count` copies of a cue, numbered from the first, in one frame.
    Copies(u32, u32, String, f32),
    /// Stops the copies `Copies` started.
    StopCopies(u32, u32),
    /// How fast a sound plays from its next play: 1.0 is as recorded.
    Rate(String, f32),
    Pause,
    Resume,
    Load(String),
    Music(String, f32),
    Gain(String, f32),
    StopMusic(String),
    /// Nothing: a moment the state is logged at.
    Tick,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    block_on(start());
    // What `#[macroquad::main]` expands to.
    #[cfg(target_arch = "wasm32")]
    macroquad::Window::new("Shuyi Engine's sound probe", start());
}

async fn start() {
    if let Err(e) = run().await {
        say(&format!(r#"{{"event":"failed","text":"{}"}}"#, clean(&e)));
    }
    say(r#"{"event":"done"}"#);
    // The page stays open, with its frames running, until it is closed.
    #[cfg(target_arch = "wasm32")]
    loop {
        macroquad::window::next_frame().await;
    }
}

async fn run() -> Result<(), String> {
    let order = String::from_utf8_lossy(&read("probe.txt").await?).into_owned();
    let plan = read("scenarios.ron").await?;
    let plan: Plan = ron::de::from_bytes(&plan).map_err(|e| format!("scenarios.ron: {e}"))?;
    match order.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["scenario", name] => {
            let scenario = plan.scenarios.iter().find(|s| s.name == *name);
            let scenario = scenario.ok_or_else(|| format!("no scenario {name}"))?;
            play_scenario(&plan, scenario).await
        }
        ["perf", burn] => {
            let burn = burn.parse().map_err(|e| format!("burn {burn}: {e}"))?;
            perf(&plan, burn).await
        }
        ["decoders", pair] => {
            let pair = pair.parse().map_err(|e| format!("pair {pair}: {e}"))?;
            decoders(pair).await
        }
        ["mixer", pair] => {
            let pair: u32 = pair.parse().map_err(|e| format!("pair {pair}: {e}"))?;
            mixer(pair).await
        }
        #[cfg(all(feature = "in-house-sound", not(target_arch = "wasm32")))]
        ["latency"] => latency().await,
        _ => Err(format!("probe.txt says {order:?}")),
    }
}

/// The player on the real backend, with every cue of `plan` loaded.
async fn player(
    plan: &Plan,
    backend: &mut SoundProbeBackend,
) -> Result<Player<SoundProbeBackend>, String> {
    let mut manifest = AudioManifest::default();
    let mut bytes = BTreeMap::new();
    for (cue, (file, looped)) in &plan.sounds {
        let sound = SoundCue::new(vec![file.clone()], 100, CreditRef::Own).with_looped(*looped);
        manifest.sounds.insert(cue.clone(), sound);
        bytes.insert(file.clone(), read(&format!("corpus/{file}")).await?);
    }
    for (cue, (file, length_ms)) in &plan.music {
        let music = MusicCue {
            file: file.clone(),
            volume: 100,
            looped: true,
            length_ms: *length_ms,
            credit: CreditRef::Own,
        };
        manifest.music.insert(cue.clone(), music);
    }
    let sound = |file: &str| bytes.get(file).map(|b| Cow::Owned(b.clone()));
    let shown = |file: &str| file.to_owned();
    let files = Files::new(&sound, &shown, &|_| false, "corpus");
    let player = Player::load(
        backend,
        manifest,
        &files,
        "corpus".to_owned(),
        "corpus".to_owned(),
        1,
    );
    Ok(player.await)
}

/// Waits, on the web, for the click that lets the page sound, and tells the
/// capture when it may click.
// Only the web awaits here.
#[cfg_attr(not(target_arch = "wasm32"), allow(clippy::unused_async))]
async fn wait_for_gesture() {
    say(r#"{"event":"ready"}"#);
    #[cfg(target_arch = "wasm32")]
    {
        use macroquad::input::{MouseButton, is_mouse_button_pressed};
        while !is_mouse_button_pressed(MouseButton::Left) {
            macroquad::window::next_frame().await;
        }
    }
}

async fn play_scenario(plan: &Plan, scenario: &Scenario) -> Result<(), String> {
    let mut backend = SoundProbeBackend::default();
    let mut player = player(plan, &mut backend).await?;
    let mut ids = Ids::default();
    let mut requests = Vec::new();
    let mut music = Vec::new();
    for action in &scenario.setup {
        request(action, &mut ids, &mut requests, &mut music);
    }
    player.play(&mut backend, &requests, &music, date::now());
    wait_for_gesture().await;
    let settle = date::now() + f64::from(scenario.settle) / 1000.0;
    while date::now() < settle {
        player.play(&mut backend, &[], &[], date::now());
        tick().await;
    }
    let started = date::now();
    let mut steps = scenario.steps.iter().peekable();
    loop {
        let now = date::now();
        let elapsed_ms = (now - started) * 1000.0;
        requests.clear();
        music.clear();
        let mut due = Vec::new();
        while let Some(step) = steps.next_if(|step| f64::from(step.at) <= elapsed_ms) {
            if let Action::Rate(cue, rate) = &step.action {
                // Fixed point, as the player takes it.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let fixed = (f64::from(*rate) * f64::from(RATE_ONE)) as u32;
                player.set_rate(&mut backend, cue, fixed);
            }
            request(&step.action, &mut ids, &mut requests, &mut music);
            due.push(step.at);
        }
        player.play(&mut backend, &requests, &music, now);
        for at in due {
            let (cue, secs) = player.music_playing(now).unwrap_or(("", 0.0));
            say(&format!(
                r#"{{"event":"step","at":{at},"wall":{now:.4},"cue":"{cue}","secs":{secs:.4}}}"#
            ));
        }
        for warning in player.take_warnings() {
            say(&format!(
                r#"{{"event":"warn","text":"{}"}}"#,
                clean(&warning)
            ));
        }
        if elapsed_ms >= f64::from(scenario.end) {
            break;
        }
        tick().await;
    }
    player.stop_all(&mut backend);
    #[cfg(all(feature = "in-house-sound", not(target_arch = "wasm32")))]
    {
        let (rebuilds, silent) = backend.output_stats();
        say(&format!(
            r#"{{"event":"output","rebuilds":{rebuilds},"silent_callbacks":{silent}}}"#
        ));
    }
    Ok(())
}

/// The handles the plan's numbers stand for. Only a queue makes a [`SoundId`]
/// (the same calls give the same ids on every run).
#[derive(Default)]
struct Ids {
    queue: AudioQueue<()>,
    by_number: BTreeMap<u32, SoundId>,
}

/// `action` as the requests and commands of a frame.
fn request(
    action: &Action,
    ids: &mut Ids,
    requests: &mut Vec<SoundRequest>,
    music: &mut Vec<MusicCommand>,
) {
    let id = |ids: &Ids, number: &u32| ids.by_number.get(number).copied();
    match action {
        Action::Sound(cue, volume) => requests.push(SoundRequest::Sound {
            cue: cue.clone(),
            volume: *volume,
        }),
        Action::Start(number, cue, volume) => {
            let sound = ids.queue.start_sound(cue.as_str());
            ids.queue.take();
            ids.by_number.insert(*number, sound);
            requests.push(SoundRequest::start(sound, cue.as_str(), *volume));
        }
        Action::Copies(first, count, cue, volume) => {
            for number in *first..first + count {
                request(
                    &Action::Start(number, cue.clone(), *volume),
                    ids,
                    requests,
                    music,
                );
            }
        }
        Action::StopCopies(first, count) => {
            for number in *first..first + count {
                request(&Action::Stop(number), ids, requests, music);
            }
        }
        Action::Stop(number) => {
            requests.extend(id(ids, number).map(|id| SoundRequest::Stop { id }));
        }
        Action::Volume(number, volume) => {
            requests.extend(id(ids, number).map(|id| SoundRequest::Volume {
                id,
                volume: *volume,
            }));
        }
        Action::Pause => requests.push(SoundRequest::PauseSounds),
        Action::Resume => requests.push(SoundRequest::ResumeSounds),
        Action::Load(cue) => music.push(MusicCommand::Load { cue: cue.clone() }),
        Action::Music(cue, gain) => music.push(MusicCommand::start(cue.as_str(), *gain)),
        Action::Gain(cue, gain) => music.push(MusicCommand::Gain {
            cue: cue.clone(),
            gain: *gain,
        }),
        Action::StopMusic(cue) => music.push(MusicCommand::Stop { cue: cue.clone() }),
        Action::Tick | Action::Rate(..) => {}
    }
}

/// The timed workload: file reads and the context's start apart, the same
/// playback for [`PERF_PLAY`] seconds, then whole decodes. Each phase's time
/// grows by `burn` percent of it, so the gate can prove it sees a slowdown.
async fn perf(plan: &Plan, burn: u32) -> Result<(), String> {
    let measure = say_measure;
    let timed = date::now();
    let wav = read("corpus/long.wav").await?;
    let ogg = read("corpus/long.ogg").await?;
    measure("file_io_ms", burned(timed, burn) * 1000.0);

    #[cfg(not(feature = "in-house-sound"))]
    let ctx = {
        let timed = date::now();
        let ctx = AudioContext::new();
        measure("context_ms", burned(timed, burn) * 1000.0);
        ctx
    };

    let mut backend = SoundProbeBackend::default();
    let mut player = player(plan, &mut backend).await?;
    let mut ids = Ids::default();
    let mut requests = Vec::new();
    let mut music = Vec::new();
    let copies = u32::try_from(PERF_COPIES).unwrap_or(u32::MAX);
    for action in [
        Action::Load("music_a".to_owned()),
        Action::Load("music_b".to_owned()),
    ] {
        request(&action, &mut ids, &mut requests, &mut music);
    }
    player.play(&mut backend, &requests, &music, date::now());
    wait_for_gesture().await;
    let settle = date::now() + PERF_SETTLE;
    while date::now() < settle {
        player.play(&mut backend, &[], &[], date::now());
        tick().await;
    }
    requests.clear();
    music.clear();
    // The marker first: its onset in the recording gives the output latency.
    request(
        &Action::Sound("marker".to_owned(), 1.0),
        &mut ids,
        &mut requests,
        &mut music,
    );
    player.play(&mut backend, &requests, &music, date::now());
    say(&format!(
        r#"{{"event":"step","at":0,"wall":{:.4},"cue":"","secs":0}}"#,
        date::now()
    ));
    let settle = date::now() + 0.5;
    while date::now() < settle {
        player.play(&mut backend, &[], &[], date::now());
        tick().await;
    }
    requests.clear();
    music.clear();
    for number in 1..=copies {
        let action = Action::Start(number, "noise_loop".to_owned(), 0.25);
        request(&action, &mut ids, &mut requests, &mut music);
    }
    for cue in ["music_a", "music_b"] {
        let action = Action::Music(cue.to_owned(), 0.25);
        request(&action, &mut ids, &mut requests, &mut music);
    }
    player.play(&mut backend, &requests, &music, date::now());
    let cpu = cpu_ns();
    let started = date::now();
    say(r#"{"event":"playing"}"#);
    while date::now() - started < PERF_PLAY {
        let tick_started = date::now();
        player.play(&mut backend, &[], &[], date::now());
        tick().await;
        burned(tick_started, burn);
    }
    if let (Some(before), Some(after)) = (cpu, cpu_ns()) {
        #[allow(clippy::cast_precision_loss)]
        measure("cpu_ms", (after - before) as f64 / 1e6);
    }
    say(r#"{"event":"played"}"#);
    let drained = date::now() + PERF_DRAIN;
    while date::now() < drained {
        player.play(&mut backend, &[], &[], date::now());
        tick().await;
    }
    player.stop_all(&mut backend);
    // Before the decodes: quad-snd frees each on its mixer thread, so how many
    // pile up in the allocator depends on that thread's timing, and identical runs
    // peaked anywhere from 33 to 82 MB. This is what playing holds.
    if let Some(kib) = peak_kib() {
        measure("peak_kib", kib);
    }

    #[cfg(not(feature = "in-house-sound"))]
    perf_decodes_old(&ctx, &wav, &ogg, burn).await;
    #[cfg(feature = "in-house-sound")]
    perf_decodes_new(&wav, &ogg, burn)?;
    Ok(())
}

/// The workload's whole decodes, the old backend's way.
#[cfg(not(feature = "in-house-sound"))]
#[cfg_attr(not(target_arch = "wasm32"), allow(clippy::unused_async))]
async fn perf_decodes_old(ctx: &AudioContext, wav: &[u8], ogg: &[u8], burn: u32) {
    for (metric, bytes) in [("decode_wav_ms", wav), ("decode_ogg_ms", ogg)] {
        let timed = date::now();
        for _ in 0..PERF_DECODES {
            let sound = Sound::load(ctx, bytes);
            // The browser decodes in the background; natively it is done.
            #[cfg(target_arch = "wasm32")]
            while !sound.is_loaded() {
                tick().await;
            }
            sound.delete(ctx);
        }
        say_measure(metric, burned(timed, burn) * 1000.0);
    }
}

/// The workload's whole decodes, the new backend's way.
#[cfg(feature = "in-house-sound")]
fn perf_decodes_new(wav: &[u8], ogg: &[u8], burn: u32) -> Result<(), String> {
    for (metric, kind, bytes) in [
        ("decode_wav_ms", Kind::Wav, wav),
        ("decode_ogg_ms", Kind::Ogg, ogg),
    ] {
        let timed = date::now();
        for _ in 0..PERF_DECODES {
            decode_new(kind, bytes)?;
        }
        say_measure(metric, burned(timed, burn) * 1000.0);
    }
    Ok(())
}

/// One number of the timed workload.
fn say_measure(metric: &str, value: f64) {
    say(&format!(
        r#"{{"event":"perf","metric":"{metric}","value":{value:.3}}}"#
    ));
}

/// How long a play takes to reach the device's buffer, a hundred times at a phase of
/// its own, at no volume, so that it can be measured on a machine someone is at.
#[cfg(all(feature = "in-house-sound", not(target_arch = "wasm32")))]
async fn latency() -> Result<(), String> {
    use shuyi_platform::audio::player::Backend;

    let bytes = read("corpus/silence.wav").await?;
    let mut backend = SoundProbeBackend::default();
    let sound = backend.load_sound(&bytes).await?;
    // The first play moves the mixer to the callback.
    let _ = backend.play(&sound, 0.0, false);
    tick().await;
    let mut measured = Vec::new();
    for turn in 0..100u32 {
        std::thread::sleep(std::time::Duration::from_micros(u64::from(
            7000 + turn * 137 % 5000,
        )));
        if let Some(ms) = backend.play_latency_ms(&sound) {
            measured.push(ms);
        }
    }
    measured.sort_by(f64::total_cmp);
    let at = |share: usize| {
        measured
            .get(measured.len() * share / 100)
            .copied()
            .unwrap_or(0.0)
    };
    say(&format!(
        r#"{{"event":"latency","plays":{},"median_ms":{:.2},"p95_ms":{:.2},"worst_ms":{:.2}}}"#,
        measured.len(),
        at(50),
        at(95),
        measured.last().copied().unwrap_or(0.0),
    ));
    Ok(())
}

/// Which adapter of `shuyi-sound` a file goes through.
#[derive(Clone, Copy)]
enum Kind {
    Wav,
    Ogg,
}

/// `bytes` decoded whole by `shuyi-sound`.
fn decode_new(kind: Kind, bytes: &[u8]) -> Result<(), String> {
    let limits = Limits::default();
    let pcm = match kind {
        Kind::Wav => WavSource::new(bytes, &limits).and_then(|mut s| read_all(&mut s, &limits)),
        Kind::Ogg => VorbisSource::new(bytes, &limits).and_then(|mut s| read_all(&mut s, &limits)),
    };
    core::hint::black_box(pcm.map_err(|e| e.to_string())?);
    Ok(())
}

/// [`DECODER_DECODES`] whole decodes by `shuyi-sound`, in milliseconds, with
/// `burn` percent more.
fn timed_new(kind: Kind, bytes: &[u8], burn: u32) -> Result<f64, String> {
    let timed = date::now();
    for _ in 0..DECODER_DECODES {
        decode_new(kind, bytes)?;
    }
    Ok(burned(timed, burn) * 1000.0)
}

/// [`DECODER_DECODES`] whole decodes by the old path, in milliseconds.
#[cfg_attr(not(target_arch = "wasm32"), allow(clippy::unused_async))]
async fn timed_old(ctx: &AudioContext, bytes: &[u8]) -> f64 {
    use quad_snd::Sound;

    let timed = date::now();
    for _ in 0..DECODER_DECODES {
        let sound = Sound::load(ctx, bytes);
        // The browser decodes in the background; natively it is done.
        #[cfg(target_arch = "wasm32")]
        while !sound.is_loaded() {
            tick().await;
        }
        sound.delete(ctx);
    }
    (date::now() - timed) * 1000.0
}

/// Old against new on the same bytes in the same process, then the new side
/// slowed: one line per kind of file, in milliseconds per decode. Each side
/// decodes once untimed first, as a process's first decode pays for what later
/// ones find ready, then they take turns old, new, new, old (new first by
/// `pair`), so neither is always measured just after the other (ticket 2044).
async fn decoders(pair: u32) -> Result<(), String> {
    let wav = read("corpus/long.wav").await?;
    let ogg = read("corpus/long.ogg").await?;
    let ctx = AudioContext::new();
    wait_for_gesture().await;
    let per_decode = |ms: f64, measures: u32| ms / f64::from(measures * DECODER_DECODES);
    for (file, kind, bytes) in [("wav", Kind::Wav, &wav), ("ogg", Kind::Ogg, &ogg)] {
        timed_old(&ctx, bytes).await;
        timed_new(kind, bytes, 0)?;
        let (old, new) = if pair.is_multiple_of(2) {
            let first = timed_old(&ctx, bytes).await;
            let new = timed_new(kind, bytes, 0)? + timed_new(kind, bytes, 0)?;
            (first + timed_old(&ctx, bytes).await, new)
        } else {
            let first = timed_new(kind, bytes, 0)?;
            let old = timed_old(&ctx, bytes).await + timed_old(&ctx, bytes).await;
            (old, first + timed_new(kind, bytes, 0)?)
        };
        let burn = timed_new(kind, bytes, DECODER_BURN)?;
        say(&format!(
            r#"{{"event":"decode","file":"{file}","pair":{pair},"old":{:.3},"new":{:.3},"burn":{:.3},"audio_ms":{DECODER_AUDIO_MS}}}"#,
            per_decode(old, 2),
            per_decode(new, 2),
            per_decode(burn, 1),
        ));
    }
    Ok(())
}

/// A file of the corpus, decoded whole.
async fn decoded(file: &str) -> Result<SharedPcm, String> {
    let bytes = read(&format!("corpus/{file}")).await?;
    let limits = Limits::default();
    let pcm = WavSource::new(&bytes, &limits).and_then(|mut s| read_all(&mut s, &limits));
    Ok(SharedPcm::new(pcm.map_err(|e| e.to_string())?))
}

/// The hardest mix a game would make: every voice busy, each at a rate of its own
/// and looped, two of them streams. [`MIXER_AUDIO_MS`] of it, in milliseconds, with
/// `burn` percent more.
fn timed_mix(sounds: &[SharedPcm], burn: u32) -> Result<f64, String> {
    let full = |e| format!("{e}");
    let mut mixer = Mixer::new(Config::new(MIXER_RATE, Channels::Stereo, MIXER_VOICES));
    let mut ids = Vec::new();
    for sound in sounds {
        ids.push(mixer.add_sound(sound.clone()).map_err(full)?);
    }
    let params = |voice: u32| {
        Params::new()
            .looped()
            .with_volume(UNITY / 8)
            .with_rate(50_000 + voice * 1000)
    };
    let mut numbers = 0u32..;
    for stream in sounds.iter().take(2) {
        let number = numbers.next().unwrap_or(0);
        mixer
            .stream(SharedSource::new(stream.clone()), params(number))
            .map_err(full)?;
    }
    for (number, id) in numbers.zip(ids.iter().cycle()).take(MIXER_VOICES - 2) {
        mixer.play(*id, params(number)).map_err(full)?;
    }
    let frames = u64::from(MIXER_RATE) * u64::from(MIXER_AUDIO_MS) / 1000;
    let mut block = vec![0; MIXER_BLOCK * 2];
    let timed = date::now();
    for _ in 0..frames / MIXER_BLOCK as u64 {
        mixer.render(&mut block);
        core::hint::black_box(&block);
    }
    Ok(burned(timed, burn) * 1000.0)
}

/// The mixer rendering to memory, then slowed, so the gate can prove it sees that.
async fn mixer(pair: u32) -> Result<(), String> {
    let mut sounds = Vec::new();
    for file in ["long.wav", "music_a.wav", "fmt_22k_m.wav", "noise_loop.wav"] {
        sounds.push(decoded(file).await?);
    }
    wait_for_gesture().await;
    let render = timed_mix(&sounds, 0)?;
    let burn = timed_mix(&sounds, MIXER_BURN)?;
    say(&format!(
        r#"{{"event":"mix","pair":{pair},"render":{render:.3},"burn":{burn:.3},"audio_ms":{MIXER_AUDIO_MS}}}"#,
    ));
    Ok(())
}

/// The seconds since `timed`, after spinning for `burn` percent more.
fn burned(timed: f64, burn: u32) -> f64 {
    let took = date::now() - timed;
    let until = date::now() + took * f64::from(burn) / 100.0;
    while date::now() < until {
        core::hint::spin_loop();
    }
    date::now() - timed
}

/// What this process's threads have run for, the mixers' among them, in
/// nanoseconds: the first number of each thread's `schedstat`. `None` where
/// that isn't there; on the web the capture reads the browser's.
fn cpu_ns() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let mut total = 0;
        for task in std::fs::read_dir("/proc/self/task").ok()?.flatten() {
            let stat = std::fs::read_to_string(task.path().join("schedstat")).ok()?;
            total += stat.split_whitespace().next()?.parse::<u64>().ok()?;
        }
        Some(total)
    }
    #[cfg(not(target_os = "linux"))]
    None
}

/// This process's peak memory, in KiB, where the system says.
fn peak_kib() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
        line.split_whitespace().nth(1)?.parse().ok()
    }
    #[cfg(not(target_os = "linux"))]
    None
}

/// `text` safe inside a JSON string.
fn clean(text: &str) -> String {
    text.replace(['"', '\\', '\n'], " ")
}

/// One line for the capture to read.
fn say(line: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("shuyi-sound-probe: {line}");
    #[cfg(target_arch = "wasm32")]
    macroquad::logging::info!("shuyi-sound-probe: {}", line);
}

/// One pass of the main loop goes by.
#[cfg_attr(not(target_arch = "wasm32"), allow(clippy::unused_async))]
async fn tick() {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::sleep(std::time::Duration::from_secs_f64(TICK));
    #[cfg(target_arch = "wasm32")]
    macroquad::window::next_frame().await;
}

#[cfg_attr(not(target_arch = "wasm32"), allow(clippy::unused_async))]
async fn read(path: &str) -> Result<Vec<u8>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let bytes = std::fs::read(path).map_err(|e| e.to_string());
    #[cfg(target_arch = "wasm32")]
    let bytes = macroquad::file::load_file(path)
        .await
        .map_err(|e| e.to_string());
    bytes.map_err(|e| format!("{path}: {e}"))
}

/// Runs `future` here: natively nothing it awaits ever waits.
#[cfg(not(target_arch = "wasm32"))]
fn block_on(future: impl Future<Output = ()>) {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    while future.as_mut().poll(&mut context).is_pending() {
        std::thread::yield_now();
    }
}
