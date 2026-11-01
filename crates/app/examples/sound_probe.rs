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
//! whose one line says what to do: `scenario <name>`, or `perf <burn>` for
//! the timed workload, `<burn>` being the slowdown it injects in percent (0
//! for none). Every line it says starts `shuyi-sound-probe: ` and ends in a
//! JSON object: on stderr natively, on the console on the web.
//!
//! On the web it waits for a click, the gesture a browser asks for before it
//! sounds anything. Natively it opens no window: the backend needs none.

use std::borrow::Cow;
use std::collections::BTreeMap;

use macroquad::miniquad::date;
use quad_snd::{AudioContext, Sound};
use serde::Deserialize;
use shuyi_app::SoundProbeBackend;
use shuyi_assets::audio::{AudioManifest, CreditRef, Files, MusicCue, SoundCue};
use shuyi_platform::audio::SoundRequest;
use shuyi_platform::audio::player::Player;
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
        Action::Tick => {}
    }
}

/// The timed workload: file reads, the context's start and whole decodes
/// apart, then the same playback for [`PERF_PLAY`] seconds. Each phase's time
/// grows by `burn` percent of it, so the gate can prove it sees a slowdown.
async fn perf(plan: &Plan, burn: u32) -> Result<(), String> {
    let measure = |metric: &str, value: f64| {
        say(&format!(
            r#"{{"event":"perf","metric":"{metric}","value":{value:.3}}}"#
        ));
    };
    let timed = date::now();
    let wav = read("corpus/long.wav").await?;
    let ogg = read("corpus/long.ogg").await?;
    measure("file_io_ms", burned(timed, burn) * 1000.0);

    let timed = date::now();
    let ctx = AudioContext::new();
    measure("context_ms", burned(timed, burn) * 1000.0);

    for (metric, bytes) in [("decode_wav_ms", &wav), ("decode_ogg_ms", &ogg)] {
        let timed = date::now();
        for _ in 0..PERF_DECODES {
            let sound = Sound::load(&ctx, bytes);
            // The browser decodes in the background; natively it is done.
            #[cfg(target_arch = "wasm32")]
            while !sound.is_loaded() {
                tick().await;
            }
            sound.delete(&ctx);
        }
        measure(metric, burned(timed, burn) * 1000.0);
    }

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
    if let Some(kib) = peak_kib() {
        measure("peak_kib", kib);
    }
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
