# ADR-0126: Asset memory: bytes handed out, not leaked; a pack read file by file; folders as groups; a load that counts and waits

- **Status:** Accepted
- **Date:** 2026-11-16
- **Extends:** ADR-0023 §3 (a pack's layer), ADR-0024 §6 (the loading
  phase), ADR-0097 (a load in steps), ADR-0008 §6 (the budgets)
- **Related tickets:** 2919 (this); 2906 (the numbers,
  [`docs/research/2906-asset-memory.md`](../research/2906-asset-memory.md));
  the build tickets 2262, 2263, 2264 (parked), 3315, 3316, 3317 and 3318;
  2005 (the first release), 2207, 2220, 2222, 2228, 3008, 3009, 3010,
  3011, 3052; 2368 and 2909 (parked)

## Context

Today every file of a game stays in memory for the whole run, with its
decoded value beside it. Every read of the bundle is `&'static`
(`Bundle::bytes`, `Bundle::file`, `DecodeCx::bytes`, `DecodeCx::text`),
and a layer handed in at run time is leaked once (`Layer::owned`, 2207;
`Pack::into_layer`). Ticket 2906 measured where that breaks
(`docs/research/2906-asset-memory.md`, 2026-11-16, engine `eea1e160`):

- **The Playdate.** A pack at the `playdate` profile's `package_bytes`
  (40 MB) takes **67 MB** of heap read whole and kept, as
  `shuyi_playdate::read_pack` reads it. The heap budget is 10 MB of
  16 MB. A pack read whole holds 1.6 times its bytes in the game's
  proportions, so **one over about 6 MB doesn't fit**. Embedding it
  doesn't help: the launcher loads the program into the same RAM. The
  SDK's file API can `seek` and `read` part of a bundled file (*Inside
  Playdate with C*, section 7.4). Read file by file, the same pack
  peaks at **0.6 MB**, and keeps 0.4 MB: its table (about 52 bytes a
  file, 376 KB for 7,248 files) and the decoded values.
