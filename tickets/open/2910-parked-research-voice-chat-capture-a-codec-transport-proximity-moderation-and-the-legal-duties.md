---
id: "2910"
title: "Parked: Research: voice chat in multiplayer games — microphone capture per platform, a permissive codec, transport over the net tickets, proximity voice, moderation, and the legal duties for Nick"
type: research
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: []
nick_input: decision
completed:
---

# 2910 — Research: voice chat

> **Parked** (Nick, 2026-10-22: "some random thoughts I have are supporting weather system, day/night cycle, and proximity audio and voice chat (voice chat in multiplayer enabled games)"): voice chat comes with networking (1140–1150), so this waits until networking is unparked (the game's 0053 picks an online mode). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick asked on 2026-10-22 for voice chat in multiplayer games, and
proximity voice, where a player is heard louder the nearer they are
(`docs/decisions.md`, "weather, day and night, voice chat and ray
tracing get tickets"). No ticket builds it: 1149 (text chat channels
with moderation) lists voice chat under **Out**.

Where engines stand: Unreal has its Voice Chat interface (with Vivox
or EOS Voice behind it); Unity sells Vivox as a service; Godot has
`AudioEffectCapture` for the microphone and leaves the rest to the
game; Steamworks has its own voice API (`ISteamUser::StartVoiceRecording`
and `GetVoice`), and consoles have their own system voice chat a game
must use or allow. Each of these is a service or a platform API, not
code an engine ships.

What Shuyi has today, and plans:

- **No sound input at all.** Every audio path goes out:
  `shuyi-app`'s player (`crates/app/src/audio.rs`) over macroquad and
  quad-snd. Only platform backends touch audio devices (`AGENTS.md`
  rule 5, ADR-0024).
- **The net tickets, all parked:** 1140's `Transport` trait with a
  `Reliable` and an `Unreliable` channel; the relay and native
  WebSocket (1141), WebSocket in the browser through our own JS plugin
  (1142), UDP on the LAN (1143), Steam networking and lobbies (1144),
  sessions and lobbies over all of them (1145), real-time play and how
  the browser joins (1147, which names WebRTC and WebTransport), and
  text chat with filter, block, mute and report (1149).
- **Positional sound** (2363, parked): gain and pan from a sound's
  world position to a listener. Proximity voice is the same falloff on
  a voice stream.
- **Sound after quad-snd** (2902, parked): whether we write our own
  mixer and output (3030–3032) or use a library. Capture would live
  beside output.
- **The web build** uses macroquad's own loader, not `wasm-bindgen`
  (`AGENTS.md`, *Environment*), so the browser's microphone
  (`getUserMedia`) needs a JS plugin of our own, like
  `crates/cli/web/gamepad.js`.
- **Licenses**: ADR-0002 §8 denies copyleft and anything that costs
  money, which rules out a paid voice service as the engine's default.

Why it matters: voice is common in co-op and competitive games, and
proximity voice is a genre of its own (social deduction, horror,
survival). It is also the one networking feature with the heaviest
duties: players talking live to strangers, often minors, and platform
rules about muting and reporting.

This ticket is research: a report, an ADR, a question for Nick, and
parked tickets for the build. It promises no crate.

## Nick input

**Decision**, at the end of the research (the `ask-nick` skill): whether
the engine ships voice chat at all, given the duties the report finds
(moderation, reporting, data about minors, recording, and each store's
rules), and on which targets. Everything technical is decided here.

## Scope

**In:**
- **The needs**, written first: push-to-talk and open mic with a level
  gate; per-player mute and volume; team, lobby and proximity channels;
  latency a conversation tolerates (say a number and its source);
  bandwidth per speaker; what happens on packet loss and when a player
  leaves; split screen and several local players (2406).
- **Capture per platform**, with the facts of the day: Windows
  (WASAPI), macOS (Core Audio, with its microphone permission prompt),
  Linux and the Steam Deck (ALSA, PulseAudio, PipeWire), the web
  (`getUserMedia` through a miniquad JS plugin, its permission prompt
  and the user gesture it needs), and consoles (one line each: their
  SDKs' own voice systems). Libraries such as `cpal` (said to be
  Apache-2.0): its license, whether it builds on Windows with the GNU
  toolchain, and whether it fits beside quad-snd or after 2902.
  **Either order with 2902:** if it is done, follow the sound path it
  chose; if not, say what capture needs from it and add a line to
  2902's ticket.
- **The codec**: Opus (libopus, said to be BSD-3-Clause, royalty-free)
  through a binding and any pure Rust Opus encoder and decoder, each
  checked with `cargo deny check licenses` in a scratch crate with
  `default-features = false`; whether each builds for
  `wasm32-unknown-unknown` without `wasm-bindgen`; or the browser's own
  encoder (WebCodecs or WebRTC) on the web.
- **Noise and echo**: noise suppression (for example `nnnoiseless`, a
  port of RNNoise, said to be BSD-3-Clause) and echo cancellation (the
  WebRTC audio processing module, said to be BSD-3-Clause; the
  browser's own in `getUserMedia`'s constraints). What a game gets
  without them.
- **Transport**: voice over 1140's `Unreliable` channel with a jitter
  buffer and loss concealment; why voice over the relay's WebSocket
  (TCP, 1141) is or isn't good enough; WebRTC or WebTransport for the
  browser (1147). **Either order with 1140, 1141 and 1147:** if done,
  build on what they made; if not, name what voice adds to each and add
  a line to each ticket's scope. Peer to peer against through the
  relay, and what each costs to run.
- **Platform voice services**: Steam's voice API behind the `steam`
  feature (1144), console voice, and Vivox or EOS Voice as an opt-in a
  game brings itself, never the default (ADR-0002 §8).
- **Proximity voice**: the falloff and pan of 2363 applied to a voice
  stream; where the positions come from (the game's state, by player,
  never by the network's own idea of place); whether the listener is
  the player's unit or the camera. **Either order with 2363:** if it is
  done, reuse its falloff; if not, name what it must expose and add a
  line to its ticket.
- **The layers** (`AGENTS.md` rule 5, ADR-0024): capture, encoding and
  playback live in platform backends; the rules never see a voice
  sample, and a replay never contains one. Push-to-talk is an action a
  screen declares (`AGENTS.md` rule 8), never a key. Say what the
  headless backend does in tests (a scripted voice stream of a tone).
- **Moderation and the law**, for Nick: per-player mute and block,
  report (what a report can carry when there is no recording), whether
  to record at all, voice for players under 13 or 16 (COPPA, the GDPR's
  age of consent, the UK's Age Appropriate Design Code), the EU's
  Digital Services Act duties for a service that carries user speech,
  and the stores' rules (Steam, the consoles' certification
  requirements on muting and on blocking). Facts dated, with sources;
  this is a summary for a decision, not legal advice, and it says so.
  It builds on 1149's legal list for text chat (**either order with
  1149:** if done, link its "what running chat means" page; if not,
  write the voice part as a section 1149's page can take).
- **Outputs**:
  - the report, `docs/research/2910-voice-chat.md` (create
    `docs/research/` if no earlier research has), with the facts dated;
  - the question to Nick (the `ask-nick` skill), with the report's
    recommendation, and his answer in `docs/decisions.md`;
  - an ADR (the `write-adr` skill) with the technical decision: capture,
    codec, transport and the module shape (a `shuyi-voice` module,
    ADR-0009, with the capability it needs on a target; **either order
    with 3017:** if it is done, name the capability, for example
    `microphone: bool`; if not, write it as an edit to 3017's ticket);
  - if Nick says yes, the build tickets (the `write-ticket` skill), each
    one PR, each `Parked:` with the networking note and milestone
    "Engine: networking", listed in `docs/ROADMAP.md`'s networking row.
    If he says no, the ADR records why and no build ticket is written.
- **Size:** if the PR passes about 600 changed lines with the report and
  the ADR, the build tickets become a follow-up ticket blocked by this
  one; the PR says so.

**Out (do not do):**
- building any of it: the tickets it writes do;
- text chat (1149) and global chat or MMO scale (1150);
- speech to text, text to speech, voice changers and effects;
- recording voice for a replay, a crash report or a session record
  (2518 records inputs, never voice);
- reading a codec's source to copy it (ADR-0021 §2); reading its docs
  to judge it is fine;
- anything in Visions of Shuyi.

## Implementation steps

1. Read 1140–1149, 2363, 2902 and 3017 as they stand, and ADR-0002 §8,
   ADR-0021 and ADR-0024. Note which are done, for the either-order
   rules above.
2. Write the needs into the report.
3. Survey capture, codecs, noise and echo, and transports; check the
   licenses with `cargo deny` in a scratch crate.
4. Prototype, in a scratch folder never committed, a loopback on the
   desktop: capture, encode, a lossy in-memory link, decode, play; and
   measure the latency and the CPU it takes. Say if the web prototype
   wasn't tried, and why.
5. Write the moderation and law section, then the recommendation.
6. Ask Nick, record his answer, write the ADR and, if yes, the parked
   build tickets and the roadmap row.
7. Run the gates.

## Acceptance criteria

- [ ] `docs/research/2910-voice-chat.md` has the needs, a dated table of
      every capture library and codec (license, pure Rust or not,
      Windows GNU, `wasm32-unknown-unknown` without `wasm-bindgen`, size
      added), and the loopback prototype's measured latency and CPU.
- [ ] The report says, for each of Windows, Linux, macOS, the web, the
      Steam Deck and the consoles, how voice is captured and sent, or
      that the target refuses a game with voice.
- [ ] The report names how proximity voice uses 2363's falloff and
      where the positions come from.
- [ ] The moderation and law section is dated, cites its sources, and
      says it is not legal advice.
- [ ] Nick's answer is in `docs/decisions.md`.
- [ ] The ADR is in `docs/adr/`, linked from the report.
- [ ] If Nick said yes, the build tickets are in `tickets/open/`, each
      parked, and `docs/ROADMAP.md` lists them; if no, the ADR says why.
- [ ] No recommended crate is outside ADR-0002 §8.
- [ ] `cargo xtask ticket-lint` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it writes say their tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
