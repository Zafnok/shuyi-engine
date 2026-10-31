---
id: "2906"
title: "Research: asset memory: measure what stays resident, decide lifetimes, asset groups and loading in steps in an ADR, and write the build tickets the numbers call for"
type: research
milestone: After MVP
model: reasoning
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2906 — Research: asset memory

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker and Defold. Every one of
them can load assets while the game runs, report progress, and let
assets leave memory:

- **Bevy:** `AssetServer` loads in the background and hands out
  ref-counted handles with a load state; an asset is dropped with its
  last strong handle
  (<https://docs.rs/bevy/latest/bevy/asset/struct.AssetServer.html>).
- **Godot:** `ResourceLoader.load_threaded_request` with a progress
  array, and resource packs loaded at run time
  (<https://docs.godotengine.org/en/stable/classes/class_resourceloader.html>).
- **Unity:** Addressables load and release groups of assets by label.
- **Unreal:** the Asset Manager and `FStreamableManager` load bundles on
  demand
  (<https://dev.epicgames.com/documentation/en-us/unreal-engine/asset-management-in-unreal-engine>).
- **GameMaker:** texture groups and audio groups the game loads and
  frees (<https://manual.gamemaker.io/monthly/en/Settings/Texture_Groups.htm>).
- **Defold:** collection proxies load and unload a part of a game
  (<https://defold.com/manuals/collection-proxy/>).

**What Shuyi has today** (read on 2026-10-20):

- `crates/assets/src/bundle.rs`: one bundle per process (`install`,
  `get_or_install`, `static INSTALLED: OnceBox<Bundle>`), and every read
  is `&'static`: `Bundle::bytes`, `Bundle::file`, `Bundle::files_under`,
  `Bundle::files_in`, `Bundle::test_part`. A layer is `Files::Dir` (an
  `include_dir!` directory) or `Files::Table` (a `&'static` table).
- `crates/assets/src/pack.rs`: `Pack::open` checks the whole pack and
  unpacks every compressed file at once. A stored file borrows the
  pack's bytes (`Cow::Borrowed`); an LZ4 file is unpacked into memory of
  its own.
  `Pack::into_owned` copies every file, for a pack read at run time.
  `Pack<'static>::into_layer` leaks the paths, the unpacked files and
  the table (`Box::leak`). The checksum covers every byte after the
  header, so checking it reads the whole pack.
- `crates/assets/src/format.rs`: `DecodeCx::bytes`, `DecodeCx::text` and
  `DecodeCx::files_under` hand out `&'static` data; `Formats::load_under`
  and `Formats::load_each` key their maps by `&'static str`;
  `AssetKind: Sized + 'static`. In `crates/assets/src/image.rs`,
  `ImageId(&'static str)` and `ImageTable::images: BTreeMap<&'static
  str, ImageInfo>` hold paths borrowed from the bundle.
- `crates/platform/src/app.rs`: a game's `Load<G>` is one
  `FnOnce(Setup)` run on frame 2 under the `Card` (ADR-0024 §6). No
  steps, no progress. `Card` is `#[non_exhaustive]`.
- `crates/app/src/render.rs`: `Renderer::new` (line 72) uploads the font
  atlas and a texture for every image of the `ImageTable` at start. Its
  `images` and `recoloured` maps are never emptied, and it keeps the
  game's file function (`bytes`, from `AppConfig::files: fn(&str) ->
  Option<&'static [u8]>`) to make recoloured copies, so the PNG bytes
  stay beside their textures.
- `crates/app/src/audio.rs`: the one precedent. Music and voice are
  read from a folder beside the executable or the page when needed and
  freed when they stop (`folder_beside`, `platform_folder`). Sounds are
  embedded and decoded once at start.
- `docs/formats.md`, *Measurements*: today's packs are small (Hello,
  Grid's is 20,631 bytes; the game's public assets about 600 KB).

[ADR-0023](../../docs/adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md)
§3 and *Consequences*: "Streaming is out of scope (ticket 2204). A
ticket comes if measurements show a need." No ticket measures it. This
is that ticket.

**What is planned:**

- 2207 lays run-time layers that are "leaked once", and 2220 leaks old
  bytes on reload in debug builds. Both are bounded, and both keep the
  `&'static` rule.
- 3064 counts the heap (`shuyi-alloc-count`, in every debug build); 3008
  gates peak heap against each target's `peak_heap_bytes`; 3011
  measures peak heap on a Playdate.
- 3010 reads the Playdate's pack through the SDK's file API, so the
  whole pack and its unpacked files sit in the heap. If this ticket is
  done first, 3010 reads it as this ticket's ADR says.
- 3029 streams music through the Playdate SDK's file players.
- 3021 (parked) reads a GBA pack in place from ROM.
- 2222 uploads atlas pages again when they change, and never frees one.
- 3052 shows the web build's download progress in the page.
- 2005 publishes the crates, with a version policy of 0.x while the API
  moves.

**Why it matters:**

- **The budgets don't fit the model.** `targets/playdate.ron` allows a
  40 MB package and a 10 MB heap. ADR-0012's parked `nds` row allows a
  64 MB ROM and a 2.5 MB heap. Today every file a game embeds or packs
  stays in memory for the whole run, with its decoded value beside it;
  only music and voice, read from beside the game, don't.
  `targets/web.ron` allows a 25 MB download (26,214,400 bytes) and a
  5 s start at 25 Mbit/s, but those bytes take about 8.4 s at 25 Mbit/s,
  so a game at its download budget can't have every byte before its
  first frame. Files embedded in a wasm live in its linear memory for
  the whole run.
- **The game first.** Visions of Shuyi's public assets are small, its
  music already loads beside it, and its own Playdate port (game#1025)
  waits until after the launch. So this ticket measures and decides; it
  builds nothing. The numbers say whether builds come, and when (Nick,
  2026-10-20: the engine is built for one game first).
- **Other 2D games** with more art than a Playdate's heap, or a web game
  that should show its title before all of its art has downloaded.
- **The end goal** (Nick, 2026-10-19): an agent makes a game "with a web
  ver, a playdate ver". A Playdate build of a game with a few megabytes
  of art needs an answer here.
- **The API is cheapest to change now.** `&'static` reads run through
  `Bundle`, `DecodeCx` and kinds such as `ImageId`. They get dearer to
  change once outside `Format` plugins exist (2005 publishes the crates;
  2526 adds outside modules).

## Nick input

None. This is technical.

## Scope

**In:**
- **A measuring program**, `examples/hello-grid/examples/asset_memory.rs`
  (an `[[example]]` with `test = true` in
  `examples/hello-grid/Cargo.toml`, so `cargo test --workspace` runs its
  tests). It prints with `writeln!` to a locked `std::io::stdout()`,
  since the workspace's lints warn on `println!`:
  - a counting allocator that reports the current and the peak heap and
    can reset the peak, installed as the program's
    `#[global_allocator]`. A static of a type that implements
    `GlobalAlloc` needs no `unsafe` of its own, but the type does
    (`unsafe impl`), and the workspace's lints forbid `unsafe_code` in
    `examples/` (ADR-0002 §9). **From 3064 (done):** use its counting
    allocator, `CountingAlloc` in `shuyi-alloc-count` (ADR-0046 §9 names
    the crate). Its `global` static is the `#[global_allocator]` of
    every debug build that links `shuyi-app`, as this program does
    through `shuyi`, and a second static would not link there: read that
    one (`global::reading()`, and 3065's tags, `global::rows()`) in a debug
    build, and declare a static of the program's own only where it is
    compiled out (a release build; `global::COUNTING` says which);
  - the packs: Hello, Grid's baked in memory as
    `examples/hello-grid/tests/it/pack.rs` bakes it (`HelloTools.bake`),
    the others read from a file or written in memory (below). Each is
    leaked before the counter's peak is reset, so its bytes count as a
    binary's do under `include_bytes!`, and only the "read whole" way
    counts them;
  - three ways a game can hold a pack, each run under the counter:
    1. **embedded:** the leaked pack opened with `Pack::open` and
       `into_layer`, then every asset loaded, as `Hello::from_pack`
       does (`examples/hello-grid/src/lib.rs:168`);
    2. **read whole:** the pack read into a `Vec<u8>`, then `Pack::open`,
       `into_owned`, `into_layer` and the load: what 3010 plans on the
       Playdate, and what a page that fetches its pack would do;
    3. **on first read:** computed from measured parts, with the
       arithmetic shown: the index, the largest file's stored and
       unpacked bytes, the decoded values that stay, and the load's own
       peak above them. Nothing in `shuyi-assets` changes for it;
  - for each way, the peak during the load and what stays after it,
    split into: the raw bytes kept (baked data, PNG files, sounds, other
    files) and the decoded values;
  - run on:
    - Hello, Grid (its files and its pack);
    - the game's public assets: Visions of Shuyi's `assets/` folder from
      a checkout of its public repository at a recorded commit, with the
      same engine kinds baked as `docs/formats.md` baked. Only numbers
      are committed, never a file of the game (`AGENTS.md` rule 4);
    - synthetic packs at the `web` (`package_bytes: 26_214_400`) and
      `playdate` (`41_943_040`) budgets, written in memory with
      `pack::write` and `Content::Raw`: incompressible bytes from
      `shuyi_core::rng::SimRng` for pictures and sounds, and repeated
      text for data, in the proportions of the game's public assets. A
      flag writes one to a file for the web runs below. Nothing is
      committed.
- **The web**, by hand in Chromium (the desktop app's browser), as
  `docs/formats.md`'s web numbers were; 2019's headless Chromium
  (`tools/web-check/check.py`, done) may drive the runs:
  - the `.wasm` size of Hello, Grid from `cargo shuyi web --release`
    (run in `examples/hello-grid`) with its files embedded (today) and
    with them left out (a scratch build), so the bytes the files add are
    known;
  - the time from navigation to the loading card, and to the game's
    first frame, with the network throttled to 25 Mbit/s, for: today's
    build; a scratch build that fetches its pack from beside the page
    with macroquad's `load_file` (as `crates/app/src/audio.rs` fetches
    music); and that build with the synthetic `web` pack. If the browser
    can't throttle, time the runs unthrottled and add each download's
    bytes at 25 Mbit/s, and say so;
  - whether a page can fetch part of a file (an HTTP `Range` request)
    without `wasm-bindgen` (`AGENTS.md`, *Environment*): through
    macroquad's loader, or a JS plugin of our own. A finding, not built.
- **Textures in `shuyi-app`**, native and web: for Hello, Grid, and for
  every PNG of the game's public assets uploaded as `Renderer::new` does
  (a scratch run): the count, the bytes (width × height × 4) and the
  time `Renderer::new` takes, read from the backend's clock and logged,
  never handed to the game.
- **The Playdate, from the SDK's documentation** (no device): whether the
  program's binary is loaded into the device's RAM, so that files
  embedded with `include_bytes!` count against the heap as a pack read
  into it does; and whether the SDK's file API can seek and read part of
  a file in the `.pdx`. Record the sections read.
- **`docs/research/asset-memory.md`** (create `docs/research/` if no
  other ticket has): the method, every number, the machine, the
  commits, and how to run the program again.
- **An ADR** (the `write-adr` skill, the next free number) that extends
  ADR-0023 §3, ADR-0024 §6 and ADR-0008 §6, and decides, with the
  numbers:
  1. **Lifetimes.** What stays `&'static`. Whether run-time layers and
     packs hand out bytes borrowed from the bundle, shared bytes
     (`alloc::sync::Arc<[u8]>`, which needs pointer-sized atomics that
     the parked GBA's `thumbv4t` lacks, or a handle into an asset store
     the engine owns), or stay leaked. Whether paths stay `&'static` (small, leaked once)
     while bytes don't. What it means for `Bundle`, `DecodeCx`,
     `Formats::load_under` and `load_each`, `ImageId`, `ImageTable`,
     `AssetKind: 'static`, `AppConfig::files`, and 2228's
     `Bundle::versions` if it is done. Count the call sites each option
     changes, in the engine and in the game's public code.
  2. **Reading on demand.** Whether a pack is read file by file through
     a platform hook (a `Platform` method in
     `crates/platform/src/platform.rs` that reads a byte range of a file
     beside the game: a file seek on native, the SDK's file API on the
     Playdate, a `Range` request on the web), and what the pack needs
     for it: its index read alone, and a check per file, since today's
     checksum covers the whole pack (a `pack::VERSION` bump).
  3. **Asset groups.** What a group is: a list of paths in data, a
     folder, or what a stage or screen declares. Judge each against two
     generic uses: a Playdate game that keeps one part's pictures in
     memory at a time, and a web game that shows its title before the
     rest of its art has arrived.
  4. **Loading in steps, with progress.** Whether a load becomes a value
     stepped over frames in the style of 3009's `Think` (`step(units)
     -> Progress`), counting work units, never time; a unit could be a
     file decoded, as 2219's `Formats::load_each` reads a folder file by
     file. What the card shows (a progress form of `Card`), and what a
     backend does between steps (a fetch on the web, an SDK read on the
     Playdate). No threads and no async asset server (ADR-0007,
     *Alternatives*). Whether the macroquad backend's own loading off
     the frame (music: a coroutine on the web, a thread on native) stays
     a backend detail. **Either order with 3009:** if it is done, use
     `shuyi_core::think::Progress`; if not, count units as 3009 plans.
  5. **Freeing textures.** Whether a backend uploads a texture on first
     draw and frees it with its group, or keeps uploading everything at
     start, and how a dropped group reaches the backend. The same for
     recoloured copies and 2222's atlas pages.
  6. **The budgets.** For `playdate` and `web`, what must not stay in
     memory, or what must arrive after the first frame, and whether the
     `web` profile's start-up means the loading card or the game's
     first frame. Note the parked `nds` row. A budget's number changes
     only with a device measurement (ADR-0008 §6), so this PR changes
     words only: the ADR, and the comments in `targets/*.ron` if they
     are wrong.
  7. **Triggers.** For each build (lifetimes changed, reading on demand,
     groups loaded in steps with progress, freeing textures), the
     measured number that calls for it.
- **The build tickets the numbers call for** (the `write-ticket` skill):
  22xx for the bundle, packs and groups; 30xx for a platform hook and
  for a backend that frees textures.
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
- **Tickets this changes:** update 2207, 2220, 2222, 2228, 3008,
  3010 and 3011 to match the ADR, or say in the completion notes why
  one needs no change. 3008 gets a bench scenario over the load if the
  load's peak matters; 3011 reports the peak heap during the load as
  well as in play. **Either order with each of them:** if one is done,
  measure what it built where it applies (the layers 2207 and 2220
  keep), and the change the ADR asks of it is a build ticket.
- **Either order with 2005:** the first release may publish today's
  `&'static` API. Its policy is 0.x while the API moves, so a change of
  lifetimes after it is a break that 2010's `Breaking:` line names.
  Neither ticket waits for the other.
- **Either order with 2909** (parked, video playback): if 2909 is done,
  weigh what a video streamed from the pack needs (the note 2909 left
  here, from its ADR) in points 1 and 2 of the ADR above; if not,
  nothing to do (2909 uses this ADR's terms).
- `docs/adr/README.md`: the new ADR's row, and "extended by ADR-NNNN" on
  the rows of ADR-0008, ADR-0023 and ADR-0024.
- **Size:** if the diff passes about 600 changed lines when step 8
  starts, the ADR, the build tickets and the seven tickets' updates
  (steps 8 and 9) become a follow-up research ticket in this milestone,
  blocked by this one, which starts from
  `docs/research/asset-memory.md`; the PR says so.

**Out (do not do):**
- building any of it (the build tickets do);
- threads, or an async asset server (ADR-0007, *Alternatives*);
- audio decisions: music and voice already load on demand
  (`crates/app/src/audio.rs`), 3029 streams music on the Playdate, and
  2902 (parked) studies sound. Sounds are measured, not decided;
- ROM targets: 3021 (parked) reads its pack in place;
- streaming a world or its levels around a camera: real-time work,
  parked. 1124 loads a world's levels whole and leaves streaming to a
  ticket written when a game asks. A note in the ADR only;
- GPU texture compression (2901), content downloaded after install (DLC,
  a CDN), and encrypted or signed packs (ADR-0023, *Alternatives*);
- the web page's download progress bar (3052);
- how a game's files are embedded (2215, parked);
- the peak-heap gate in CI and its bench scenarios (3008), and the heap
  shown to a developer in the debug view (2368, parked);
- the largest shipped files and the files no load reads (2516);
- changing any budget's number (3011 and the device tickets do, with
  measurements).

## Implementation steps

1. Read ADR-0007, ADR-0008 §5 and §6, ADR-0019, ADR-0023, ADR-0024 §6,
   `bundle.rs`, `pack.rs`, `format.rs`, `image.rs`,
   `crates/platform/src/app.rs`, `crates/app/src/render.rs` and
   `audio.rs`, `examples/hello-grid/tests/it/pack.rs`, and tickets
   2207, 2220, 2222, 2228, 3008, 3009, 3010, 3011 and 3029.
2. Write the measuring program and its tests, and add its `[[example]]`
   to `examples/hello-grid/Cargo.toml`.
4. Check out Visions of Shuyi's public repository outside this one, and
   record its commit.
5. Run the program on Hello, Grid, the game's public assets and the
   synthetic packs, release builds, native.
6. Measure the web and the textures by hand, and read the Playdate SDK's
   documentation.
7. Write `docs/research/asset-memory.md`.
8. Write the ADR and update `docs/adr/README.md`.
9. Write the build tickets the triggers call for, update 2207, 2220,
   2222, 2228, 3008, 3010 and 3011, and add the build tickets to
   `docs/ROADMAP.md`.
10. Run the gates.

## Acceptance criteria

- [ ] `docs/research/asset-memory.md` gives, for Hello, Grid, the game's
      public assets and both synthetic packs, the peak and the resident
      heap of each of the three ways, split into raw bytes by kind and
      decoded values.
- [ ] It gives the web numbers (the `.wasm` with and without the files,
      the times to the card and to the first frame at 25 Mbit/s), the
      textures' count, bytes and upload time, and the Playdate SDK's
      answers with their sources.
- [ ] `cargo run -p hello-grid --example asset_memory --release`
      prints the native numbers again, and the document says how to run
      it on the game's assets.
- [ ] The ADR decides all seven points, each with its numbers.
- [ ] Each trigger the numbers cross has a build ticket in
      `tickets/open/`; each one not crossed is in the ADR with its
      number.
- [ ] 2207, 2220, 2222, 2228, 3008, 3010 and 3011 match the ADR, or
      the completion notes say why one needs no change.
- [ ] The program uses 3064's counting allocator
      (`shuyi-alloc-count`); no `unsafe` is in `examples/`.
- [ ] No file of the game is committed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, if this ticket writes the counting allocator: its current and
  peak heap, and its reset, on known allocations, through an instance
  called directly (not the global one, which other test threads use).
- Unit: the synthetic pack is the same bytes for the same seed, and
  `Pack::open` opens it.
- Integration: the program's three ways run on a small synthetic pack
  inside `cargo test --workspace`, so the program can't rot. It checks
  relations (what stays is at most the peak; every way reports every
  part), never exact byte counts.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
