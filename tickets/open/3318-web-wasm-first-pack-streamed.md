---
id: "3318"
title: "The web page fetches the .wasm beside the pack, streams the pack into the game in the order it was baked, and keeps no copy of it in JS"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2262", "2263", "2265", "3316", "3317"]
nick_input: none
completed:
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

- [ ] In the browser (the built-in pane, frames driven by
      `wasm_exports.frame()` if it is hidden), Hello, Grid's release export
      with the synthetic `web` pack
      (`examples/hello-grid/examples/asset_memory.rs --write-synthetic
      web`) shows its card before the pack has arrived: the resource
      timings show the `.wasm` fetched beside the pack, not after it.
- [ ] Its card counts the download, and its first frame comes once the
      folders it names first have arrived, before the rest.
- [ ] The page's JS heap, read once the game runs, holds less than 5 MB
      with the synthetic pack, against 27.9 MB before (the completion
      notes give both).
- [ ] The source answers `NotYet` past what arrived and the bytes before
      it (unit tests, off the browser).
- [ ] A stream marked failed at half the pack answers `Failed` past what
      arrived and `arrived` true, and a load in steps waiting on a folder
      past it ends with the missing file's error, not `Wait` (unit tests).
- [ ] Served by Python's `http.server`, which answers no `Range`, the
      game runs.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the source over chunks handed in order; `arrived`; a stream
  that fails half way; the plugin's version test.
- Browser, by hand: the timings and the JS heap above, recorded in the
  completion notes after merging the tip of `main`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
