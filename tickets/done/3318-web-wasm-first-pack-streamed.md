---
id: "3318"
title: "The web page fetches the .wasm beside the pack, streams the pack into the game in the order it was baked, and keeps no copy of it in JS"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2262", "2263", "2265", "3316", "3317"]
nick_input: none
completed: 2026-11-18
---

# 3318 — The web: the `.wasm` first, the pack streamed

## Context

[ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§2, §3 and §6, from ticket 2906's numbers
([`docs/research/2906-asset-memory.md`](../../docs/research/2906-asset-memory.md),
*The web*):

- `index.html` calls `shuyi_fetch_pack(name, start)`
  (`crates/cli/web/pack-file.js`, 3300), which fetches the whole pack and
  only then the `.wasm`. The card is the `.wasm`'s first frame, so it
  waits for every byte: a pack at the `web` budget (25 MB) puts it at
  9.5 s at 25 Mbit/s, against `startup_ms` 5,000, which is to the game's
  first frame (ADR-0128 §6). About 12 MB of pack is the most that fits.
- With the `.wasm` first, the card shows at about 1.2 s, and the first
  frame waits only for the files it reads.
- `pack-file.js` keeps the bytes it fetched for the whole run: the page's
  JS heap held 27.9 MB with the synthetic pack, against 1.6 MB, a second
  copy beside the one in the `.wasm`'s linear memory.
- macroquad's loader can't ask for a byte range; a streamed GET
  (`response.body.getReader()`) needs no `Range`, so it works on every
  host, Python's `http.server` included.
- 2263 gives the pack's index alone, `pack::Source` with `NotYet` and
  the layer over it (`Index::read(prefix, stamp)`, a `Source` that is
  `Send + Sync + RefUnwindSafe`, failures listed by `Layer::problems()`);
  **from 2265 (done):** the pack is version 19; `Bake::first(dir)` lays
  a folder's files at the front, `Index::spans()` gives each folder laid
  first and its byte range from the pack's start, in the order named
  (a span may hold more than its folder, when folders nest),
  `Source::arrived(at, len)` (default
  `true`) is what this ticket's source answers from how far it is
  filled, and `Bundle::arrived(dir)` asks it for the folder's span, or
  for each file of a folder not laid first; 3316 gives
  `Progress::Wait` and the card's count; 3317 makes a texture when its
  picture is first drawn.

## Nick input

None.

## Scope

**In:**

- **The page** (`crates/cli/web/pack-file.js`, `index.html`):
  - starts the pack's fetch and the `.wasm`'s together, and starts the
    game once the `.wasm` is loaded and the pack's header and index have
    arrived (`Index::read` says how many bytes that is; the plugin asks
    the Rust side, or reads the header's length itself);
  - reads the rest of the pack as a stream, and hands each chunk to the
    game, keeping none once handed over;
  - a failed fetch is a problem the game shows, as `PackFileError` does
    today; a host that answers without a body stream gives the whole
    response at once, as today.
  - The plugin's `VERSION` and `PLUGIN_VERSION` move together (the test
    that compares them stays).
- **The source** (`crates/app/src/pack_file.rs`): a `pack::Source` over a
  buffer of the pack's whole length, made once the index arrives and
  leaked for the run (one copy, in linear memory). Between frames,
  `shuyi-app`'s web backend copies the chunks the page received into it,
  in order, and marks how far it is filled; `read` answers `NotYet`
  beyond that, and `arrived` from it.
- **A stream that breaks:** when the fetch fails or the body ends short
  after the game has started, the page tells the source, which marks
  itself failed: a read past what arrived answers `SourceError::Failed`
  (the file reads as missing and `Layer::problems()` lists it once), and
  `arrived` answers `true`, as `SourceFiles::arrived` counts a failed
  file as arrived. A load waiting on a folder then reads it, gets the
  missing file and fails with its error, instead of waiting forever.
- **Either order with 2267:** if 2267 is done, the source implements
  `Source::lend` for the bytes that have arrived (they never change
  again), so a read of a stored file copies nothing; if not, nothing to
  do (2267 adds `lend` to this source).
- **`pack_beside_program`** on the web gives the layer over that source
  (2263's `Index::into_layer`) in place of the pack read whole. On
  desktop it stays as it is.
- **The load waits:** a game's load in steps that needs a folder returns
  `Progress::Wait` until `Bundle::arrived(dir)` (3316), and gives
  `Progress::Of` from the bytes arrived, so the card counts the download.
  Hello, Grid's packed web build does so, with `Bake::first` for the
  folders its first frame reads.
- **The export** (`crates/cli/src/export/web.rs` or where the page is
  written): unchanged but for the page's script.
- **Docs:** `docs/formats.md` or the book's chapter on shipping for the
  web: the order of the fetch, `Bake::first`, and that a game waits for a
  folder with `arrived`.
- **Either order with 3052:** if 3052 is done, its page progress bar
  counts the `.wasm` and the pack's index (what the page waits for before
  the game starts), and the rest of the pack shows on the card (3316);
  if not, nothing to do (3052 counts what the page fetches before the
  game starts).

**Out (do not do):**

- `Range` requests (ADR-0128, *Alternatives*);
- reading a desktop pack file by file (ADR-0128 §7: not crossed);
- the page's progress bar and its errors (3052);
- freeing textures (ADR-0128 §7: not crossed).

**Estimated size:** about 900 lines (code 400, tests 300, docs 200).

## Implementation steps

1. Write the page's stream and the plugin's calls that hand chunks over.
2. Write the source and the backend's copy of the chunks between frames.
3. Give `pack_beside_program` the streamed layer on the web.
4. Move Hello, Grid's packed web build onto `Bake::first`, `arrived`,
   `Wait` and `Of`; write the docs.
5. Run the gates.

## Acceptance criteria

- [x] In the browser (the built-in pane, frames driven by
      `wasm_exports.frame()` if it is hidden), Hello, Grid's release export
      with the synthetic `web` pack
      (`examples/hello-grid/examples/asset_memory.rs --write-synthetic
      web`) shows its card before the pack has arrived: the resource
      timings show the `.wasm` fetched beside the pack, not after it.
- [x] Its card counts the download, and its first frame comes once the
      folders it names first have arrived, before the rest.
- [x] The page's JS heap, read once the game runs, holds less than 5 MB
      with the synthetic pack, against 27.9 MB before (the completion
      notes give both).
- [x] The source answers `NotYet` past what arrived and the bytes before
      it (unit tests, off the browser).
- [x] A stream marked failed at half the pack answers `Failed` past what
      arrived and `arrived` true, and a load in steps waiting on a folder
      past it ends with the missing file's error, not `Wait` (unit tests).
- [x] Served by Python's `http.server`, which answers no `Range`, the
      game runs.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the source over chunks handed in order; `arrived`; a stream
  that fails half way; the plugin's version test.
- Browser, by hand: the timings and the JS heap above, recorded in the
  completion notes after merging the tip of `main`.

## Completion notes

**What was done.**

- **The page** (`crates/cli/web/pack-file.js`, `index.html`): the head
  preloads the `.wasm` (`<link rel="preload" as="fetch" crossorigin>`,
  the same fetch miniquad's `load` makes, so `load` takes it over: the
  resource timings show one `.wasm` fetch, initiator `link`). The
  plugin streams the pack with `response.body.getReader()` (a host with
  no body stream gives `arrayBuffer()` at once), reads the index's
  length from the header's bytes 20 to 24, and starts the game once the
  header and index are queued, or the fetch ended or failed. The game
  takes the queue with `shuyi_pack_file_take(buffer, capacity)`, which
  writes straight into linear memory and drops what it wrote;
  `shuyi_pack_file_ended` and `shuyi_pack_file_problem` say how the
  fetch went. `VERSION` and `PLUGIN_VERSION` are both 2; the test that
  compares the function lists and versions stays, and a new one checks
  the stream's shape (no `headers`, the body reader, the drop).
- **The source** (`crates/app/src/pack_file/stream.rs`): `Stream`, a
  `pack::Source` over a buffer of the pack's whole length (its end from
  the index), behind a `Mutex`, leaked once. `read` answers the bytes
  before `filled`, `NotYet` past them, `Failed` once marked failed, and
  `Failed("past the end of the pack")` past its length; `arrived` is
  true for what has arrived, and for everything once failed or past the
  end. `Stream::pump(&impl Page)` copies the chunks in, and fails the
  stream when the page reports a problem or the body ended short. `open`
  reads the index from the page with `Index::read`'s `Short { needs }`.
  `shuyi-app`'s loop calls `pack_file::pump()` at the top of each frame.
