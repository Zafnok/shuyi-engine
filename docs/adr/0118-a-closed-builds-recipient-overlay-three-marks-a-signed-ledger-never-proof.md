# ADR-0118: A closed build's recipient overlay: one code in three marks, a ledger signed with the game's SSH key, never proof

- **Status:** Accepted
- **Date:** 2026-11-14
- **Related tickets:** 3302 (this); 2917 (the research); 3300 (the pack beside
  the program); 2816 (the reveal kit); 3050 (a release's attestations); 3081
  and 3099 (the gates an export passes)
- **Extends:** ADR-0023 §1 and §3 (a pack's files' bytes may be laid in any
  order, with bytes between them: `pack::VERSION` 15)

## Context

- A press, playtest or publisher build that leaks can't be traced to a
  person. Steam gives per-account keys but no per-download record
  ([2917's report](../research/2917-reverse-engineering-angles-not-yet-covered.md),
  §8). Nothing marks the build itself.
- The report's §7 says what is honest:
  - one ID string falls to a hex edit or to two copies compared;
  - codes that resist collusion (Tardos) need hundreds of independent places
    a small Rust game doesn't have;
  - rustc and LLVM have no per-copy randomization;
  - no case was found of a press-build leaker named by a mark in the binary.
- Reproducible builds and per-copy marks conflict, unless the marks are a
  small overlay applied after the reproducible build. 3050's attestation then
  covers the base package, the same for every recipient.
- A call home at launch collects personal data. It is a product decision,
  and is left out.
- 3300's pack refused files' bytes out of path order and any byte after the
  last file, and its checksum covers every byte after the header. So a mark
  in the pack's layout needs a change of its format, or a place the reader
  ignores. Its only such places (each file's codec, LZ4's choices of match)
  hold a few bits a file, and fewer for the PNG and OGG files a game is made
  of: not a code, on every game.
- There is no release signing key: 3050 attests packages with Sigstore in CI
  (ADR-0116 §4). A closed build is made on a developer's machine, for a
  person, not in the release workflow.

## Decision

`shuyi export <target> --release --recipient <id> [--label <text>]
[--signing-key <file>]`, and `shuyi export --detect-recipient <file>`
(`crates/cli/src/export/recipient.rs`), for `windows`, `linux`, `macos` and
`web`.

### 1. The code

- `shuyi_assets::recipient::Code`: 32 bits and their CRC-8 (polynomial
  `0x07`), 40 bits, written as 8 characters of Crockford's base 32. Zero is no
  code.
- The CRC refuses a mark read wrong, so a damaged mark is "no mark", not
  another recipient.
- The 32 bits are the first 4 bytes of the SHA-256 of the game's key's SSH
  signature (namespace `shuyi-recipient-code`) over the recipient's id and the
  base package's SHA-256:
  - only the key's holder can draw a recipient's code, so one recipient can't
    make a copy that names another;
  - an Ed25519 key signs the same way twice, so a copy made again is the same
    copy.
- A code the ledger gave another recipient is drawn again (up to 16 times).

### 2. Three marks, each holding the whole code

One surviving mark names the recipient; two agreeing say more.

**(a) The program's field.**

- `shuyi-app` holds a `#[used] static RECIPIENT: [u8; 32]`: 16 magic bytes,
  then a 16-byte slot.
- The slot holds the code's 5 bytes twice and then zeros, each byte XOR-ed
  with a fixed key. Every stored byte has its high bit set, so:
  - `strings` shows nothing;
  - `wasm-opt`'s memory packing, which drops runs of zero bytes, keeps the
    slot whole.
- A public build's slot holds zeros under the key: no code.
- `shuyi_app::recipient() -> Option<&'static str>` reads the slot with a
  volatile read, so the compiler can't fold in the build's value. `run`
  reads it once, so the linker keeps the static.
- The program reads only its slot (`slot_code`). The magic is therefore in
  the program once (measured on Hello, Grid's release build).
- The overlay writes every field whose magic it finds and whose slot reads
  as a field. A universal macOS program has two.

**(b) The pack's layout.**

- `pack::VERSION` is 15. The index is unchanged and still in path order.
  Each file's bytes may lie anywhere in the data, so long as:
  - no two overlap;
  - the last ends the pack.
- Reading the same files from it is unchanged, and the checksum still covers
  every byte.
- `pack::write` lays the files as before: in path order, nothing between
  them.
- `pack::Raw` reads a pack whatever its stamp:
  - `Raw::write_marked(code)` lays its files in an order drawn from the code
    (a Fisher–Yates shuffle over SplitMix64), with the code twice (10 bytes,
    XOR-ed with a stream) between the first of them;
  - `Raw::mark` reads the code back, and whether the order agrees.

**(c) One picture's lowest bits.**

- `shuyi_assets::image::mark` marks by color, not by pixel, so the picture
  keeps its palette (a pixel-art game's strict colors).
- Each visible color alone in its group (equal to no other color but in the
  two lowest bits of red, green and blue) carries 6 bits there. Every pixel
  of that color changes the same way, by at most 3 in a channel.
- The carriers, in group order, hold the code's 40 bits and repeat them as
  far as they go. Every copy must agree.
- A picture needs 7 carriers. The game names it in `shuyi.ron`'s
  `export.overlay_picture`, by its path in the pack: a picture, never the
  player's text.
- The overlay decodes it, marks it, encodes it as RGBA PNG, decodes it back
  as the engine does, and checks the code and the palette count before
  writing the pack.

### 3. The overlay

1. The ordinary release export runs first, unchanged, and passes its gates.
2. The copy is `dist/recipients/<id>/<target>` (`<edition>/<target>` for an
   edition): the package's folder copied, then marks (a), (c) and (b).
3. The copy is packaged again: the zip or `tar.gz` made again; on macOS the
   `.app` signed again ad hoc (on macOS only).
4. The copy is held to the `disclosure` part and 3099's symbol check by the
   same gate (`disclosure::gate`), with its own `exposure-report.json`.
5. It is refused before anything is built when:
   - the build isn't a release, isn't packed, or is for the Playdate;
   - the game names no picture;
   - the id isn't plain (1 to 64 of letters, digits, `.`, `_`, `-`);
   - there is no signing key;
   - the reveal kit doesn't verify.

### 4. The ledger

- `recipients.ron` sits beside the reveal kit's folder (`dist/recipients.ron`
  for `dist/reveal`), never in it: `reveal verify` refuses a file its manifest
  doesn't name.
- It holds a note that a match is circumstantial, and the signer's public
  key.
- Each entry holds:
  - the recipient's id and label;
  - the day;
  - the code;
  - the target, the edition and the version;
  - the base package's SHA-256 (what the release attests);
  - the copy's SHA-256;
  - the kit's digest.
- A folder package (macOS) is hashed as the SHA-256 of `sha256sum`'s lines
  for its files.
- It is signed in `recipients.ron.sig` with `ssh-keygen -Y sign -n
  shuyi-recipients`, with the game's SSH private key: `--signing-key` or
  `SHUYI_SIGNING_KEY`, kept outside the repository.
- Before an entry is added, a ledger that doesn't verify against its signer,
  or that another key signed, is refused.

### 5. The detector

- `--detect-recipient <file>` reads a package (zip, `tar.gz` or folder), a
  program, a pack or a PNG.
- It reports each mark it reads and the code it holds, and looks the code up
  in the ledger, whose signature it checks.
- A picture whose pixels equal the game's own (`assets/<overlay_picture>`) is
  "the game's own picture". Without that, a picture with no mark names a code
  about once in 2⁸⁺ʳ (the CRC, then `r` repeated bits).
- Its verdict is "no mark found", or the code the marks name. Every answer,
  and every doc that names a match, says it is circumstantial evidence, never
  proof.

## Consequences

- A copy that leaks names its recipient through any one mark left. What
  Hello, Grid's tests measure (`export::recipient::tests`, the book's
  passage):
  - an overwritten field leaves (b) and (c);
  - a repacked pack leaves (a) and (c);
  - a picture re-encoded at a lower quality leaves (a) and (b);
  - a crop that keeps every color keeps (c), and one that drops colors loses
    it.
- Two recipients who compare their copies find every mark: the program
  differs in 10 bytes, and the pack in its layout and one picture's low
  bits. Nothing here claims more.
- The base package is byte-for-byte the public one, so its attestation (3050)
  covers every copy's base. The copies themselves aren't attested.
- Every pack is version 15, and an older pack is refused: a game bakes again,
  as export does.
- The reader no longer refuses bytes between files. The checksum still
  catches any changed byte, and no size is trusted before it is checked.
- The game's picture changes by at most 3 per channel in a copy. Its scene,
  state and play don't change. The capture plan can't run on a release
  build, which refuses the `capture` feature (3099). CI's `export` job
  instead checks the two copies (`tools/export-check/recipient.sh`).
- Signing needs `ssh-keygen` (OpenSSH 8 or later), which Windows, Linux and
  macOS ship. A key with a passphrase needs `ssh-agent`.

## Alternatives considered

- **Marks in the codec or LZ4 choices, the pack's format unchanged** — a few
  bits a file, and nearly none in the PNG and OGG files that are most of a
  game. Not a 40-bit code on a small game (Hello, Grid has 12 files).
- **The code as a field in the pack's index** — no better than (a): a
  diff of two copies shows it.
- **A mark per pixel** — it adds colors, and pixel art keeps strict palettes.
- **Sigstore for the ledger** — keyless signing needs an identity provider's
  login at each copy, and a transparency log entry naming the game each time
  a copy is made. The release workflow makes no closed builds.
- **A key and signing crate of our own (Ed25519)** — a dependency for what
  `ssh-keygen` already does with the key a developer signs commits with.
- **Random codes** — a copy made again would differ, and anyone could write
  a code that names another recipient.
- **Collusion-resistant codes, binary diversity, a call home** — out of scope
  (2917 §7; Nick's product decision for a call home).
