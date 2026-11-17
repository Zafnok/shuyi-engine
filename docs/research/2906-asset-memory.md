# Asset memory: what stays resident, and what arrives before the first frame

Research for ticket 2906, measured **2026-11-16**. The decisions it
leads to (lifetimes, reading on demand, asset groups, loading in steps
with progress, freeing textures, the budgets, and the triggers for each
build) are ticket 2919's ADR. This page holds the numbers.

**Today every file of a pack stays in memory for the whole run, and
three numbers say where that breaks:**

- **The Playdate:** a pack at its `package_bytes` budget (40 MB) takes
  67 MB of heap read whole, as the Playdate reads its pack today. The
  heap budget is 10 MB of the device's 16 MB. In the proportions of the
  game's public assets, a pack read whole holds about 1.6 times its own
  bytes, so **a pack over about 6 MB doesn't fit the heap**. Embedding it
  doesn't help: the launcher loads the program into the same RAM. Read
  file by file, the same pack takes 0.6 MB at its peak.
- **The web:** the page fetches the whole pack before it even fetches the
  `.wasm`, so nothing shows until every byte is in. At 25 Mbit/s a pack at
  the `web` budget (25 MB) puts the loading card at **9.5 s**, against the
  5 s start-up budget. The card could show at 1.2 s if the `.wasm` came
  first; the game's first frame then waits for the files its first
  screen reads.
- **Decoded values outweigh their files.** The game's public sounds take
  2.1 MB decoded, against 0.4 MB of files, and its 33 pictures take
  3.5 MB as textures, against 23 KB of PNG files.

Visions of Shuyi's public assets, 0.6 MB packed, fit every budget in
every way. Nothing here is needed for the game's launch.

## Method

- **The machine:** Nick's (Windows 11, x86-64, 24 cores), Rust 1.99.1
  (`x86_64-pc-windows-gnu`), engine commit `eea1e160` with this ticket's
  program. Native numbers are from a release build.
- **The game's public assets:** Visions of Shuyi's `assets/` folder at
  commit `38553774` (2026-11-16), 116 files, 846,051 bytes. They were
  exported with `git archive` outside this repository; no file of the game
  is committed, only numbers.
- **The program:** [`examples/hello-grid/examples/asset_memory.rs`](../../examples/hello-grid/examples/asset_memory.rs).
  It holds each pack in each way under the heap counter
  (`shuyi-alloc-count`, ADR-0046 §9): a release build counts with a
  counter of its own, a debug build with the global one's memory tags. It
  counts the bytes asked for, never the allocator's overhead. Each pack's
  bytes are leaked before the count starts, so they count as a binary's
  do under `include_bytes!`; only the ways that read the pack count them.
- **The ways a game holds a pack:**
  1. **embedded:** the pack is in the program, opened and laid in a
     bundle (`Pack::open`, `Pack::into_layer`). Its stored files borrow
     the program's bytes; only its compressed files are unpacked into the
     heap;
  2. **read whole, copied:** the pack read into the heap, every file
     copied out (`Pack::into_owned`), and the read dropped. This is what
     ticket 2906 expected ticket 3010 to do;
  3. **read whole, kept:** the pack read into the heap and kept, its
     stored files borrowing it. This is what the engine does today:
     `shuyi_app::pack_beside_program` on desktop and the web, and
     `shuyi_playdate::read_pack` on the Playdate;
  4. **on first read:** each file read from the pack beside the game when
     the load asks for it, and dropped once decoded. Nothing in the engine
     reads so yet, so it is computed from the embedded way's parts:
     - peak = the pack's table and paths + the largest file stored +
       unpacked + the load's own peak;
     - resident = the table and paths + the decoded values.
- **The loads:**
  - Hello, Grid: its own load (`Hello::load_from`).
  - The game's folder: every file goes in as its own bytes, and the load
    reads the engine's own kinds. That is each picture's size (the image
    table) and each sound decoded whole with `shuyi-sound`, as the
    desktop backend decodes a sound at start. Its data tables need the
    game's types, so they go in raw and aren't decoded.
