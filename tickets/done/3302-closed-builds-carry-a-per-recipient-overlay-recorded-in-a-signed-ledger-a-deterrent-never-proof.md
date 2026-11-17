---
id: "3302"
title: "Closed builds carry a per-recipient overlay recorded in a signed ledger: a deterrent, never proof"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2816", "3300"]
nick_input: none
completed: 2026-11-14
---

# 3302 — A per-recipient overlay for closed builds

## Context

A press, playtest or publisher build that leaks cannot be traced to a
person today. Steam's free tools (research ticket 2917's
[report](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md),
section 8) give per-account keys and a Hidden Playtest but no NDA, weak
revocation and no confirmed per-download record; a password branch is
shared and untraceable. The parked 3083 marks a *hosted* preview's
pixels; 2915 showed per-recipient marks in video are faint and removable.
Nothing marks the build artifact itself.

Section 7 of the report says what is honest here. A single ID string
falls to a hex edit or a two-copy diff; collusion-resistant codes
(Tardos) need hundreds of independent natural positions a small Rust
game does not offer; rustc and LLVM expose no per-copy randomization
knob; no confirmed case of a press-build leaker identified by a binary
mark was found. Reproducible builds conflict with per-copy diversity
unless the recipient part is a small overlay applied after the
reproducible build. A launch-time honeytoken collects personal data and
is a product decision, so it is out.

So this ticket is a deterrent with circumstantial evidence, stated as
such everywhere: a small overlay on a reproducible build, several
redundant marks a casual leaker will not find all of, a signed ledger of
who got which, and a detector. It pairs with per-recipient Steam keys
or override keys, which do the account-side half for free. It never
calls a match proof.

**From 3300 (done):** a release desktop or web export ships the pack as
`<package>.pack` beside the program (in a `.app`'s `Contents/Resources`,
beside `index.html` on the web), baked by `ExportCx::bake_pack` and proven
by `bake --check` before the build; the exposure report lists it as one
`pack` file and each of its entries. `Pack::open` refuses entries out of
path order and any byte after the last file, and the checksum covers every
byte after the header ([ADR-0023](../../docs/adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md)
§1, §3): the overlay's (b), the pack's entry order and padding, is a change
of the pack's format (`pack::VERSION`, a `Breaking:` line), or moves to a
place the reader ignores.

**From 2816 (done):** the reveal kit is a folder `shuyi reveal prepare`
writes (`dist/reveal` by default), sealed by its manifest's SHA-256 in
`reveal.ron.sha256`; `reveal verify` fails a file in the folder that its
manifest doesn't name, so `recipients.ron` goes beside the kit's folder,
never inside it, and may record the kit's digest
([ADR-0116](../../docs/adr/0116-the-reveal-kit-a-folder-sealed-by-its-manifests-sha-256-dated-where-the-digest-is-published.md)).
There is no release signing key: 3050 attests each package with Sigstore
(`actions/attest`, keyless, on a public repository), and the kit keeps
those attestations as receipts. ADR-0116 §4 says why the kit has no key of
its own; signing the ledger needs a key this ticket chooses and records
in its ADR, or a Sigstore attestation of the ledger from the release
workflow. The kit's SHA-256 is `crates/cli/src/sha256.rs` (`hex`,
`file`, `Hasher`).

## Nick input

None.

## Scope

**In:**
- `shuyi export <target> --recipient <id>` in release: after the ordinary
  reproducible export (3050's attestation stays valid for the base), an
  overlay step writes the recipient's code into: (a) a data-segment
  field reserved by the game (`shuyi::app::recipient()` reads it, for a
  faint corner text the game may draw, as 3083 plans for previews); (b)
  the pack's entry order and padding (3300's pack, index unchanged in
  meaning); (c) one low-bit mark in one approved image the game names in
  its `shuyi.ron` export part (`overlay_picture`), outside the player's
  text.
- A code per recipient that survives any one of the three marks being
  removed: the same short code in each, with a checksum, so one surviving
  mark names the recipient and two agreeing marks raise confidence.
- A ledger `recipients.ron` kept beside the kit (2816), signed with the
  release signing key, recording recipient id, label, date, build digest
  and the overlay digest; the public build never has an overlay or a
  ledger entry.
