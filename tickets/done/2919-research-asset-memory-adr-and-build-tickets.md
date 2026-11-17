---
id: "2919"
title: "Research: asset memory, part two: decide lifetimes, reading on demand, asset groups, loading with progress and freeing textures in an ADR from 2906's numbers, and write the build tickets they call for"
type: research
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2906"]
nick_input: none
completed: 2026-11-16
---

# 2919 — Research: asset memory, part two: the ADR and the build tickets

## Context

Split from 2906 by its own size rule: its measuring program passed 600
changed lines before its step 8, so its ADR, its build tickets and its
updates to other tickets come here. This ticket starts from
[`docs/research/2906-asset-memory.md`](../../docs/research/2906-asset-memory.md),
which holds every number, its method, and how to run
`examples/hello-grid/examples/asset_memory.rs` again. Read 2906
(`tickets/done/`) for the whole background: the comparison with Bevy,
Godot, Unity, Unreal, GameMaker and Defold, and what Shuyi had on
2026-10-20.

**What 2906 measured** (2026-11-16, engine `eea1e160`):

- **The Playdate.** A pack at the `playdate` profile's `package_bytes`
  (40 MB) takes 67 MB of heap read whole and kept, as
  `shuyi_playdate::read_pack` reads it today. The heap budget is 10 MB of
  16 MB.
  - In the proportions of the game's public assets, a pack read whole
    holds 1.6 times its bytes, so one over about 6 MB doesn't fit.
  - The SDK loads the program into RAM, so embedding the pack doesn't
    help.
  - The file API can `seek` and `read` part of a bundled file.
  - Read file by file, the 40 MB pack peaks at 0.6 MB.
- **The web.** The page fetches the whole pack before the `.wasm`
  (`crates/cli/web/pack-file.js`), so the card waits for every byte.
  - A pack at the `web` budget (25 MB) puts the card at 9.5 s at
    25 Mbit/s, against the 5 s start-up. About 12 MB of pack is the most
    that fits.
  - The page's JS heap keeps a second copy of the pack for the whole run.
  - macroquad's loader can't send a `Range` request; a JS plugin of our
    own can.
- **Decoded values.** The game's public sounds decode to about 1.9 MB
  from 0.4 MB of files. Its 33 pictures take 3.5 MB as textures from 23 KB of
  PNG files, uploaded at start in about 10 ms native and 5–8 ms on the
  web.
- **The game's public assets** (0.6 MB packed) fit every budget in every
  way.

**What changed while 2906 waited**, which this ticket's ADR starts from:

- ADR-0097 (3097) added a load in steps (`Start::in_steps`, `Progress`,
  `LoadStep`). Its slices are time from the platform (`load_slice_ms`),
  and its card shows no progress.
- 2207 and 2220 (done) lay run-time layers, leaked once
  (`Layer::owned`), and reload files on disk.
- 3010 (done) reads the Playdate's pack whole.
- 3008 (done) gates the bench's peak heap.
- 3064 and 3065 (done) count the heap, with memory tags.
- 3300's `pack-file.js` fetches a release export's pack beside the page.

## Nick input

None. This is technical.

## Scope

**Before the first release** (Nick, 2026-11-14: no run of breaking
versions after the MVP): 2005 waits for this ticket. Each build ticket
it writes that changes the bundle's or the formats' public API goes in
2005's `blocked_by` in the same PR, and its milestone is MVP. A build
ticket that only adds stays after the MVP.

**In:**

