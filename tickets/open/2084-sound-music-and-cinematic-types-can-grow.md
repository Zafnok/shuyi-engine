---
id: "2084"
title: "Before the first release, the sound, music and cinematic types that open tickets grow move under ADR-0039, and shuyi-app's sound features keep their names for good"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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
  **Either order with 2089:** if 2089 is done, take the `quad_snd` line
  for `shuyi-app` out of its allowed outside crates
  (`crates/xtask/src/api.rs`); if not, nothing to do (2089 allows none).
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

- [ ] Every variant and struct in the table is `#[non_exhaustive]` with
      its constructor, and the sound and cinematic files in the
      repository and the templates load unchanged.
- [ ] `SoundProbeBackend` is hidden from the docs, and `cargo xtask api`
      shows the change.
- [ ] ADR-0068 says the features' names are kept and none names
      quad-snd.
- [ ] The golden sound tables and the sound gates' results are
      unchanged.
- [ ] No open ticket still plans a `Breaking:` line for one of these
      types.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each constructor builds what a literal built before.
- The existing audio, music and cinematic tests pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