- `shuyi export --detect-recipient <file>` over a leaked binary, pack or
  picture: reports which marks were found, the recipient code each names,
  and "no mark found" otherwise; its output says circumstantial evidence
  in every case.
- A written limit, in the book and the skill: two recipients comparing
  their copies find and remove every mark; a repack or re-encode drops
  marks (b) and (c); a stripped data segment drops (a). Numbers from the
  tests.
- The disclosure gate (3081) and 3099's symbol checks pass on an
  overlaid build.

**Out (do not do):**
- A launch-time or first-run call home, a per-copy URL, or any network
  request (a product decision for Nick, with privacy duties; 2917 §7).
- Collusion-resistant codes, binary diversity, or any claim of
  robustness against two copies.
- Marks in the player's text, in sound, or in any public build.
- Hosted previews or their marks (2602, 3083 stay parked).

## Implementation steps

1. Read 2816's kit manifest and 3050's signing; decide the ledger's place
   and the signing call to reuse.
2. `crates/app`: a `#[used] static RECIPIENT: [u8; 32]` with a magic
   prefix, zero in ordinary builds; `shuyi::app::recipient() -> Option<&str>`,
   additive (ADR-0039). The overlay step finds the magic in the final
   program (exe, Mach-O, Wasm data segment) and writes the code; a test
   proves the write changes nothing else (`objcopy`-free, byte search).
3. `crates/assets/src/pack.rs`: `write` takes an optional entry order and
   padding seed; `Pack::open` is unchanged; a function reads the order
   back as a code. Add the pack's new input to `Bake` without changing
   `VERSION` (the index format is the same).
4. The picture mark: a low-bit mark over the named PNG with a short
   repeated code and a checksum, in `crates/assets/src/image/mark.rs`,
   with its detector; it must survive the image codec round trip the
   engine does at load and must not change the image's palette count
   (pixel-art games keep strict palettes; test it).
5. `crates/cli/src/export.rs`: the `--recipient` and `--detect-recipient`
   flags, the overlay step after the exporter and the gates, the ledger
   append and signing, and the refusal of `--recipient` on a non-release
   export or without a kit.
6. Tests of the limits: strip the data segment, repack the pack, re-encode
   the picture at a lower quality, and crop it; record which marks survive
   each, and write the numbers into the book's passage.
7. Docs per the `document` skill: the `shuyi-ship` skill, the book's
   release chapter (with the limit stated first), and `docs/contract.md`
   for the new flags.

## Acceptance criteria

- [x] Two recipients' exports of the same build have identical gameplay
      (capture plan passes on both) and different overlay digests, and
      the base build's attestation verifies for both. The capture plan
      can't run on a release build, which refuses `capture` (3099): the
      checks under *Deviations* stand in for it.
- [x] `--detect-recipient` names the right recipient from each mark alone
      on the three fixtures, and says "no mark found" on the public build.
- [x] A build with its data segment stripped, a repacked pack and a
      re-encoded picture each still name the recipient through at least
      one other mark; the completion notes record which marks survived
      which attack.
- [x] The public build has no overlay, no ledger entry and no reserved
      field with a non-zero code.
- [x] Every output and doc that names a match calls it circumstantial.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the code's checksum rejects a corrupted mark; the data-segment
  write touches only the reserved bytes; the pack order encodes and
  decodes the code; the picture mark round-trips through the engine's
  decoder and keeps the palette count.
- Property: random recipient codes over the three marks decode to
  themselves.
- Integration: two overlaid exports of `hello-grid`, the detector over
  each and over the public build, the attack fixtures of step 6.

## Completion notes

- **The code** (`shuyi_assets::recipient::Code`): 32 bits and a CRC-8,
  written as 8 characters of Crockford's base 32. The 32 bits come from
  the game's SSH key's signature over the recipient's id and the public
  package's SHA-256, so only the key's holder draws a recipient's code, and
  an Ed25519 key draws the same one again for the same build.
- **(a) The program's field**: `shuyi-app`'s `#[used] static RECIPIENT`, 16
  magic bytes and a 16-byte slot. The slot holds the code twice, XOR-ed
  with a key whose every byte has its high bit set: no text, and no zero
  run for `wasm-opt` to drop. `shuyi_app::recipient()` (so
  `shuyi::app::recipient()`) reads it with a volatile read, and `run` reads
  it once so the linker keeps it. Hello, Grid's release Windows program
  holds the magic exactly once. A first build held it twice, because the
  reader compared the magic; it now reads only the slot (`slot_code`).