- **An ADR** (the `write-adr` skill, the next free number) that extends
  ADR-0023 §3, ADR-0024 §6, ADR-0097 and ADR-0008 §6, and decides each
  point below with 2906's numbers:
  1. **Lifetimes.** What stays `&'static`. Whether run-time layers and
     packs hand out bytes borrowed from the bundle, shared bytes
     (`alloc::sync::Arc<[u8]>`, which needs pointer-sized atomics that
     the parked GBA's `thumbv4t` lacks), a handle into an asset store
     the engine owns, or stay leaked. Whether paths stay `&'static`
     (2906: about 50 bytes a file) while bytes don't.
     - Say what each option means for `Bundle`, `DecodeCx`,
       `Formats::load_under` and `load_each`, `ImageId`, `ImageTable`,
       `AssetKind: 'static`, `AppConfig::files`, `Layer::owned` (2207)
       and 2228's `Bundle::versions`.
     - Count the call sites each option changes, in the engine and in the
       game's public code.
  2. **Reading on demand.** Whether a pack is read file by file through
     a platform hook: a `Platform` method in
     `crates/platform/src/platform.rs` that reads a byte range of a file
     beside the game. That is a file seek on native, the SDK's `seek` and
     `read` on the Playdate (2906: section 7.4 of *Inside Playdate with
     C*), and a `Range` fetch from a JS plugin on the web. Say what the
     pack needs for it: its index read alone, and a check per file, since
     today's checksum covers the whole pack (a `pack::VERSION` bump).
  3. **Asset groups.** What a group is: a list of paths in data, a
     folder, or what a stage or screen declares. Judge each against a
     Playdate game that keeps one part's pictures in memory at a time,
     and a web game that shows its title before the rest of its art has
     arrived.
  4. **Loading in steps, with progress.** Whether ADR-0097's load in
     steps reports progress, counting work units, never time (a file
     decoded, as `Formats::load_each` reads a folder file by file). Say
     what the card shows (a progress form of `Card`), and what a backend
     does between steps (a fetch on the web, an SDK read on the
     Playdate). No threads and no async asset server (ADR-0007,
     *Alternatives*). Say whether the macroquad backend's own loading
     off the frame (music) stays a backend detail. **Either order with
     3009:** if it is done, use `shuyi_core::think::Progress`; if not,
     count units as 3009 plans.
  5. **Freeing textures.** Whether a backend uploads a texture on first
     draw and frees it with its group, or keeps uploading everything at
     start, and how a dropped group reaches the backend. The same for
     recoloured copies and 2222's atlas pages.
  6. **The budgets.** For `playdate` and `web`, what must not stay in
     memory, and what must arrive after the first frame. Whether the
     `web` profile's start-up means the loading card or the game's first
     frame, and whether the page fetches the `.wasm` before the pack.
     Note the parked `nds` row. A budget's number changes only with a
     device measurement (ADR-0008 §6), so this PR changes words only:
     the ADR, and the comments in `targets/*.ron` if they are wrong.
  7. **Triggers.** For each build (lifetimes changed, reading on demand,
     groups loaded with progress, freeing textures, the page's fetch
     order), the measured number that calls for it.
- **The build tickets the numbers call for** (the `write-ticket` skill):
  22xx for the bundle, packs and groups; 30xx for a platform hook, the
  web page's fetch, and a backend that frees textures.
  - After MVP when a target that isn't parked needs it (`web`,
    `playdate`, `desktop`).
  - `Parked:` (the title prefix, `status: blocked` and the note) when
    only parked work or a parked target (`nds`) needs it. An After MVP
    build ticket is never blocked by a parked one.
  - A trigger not crossed is written in the ADR with its number, and no
    ticket is written for it.
  - In `docs/ROADMAP.md`, a build ticket the game doesn't need goes
    after the game's work (`docs/decisions.md`, 2026-10-20: built for
    one game first).
  - **Either order with 2368 (parked):** if the ADR decides an asset
    store the engine owns, the store's build ticket also shows resident
    bytes per asset kind in the debug view, which 2368's Out leaves to
    that store. If 2368 is done, the build ticket adds them to 2368's
    `memory` channel. If not, it says so in an either-order clause with
    2368, and 2368's Out line names it. With no store, nothing to do.
- **Tickets this changes:**
  - Update 2222, 2228 and 3011 to match the ADR, or say in the
    completion notes why one needs no change. 3011 reports the peak heap
    during the load as well as in play.
  - 2207, 2220, 3008 and 3010 are done: what the ADR asks of each is a
    build ticket. For 3008, that is a bench scenario over the load, if
    the load's peak matters. For 3010, it is reading the pack on demand,
    if the ADR decides it.
