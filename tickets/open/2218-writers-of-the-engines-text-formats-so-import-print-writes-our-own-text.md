---
id: "2218"
title: "Writers of the engine's text formats, so shuyi import --print writes our own text"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["1129"]
nick_input: none
completed:
---

# 2218 — Writers of the engine's text formats

## Context

`shuyi import <file> --print` (ticket 1129,
[ADR-0022](../../docs/adr/0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md)
§6) reads a file through the format registry and prints the asset. ADR-0005
§1 says it gives "the result as our text format". No engine format can
write its kind yet: the `Encode` trait exists (`shuyi_assets::format`,
ADR-0019), and only a test's toy format implements it. So `--print`
prints the asset in Rust's debug form and says so in the answer's
limits.

Writers matter most once Tiled, LDtk and Aseprite are formats (tickets
1123 and 1124): importing a Tiled map should print the engine's own map
text, which a game then keeps. This ticket writes them for the engine's
own text formats first, so the pattern and its tests are in place.

## Nick input

None.

## Scope

**In:**
- `Encode` for the engine's standard text formats whose kinds hold all
  their file says: `palette::PaletteRon` (`PaletteDef`),
  `names::NamesRon` (`Names`), `lighting::LightingRon` (`Lighting`),
  `audio::AudioManifestRon` (`AudioManifest`), `credits::CreditsRon`
  (`CreditsFile`).
- For each, a round-trip test: decoding what `encode` writes gives an
  asset equal to the one encoded (ADR-0007 §4), on the engine's fixtures
  and on `examples/hello-grid/files/`.
- `contract::import::Importers::standard()` registers each of these kinds
  with `kind_with_text`, the text from its `Encode`, so `--print` writes
  it and the "no writer" limit goes away for them.
- The output is the engine's RON as a person writes it: one entry a line,
  sorted keys, a trailing comma (RON's `PrettyConfig`).
- **Either order with 2231:** if 2231 is done, its `AnimationSetRon`
  (animation clips) already has an `Encode`. Give it the `PrettyConfig`
  these use, through the shared helper if this ticket makes one, and
  keep its round trip. If not, nothing to do (2231 writes its own the
  way these are written).
- **Either order with 2345:** if 2345 is done, the `AudioManifestRon`
  writer writes a cue's rules flat on its sound entry, as 2345's RON
  has them (`pick`, `volume_min` written bare, `min_gap_ms`), each only
  when it isn't the default, and the round trip covers a cue with
  rules. If not, nothing to do (2345 adds them to this writer).
- **Either order with 2378:** if 2378 is done, the `AudioManifestRon`
  writer also writes a cue's `max_copies` (bare), `when_full` and
  `priority`, and the manifest's `max_voices` (bare), each only when it
  isn't the default, and never a cue's `lengths_ms` (measured, never
  written); the round trip covers a cue and a manifest with them. If
  not, nothing to do (2378 adds them to this writer).
- **Either order with 2363:** if 2363 is done, the `AudioManifestRon`
  writer writes a cue's `falloff` when it has one (its `curve` by its
  `Ease` name) and leaves it out when it is `None`, and the round trip
  covers a cue with a falloff. If not, nothing to do (2363 adds it to
  this writer).
- **Either order with 2365:** if 2365 is done, the `AudioManifestRon`
  writer writes a music cue's `tempo` when it has one, in the file's
  form (`bpm` as a number, so `bpm_milli` 92 500 is `bpm: 92.5`), and
  the round trip covers a cue with a tempo. If not, nothing to do
  (2365 adds it to this writer).
- **Either order with 2366 (parked):** if 2366 is done,
  `AudioManifestRon`'s writer writes a cue's `caption` only when it is
  `Some`, in the implicit-`Some` form (`caption: "cc.door_opens"`), and
  the round trip covers a captioned cue; if not, nothing to do (2366
  extends the writer, its clause says so).
- **Either order with 2379 (parked):** if 2379 is done, the
  `AudioManifestRon` writer writes a cue's `pitch_min` and `pitch_max`,
  each only when it isn't 100, and the round trip covers a cue with a
  pitch range; if not, nothing to do (2379 adds them to this writer,
  its clause says so).
- **Either order with 2741 (parked):** if 2741 is done, add `Encode`
  for its `crawl-ron` format (`CrawlMap`, in `shuyi-crawl`), with its
  round-trip test (read, write, read gives the same map and the same
  text); if not, nothing to do (2741 writes it, its clause says so).

**Out (do not do):**
- Writers for `.dlg`, `.map`, keymaps and cinematics (their files hold
  comments and layout a writer would lose; a ticket of their own if
  wanted). Language packs belong to the parked 2227, which writes PO
  files and the pack files a PO import changes (not as an `Encode`: a
  pack is several files).
- Tiled, LDtk and Aseprite (tickets 1123 and 1124).
- Changing what any format reads.
- Bringing a game's files up to a newer form of a format: 2517 rewrites
  old names in the text at token spans and keeps comments; a writer here
  would lose them.

## Implementation steps

1. Add `Serialize` where a kind lacks it, or write the text by hand where
   serde's form isn't the file's.
2. Implement `Encode` for each format, with its round-trip tests.
3. Use them in `Importers::standard()` (`kind_with_text`), and update the
   `import` tests that expect the debug form.
4. Mention the writers in `docs/formats.md` (the built-in formats table)
   and `docs/contract.md` (`import`).

## Acceptance criteria

- [ ] Each of the five formats implements `Encode`, and its round trip
      holds on every fixture.
- [ ] `shuyi import files/palette.ron --print` in `examples/hello-grid`
      prints RON that decodes to the same palette, with no "no writer"
      limit.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: encode and decode each format's fixtures; a kind with no entries.
- Property: for random palettes, names, lights, decode(encode(x)) == x.
- Integration: `import --print` on Hello, Grid's palette.

## Completion notes