- **`pack_beside_program`** now returns `OpenedPack`: the `Pack` read
  whole on desktop, the index over the stream on the web;
  `OpenedPack::into_layer(name)` and `info()`.
- **Hello, Grid**: `hello_grid::FIRST` lists what its first frame reads
  (five files at the root, `font`, `tiles.png`); `bake()` lays them
  first; `loading::in_steps` gives `Of { arrived, of }` then `Wait`, a
  frame each, until every one has arrived, then reads its files as
  before. Its card counts 14 units (7 groups arrived, 7 files read).
- **Tests**: `stream.rs`'s five unit tests (the source over chunks in
  order, `NotYet` and the bytes before it, a stream failed half way, a
  body that ends short, a page with no pack or too few bytes); Hello,
  Grid's `pack.rs` gains the first frame's files laid first, and its
  load over a streaming source: waiting with a count, counting what
  arrived, loading once the first files arrive, and ending with the
  missing file's error once the stream fails half way through them;
  `first_tests.rs` gains a file laid first alone, and a span naming a
  file checked.

**Deviations.**

- **A file can be laid first, and waited for, alone** (`shuyi-assets`).
  Hello, Grid's first frame reads five files at the root and
  `tiles.png`; ADR-0128 §3's groups are folders, and the root's own
  files can't be named as one (`""` is every file, `synthetic/` sorts
  before `tiles.png`). Moving its files into a folder would touch about
  110 files that name them. So `Bake::first(path)` of a file's path lays
  that file alone, `Index::read` checks that the span holds it, and
  `Bundle::arrived(path)` asks for that file: `in_group` in `pack.rs`,
  used by the bake's order, the spans, their check and
  `SourceFiles::arrived`. A folder and a file can't share a path, so no
  name means both. The pinned snapshot of a pack
  (`a_pack_of_a_stored_a_compressed_and_a_baked_file_is_pinned`) named
  its baked file first, which used to lay nothing: it was taken again.
  The pack's version stays 19: the index's bytes are the same, and only
  a v19 pack that named a file's path first, with that file outside the
  empty span, is now refused (none `shuyi bake` made does that on
  purpose; the game names none, `docs/engine/pins.md`).