- **The synthetic packs:** Hello, Grid's files with synthetic ones added
  in the proportions of the game's public assets, by bytes:
  - pictures, 2.8% (706 bytes a file);
  - sounds, 49.9% (20,110 bytes a file);
  - data, 47.3% (6,458 bytes a file).

  Pictures and sounds are bytes from `SimRng` that don't compress. Data is
  runs of random letters between runs of repeated text: LZ4 halves it,
  about as it halves the game's data (1.9×). The sources are scaled so the
  pack lands at the profile's `package_bytes`. The load is Hello, Grid's,
  so the synthetic files are held but not decoded.

### Running it again

```bash
cargo run -p hello-grid --example asset_memory --release
```

With the game's public assets, from a checkout of its repository
(`../visions-of-shuyi` here) at a commit you record:

```bash
git -C ../visions-of-shuyi archive 38553774 assets | tar -x -C /tmp/vos
cargo run -p hello-grid --example asset_memory --release -- --folder /tmp/vos/assets
```

To write the synthetic pack of a target's budget, for the web runs:

```bash
cargo run -p hello-grid --example asset_memory --release -- --write-synthetic web /tmp/hello-grid.pack
```

`cargo test -p hello-grid --example asset_memory` runs the program's
tests: the three ways on a small synthetic pack, checked by their
relations, never by exact counts.

## Native: what each way holds

All numbers are bytes of heap. "Peak" is the most at once while the pack
is opened and the game loaded. "Resident" is what stays after. The file
columns are the files' bytes the heap holds after. "Table" is the pack's
table and paths (and its index, where the read is kept).

### Hello, Grid

The pack has 22,585 bytes: 6 baked files (10,293 bytes unpacked), 2 PNG
files (7,088) and 3 other files (15,471).

| Way | Peak | Resident | Baked | PNG | Sounds | Other | Table | Decoded | In the program |
| --- | ---: | -------: | ----: | --: | -----: | ----: | ----: | ------: | -------------: |
| embedded | 76,266 | 75,101 | 10,293 | 6,673 | 0 | 15,471 | 496 | 42,168 | 22,585 |
| read whole, copied | 76,681 | 75,516 | 10,293 | 7,088 | 0 | 15,471 | 496 | 42,168 | 0 |
| read whole, kept | 98,851 | 97,686 | 17,053 | 13,723 | 0 | 23,891 | 851 | 42,168 | 0 |
| on first read | 57,137 | 42,664 | 0 | 0 | 0 | 0 | 496 | 42,168 | 0 |

On first read: peak = 496 + `font/atlas.png` stored 6,635 + unpacked
6,673 + the load's peak 43,333 = 57,137.

### The game's public assets

The pack has 631,985 bytes: 33 PNG files (23,314 bytes unpacked), 21
sounds (422,320) and 62 other files (400,417), none baked.

| Way | Peak | Resident | PNG | Sounds | Other | Table | Decoded | In the program |
| --- | ---: | -------: | --: | -----: | ----: | ----: | ------: | -------------: |
| embedded | 3,161,092 | 2,915,096 | 18,059 | 404,812 | 400,417 | 6,178 | 2,085,630 | 631,985 |
| read whole, copied | 3,183,855 | 2,937,859 | 23,314 | 422,320 | 400,417 | 6,178 | 2,085,630 | 0 |
| read whole, kept | 3,793,077 | 3,547,081 | 35,422 | 807,662 | 608,501 | 9,866 | 2,085,630 | 0 |
| on first read | 2,619,677 | 2,091,808 | 0 | 0 | 0 | 6,178 | 2,085,630 | 0 |

On first read: peak = 6,178 + `audio/sfx/heal.wav` stored 137,267 +
unpacked 144,606 + the load's peak 2,331,626 = 2,619,677.