- **(b) The pack's layout**: `pack::Raw` reads a pack whatever its stamp.
  `write_marked` lays its files' bytes in an order drawn from the code,
  with the code twice in 10 bytes between the first files; `mark` reads it
  back. `pack::write` is unchanged byte for byte; it builds a `Raw` now.
- **(c) The picture**: `shuyi_assets::image::mark`, by color, not by
  pixel. Each visible color alone in its group carries 6 bits in the two
  lowest bits of red, green and blue, so the palette count stays the same
  (tested, and checked after each PNG round trip). It needs 7 such colors.
  Hello, Grid's `tiles.png` has 8, and its `shuyi.ron` names it
  (`export.overlay_picture`).
- **The overlay** (`crates/cli/src/export/recipient.rs`), in this order:
  1. the ordinary release export runs, with its gates;
  2. the package's folder is copied into `dist/recipients/<id>/<target>`;
  3. the field, the picture and the layout are marked;
  4. it is packaged again (`Exporter::repackage`; on macOS, signed again);
  5. it goes through `disclosure::gate` with its own exposure report, which
     runs 3099's symbol check too;
  6. an entry goes in `recipients.ron`, signed.
- **What it refuses**, before anything is built: a build that isn't a
  release, isn't packed, or is for the Playdate; no picture named; an id
  that isn't plain; no key; a kit that doesn't verify.
- **The ledger**: `recipients.ron` beside the kit, with its signer's public
  key, signed in `recipients.ron.sig` by `ssh-keygen -Y sign -n
  shuyi-recipients` (ADR-0118 §4 says why not Sigstore). A ledger that
  doesn't verify, or that another key signed, is refused before an entry
  is added.
- **The detector**: `export --detect-recipient` reads a zip, a `tar.gz`, a
  folder, a program, a pack or a PNG. It reports every mark, the codes,
  the ledger's entries and whether its signature verifies. A picture equal
  to the game's own is "no mark".
- **Measured on this machine** (Windows, commit `ad37e314` plus the docs):
  - two real release exports of Hello, Grid, `press-a` and `press-b`;
  - the public zip was byte-identical across both runs;
  - the ledger names the same `build` for both, so 3050's attestation of
    the public package covers both copies;
  - the overlays' digests differ;
  - the detector named each copy's recipient from all three marks, and said
    "no mark found" on the public zip;
  - each copy passed the disclosure gate;
  - each copy's program differs from the public one in 10 bytes, all in the
    field;
  - a copy runs like the public build (still running after 8 s).

  `tools/export-check/recipient.sh` does the same in CI's `export` job on
  each target, `web` (with `wasm-opt`, which this machine lacks) and
  `macos` among them.