- **The card counts groups, not bytes.** The load counts each of
  `FIRST` arrived (a unit each), then each file read: a game can't see
  bytes through `Bundle`, and adding that was out of scope.
- **Waiting for the index:** the ticket's "the plugin asks the Rust
  side, or reads the header's length itself": the plugin reads it.
- **2267 and 3052** are open: their clauses now read **From 3318
  (done)**, and 2267's says where `lend` goes in `Stream` and what its
  `Mutex` asks of it.

**Browser, by hand**, on `0ac37df0` (this branch with `main`'s tip
`c0da2ec7` merged), Hello, Grid's `cargo shuyi export web --release`
with a scratch `export.wasm_opt: optional` (Binaryen isn't installed, as
for 2906; not committed), its pack replaced by the synthetic `web` pack
(`asset_memory --write-synthetic web`, 26,210,861 bytes). In the
built-in pane, hidden, frames driven by `wasm_exports.frame()` every
50 or 100 ms:

| Served by | `.wasm` (3,762,328 bytes) | Pack | Card | Title (first frame) |
| --------- | ------------------------: | ---: | ---: | ------------------: |
| a scratch server at 25 Mbit/s a response | 8 → 1,212 ms, initiator `link` | 21 → 8,401 ms | 1,323 ms | 1,467 ms |
| the same, the pack alone at 20 KB/s | 35 → 1,240 ms | still arriving | 9,359 ms (its index is about 185 KB) | 10,339 ms, the card's bar growing over the two frames before |
| `python -m http.server` (no `Range`), unthrottled | 33 → 74 ms | 81 → 289 ms | 250 ms | 285 ms |