- **Either order with 2005:** the first release may publish today's
  `&'static` API. Its policy is 0.x while the API moves, so a change of
  lifetimes after it is a break that 2010's `Breaking:` line names.
  Neither ticket waits for the other beyond 2005's `blocked_by`.
- **Either order with 2909** (parked, video playback): if 2909 is done,
  weigh what a video streamed from the pack needs (the note 2909 left
  here, from its ADR) in points 1 and 2; if not, nothing to do (2909
  uses this ADR's terms).
- `docs/adr/README.md`: the new ADR's row, and "extended by ADR-NNNN" on
  the rows of ADR-0008, ADR-0023, ADR-0024 and ADR-0097.
- A line in `docs/research/2906-asset-memory.md` that names the ADR.

**Out (do not do):**

- building any of it (the build tickets do);
- measuring again, unless a number the ADR needs is missing: then run
  2906's program and add the number to its page;
- threads, or an async asset server (ADR-0007, *Alternatives*);
- audio decisions: music and voice already load on demand
  (`crates/app/src/audio.rs`), 3029 streams music on the Playdate, and
  2902 chose the sound plan. The ADR may note sound residency, not
  change that plan;
- ROM targets: 3021 (parked) reads its pack in place;
- streaming a world or its levels around a camera (1124 loads a world's
  levels whole): a note in the ADR only;
- GPU texture compression (2901), content downloaded after install (DLC,
  a CDN), and encrypted or signed packs (ADR-0023, *Alternatives*);
- the web page's download progress bar (3052);
- how a game's files are embedded (2215, parked);
- the peak-heap gate in CI (3008), and the heap shown to a developer in
  the debug view (2368, parked);
- changing any budget's number (3011 and the device tickets do, with
  measurements).

## Implementation steps

1. Read `docs/research/2906-asset-memory.md`, 2906, ADR-0007, ADR-0008
   §5 and §6, ADR-0019, ADR-0023, ADR-0024 §6, ADR-0097, and
   `bundle.rs`, `pack.rs`, `format.rs`, `image.rs` in `shuyi-assets`,
   `crates/platform/src/app.rs` and `platform.rs`,
   `crates/app/src/render.rs`, `pack_file.rs` and `audio.rs`,
   `crates/playdate/src/backend.rs`, `crates/cli/web/pack-file.js`, and
   tickets 2207, 2220, 2222, 2228, 3008, 3009, 3010 and 3011.