- **The attacks** (`export::recipient::tests::the_limits_hold_as_the_book_states`,
  on Hello, Grid's files), written into the book's *Closed builds*:
  - the field overwritten, or cut out: the pack and the picture name the
    recipient;
  - the pack repacked: the program and the picture do;
  - the picture re-encoded at a lower quality (its 3 lowest bits dropped)
    in a repacked pack: the program alone does;
  - the picture cropped to a band keeping every tile: it still does;
  - cropped to 4 of its 8 tiles (colors lost): it doesn't;
  - two copies compared: the programs differ only in the field's bytes.

### Deviations

- **The pack's format changed** (`pack::VERSION` 15). Step 3 planned no
  change, which 3300's note had already shown impossible. The reader
  refused any byte outside a file and any order but the path's, and the
  places it ignores (each file's codec, LZ4's match choices) hold a few
  bits a file, nearly none in PNG and OGG, not 40 bits on a 12-file pack.
  The index is unchanged. Bytes may now sit between files, and files'
  bytes in any order, so long as none overlap and the last ends the pack.
  ADR-0118 extends ADR-0023 §1 and §3.
- **The public field isn't zero bytes**: its slot is zeros XOR-ed with a
  key with no zero byte, so `wasm-opt` can't drop it. The code it holds is
  zero (no code), as the criterion asks.
- **"Signed with the release signing key"**: there is none (2816's note).
  The ledger is signed with the game's own SSH key through `ssh-keygen`
  (`--signing-key` or `SHUYI_SIGNING_KEY`), and `doctor`'s optional
  `ssh-keygen` check says `export --recipient` needs it.
- **The capture plan on both copies**: a release build refuses the
  `capture` feature (3099), so the plan can't run on an exported program.
  What stands in for it:
  - each copy's program equals the public one but for the field;
  - its pack opens to the same files but the named picture;
  - that picture's pixels differ by at most 3 in a channel, with the same
    size and palette count (`the_copy_is_the_public_build_but_for_its_marks`);
  - the copy runs.
- **`Exporter` gained `overlays` and `repackage`**, both defaulted, for
  the copy's archive and the macOS signature.
- The test fixtures are Hello, Grid's real files in a synthetic package
  for the unit tests, and its real release exports in CI and here.

### Follow-ups

No ticket: a call home, collusion-resistant codes and marks in hosted
previews are the ticket's *Out* list (Nick's product decision; 2917 §7;
3083, parked, which now names this ticket).

Breaking: shuyi-assets: `pack::VERSION`: a pack is of version 15 (its files' bytes may lie in any order with bytes between them); a pack baked before is refused, bake it again with `shuyi bake` (an export bakes its own).

```migration
since: 0.1.0
what: The pack format is version 15; packs baked before are refused.
old: shuyi_assets::pack::VERSION == 14
new: shuyi_assets::pack::VERSION == 15
before: |
  cargo shuyi bake   # a pack of version 14, kept from before
after: |
  cargo shuyi bake   # the same command bakes version 15
```

Breaking: shuyi-cli: `contract::export::ExportOutput`: a new public field, `recipient`; code that builds one with a struct literal adds `recipient: None`.

```migration
since: 0.1.0
what: An export's report names the closed build's copy it made for a recipient.
old: shuyi_cli::contract::export::ExportOutput { …, edition, pack }
new: shuyi_cli::contract::export::ExportOutput { …, edition, pack, recipient }
before: |
  let report = ExportOutput { edition: None, pack: None, ..report };
after: |
  let report = ExportOutput { edition: None, pack: None, recipient: None, ..report };
```

Breaking: shuyi-cli: `contract::export::ExportResult`: a new variant, `Detected`; a `match` over it adds an arm.

```migration
since: 0.1.0
what: export's answer may be what --detect-recipient found.
old: shuyi_cli::contract::export::ExportResult::{Package, Plan}
new: shuyi_cli::contract::export::ExportResult::{Package, Plan, Detected}
before: |
  match result { ExportResult::Package(p) => {}, ExportResult::Plan(p) => {} }
after: |
  match result { ExportResult::Package(p) => {}, ExportResult::Plan(p) => {}, ExportResult::Detected(found) => {} }
```

Behaviour: shuyi-assets: `Pack::open` (and `pack::length`) take files' bytes in any order and with bytes between them; the refusal "<file> starts at <n> and the file before it ends at <m>" is gone, and two files whose bytes overlap are refused as "<file> overlaps <file>".

Behaviour: shuyi-app: every program reserves the 32-byte recipient field, and `run` reads it once.

Behaviour: shuyi-cli: the contract is version 52 (`export`'s `recipient`, `label`, `signing_key` and `detect_recipient`, and its answers `recipient` and `RecipientFound`); `shuyi.ron`'s `export` part takes `overlay_picture`; `doctor`'s optional `ssh-keygen` check's help names `export --recipient` too.

Book: `site/content/learn/book/targets-and-export.md`, *Closed builds*: the limit first, then the copy, the ledger, the detector and what each change leaves.

Docs: ADR-0118 and the status lines of ADR-0023, `docs/contract.md`,
`docs/formats.md`, the `shuyi-ship` skill, the CI `export` job's comment,
and the rustdoc of `shuyi_assets::recipient`, `image::mark`, `pack::Raw`,
`shuyi_app::recipient` and `shuyi_cli::export::recipient`.

Game: nothing to adopt: searched the game (`gh search code --repo
Zafnok/visions-of-shuyi`) for `ExportOutput`, `ExportResult`,
`ExportInput`, `pack::VERSION`, `overlay_picture` and the removed refusal's
text: hits only in `docs/engine/pins.md` and a done ticket. Its packs are
baked by its exports, which bake version 15 at the next pin.
