---
id: "2084"
title: "Before the first release, the sound, music and cinematic types that open tickets grow move under ADR-0039, and shuyi-app's sound features keep their names for good"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-15
---

# 2084 — The sound, music and cinematic types that open tickets grow move under ADR-0039 before the first release

## Context

The third of five tickets that move, before 2005, every exhaustive
public type an open ticket plans to grow (2082 has the reasons and
Nick's words of 2026-11-14). Nick asked that the sound backend be worked
out before the release. The audit found that the swap from quad-snd to
the in-house backend is internal to `shuyi-app`. Games use
`shuyi_ui::audio` and `shuyi_platform::audio`, never a quad-snd type.
The backend is chosen inside `shuyi-app` (`Chosen`,
`crates/app/src/audio.rs:23-25`), and the shared `Backend` trait adds
features as provided methods (ADR-0063). Three things would still break
a game after the release:

1. **Struct-like variants and structs that open tickets grow.**
   `SoundRequest` and `AudioRequest` are `#[non_exhaustive]`, but some
   of their variants are not. `SoundRequest::Start` already is, with
   `SoundRequest::start` (`crates/platform/src/audio.rs:36,121`).

   | Type | Where | Grown by |
   | ---- | ----- | -------- |
   | `SoundRequest::Sound` | `crates/platform/src/audio.rs:27` | 2345, 2378, 2379, 2363 |
   | `SoundRequest::Voice` | `audio.rs:66` | 2725 |
   | `AudioRequest::PlaySound` | `crates/ui/src/audio.rs:146-150` | 2363 |
   | `AudioRequest::PlayVoice` | `crates/ui/src/audio.rs:207` | 2725 |
   | `AudioFrame` | `crates/platform/src/audio.rs:173` | the backends' frames |
   | `MusicCue` | `crates/assets/src/audio.rs:221` (kind 5) | 2397, 2366, 2365, 2206 |
   | `MusicClock` | `crates/ui/src/audio.rs:464` | 2365 |
   | `MusicCommand` | `crates/ui/src/audio.rs:512` | 2365, 2397 |
   | `CinematicDef`, `ShotDef`, `CinematicFile`, `TimedShot` | `crates/assets/src/cinematic.rs:105,120,138,165` (kind 5) | 2364, 2365 |

2. **A cargo feature removed or renamed.** `shuyi-app` has
   `in-house-sound` and `sound-probe` (`crates/app/Cargo.toml`). 3079
   removes "obsolete features", and 3084 weighs making quad-snd
   optional as the feature's other side. A feature a game names that
   later goes away is a breaking release.
3. **`SoundProbeBackend`** (`crates/app/src/lib.rs:196`), a re-export
   of the internal `Chosen` for the sound gates. Its type changes when
   the backend does.

## Nick input

None.

## Scope

**In:**
- The variants and structs in the table under ADR-0039: a
  `#[non_exhaustive]` variant gets a constructor as `Start` has
  (`SoundRequest::sound(cue, volume)`, `SoundRequest::voice(clip)`,
  `AudioRequest::play_sound(..)`, `AudioRequest::play_voice(..)`). Kind 5
  structs take `#[serde(default)]` on later fields, so sound manifests
  and cinematic files keep loading.
- `SoundProbeBackend` under `#[doc(hidden)]`, with a doc comment that it
  is for the engine's sound gates and not covered by semver.
  **From 2087 (done):** its methods are the one path of `quad_snd` left
  in the public API listing (ADR-0119, §3), so hiding it leaves none.
  **From 2089 (done):** take the `quad_snd` line for `shuyi-app` out of
  `OUTSIDE_CRATES` (`crates/xtask/src/api.rs`), and its comment; `cargo
  xtask api` then fails any `quad_snd` path left in the listing.
- A paragraph in [ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md):
  - `in-house-sound` stays a feature of `shuyi-app` for good, and does
    nothing once it is the only backend;
  - no feature is named for quad-snd. Going back to it is
    `default-features = false`, and quad-snd is an optional dependency
    named with `dep:`, so it makes no feature of its own;
  - `sound-probe` is the gates' own, and its doc says so.

  **Either order with 3084:** if 3084 is done, check that quad-snd is an
  optional `dep:` with no feature of its own, and make it one here if
  not; if not, nothing to do (3084 follows the rule). **Either order with
  3079:** if 3079 is done, check that `in-house-sound` is still a
  feature, and restore it as one that does nothing if not; if not,
  nothing to do (3079 keeps it).
- Every engine, template and example call site; the listed tickets'
  planned `Breaking:` lines for these types become additive.
- A `Breaking:` line in the completion notes, after a search of the
  game for literals of each variant
  (`gh search code --repo Zafnok/visions-of-shuyi 'SoundRequest::Sound'`).

**Out (do not do):**
- Any new field the listed tickets add, and any change to which backend
  is the default (3084) or to quad-snd (3079).
- The backend's own types inside `shuyi-app` and `shuyi-sound`.

## Implementation steps

1. The attributes and constructors; doc lines naming the kind.
2. Build the workspace, the templates and the examples, and fix each
   literal and exhaustive match.
3. `#[doc(hidden)]` on `SoundProbeBackend` and its doc comment; the doc
   comments of the two features in `crates/app/Cargo.toml`.
4. ADR-0068's paragraph.
5. Edit the listed tickets' planned `Breaking:` lines.

## Acceptance criteria

- [x] Every variant and struct in the table is `#[non_exhaustive]` with
      its constructor, and the sound and cinematic files in the
      repository and the templates load unchanged.
- [x] `SoundProbeBackend` is hidden from the docs, and `cargo xtask api`
      shows the change.
- [x] ADR-0068 says the features' names are kept and none names
      quad-snd.
- [x] The golden sound tables and the sound gates' results are
      unchanged.
- [x] No open ticket still plans a `Breaking:` line for one of these
      types.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each constructor builds what a literal built before.
- The existing audio, music and cinematic tests pass unchanged.

## Completion notes

What was done:

- Every type of the table is under ADR-0039, each with a doc line naming
  its kind:
  - kind 6, struct-like variants made `#[non_exhaustive]` with a
    constructor: `SoundRequest::sound(cue, volume)`,
    `SoundRequest::voice(clip)`, `AudioRequest::play_sound(cue, volume)`
    and `AudioRequest::play_voice(line, variant)` (a `const fn`);
  - kind 6, `MusicCommand` itself is `#[non_exhaustive]`: a match on it
    has a `_` arm. Its `Start` was already a `#[non_exhaustive]` variant
    with `MusicCommand::start`;
  - kind 3: `AudioFrame::new(requests, music, gains)` (a `const fn`) and
    `MusicClock::new(cue, position, length)`, beside `from_elapsed`;
  - kind 5: `MusicCue::new(file, volume, length_ms, credit)` (looped, in
    the `music` group) with `with_looped` and `with_group`, as
    `SoundCue` has; `CinematicDef::new(id, length_ms, shots)` with
    `with_music`, `ShotDef::new(at_ms, shot)`,
    `CinematicFile::new(id, length, shots)` and `TimedShot::new(at,
    shot)`. Their serde forms are unchanged: no field was added, so every
    manifest, cinematic file and pack loads as before, and
    `pack::VERSION` stays.
- Call sites outside the defining crates: the toy's tracks, `App`'s voice
  pattern (`..`), the shared player's and the headless speaker's music
  matches (a `_` arm: a command they don't know plays nothing), the
  contract's music rows (`_` names an unknown command `other`; none exists
  today), the sound probe, the in-house backend's test, and the tests of
  `shuyi-ui`, `shuyi-platform`, `shuyi-playdate` and `shuyi-cli` that
  built these by literal. The templates, the tutorial, the snippets, the
  examples and the skill pack built none of them by literal.
- `SoundProbeBackend` is `#[doc(hidden)]`, with a doc comment that it is
  the sound gates' own and not covered by semver; the crate docs and the
  two features' comments in `crates/app/Cargo.toml` say the same.
  `tools/sound-parity/footprint.ron`'s exact edits follow the new text
  (`cargo xtask sound-parity give-up` checks: ok).
- ADR-0068 gains §8: `in-house-sound` stays for good and does nothing
  once it is the only backend, no feature is named for quad-snd, and
  `sound-probe` is the gates' own.
- 2089 landed while this PR was open (#444): the merge of `main` took
  the `quad_snd` line for `shuyi-app`, and its comment, out of
  `OUTSIDE_CRATES` (`crates/xtask/src/api.rs`), so `cargo xtask api` now
  fails any `quad_snd` path in the listing. 3084 and
  3079 are not done; their clauses now say "From 2084 (done)".
- The planned `Breaking:` lines of 2345, 2378, 2379, 2363, 2725, 2397,
  2366, 2365 and 2364 for these types are now additive. 2378 keeps its
  breaks for `AudioManifest` and the `Sound` kind, and 2365 its break of
  `TimedShot::at`'s type, which no attribute prevents. 2206 planned none.
- Tests: `each_constructor_builds_what_a_literal_built` in
  `shuyi-platform`'s and `shuyi-ui`'s audio and `shuyi-assets`'
  cinematic tests, and `a_music_cues_constructor_builds_what_a_literal_built`
  in `shuyi-assets`' audio. No snapshot or golden table changed.

Deviations:

- Some existing tests changed, against "pass unchanged": those outside
  the defining crate (or in another crate's test module) that built a
  type by literal now call its constructor, which the attribute makes
  necessary. Their assertions are unchanged.
- `MusicCommand`'s `Load`, `Gain` and `Stop` are not `#[non_exhaustive]`
  variants: no open ticket plans a field on them, only new commands.
- `CinematicDef::with_music` is new: `music` is optional, and kind 5 as
  kind 2 sets an optional field with a `with_` method.

Tickets written:

- 2091: giving up the sound replacement still takes the `in-house-sound`
  feature out (the footprint and CI's `sound-give-up` grep), which
  ADR-0068 §8 now forbids after the first release.

The game, searched on 2026-11-15 at its `main` (`695f222`, pin
`cfd79d10`) with a shallow clone and `grep`, after `gh search code
--repo Zafnok/visions-of-shuyi '<Type>'` for the first nine types ran
out of search quota. Its pin move changes:

- `AudioRequest::PlaySound`: about 4 literals (`ui/src/game.rs`,
  `screens/title/tests.rs`, `tests/it/title_cinematic.rs`,
  `playdate/src/audio.rs`) and one pattern without `..`
  (`playdate/src/audio.rs:218`).
- `AudioRequest::PlayVoice`: 6 literals (`debug.rs`, `screen.rs`,
  `battle/guide_voice.rs`, `dialogue/tests/voice.rs`, `tests/it/lessons.rs`,
  `tests/it/voice.rs`).
- `MusicCue`: 3 literals (`ui/src/audio.rs`, `ui/src/screen.rs`,
  `playdate/src/audio.rs`).
- `MusicClock`: 3 literals (`camp/music.rs`, `camp/tests.rs`,
  `title/tests.rs`).
- `ShotDef`: 6 literals (`content/src/cinematic/tests.rs`,
  `cinematic/tests/combat.rs`, `tests/it/playdate.rs`).
- `MusicCommand`: one exhaustive match (`playdate/src/audio.rs:252`)
  gets a `_` arm.
- `SoundRequest`, `AudioFrame`, `CinematicDef`, `CinematicFile`,
  `TimedShot`, `SoundProbeBackend`, `in-house-sound` and `sound-probe`:
  no literal, match or use found.

Game: nothing to adopt beyond the pin move: the game's next pin move past
this change makes the edits above (about 25 call sites); no game ticket
is needed before it.

Book: none, no page of the book or snippet builds one of these types by
literal or matches `MusicCommand`.

Breaking: shuyi-platform: SoundRequest::Sound, SoundRequest::Voice: #[non_exhaustive] variants; build them with SoundRequest::sound and SoundRequest::voice, and match them with `..`

```migration
since: 0.1.0
what: SoundRequest's Sound and Voice are built with SoundRequest::sound and SoundRequest::voice, and matched with `..`.
old: "SoundRequest::Sound { cue, volume }, SoundRequest::Voice { clip }"
new: SoundRequest::sound(cue, volume), SoundRequest::voice(clip)
before: |
  let click = SoundRequest::Sound { cue: "click".into(), volume: 0.5 };
  let say = SoundRequest::Voice { clip: file };
after: |
  let click = SoundRequest::sound("click", 0.5);
  let say = SoundRequest::voice(file);
find: ["SoundRequest::Sound {", "SoundRequest::Voice {"]
```

Breaking: shuyi-platform: AudioFrame: #[non_exhaustive]; build it with AudioFrame::new

```migration
since: 0.1.0
what: An AudioFrame is built with AudioFrame::new, not by a literal.
old: AudioFrame { requests, music, gains }
new: AudioFrame::new(requests, music, gains)
before: |
  let frame = AudioFrame { requests: &requests, music: &[], gains };
after: |
  let frame = AudioFrame::new(&requests, &[], gains);
find: ["AudioFrame {"]
```

Breaking: shuyi-ui: AudioRequest::PlaySound, AudioRequest::PlayVoice: #[non_exhaustive] variants; build them with AudioRequest::play_sound and AudioRequest::play_voice, and match them with `..`

```migration
since: 0.1.0
what: AudioRequest's PlaySound and PlayVoice are built with play_sound and play_voice, and matched with `..`.
old: "AudioRequest::PlaySound { cue, volume }, AudioRequest::PlayVoice { line, variant }"
new: AudioRequest::play_sound(cue, volume), AudioRequest::play_voice(line, variant)
before: |
  let click = AudioRequest::PlaySound { cue: "click".into(), volume: 1.0 };
  let say = AudioRequest::PlayVoice { line: line.clone(), variant: Voice::Only };
after: |
  let click = AudioRequest::play_sound("click", 1.0);
  let say = AudioRequest::play_voice(line.clone(), Voice::Only);
find: ["AudioRequest::PlaySound {", "AudioRequest::PlayVoice {"]
```

Breaking: shuyi-ui: MusicClock: #[non_exhaustive]; build it with MusicClock::new

```migration
since: 0.1.0
what: A MusicClock is built with MusicClock::new (or from_elapsed), not by a literal.
old: MusicClock { cue, position, length }
new: MusicClock::new(cue, position, length)
before: |
  let clock = MusicClock { cue: "camp".to_owned(), position: 1.5, length: 60.0 };
after: |
  let clock = MusicClock::new("camp", 1.5, 60.0);
find: ["MusicClock {"]
```

Breaking: shuyi-ui: MusicCommand: #[non_exhaustive]; a match on it has a `_` arm

```migration
since: 0.1.0
what: A match on MusicCommand has a wildcard arm.
old: MusicCommand (exhaustive)
new: MusicCommand (#[non_exhaustive])
before: |
  match command { MusicCommand::Load { cue } => a(cue), MusicCommand::Start { cue, .. } => b(cue), MusicCommand::Gain { cue, gain } => c(cue, *gain), MusicCommand::Stop { cue } => d(cue) }
after: |
  match command { MusicCommand::Load { cue } => a(cue), MusicCommand::Start { cue, .. } => b(cue), MusicCommand::Gain { cue, gain } => c(cue, *gain), MusicCommand::Stop { cue } => d(cue), _ => {} }
```

Breaking: shuyi-assets: MusicCue: #[non_exhaustive]; build it with MusicCue::new, with_looped and with_group

```migration
since: 0.1.0
what: A MusicCue is built with MusicCue::new and its with_ methods, not by a literal.
old: MusicCue { file, volume, looped, length_ms, credit, group }
new: MusicCue::new(file, volume, length_ms, credit).with_looped(looped).with_group(group)
before: |
  let track = MusicCue { file: "theme.ogg".into(), volume: 80, looped: false, length_ms: 60_000, credit: CreditRef::Own, group: None };
after: |
  let track = MusicCue::new("theme.ogg".into(), 80, 60_000, CreditRef::Own).with_looped(false);
find: ["MusicCue {"]
```

Breaking: shuyi-assets: CinematicDef, ShotDef, CinematicFile, TimedShot: #[non_exhaustive]; build them with their `new`

```migration
since: 0.1.0
what: CinematicDef, ShotDef, CinematicFile and TimedShot are built with their new, not by a literal.
old: "CinematicDef { id, music, length_ms, shots }, ShotDef { at_ms, shot }, CinematicFile { id, length, actions, shots }, TimedShot { at, shot }"
new: CinematicDef::new(id, length_ms, shots).with_music(music), ShotDef::new(at_ms, shot), CinematicFile::new(id, length, shots), TimedShot::new(at, shot)
before: |
  let logo = ShotDef { at_ms: 0, shot: Shot::Logo };
  let def = CinematicDef { id, music: None, length_ms: 9000, shots: vec![logo] };
  let raw = CinematicFile { id, length, actions: BTreeMap::new(), shots: vec![TimedShot { at: 0.0, shot }] };
after: |
  let logo = ShotDef::new(0, Shot::Logo);
  let def = CinematicDef::new(id, 9000, vec![logo]);
  let raw = CinematicFile::new(id, length, vec![TimedShot::new(0.0, shot)]);
find: ["CinematicDef {", "ShotDef {", "CinematicFile {", "TimedShot {"]
```

Breaking: shuyi-app: SoundProbeBackend: #[doc(hidden)] and not covered by semver; it is the sound gates' own, and no game names it

```migration
since: 0.1.0
what: SoundProbeBackend, behind the sound-probe feature, is hidden and not covered by semver.
old: shuyi_app::SoundProbeBackend (documented)
new: shuyi_app::SoundProbeBackend (#[doc(hidden)], for the engine's sound gates only)
before: |
  let backend = shuyi_app::SoundProbeBackend::default();
after: |
  // No replacement: a game plays sound through shuyi_ui::audio, never a backend.
find: ["SoundProbeBackend"]
```