2. Count the call sites each lifetime option changes, in the engine and
   in the game's public code (`gh search code --repo
   Zafnok/visions-of-shuyi`).
3. Write the ADR and update `docs/adr/README.md`.
4. Write the build tickets the triggers call for, update 2222, 2228 and
   3011, add the API-changing build tickets to 2005's `blocked_by`, and
   add the build tickets to `docs/ROADMAP.md`.
5. Run the gates.

## Acceptance criteria

- [x] The ADR decides all seven points, each with its numbers from
      `docs/research/2906-asset-memory.md`.
- [x] Each trigger the numbers cross has a build ticket in
      `tickets/open/`; each one not crossed is in the ADR with its
      number.
- [x] 2222, 2228 and 3011 match the ADR, or the completion notes say
      why one needs no change; what the ADR asks of 2207, 2220, 3008 and
      3010 is a build ticket, or the notes say why none is needed.
- [x] Each build ticket that changes the bundle's or the formats' public
      API is in 2005's `blocked_by`, milestone MVP.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None: an ADR and tickets. The measuring program's tests (2906) keep
  running in `cargo test --workspace`.

## Completion notes

**What was done:**

- [ADR-0126](../../docs/adr/0126-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
  decides the seven points from 2906's numbers, and extends ADR-0008 §6,
  ADR-0023 §3, ADR-0024 §6 and ADR-0097 (their status lines and rows in
  `docs/adr/README.md` say so):
  1. **Lifetimes:** a read hands out `Cow<'static, [u8]>` (borrowed from
     every layer of today, owned from a pack read file by file); paths
     stay `&'static` (52 bytes a file); no asset store; `Layer::owned` and
     hot reload stay leaked. The call sites of each option are counted in
     the ADR, in the engine and in the game.
  2. **Reading on demand:** a pack with its index read alone and a
     checksum per file (`pack::VERSION` 17), read through a
     `pack::Source` the backend implements. The Playdate reads with the
     SDK's seek and read; the web streams the pack; desktop keeps reading
     it whole.
  3. **Groups:** a folder, laid at the front of the pack by
     `Bake::first`, with `Bundle::arrived(dir)`.
  4. **Progress:** `Progress::Of { done, of }` counts units, never time,
     and `Progress::Wait` ends a frame's steps; the card shows a bar.
  5. **Textures:** made when first drawn; never freed until textures
     pass a quarter of `peak_heap_bytes` (the game's are 3.8 MB).
  6. **Budgets:** `startup_ms` is to the game's first frame; what must not
     stay on `playdate` and `web`; the page fetches the `.wasm` beside the
     pack. The `nds` row is noted. Only the comment on `startup_ms` in
     `targets/web.ron` changed.
  7. **Triggers:** a table of each build, its number, whether it is
     crossed, and its ticket.
- **Build tickets:** 2262 (MVP, the one break, in 2005's `blocked_by`),
  2263, 3315, 3316, 3317 and 3318 (After MVP), and 2264 (parked: only the
  DS's heap needs it). They are in `docs/ROADMAP.md`: 2262 before 2005,
  the others in a section of their own after the game's work, 2264 with
  the DS.
- **Tickets changed:** 2222 (a page is a texture kept for the run;
  either order with 3317), 2228 (`LayerCopy`'s lifetimes; either order
  with 2262), 3011 (peak heap during the load and in play, and how the
  pack was read; either order with 3315), 2005 (2262 in `blocked_by`, its
  clause rewritten), 3009 (its clause rewritten; either order with 3316),
  3052 (either order with 3318), 1124 (its Out line names ADR-0126), and
  the parked 2368, 2909 and 3024 (their clauses only).
- **Done tickets:** 2207 and 2220 need no build: their layers stay
  leaked, bounded by what is handed in and what a developer saves
  (ADR-0126 §1). 3008 needs no bench scenario over the load: its peak is
  the program's high-water from its start, so it already holds the
  load's (`crates/cli/src/bench.rs`, `measured_start`). 3010's whole read
  becomes 3315. 2909 isn't done, so there was no video note to weigh.
- `docs/research/2906-asset-memory.md` names the ADR.

**Deviations:**

- **The platform hook is a `pack::Source` trait, not a `Platform`
  method** (point 2): the bundle's readers run inside the game's load and
  screens, which have a `Setup` or a `Ctx`, not the `Platform`. The ADR's
  *Alternatives* say so.
- **The web streams one GET, not `Range` requests:** the web's heap holds
  the whole pack (42 MB of 256 MB), and a stream works on every host,
  Python's `http.server` included. The order of the bytes (`Bake::first`)
  gives the first frame its files first.
- **New tickets in 33xx, not 30xx:** 30xx filled up on 2026-11-08
  (`tickets/README.md`).
- No number was measured again: every one the ADR needs is on 2906's page.

**Follow-up tickets:** 2262, 2263, 2264, 3315, 3316, 3317 and 3318.

**Notes for Nick:** nothing to decide. The engine gets one break before
the first release (2262: the bundle hands out its bytes so a pack needn't
stay in memory). The rest waits until a game needs a Playdate pack over
about 6 MB or a web pack over about 12 MB; Visions of Shuyi's is 0.6 MB.

No break: an ADR, tickets, and a comment in `targets/web.ron`; no public
item changes.