The decoded values are almost all sound: 422 KB of WAV and Ogg files
decode to about 2.1 MB of 16-bit samples, five times their files. The
game's music isn't among its public assets; it already loads from beside
the game when it plays (`crates/app/src/audio.rs`).

### Synthetic, the `web` budget

The pack has 26,209,671 bytes (`package_bytes`: 26,214,400): 1,304
pictures (926,185 bytes), 815 sounds (16,379,629) and 2,408 data files
(15,541,652 unpacked, 8,756,026 stored).

| Way | Peak | Resident | Baked | PNG | Sounds | Other | Table | Decoded | In the program |
| --- | ---: | -------: | ----: | --: | -----: | ----: | ----: | ------: | -------------: |
| embedded | 16,374,196 | 15,835,939 | 10,293 | 6,673 | 0 | 15,541,652 | 235,153 | 42,168 | 26,209,671 |
| read whole, copied | 59,520,368 | 33,135,080 | 10,293 | 926,185 | 16,379,629 | 15,541,652 | 235,153 | 42,168 | 0 |
| read whole, kept | 42,583,867 | 42,045,610 | 17,053 | 932,820 | 16,379,629 | 24,297,678 | 376,262 | 42,168 | 0 |
| on first read | 446,792 | 277,321 | 0 | 0 | 0 | 0 | 235,153 | 42,168 | 0 |

### Synthetic, the `playdate` budget

The pack has 41,937,661 bytes (`package_bytes`: 41,943,040): 2,086
pictures (1,478,161 bytes), 1,304 sounds (26,216,631) and 3,852 data
files (24,866,105 unpacked, 14,010,188 stored).

| Way | Peak | Resident | Baked | PNG | Sounds | Other | Table | Decoded | In the program |
| --- | ---: | -------: | ----: | --: | -----: | ----: | ----: | ------: | -------------: |
| embedded | 26,187,056 | 25,301,279 | 10,293 | 6,673 | 0 | 24,866,105 | 376,040 | 42,168 | 41,937,661 |
| read whole, copied | 95,232,996 | 52,989,398 | 10,293 | 1,478,161 | 26,216,631 | 24,866,105 | 376,040 | 42,168 | 0 |
| read whole, kept | 68,124,717 | 67,238,940 | 17,053 | 1,484,796 | 26,216,631 | 38,876,293 | 601,999 | 42,168 | 0 |
| on first read | 587,679 | 418,208 | 0 | 0 | 0 | 0 | 376,040 | 42,168 | 0 |

### What the tables say

