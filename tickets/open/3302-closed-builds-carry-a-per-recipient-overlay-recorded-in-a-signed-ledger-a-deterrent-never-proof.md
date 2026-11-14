---
id: "3302"
title: "Closed builds carry a per-recipient overlay recorded in a signed ledger: a deterrent, never proof"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2816", "3300"]
nick_input: none
completed:
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

- [ ] Two recipients' exports of the same build have identical gameplay
      (capture plan passes on both) and different overlay digests, and
      the base build's attestation verifies for both.
- [ ] `--detect-recipient` names the right recipient from each mark alone
      on the three fixtures, and says "no mark found" on the public build.
- [ ] A build with its data segment stripped, a repacked pack and a
      re-encoded picture each still name the recipient through at least
      one other mark; the completion notes record which marks survived
      which attack.
- [ ] The public build has no overlay, no ledger entry and no reserved
      field with a non-zero code.
- [ ] Every output and doc that names a match calls it circumstantial.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

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

*(Filled in by the session that completes the ticket.)*
