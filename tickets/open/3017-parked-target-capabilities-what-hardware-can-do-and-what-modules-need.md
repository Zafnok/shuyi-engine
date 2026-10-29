---
id: "3017"
title: "Parked: Target capabilities: each profile says what its hardware can do, each module says what it needs, and shuyi check --target names what doesn't fit"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3001", "2003"]
nick_input: none
completed:
---

# 3017 — Target capabilities and module needs

> **Parked** (Nick, 2026-10-17: "not all hardware supports all features"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It is unparked with 3D too, because 3104 and 3107 need it.

## Context

Nick, 2026-10-17 (`docs/decisions.md`): "not all hardware supports all
features", and a game that uses something its target lacks doesn't
export there. [ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§1 turns that into data and a check:

- `TargetProfile` ([ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
  §1, built by 3001) gains `capabilities`;
- each module's metadata ([ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
  §4, built by 2003) gains `needs`;
- `shuyi check --target <id>` fails when a game turns on a module, or a
  stage uses a feature, that the target can't run. The error names the
  module or stage, what it needs, and what the target has;
- an `optional` part (an HD-2D effect, [ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
  §7) is left out and listed, never failed;
- the engine never drops a required feature silently.

This is the same rule as the controls check (3002): what doesn't fit
doesn't ship. The GBA and DS tickets (3019–3026) and the 3D tickets
(3104, 3107) all read these capabilities, so this comes first.

**From 3101 (open):** Parked: 3101's `ModuleInfo` `needs` takes `three_d: software` here when this lands first; otherwise this ticket adds it to every module that exists then.

## Nick input

None.

## Scope

**In:**
- **`Capabilities`** in `shuyi_assets::target`, a new field of
  `TargetProfile`:
  - `colour`: `Rgba8`, `OneBit`, or `Indexed { banks, colours_per_bank,
    bits_per_pixel }`;
  - `blend`: `Full` (every `Blend` mode and `Paint::Tinted`, drawn as
    `shuyi_ui::frame::blend` says, 2337), `OneBit` (add as lighten,
    multiply as darken, tint drawn as the image: the one-bit renderer's
    rule), or `None` (blending is per layer and a tint is a palette
    swap; 3021 and 3024 confirm);
  - `screens`: a list of `Surface { id, size, touch }`, where `size` is
    `Any` or `Fixed(w, h)` in device pixels;
  - `three_d`: `None`, `Software { max_triangles }`, `Hardware {
    max_triangles, max_texture_bytes, effects }` or `Full`;
  - `sprites`: `None` (no limit) or `Some(SpriteLimits { max,
    max_per_line })`, with `max_per_line` an `Option`;
  - `memory`: `ram_bytes` and `video_ram_bytes`, each an `Option<u64>`
    (`None` means no fixed limit; the heap budget still applies);
  - `audio`: `channels` (`Option<u8>`), `music` (`Streamed` or
    `Mixed`), and `features`: the set of `AudioFeature`s the target's
    backend plays beyond cues and music, which are `Pause`, `Rate`,
    `Pan` and `LoopRegion`. They are named after the provided methods of
    3029's shared backend trait: `pause` and `resume`, which 3029 adds,
    and the rate, the pan and the loop region that later tickets add to
    it (3029 leaves the rate to 3030's mixer; the pan is 2363's and the
    loop region 2902's follow-up's, all parked). A backend ticket that
    implements one adds it to its profile.
- **The voice limit.** The cue rules' voice limit is the smaller of the
  audio manifest's `max_voices` (2378 adds it) and the target's
  `channels`, a `None` on either side being no limit. `shuyi
  check --target` warns when a game's `max_voices` is over its target's
  `channels`: a warning in the target's report, which doesn't fail.
  **Either order with 2378:** if 2378 is done, `App` caps `max_voices`
  with `channels` here; if not, nothing to do (2378 reads `channels`
  when it lands).
- **`display.color` moves to `capabilities.colour`.** The profile format
  has no version field (read 2026-10-20). `display.color` is still read
  for one more minor release (2005's rule), so a game's own profile
  keeps loading. **Either order with 2517:** if 2517 is done, list the
  move as a `Change::manual` of `TargetRon` (`Format::changes`), so
  `shuyi check` warns and `shuyi upgrade` says what to do, and add the
  target profile kind to `Importers::standard()` so `upgrade` finds a
  game's own profiles; if not, 2517 lists it when it lands. 3006's one-bit
  check (done; `check --target` runs it on every `color: OneBit`
  target) reads the new field in this PR.
- **The run-time facts read the same profiles.** 3053's
  `PlatformFacts::from_profile` reads `quit`, `display.windowed` and
  `input`'s `pointer`, `touch` and `keyboard`. If this ticket moves a
  field it reads (touch into `screens`' `Surface.touch`, say), change
  `from_profile` in the same PR, so check time and run time read one
  place. If 3053 isn't done, it reads them where this ticket leaves
  them.
- **Every profile gets its capabilities**, with a comment on each
  provisional number naming the ticket that checks it:

  | Profile | `colour` | `blend` | `screens` | `three_d` | `sprites` | `memory` (RAM / video RAM) | `audio` |
  | ------- | -------- | ------- | --------- | --------- | --------- | -------------------------- | ------- |
  | `desktop` | `Rgba8` | `Full` | one, `Any` | `Full` | none | none / none | none, `Streamed` |
  | `web` | `Rgba8` | `Full` | one, `Any` | `Full` (WebGL 2) | none | none / none | none, `Streamed` |
  | `steam-deck` | `Rgba8` | `Full` | one, 1280 × 800, `touch: false` | `Full` | none | 16 GB / none | none, `Streamed` |
  | `playdate` | `OneBit` | `OneBit` | one, 400 × 240 | `Software { max_triangles: 300 }` (prov., ADR-0011 §8) | none | 16 MB / 0 | none, `Streamed` |
  | `switch` (planned) | `Rgba8` | `Full` | one, 1280 × 720 | `Full` | none | to be filled by 3014 | to be filled by 3014 |
  | `android`, `ios` (planned) | `Rgba8` | `Full` | one, `Any`, `touch: true` | `Full` (prov.) | none | none / none | none, `Streamed` |

  The Steam Deck's screen is a touchscreen, but Deck Verified wants
  nothing to need it (ADR-0008 §1), so it says `touch: false` with a
  comment saying why.

  The `switch` row's `memory` and `audio` are 3014's to fill.
  **Either order with 3014:** if 3014 is done, fill them from its
  public note `docs/targets/switch.md` (no limit where it gives none);
  if not, write no limits (`None`, `Streamed`, no features) with a
  comment naming 3014, which fills them (its scope says so).

  Each profile's audio `features` comes from what its backend plays on
  the day. For quad-snd 0.2.8 (desktop, web, Steam Deck) it is `[]`:
  quad-snd can't pause (2394 mutes instead), change the rate, pan or
  loop a region. A ticket that gives that backend a feature (2902's
  chosen stack, say) adds it. **Either order with 3029:** if 3029 is
  done, the Playdate's lists the provided methods its backend
  implements (`Pause` if the SDK can pause, say); if not, it is `[]`,
  and 3029 adds them. The GBA's and the DS's are provisional, from
  3027 and 3024.
- **Three new profiles, `status: planned`**, from ADR-0012 §2 and §3, so
  that the bake (3019), the CI builds (3020) and the two-screen work
  (3023) have a real profile to read. The backend tickets (3021, 3024)
  check every value and complete them:
  - `targets/gba.ron`: 240 × 160, `Indexed { banks: 16,
    colours_per_bank: 16, bits_per_pixel: 4 }`, `blend: None`,
    `three_d: None`, sprites `{ max: 128, max_per_line: Some(32) }`
    (prov., 3018), RAM 294 912 (32 KB + 256 KB) and video RAM 98 304,
    audio 2 channels `Mixed`, text 8 / 8 px, 60 / 30 fps, inputs as in
    ADR-0012 §2, `backend: shuyi-gba`;
  - `targets/nds.ron`: two surfaces of 256 × 192, the lower one
    `touch: true`; `colour` is the 2D layers' `Indexed { 16, 16, 4 }`,
    and the 3D engine's 15-bit colour is part of `three_d`;
    `blend: None`; `three_d: Hardware { max_triangles: 2048,
    max_texture_bytes: 524288 (prov.), effects: [fog, edge_marking,
    toon, alpha] }`; sprites
    `{ max: 128 }` per screen; RAM 4 MiB, video RAM 671 744 (656 KB);
    audio 16 channels `Mixed` (prov., 3018); text 8 / 10 px; inputs as
    in ADR-0012 §3; `backend: shuyi-nds`;
  - `targets/dsi.ron`: as `nds` (`blend: None` too), with 16 MiB of
    RAM.
- **`Needs`**, in `shuyi_assets::target`, one type read from three
  places:
  - a module's `[package.metadata.shuyi.needs]` (2003's `ModuleInfo`
    gets the field);
  - the game manifest, for the whole game and per stage
    (`stages.<id>.needs`), for example `sprites = { at_once = 200 }`;
  - a skin's settings file, through the skin's format, which gives its
    `Needs` (the fixture skin here writes the table with its `optional`
    list; 3107's diorama skin builds it from its effects, each with
    `optional: bool`).

  Its fields: `colour` (`Any`, `Indexed { min_colours }` or `Rgba8`),
  `blend` (the `Blend` modes a skin uses, each optional or required),
  `three_d` (a level, and optionally `triangles` and `effects`),
  `sprites` (`at_once`, `per_line`), `ram_bytes`, `touch`, `audio` (a
  list of `AudioFeature`s, each one required or optional). Every field
  is optional, and an empty table means "runs on every target". An
  `optional` list may be in any of the three places: it names the
  effects, blend modes and audio features its source can do without.
- **Every module states its needs.** `cargo xtask check-modules` (2003)
  fails when a module crate has no `needs` key at all, so "runs
  everywhere" is said on purpose, never by forgetting. The catalog
  `docs/modules.md` shows each module's needs.
  - **Either order with 2205** (parked: the `shuyi-midi` module): if
    2205 is done, give `shuyi-midi` its `needs` here with the other
    modules: `ram_bytes` for its largest fixture bank plus the synth,
    provisional; if not, nothing to do (2205 writes its own `needs`).
  - **Either order with 2707:** if 2707 is done, `shuyi-rts` has no
    `needs` key yet: step 4 gives it a provisional one with the other
    module crates, with a comment that it is provisional until measured;
    if not, nothing to do (2707 writes its own provisional key).
  - **Either order with 2721** (`shuyi-vn`, parked): if 2721 is done,
    `shuyi-vn` is one of the module crates above: give it an empty
    `needs` table, with a comment that 2726 checks it against the
    example; if not, nothing to do (2721 adds it).
  - **Either order with 2726** (parked, the visual novel example): if
    2726 is done, check `shuyi-vn`'s `needs` against what
    `examples/visual-novel` uses, and correct them. If not, nothing to
    do (2726 checks them).
  - **Either order with 2727** (parked, `shuyi-mystery`): if 2727 is
    done, give `shuyi-mystery` its `needs` key: what `shuyi-vn` needs,
    plus nothing more. If not, nothing to do (2727 writes its own).
- **The check**, a pure function `capabilities_fit(profile: &TargetProfile,
  needs: &[(NeedSource, Needs)]) -> FitReport`, wired into `shuyi check
  --target <id>` (an operation of the tool contract, 1129):
  - `NeedSource` is a module, a stage, the game, or a skin;
  - `FitReport` has `problems` (errors) and `left_out` (optional parts
    the target can't run);
  - `three_d` levels order as `None < Software < Hardware < Full`; a
    triangle count above the target's `max_triangles` fails; an effect
    not in the target's `effects` fails unless it is optional, and then
    it goes in `left_out`; `Full` has every effect;
  - `colour: Rgba8` fails on `Indexed` and `OneBit`; `Indexed {
    min_colours }` fails on `OneBit`, and on an `Indexed` target with
    fewer colours;
  - sprites, RAM and touch compare numbers and flags;
  - `blend`: `Full` and `OneBit` have every mode (`OneBit` by its
    rule), and `None` has `Alpha` only. A required `blend` mode a target
    lacks fails, and an optional one goes in `left_out`;
  - an audio feature that is not in the target's `features` fails,
    unless it is optional, in which case it goes in `left_out`. Example:
    a game that changes a sound's rate on a target whose backend has no
    `Rate`.
- **The error text** names the source, the need and what the target
  has, for example: "module shuyi-3d needs three_d: software; target
  gba has three_d: none", and "stage battle needs 200 sprites at once;
  target gba has 128".
- **`shuyi export <id>` refuses** a game with any problem, through the
  check it already runs (3004). **Either order with 3004:** if 3004 is
  done, add `left_out` to its `export-report.json`, beside the check's
  warnings; if not, nothing to do (3004's export refuses through the
  check, and 3004 adds `left_out` when it lands).

**Out (do not do):**
- the screen-count and layout check (3023);
- the controls check (3002 has it);
- quantizing images to a palette (3019);
- any backend (3021, 3024);
- measuring or setting budgets (3008, 3020);
- making the engine drop or swap a feature by itself (ADR-0012,
  *Alternatives*);
- a version field in the profile format, or a `Migrate` chain for it:
  2110's chain is for saves, and 2005's rule keeps the old name read.

**Either order with 2504** (the new-game wizard's targets, ADR-0013 §5):
if 2504 is done, wire `capabilities_fit` into the wizard's composer as
2504 describes, replacing its `limits` entry ("modules not checked
against targets"), add each module's `needs` and each target's
capabilities to `shuyi new --questions --json` and its schema, and add
the `gba`, `nds` and `dsi` profiles to its targets question as planned,
with their tickets (3021 for `gba`, 3024 for `nds` and `dsi`). If not,
2504 calls it and adds them to the JSON when it lands.

**Either order with 2337** (tint and blend modes): if 2337 is done,
`blend` and the skin's `blend` need are in this PR as above. If not,
leave both out and add a line to 2337 saying it adds them, with the
rule in *The check*.

**Either order with 2326** (screen effects on the wgpu renderer,
parked): if 2326 is done, `Capabilities` gains `shaders: bool` (true
on the profiles whose backend runs the wgpu renderer, false on
`playdate`, `gba`, `nds` and `dsi`), and `Needs` gains `screen_passes`,
the passes of a skin's `effects.ron` (2326's `EffectChain`), each
required unless marked `optional`. On a target without `shaders`, a
required pass fails the check and an optional one goes in `left_out`,
with a test for each. If not, nothing to do here (2326 adds them).

**Either order with 2327** (a game's own shaders and materials,
parked): if 2327 is done, `Capabilities` gains `shaders: bool` (false
on `playdate`, `gba`, `nds` and `dsi`, true on the others) and a skin's
`Needs` gains `materials`, required or optional. A required one fails
on a target without shaders; an optional one goes in `left_out`, and
its sprites are drawn without it, or as its `fallback` sprite. If not,
nothing to do (2327 adds both when it lands, with the rule in
*The check*).

**Either order with 2328** (2D lights, parked): if 2328 is done, add
a `lights` need, always optional, and the `shaders: bool` capability
it is checked against (false on `playdate`, `gba`, `nds` and `dsi`)
if this PR has none yet. 2328's lit layers have that need, so
`left_out` lists them where they are drawn unlit. If not, nothing to
do (2328 adds them when it lands).

**Either order with 2718** (the RTS budgets): if 2718 is done, give
`shuyi-rts` its `needs`: `ram_bytes` from `rts-battle-1k`'s peak heap
(in the crate's README, and here if 2718 wrote it), rounded up and
marked provisional. If `Needs` can't say "too slow for this target",
say so in that README and write a ticket for a CPU need. If not,
nothing to do (2718 sets them).

**Either order with 2909, 2910 and 2911** (research on video, voice
chat, and ray- and path-traced light and sound): each one names a
capability some targets lack (video playback, a microphone,
`ray_tracing`). If one is done, it wrote its capability into this
ticket: add it to `Capabilities`, `Needs` and *The check* as its ADR
says. If not, nothing to do here: when it lands, it writes the
capability its ADR calls for into this ticket, or into a parked build
ticket if this one is done.

## Implementation steps

1. Add `Capabilities`, `Surface`, `BlendCaps`, `ThreeD`, `SpriteLimits`,
   `Memory`, `AudioCaps` and `AudioFeature` to
   `crates/assets/src/target.rs`, with `display.color` still read for
   one release and, if 2517 is done, listed as a change. Extend the
   property test on random bytes (ADR-0007).
2. Add `capabilities` to the seven existing files in `targets/`, and
   write `gba.ron`, `nds.ron` and `dsi.ron` from ADR-0012.
3. Add `Needs` and its decoding from TOML (module metadata, the
   manifest) and RON (skin files).
4. Add `needs` to `ModuleInfo` and the rule to `check-modules`; add the
   needs column to `docs/modules.md`. Give every existing module crate a
   `needs` key.
5. Write `capabilities_fit` with a test per rule, then wire it into
   `check --target`, its JSON output and the CLI's text table.
6. Add fixture games: one with a 3D module on `gba`, one stage with 200
   sprites, one skin with an optional and a required effect on `nds`,
   one game that needs `audio: [Rate]`.

## Acceptance criteria

- [ ] Every file in `targets/` decodes, and a test checks the `gba` and
      `nds` values against ADR-0012 §2 and §3.
- [ ] A profile in the old format (with `display.color`) still loads,
      and if 2517 is done `shuyi check` warns on it (fixture).
- [ ] A fixture game with a module that needs `three_d: software` fails
      `shuyi check --target gba` with the module, the need and
      "three_d: none" in the message, and passes on `desktop`.
- [ ] A stage that needs 200 sprites at once fails on `gba`, naming the
      stage.
- [ ] On `nds`, a skin with `bloom` optional and `fog` required passes,
      and `bloom` is listed in `left_out`. With `bloom` required, it
      fails.
- [ ] If 3004 is done, `shuyi export gba` refuses the first fixture,
      and prints the check's errors.
- [ ] `cargo xtask check-modules` fails on a fixture module with no
      `needs` key.
- [ ] The JSON output matches its schema (1129's schema test).
- [ ] A fixture game that needs `audio: [Rate]` fails `shuyi check
      --target` on a profile without `Rate`. With `Rate` optional, it
      passes and lists `Rate` in `left_out`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each rule of `capabilities_fit`, passing and failing; the
  `three_d` ordering; the `blend` rule passing and failing; the audio
  features rule; the voice limit is the smaller of `max_voices` and
  `channels`; the warning when `max_voices` is over `channels`; an old
  profile's `display.color` still read.
- Property: random `Needs` against random profiles never panic, and an
  empty `Needs` fits every profile.
- Integration: `shuyi check --target` on the fixture games, through the
  CLI and the JSON contract.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