- **Read whole and kept** (today's way) holds the pack, plus every
  compressed file unpacked beside it: 1.6 times the pack in the synthetic
  proportions, and 5.6 times for the game's public assets, whose sounds
  decode. **Copied**, it holds less after (1.26 times), but peaks at 2.3
  times: the read, the unpacked files and the copies are all live at
  once.
- **Embedded** keeps only the compressed files unpacked on the heap, but
  the pack itself is in the program (see the Playdate below for where that
  lives).
- **On first read**, what stays is the table (about 50 bytes a file) and
  the decoded values. Its peak is set by the load, not by the pack: 0.4 to
  0.6 MB for both synthetic packs.
- **The table** is small: 496 bytes for Hello, Grid's 11 files and
  376 KB for the 7,248 files of the `playdate` synthetic pack. Leaked
  paths (`&'static str`, ticket 2207's rule) cost little.

## The web

Measured in Chromium (the desktop app's browser), served by Python's
`http.server` on 127.0.0.1. The browser can't be throttled from here, and
its pane was hidden, so it ran no animation frames:

- **Downloads:** the runs were timed unthrottled, and each download's
  bytes are added at 25 Mbit/s (3,125,000 bytes a second).
- **Frames:** they were driven by hand (`wasm_exports.frame()`), each
  timed with `performance.now()`.
- **Builds:** none ran `wasm-opt`, since Binaryen isn't installed. The
  export used a scratch `export.wasm_opt: optional`, not committed. A
  release export runs `wasm-opt`, so its `.wasm` would be smaller.

**The `.wasm`, release:**

| Build | `.wasm` bytes |
| ----- | ------------: |
| files embedded (`cargo shuyi web --release`) | 3,500,839 |
| files left out, read from the pack (`cargo shuyi export web --release`, the `packed` feature) | 3,484,582 |

Embedding Hello, Grid's 45,856 bytes of files adds 16,257 bytes to the
`.wasm` (the same 16,180 bytes in a plain `cargo build` of each). This is
less than the files' bytes, and why wasn't traced: the packed build also
links the code that reads the pack beside the page, which may hide part of
the difference.

**What the page does:** `index.html` calls `shuyi_fetch_pack(name,
start)`, which fetches the whole pack and only then fetches the `.wasm`
(`crates/cli/web/pack-file.js`). The resource timings show it: the pack
from 88 to 93 ms, then the `.wasm` from 94 to 104 ms. The card is the
`.wasm`'s first frame, so **the card waits for the whole pack**.

**Times at 25 Mbit/s.** "Before the card" counts the bytes downloaded
before the card's frame: the page and its scripts (85,797 bytes; 85,827
when the page names a pack), the pack, and the `.wasm`. The frames add about 20–30 ms each: the card's
frame took 20.4–27.9 ms, and the load's 18.4–34.2 ms.

| Build | Bytes before the card | Download at 25 Mbit/s | Card | First frame |
| ----- | --------------------: | --------------------: | ---: | ----------: |
| today's, files embedded | 3,586,636 | 1.15 s | about 1.2 s | about 1.2 s |
| packed, Hello, Grid's pack | 3,592,998 | 1.15 s | about 1.2 s | about 1.2 s |
| packed, the synthetic `web` pack | 29,780,080 | 9.53 s | about 9.6 s | about 9.6 s |

- The `web` profile's 5 s start-up holds about 15.6 MB at 25 Mbit/s. Less
  the `.wasm` and the shell, that leaves **about 12 MB of pack**, if the
  start-up means the card and the card waits for the pack as today.
- Fetched after the `.wasm`, a pack at the budget would leave the card at
  about 1.2 s. Its first frame stays at 9.6 s unless the first screen
  needs only part of the pack.
- **Memory on the web**, read once the game ran:
  - **the synthetic pack:** 49,086,464 bytes of `.wasm` linear memory,
    against 7,208,960 with Hello, Grid's pack. The difference (41.9 MB)
    is the "read whole, kept" resident of the native table;
  - **the page's JS heap:** 27.9 MB, against 1.6 MB, because
    `pack-file.js` keeps the pack's bytes it fetched for the whole run. So
    the pack is held twice, once in JS and once in the `.wasm`;
  - **the `web` budget's peak heap** is 256 MB, so this fits. Linear
    memory never shrinks.

**Reading part of a file without `wasm-bindgen`:** macroquad's loader
(`fs_load_file` in `mq_js_bundle.js`) is an `XMLHttpRequest` GET with no
headers, and it accepts only status 200. It can't ask for a byte range
(206). A JS plugin of our own can: `pack-file.js` is one already, and
`fetch(name, { headers: { Range: "bytes=…" } })` is plain browser code.
The host must answer `Range`. Python's `http.server` doesn't; common
static hosts do. This is a finding only, not built.

## Textures

`Renderer::new` (`crates/app/src/render.rs`) uploads the font atlas and a
texture for every image of the `ImageTable` at start, and keeps them for
the run. It was timed with the backend's clock (`get_time`) in a scratch
build, not committed. For the game's pictures, its 33 PNG files were laid
in Hello, Grid's files for that build.

| Pictures | Textures | Bytes (width × height × 4) | PNG bytes | Upload, native | Upload, web |
| -------- | -------: | -------------------------: | --------: | -------------: | ----------: |
| Hello, Grid (its atlas and tiles) | 2 | 313,344 | 7,088 | 0.9–1.2 ms | 4 ms |
| the game's public pictures (and Hello, Grid's) | 33 (+2) | 3,470,576 (3,783,920 in all) | 23,314 | 9.6–11.8 ms | 5–8 ms |

- The web's clock reads whole milliseconds. Three runs each, but one of
  the web's runs for Hello, Grid alone.
- A texture holds 149 times its PNG for the game's pictures. Its largest
  is 336 × 248 (333,312 bytes).
- The renderer also keeps each PNG's bytes (`sources`), borrowed from
  the bundle, to make recoloured copies. Its `recoloured` map never
  empties.

## The Playdate, from the SDK's documentation

No device was used. From Panic's documentation, read on 2026-11-16:

- **The program is loaded into RAM.** *Inside Playdate with C*, section
  6, "Game Initialization": the launcher loads `pdex.bin` from the `.pdx`
  into memory, and the binary is built for a fixed address
  (<https://sdk.play.date/inside-playdate-with-c/>).
  - So bytes embedded with `include_bytes!` take RAM as a pack read into
    the heap does. The device has 16 MB of RAM in all (*Inside Playdate*,
    section 1.1, and <https://help.play.date/hardware/the-specs/>).
  - The documentation doesn't say how much of it the game's heap gets.
    `targets/playdate.ron`'s 10 MB is provisional (3011 measures it).
- **The file API can read part of a bundled file.** *Inside Playdate with
  C*, section 7.4, "Filesystem", *File handles*:
  - `kFileRead` opens a file in the game's `.pdx`, and `kFileReadData`
    one in its data folder;
  - `seek(file, pos, whence)`, `tell` and `read(file, buf, len)` read a
    byte range;
  - at most 64 files are open at once.
  - Section 1.2, "Before porting to C", warns that disk access is slow on
    the hardware, and to preload images, fonts and sound effects.
- **`pdc` copies a file it doesn't know as it is.** *Inside Playdate*,
  section 4.3, "Compiling a project": the `.pdx` holds every file that
  isn't Lua source, and `--skip-unknown` leaves them out. So a `.pack` in
  the `.pdx` keeps its bytes and its offsets.
- **What the Playdate does today:** `shuyi_playdate::read_pack`
  (`crates/playdate/src/backend.rs`) reads the whole pack with
  `read_bundled` and leaks it: the "read whole, kept" way.
  - Hello, Grid's Playdate pack fits easily.
  - A pack over about 6 MB in the synthetic proportions holds more than
    the 10 MB heap: 10 MB ÷ 1.6.
  - The 40 MB `package_bytes` budget can't be met by any way that keeps
    every file. Embedding it is no way out, since the binary is loaded
    into RAM.

## The parked `nds` row

ADR-0012's parked `nds` row allows a 64 MB ROM and a 2.5 MB heap. The
same ratio says a pack read whole fits only under about 1.5 MB there, and
on first read the table alone costs about 50 bytes a file. Nothing is
measured for it: it is parked.

## What changed since the ticket was written

The ticket was written on 2026-10-21, and these tickets were done before
it was worked:

- 2207 and 2220: run-time layers (`Layer::owned`, leaked once) and hot
  reload;
- 3010: the Playdate backend, which reads its pack whole;
- 3064 and 3065: the heap counter and its memory tags;
- 3008: the bench and its CI gate;
- 3097 and ADR-0097: a load in steps, `Start::in_steps`. Its slices are
  time from the platform (`load_slice_ms`), not counted units, and its
  card shows no progress yet.

The web's packed build already exists (3300's `pack-file.js`), so the
scratch build the ticket planned, which fetches the pack with
`load_file`, wasn't needed: the release export is that build.
