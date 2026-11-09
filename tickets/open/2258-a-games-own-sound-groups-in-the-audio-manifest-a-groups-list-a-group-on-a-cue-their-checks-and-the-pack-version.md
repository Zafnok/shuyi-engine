---
id: "2258"
title: "A game's own sound groups in the audio manifest: a groups list, a group on a cue, their checks, and the pack version"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2342"]
nick_input: none
completed:
---

# 2258 — A game's own sound groups in the audio manifest

## Context

2342 made the player's volumes a tree of sound groups on a loudness curve
([ADR-0091](../../docs/adr/0091-sound-groups-the-players-volumes-as-a-tree-of-groups-on-a-loudness-curve-worked-out-before-any-backend.md)),
with the five built-in groups (`master`; `music`, `sfx` and `voice` under
it; `ui` under `sfx`). Its PR passed the ticket's size limit of about 600
changed lines, so, as 2342's *Size* paragraph says, the manifest's own
`groups` list, its checks and the cues' `group` field come here.

What there is (2026-11-08):

- `crates/assets/src/audio/groups.rs`: the built-in ids as `pub const`s
  (`MASTER`, `MUSIC`, `SFX`, `VOICE`, `UI`) and `BUILT_IN`, each with its
  parent. `shuyi_ui::audio::groups` re-exports them.
- `crates/ui/src/audio/groups.rs`: `SoundGroups::new(&AudioConfig)`
  builds the built-ins at their defaults (`master` and `ui` 100, the
  others 97) and puts the menus' cues in `ui`; `GroupGains` and `Gains`
  read the groups by index, built-ins first. `Gains::music` answers
  `music`'s gain for every cue, and `SoundGroups::music_group` `music`.
- `crates/assets/src/audio.rs`: `SoundCue` and `MusicCue` have no
  `group`; `RawFile` denies unknown fields.

So a game can't give its ambience a volume of its own yet.

## Nick input

None.

## Scope

**In:**

- **Groups in the audio manifest** (`crates/assets/src/audio.rs`, and
  the checks in `crates/assets/src/audio/groups.rs`):
  - an optional `groups` list (`#[serde(default)]` on `RawFile`):
    `(id: "ambience", parent: "sfx", volume: 80)`. `parent` is optional
    and means `master`; `volume` is the position a new player starts at
    (0–100), optional (97, as the built-ins below `master`);
  - an entry whose id is built in may set only `volume` (the game's own
    default for that group); a `parent` on it is an error;
  - an optional `group` on a sound cue and on a music cue
    (`Option<String>`, `#[serde(default)]` on `RawSound` and `RawMusic`);
    `SoundCue::new` leaves it `None`, and `SoundCue::with_group` sets it;
  - errors at the line that names them (`Check::err`): a duplicate group
    id, an unknown parent, a loop of parents, a volume over 100
    (`Check::volume`), a cue naming an unknown group;
  - `AudioManifest.groups` holds what the file declares and not the
    built-ins, so the asset stays what its file says;
  - the module docs of `crates/assets/src/audio.rs` name the groups;
  - the manifest's binary form changes: make `pack::VERSION`
    (`crates/assets/src/pack.rs`) the next number and accept the new
    snapshot of
    `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
    (`crates/assets/src/pack/tests.rs`), as that test's comment says.
- **`SoundGroups` from the manifest** (`crates/ui/src/audio/groups.rs`):
  `SoundGroups::new` takes the manifest too; the built-ins with their
  defaults, then the manifest's groups (parents first) and defaults over
  them. A sound cue plays in its `group`, else `ui` for the menus' cues,
  else `sfx`; a music cue in its `group`, else `music` (a `music` map
  beside `sounds`, so `Gains::music` reads it). `Ctx::new` and
  `Game::with_stack` pass the manifest, so a test that changes the
  manifest before making the `Game` sees the change.
- **A game's own row**: the docs of `OptionRow::music_volume` already show
  one; a toy test gives the toy's manifest a group and moves it with that
  row.
- **Docs:** ADR-0091 §1 already says groups are manifest data; the
  `AudioManifest` docs and the book's audio chapter show a `groups` entry.

**Either order:**

- **2218** (writers of the engine's text formats): if it is done, its
  `AudioManifestRon` writer writes `groups` and the cues' `group`, and its
  round-trip test covers a manifest with them; if not, 2218 writes them
  with the rest of the kind, as its clause says.
- **2343** (ducking): if it is done, a rule's `lower` may also name a
  group the manifest declares, and its check reads the declared groups
  (a test); if not, nothing here (2343's clause says so).
- **2345** (cue rules): both add a field to `SoundCue` and change the
  manifest's binary form: the second to land rebases and bumps
  `pack::VERSION` again. If 2345 is done, `SoundCue::new` leaves `group`
  at `None` beside 2345's fields.
- **2366** (parked, captions): it adds a field to `SoundCue` and
  `MusicCue` too; the later to land rebases onto the other's cue and
  bumps `pack::VERSION` again.

**For the game:** additive. A manifest without `groups` or a cue's `group`
sounds as before. `SoundCue` is `#[non_exhaustive]`; a `MusicCue` literal
adds `group: None`, which the PR says.

**Out (do not do):**

- rows made at run time from the manifest's groups (`OptionRow`'s key is
  a `&'static str`; ADR-0091's consequences);
- a group per voice clip (2725, parked);
- ducking (2343).

## Implementation steps

1. Read ADR-0091, `crates/assets/src/audio.rs` and `audio/groups.rs`,
   `crates/ui/src/audio/groups.rs`, `crates/ui/src/{ctx,game}.rs`.
2. The manifest's `groups` and cues' `group`, their checks, and the pack
   version, with tests.
3. `SoundGroups` from the manifest, `Ctx::new` and `Game::with_stack`,
   with tests.
4. The docs; run the gates.

## Acceptance criteria

- [ ] A manifest with no `groups` and no cue `group` loads as before, and
      the existing tests of `crates/assets/src/audio.rs` pass unchanged.
- [ ] Each manifest error above is reported at the line that names it (a
      test each).
- [ ] A cue with a `group` plays in it, a sound without one in `ui` or
      `sfx` and a track in `music` (a unit test of `SoundGroups`); a
      built-in's default set by the manifest is the default a new player
      hears.
- [ ] The book's chapter is updated, or the completion notes have a `Book:`
      line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the manifest's groups and their errors; `SoundGroups` from a
  manifest (a cue's group, a declared group's default, a built-in's
  default set).
- Snapshot: the pack's binary forms.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