- **The web.** The page fetches the whole pack before the `.wasm`
  (`crates/cli/web/pack-file.js`), so the card waits for every byte. A
  pack at the `web` budget (25 MB) puts the card at **9.5 s** at
  25 Mbit/s, against `startup_ms` 5,000. About **12 MB** of pack is the
  most that fits before the first frame. With the `.wasm` first, the card
  shows at **1.2 s**. The page's JS heap keeps a second copy of the pack
  for the whole run (27.9 MB against 1.6 MB). Read whole and kept, the
  pack takes 42 MB of linear memory, against a 256 MB peak-heap budget.
  macroquad's loader can't ask for a byte range; a JS plugin of our own
  can, and the host must answer `Range` (Python's `http.server`
  doesn't).
- **Decoded values.** The game's public sounds decode to **1.9 MB** from
  0.4 MB of files. Its 33 pictures take **3.5 MB** as textures (3.8 MB
  with Hello, Grid's) from 23 KB of PNG files, uploaded at start in
  **10 ms** native and 5–8 ms on the web.
- **The game.** Visions of Shuyi's public assets (0.6 MB packed) fit
  every budget in every way. Nothing here is needed for its launch, and
  the engine is built for one game first (`docs/decisions.md`,
  2026-10-20).

What has changed since 2906 was written: ADR-0097 loads in steps
(`Start::in_steps`, `Progress`, `LoadStep`), sliced by time from the
platform (`load_slice_ms`), with a card that shows no progress. 3008's
bench gates peak heap, and its peak is the program's high-water from
its start, so it holds the load's (`crates/cli/src/bench.rs`,
`measured_start` tags it `load`). 3064 and 3065 count the heap with
memory tags. 3300's `pack-file.js` fetches a release export's pack.

Nick, 2026-11-14: no run of breaking versions after the MVP. So what
breaks the bundle's or the formats' public API is built before 2005,
and what only adds waits until a game needs it.

### The call sites, counted

Counted on 2026-11-16 in this repository (code, not tests, not doc
comments), and in the game with `gh search code --repo
Zafnok/visions-of-shuyi` (files):

| Option | Engine | The game |
| ------ | ------ | -------- |
| Bytes stay leaked (`&'static`) | none | none |
| Bytes borrowed from the bundle (`&'b`) | none through `installed()`, which is `&'static`; tests that make a bundle on the stack | none |
| Bytes handed out as `Cow<'static, [u8]>` | 16 reads in 12 files; the `&'static [u8]` signatures that carry them (45 lines, 20 in `shuyi-app`: `AppConfig::files`, `Renderer::new`'s `bytes`, `sources`); about 110 lines of tests | its wrapper (`content::bundle::file`, `bytes`, `test_part`) and the files that call it: 32 through `file`, 11 through `bytes`, 7 through `test_part`, and 3 that read a `Bundle` directly; or one line in its wrapper, if it keeps `&'static` by leaking what isn't borrowed |
| Shared bytes (`Arc<[u8]>`) | the same as `Cow` | the same as `Cow` |
| A handle into a store the engine owns | the reads above, and every decoded value: `Formats::load`, `load_under`, `load_each`, `ImageId` and `ImageTable` (72 lines in 19 files) | the reads above, and its loaders: `load_each` in 9 files, `ImageId` in 20 |

Paths are a separate question: `files_in`, `files_under`, the keys of
`load_under` and `load_each`, `ImageId` and `ImageTable` hold
`&'static str` paths, and none of the first four options changes them.

## Decision

### 1. Lifetimes: bytes are handed out, paths stay `&'static`

- **A read hands out `Cow<'static, [u8]>`** (`alloc::borrow::Cow`):
  `Bundle::bytes` returns `Option<Cow<'static, [u8]>>`, `Bundle::file`
  `Option<Cow<'static, str>>`, and so do `Bundle::test_part`'s text,
  `LoadFile::text`, `DecodeCx::bytes` and `DecodeCx::text`.
  - An embedded layer, a table, a run-time layer and a pack read whole
    hand out `Cow::Borrowed`: no copy, as today.
  - A layer read file by file (§2) hands out `Cow::Owned`: the reader
    decodes it and drops it.
- **Paths stay `&'static`.** A layer's paths are leaked once when it is
  made, at about 52 bytes a file (376 KB for the 7,248 files of a 40 MB
  pack, 3.6% of the Playdate's heap). `files_in`, `files_under`,
  `ImageId`, `ImageTable`, and the keys of `Formats::load_under` and
  `load_each` don't change. Trigger to change them: §7.
- **`AssetKind: 'static` stays.** A decoded value is the game's own, and
  it is dropped when the game drops it. The engine keeps **no asset
  store**: no handles and no reference counts.
- **`Layer::owned` (2207) stays leaked**, and hot reload's old bytes
  (2220) stay leaked in debug builds: a run-time layer is made once, and
  a reload is bounded by what a developer saves. Both hand out
  `Cow::Borrowed`.
- **What each item takes:**
  - `Bundle`: `bytes`, `file`, `test_part`, `load_each`'s `LoadFile`
    change; the listings, `has`, `left_out` and `display_path` don't;
  - `DecodeCx`: `bytes` and `text` change; `files_under` doesn't;
  - `Formats::load_under` and `load_each`: no change (their keys are
    paths);
  - `ImageId`, `ImageTable`: no change;
  - `AppConfig::files` becomes `fn(&str) -> Option<Cow<'static, [u8]>>`,
    and `Renderer`'s `sources` keep the `Cow` they were given;
  - 2228's `LayerCopy`: `layer: &'static str` stays, `bytes` is a
    `Cow<'static, [u8]>`, and `text()` gives `Option<Cow<'static, str>>`.
- **Built by 2262, before the first release** (2005 waits for it): it
  breaks the bundle's and the formats' public API, so after 2005 it would
  be a 0.x break.

### 2. Reading on demand: a pack read file by file, through a source

- **The pack gains an index read alone and a check per file**
  (`pack::VERSION` 17, built by 2263):
  - the header's checksum covers the header and the index only;
  - each entry gains the checksum of its stored bytes, checked when that
    file is read;
  - `pack::Index::read(prefix)` reads the header and the index from the
    pack's first bytes, and says how many it needs when given too few;
  - `Pack::open` still reads a whole pack, checking every file, for a
    pack embedded or read whole.
- **A layer over a source.** `shuyi_assets::pack::Source` is a trait a
  backend implements: `fn read(&self, at: u64, into: &mut [u8]) ->
  Result<(), SourceError>`, where `SourceError::NotYet` says the bytes
  haven't arrived. `Index::into_layer(name, &'static dyn Source)` makes a
  layer that answers `has` and the listings from the index, and reads,
  checks and unpacks one file per `Bundle::bytes`, handing it out as
  `Cow::Owned`.
- **Not a `Platform` method.** The reads come from loaders inside the
  game's load and screens, through the bundle, which has no `Platform`
  (the load gets a `Setup`, a screen a `Ctx`). The backend makes the
  source when it opens the pack, and the bundle calls it; `shuyi-assets`
  still does no I/O (ADR-0002), it calls a trait.
- **Each backend's source:**
  - **the Playdate** reads file by file through the SDK's `seek` and
    `read` on the pack in the `.pdx` (3315): `read_pack` stops reading
    it whole;
  - **the web** streams the whole pack into linear memory in the order
    the bake laid it (§3, 3318). A file whose bytes haven't arrived is
    `NotYet`. A streamed GET needs no `Range`, so it works on every host;
  - **desktop** keeps reading its pack whole (`pack_beside_program`): it
    has no package budget and 512 MB of heap. Trigger: §7.
- **Sounds** keep their plan: music and voice load beside the game
  (`crates/app/src/audio.rs`), 3029 streams music on the Playdate, and
  2902 chose the sound plan. Sound effects stay decoded at start on
  desktop and the web: the game's are 1.9 MB.

### 3. Asset groups: a group is a folder

- **A group is a folder of the bundle** (`pictures/part2/`). A game
  loads one with the calls it has (`Formats::load_under`,
  `Formats::load_each`, `Bundle::files_under`) and drops it by dropping
  its values. No group type, no list of paths in data, and nothing a
  screen declares.
- **The bake lays folders in an order the game names.**
  `Bake::first(dir)` (2263) puts a folder's files at the front of the
  pack, in the order of the calls, and the rest follow in path order. The
  index records each folder's span. ADR-0118's layouts still apply
  within those spans.
- **`Bundle::arrived(dir) -> bool`** (2263) says whether every file under
  `dir` can be read now: always on a layer read whole or from the
  Playdate's file, and once its span has arrived on the web.
- **Judged against the two games:**
  - **a Playdate game that keeps one part's pictures at a time** loads
    `pictures/part2/` on entering the part and drops its map on leaving.
    Its pack is read file by file (§2), so nothing of it stays but the
    index; its one-bit pictures are its own values, handed to the frame
    through `OneBitLook` (the Playdate backend makes no textures);
  - **a web game that shows its title before the rest of its art** names
    `Bake::first("title/")`. The page streams the pack, and the load
    waits (§4) only until `title/` has arrived; a screen that opens a
    later part asks `arrived` first and shows its own wait.
- **What lost:**
  - **a list of paths in data** copies the folder tree, must be kept in
    step with the files, and needs a format and a check of its own;
  - **what a stage or screen declares** ties the group to code that runs:
    the bake can't read it without running the game, and a screen asks
    where it runs only through `ctx.platform` (ADR-0060).
- **Streaming a world's levels around a camera** (1124 loads a world's
  levels whole) is groups loaded and dropped as the camera moves, with
  each level a folder. A ticket when a game asks for it.

### 4. Loading in steps, with progress counted in units

- **A step reports units, never time.** `Progress` (ADR-0097) gains two
  variants, additive since it is `#[non_exhaustive]` (3316):
  - `Progress::Of { done, of }`: a unit done, `done` of `of`. A unit is
    what the load counts, such as a file decoded, as `Formats::load_each`
    reads a folder file by file;
  - `Progress::Wait`: the step waits for the platform (a file not
    arrived, `SourceError::NotYet`). `App` runs no more steps that frame
    and presents the card; the next frame calls the same step again.
    Without it a web load, which runs every step in one frame (no
    slice), would spin forever.
- **Not `think::Progress`.** 3009 isn't done. A load's `Progress<G>`
  carries the game it made, so it stays `shuyi-platform`'s own type; it
  counts units as 3009 plans for bots (a unit is the caller's, never
  time), and asks nothing of 3009.
- **The slice stays time.** `load_slice_ms` still decides how many steps
  a Playdate frame runs (ADR-0097 §5); the units only say how far the load
  is.
- **The card shows the count.** `Frame` (`#[non_exhaustive]`) gains
  `loading: Option<(u32, u32)>`, the last `Of` a step gave, and each
  backend draws a bar under the card in the card's ink (`shuyi-app`, the
  Playdate's display); the headless backend keeps the count with the
  card, for tests.
- **A recorded run.** A wait ends a frame even in a recorded or hashed
  run (ADR-0097 §4). 3316 makes a recording count the frames before the
  game's first as one (ADR-0072), so a run recorded on the web plays
  back on the headless backend, where nothing waits.
- **What a backend does between steps:** the web hands the chunks the
  page received to the pack's source (3318); the Playdate does nothing,
  its reads run inside the steps; desktop and the headless backend do
  nothing.
- **No threads and no async asset server** (ADR-0007, *Alternatives*).
  The macroquad backend's own loading off the frame (music, sounds
  decoded once `App::game` is `Some`, ADR-0024 §6) stays a backend
  detail.

### 5. Textures: made when first drawn, kept until a trigger

- **`shuyi-app` makes a picture's texture when it is first drawn**, not
  for every image of the `ImageTable` at start (3317). On the web the
  pack streams, so a picture may arrive after the first frame. A picture
  whose bytes haven't arrived is drawn as nothing that frame.
- **Textures are never freed**, as today, until the trigger in §7: the
  game's pictures take 3.8 MB as textures, against 256 MB on the web and
  512 MB on desktop. When it is crossed, a texture is freed when its
  image leaves the frame's `ImageTable`, with its recoloured copies.
- **Recoloured copies** are made when a sprite first needs one, as today,
  and freed with their texture under the same trigger.
- **2222's atlas pages** are textures like pictures: uploaded when first
  drawn and when they change, and freed with their folder's images under
  the same trigger.
- **The Playdate** draws one-bit pictures the game holds: it has no
  textures to free.

### 6. The budgets

- **`startup_ms` is to the game's first frame**, on every profile: the
  title screen 3011 measures, not the card. The card is what shows
  meanwhile (ADR-0024 §6).
- **`playdate`:**
  - must not stay in memory: the pack's bytes (§2). What stays is the
    index (52 bytes a file) and the values the game keeps;
  - must arrive after the first frame: nothing. Reads are synchronous,
    and the load runs a second's worth of steps a callback.
- **`web`:**
  - must not stay: the page's JS copy of the pack (3318). The pack in
    linear memory may: about its own bytes once LZ4 files are unpacked
    per read (§1), against 256 MB;
  - must arrive after the first frame: everything but the folders the
    game names first, once the pack passes about 12 MB;
  - **the page fetches the `.wasm` and the pack together**, and starts
    the game once the `.wasm` and the pack's index have arrived (3318).
    The card shows at about 1.2 s, whatever the pack's size.
- **The parked `nds` row** (ADR-0012: a 2.5 MB heap, a 64 MB ROM) reads a
  pack file by file through the same `Source`. Its table alone, at
  52 bytes a file, would be about 575 KB for 64 MB in the game's
  proportions: 23% of its heap (§7, 2264).
- **No number changes** (ADR-0008 §6): `targets/web.ron`'s `startup_ms`
  comment says it is to the first frame.

### 7. Triggers

| Build | The number that calls for it | Crossed | Ticket |
| ----- | ----------------------------- | ------- | ------ |
| Bytes handed out (§1) | a non-parked profile whose `package_bytes` is more than its heap holds read whole: `playdate`, 40 MB against about 6 MB | yes | 2262 (MVP) |
| A pack read file by file (§2), folders laid first (§3) | the same, and a `web` pack over about 12 MB before the first frame against 25 MB | yes | 2263 |
| The Playdate reads its pack file by file | a Playdate pack over about 6 MB, against 40 MB | yes | 3315 |
| A load that counts and waits, the card's bar (§4) | the web's card at 1.2 s and its first frame at up to 9.6 s with a 25 MB pack, and a streamed pack that needs a wait | yes | 3316 |
| Textures made when first drawn (§5) | a streamed pack (3318), where a picture arrives after the first frame | yes | 3317 |
| The page's fetch order, the pack streamed, no JS copy | a `web` pack over about 12 MB (9.5 s at 25 MB, against 5 s); 26 MB of JS copy | yes | 3318 |
| Paths not `&'static` (the index read from the source) | the table over a tenth of a profile's heap: `playdate` 3.6%, the parked `nds` 23% | only `nds` | 2264 (parked) |
| Desktop reads its pack file by file | a desktop pack held whole over a quarter of `peak_heap_bytes` (128 MB): a pack over about 80 MB | no | none |
| Textures freed (§5) | textures over a quarter of `peak_heap_bytes` on `web` (64 MB) or `desktop` (128 MB): the game's are 3.8 MB | no | none |
| Textures made on the fly for time | uploading at start over a fiftieth of `startup_ms` (100 ms on `web`): the game's take 10 ms | no | none |
| A store the engine owns, resident bytes per asset | a value two parts of a game share, which neither can drop: none in the engine or the game | no | none |
| A bench scenario over the load (3008) | the bench's peak not holding the load's: it does (the high-water since the program started) | no | none |

## Consequences

- A Playdate game's pack may grow to its 40 MB budget once 3315 is
  built: it holds its index and what its game keeps, not the pack.
- A web game's card shows in about a second whatever its pack's size, and
  its first frame waits only for the folders it names first.
- **Before the first release, 2262 breaks** `Bundle::bytes`, `file`,
  `test_part`, `LoadFile::text`, `DecodeCx::bytes`, `DecodeCx::text` and
  `AppConfig::files`. A reader that kept the `&'static` it got (the
  game's `content::bundle` wrapper) keeps a `Cow` or copies; a game that
  only decodes passes `&bytes`. 2262's `Breaking:` lines and migration
  blocks say so.
- 2263 bumps `pack::VERSION`: packs are baked again, as ADR-0023 §5
  says.
- A layer read file by file costs a seek, a read, a check and an unpack
  per file read: slower than a pack held whole, so desktop doesn't.
- `Pack::open` keeps checking every file of a pack read whole; a pack
  read file by file checks each file when it is read, so a broken file is
  found when it is first read, not at start.
- No store: the engine can't say which asset is resident. 2368's resident
  bytes per asset stay out; its `memory` channel shows the heap and its
  tags.
- Nothing changes for Visions of Shuyi's launch: none of these builds
  is in its way, so they come after its work in `docs/ROADMAP.md`, but
  2262.

## Alternatives considered

- **Bytes stay leaked.** No call site changes. A layer read file by file
  would leak each file it reads, so a run that reads its whole pack holds
  it whole again: the Playdate's 6 MB ceiling comes back, later.
- **Bytes borrowed from the bundle (`&'b [u8]`).** Free through
  `installed()`, but a file read on demand has nothing to borrow from
  unless the bundle keeps what it read, which is a store that never
  frees while anything borrows.
- **Shared bytes (`Arc<[u8]>`).** The same call sites as `Cow`, but an
  embedded file can't be an `Arc` without a copy into the heap, which
  undoes the embedded way (its stored files borrow the program's bytes),
  and the parked GBA's `thumbv4t` has no pointer-sized atomics. A file's
  bytes go to one decoder, so nothing needs to share them.
- **A handle into an asset store the engine owns** (Bevy's
  `AssetServer`, Unity's Addressables). It would track what is resident,
  but every decoded value, `ImageId` and loader changes, and the game
  already owns its values (ADR-0019): a store would hold a second copy or
  take them over. No measured value needs sharing between parts.
- **A `Platform` method that reads a byte range.** The bundle's readers
  have no `Platform`; the source trait gives the same reads where they
  are made.
- **Range requests on the web.** They read only what a group needs, but
  the host must answer `Range` (Python's `http.server` doesn't), and the
  web's heap holds the whole pack at 42 MB of 256 MB. A streamed GET
  works everywhere and gives the same first frame.
- **A pack per group.** One file per folder means many requests on the
  web and many open files on the Playdate (64 at most), and ADR-0110
  ships one pack.
- **Progress as time** (the slice's share). A slow device would show a
  different bar for the same work, and a bar should say how much is
  done, as 3009 counts a bot's work.
- **`startup_ms` to the card.** It would let a game show a card for
  10 seconds within budget; 3011 and 3090 measure to the title screen.