- Each response of the scratch server was throttled alone, so the
  `.wasm` and the pack didn't share the 25 Mbit/s. The card and the title
  came about 7 s before the pack ended, and with the pack at 20 KB/s the
  card showed its count (2 then 5 of 14) until the first frame's files
  had arrived.
- **The JS heap**, read once the game ran, after the next collection:
  **1.45 MB** (25 Mbit/s run) and **1.62 MB** (`http.server`), against
  **27.9 MB** before (2906). Read right after the download it held
  11.1 MB and 20.9 MB: the chunks dropped, not yet collected, which went
  at the next major collection (about 15 s later).
- **Linear memory**: 33,554,432 bytes with the synthetic pack, against
  49,086,464 before (2906): one copy of the pack, not a copy and its
  read.

**Size:** `cargo xtask pr-size` counts 1,293 lines, past the 1,000
planned (`stream.rs` counts its 250 lines of unit tests as code). It
would split cleanly at the pack's file groups (`in_group`, its tests and
docs, about 120 lines), which Hello, Grid's move needs and nothing else
does.

**Follow-ups:** none new. The game's start reads its files without
waiting; game#1298 (blocked by this ticket) lays its first files first
and waits.

Breaking: shuyi-app: `pack_beside_program`: returns `OpenedPack` in place of `Pack<'static>`; a game that calls `.into_layer(name)` on it compiles unchanged

```migration
since: 0.1.0
what: The pack beside the program opens as an OpenedPack, not a Pack, since on the web it streams in; into_layer is the same call.
old: "shuyi_app::pack_beside_program(name: &str, stamp: &str) -> Result<Pack<'static>, PackFileError>"
new: "shuyi_app::pack_beside_program(name: &str, stamp: &str) -> Result<OpenedPack, PackFileError>"
before: |
  let pack: Pack<'static> = shuyi_app::pack_beside_program(name, stamp)?;
  let info = pack.info().clone();
  let layer = pack.into_layer("files");
after: |
  let pack: shuyi_app::OpenedPack = shuyi_app::pack_beside_program(name, stamp)?;
  let info = pack.info().clone();
  let layer = pack.into_layer("files");
find: ["Pack<'static> = shuyi_app::pack_beside_program"]
```

Behaviour: shuyi-app: on the web a release export's page fetches the `.wasm` beside the pack and starts the game once the pack's index has arrived; a file whose bytes haven't arrived reads as missing until they have, so a game that reads its files at start without waiting (`Bundle::arrived`, `Progress::Wait`) sees them missing on a slow link (the game: game#1298)
Behaviour: shuyi-app: on the web, once the page's fetch of the pack breaks or its body ends short after the game started, every file not yet arrived reads as missing for good, `Layer::problems` lists each once ("its source failed: …"), and `Bundle::arrived` answers true
Behaviour: shuyi-assets: `Bake::first(path)`, `write_first` and `Bundle::arrived(path)` with a file's path lay that file first and ask whether it has arrived (before: nothing was laid, and `arrived` answered true); `Index::read` refuses a pack whose span names a file that lies outside it
Behaviour: shuyi-cli: the web page's `pack-file.js` is plugin version 2 (`shuyi_pack_file_take`, `shuyi_pack_file_ended`, `shuyi_pack_file_problem`; `shuyi_pack_file_bytes` is gone), and `index.html` preloads the `.wasm`

Searched the game: `gh search code --repo Zafnok/visions-of-shuyi`
for `pack_beside_program` (`crates/app/src/main.rs` and
`crates/story-preview/src/main.rs` call `.into_layer(…)` on it, which
compiles unchanged), `Bake::first`, `write_first`, `shuyi_fetch_pack`
and `pack-file.js` (its `scripts/check-release-package.sh` names the
file, which stays).

Game: game#1298

Book: the chapters on the pack (`assets-and-formats.md`) and on
exporting (`targets-and-export.md`) say the page streams the pack, that
a load waits with `arrived`, and that a file can be named first alone.

Nick input: none.
