---
id: "2212"
title: "Parked: In-house sound: Vorbis, part 1: the bit reader, the three headers, and codebooks"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2211"]
nick_input: none
completed:
---

# 2212 — In-house sound: Vorbis, part 1: the headers and codebooks

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the sound research (2902), which may keep, change or drop it.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): our own OGG reader. The engine's
music is Ogg Vorbis (`.ogg`), always loaded (ADR-0007 §3). Our Vorbis
decoder is split in three, each one PR:

- **2212 (this one):** reading bits, and the three header packets
  (identification, comment, setup), including the codebooks;
- **2213:** decoding an audio packet's floors and residues;
- **2214:** the inverse MDCT, windows and overlap, the decoder's API, the
  format, and the conformance tests.

Written from the Xiph *Vorbis I specification*
(`xiph.org/vorbis/doc/Vorbis_I_spec.html`). Never open the source of
lewton, stb_vorbis, libvorbis, Tremor or any other decoder
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§2). The packets come from 2211's `PacketReader`.

## Nick input

None.

## Scope

**In:**
- **`shuyi_sound::vorbis`**, `no_std` with `alloc`:
  - `BitReader`: Vorbis packs bits least significant first (spec §2.1);
    reads of 1–32 bits, and the end-of-packet condition the spec defines
    (a read past the end is not an error inside an audio packet: spec
    §2.1.4);
  - the helpers of spec §9: `ilog`, `float32_unpack`, `lookup1_values`,
    `low_neighbor`, `high_neighbor`, `render_point`, `render_line`
    (the last four are used by 2213; write and test them here);
  - the identification header (spec §4.2.2): channels, rate, the two
    block sizes, with every check the spec lists;
  - the comment header (spec §5): vendor and `KEY=value` comments, kept as
    plain strings, so a game can read `LOOPSTART` or `TITLE` later;
  - the setup header (spec §4.2.4): codebooks, time-domain transforms
    (all zero), floors, residues, mappings and modes, kept as plain
    data for 2213;
  - **codebooks** (spec §3): codeword lengths (ordered and unordered,
    sparse), building the Huffman tree from lengths as §3.2.1
    describes, and lookup types 1 and 2 (vector quantization). Decoding a
    scalar entry and a vector entry from a `BitReader`.
- **`VorbisError`**: every check of the spec that fails is an error that
  names it.

**Out (do not do):**
- Floors, residues and audio packets (2213), the MDCT and output (2214).
- Floor 0's decoding (2213 decides; its setup data is read here).
- Speed work: correctness first. A lookup table for short codewords is
  2214's call after measuring.

## Implementation steps

1. `src/vorbis.rs`, `src/vorbis/bits.rs`, `src/vorbis/helpers.rs`,
   `src/vorbis/header.rs`, `src/vorbis/codebook.rs`,
   `src/vorbis/setup.rs`.
2. `BitReader` with tests on hand-made bytes (spec §2.1.4 has examples).
3. The helpers of §9 with the examples in the spec's text.
4. Identification and comment headers.
5. Codebooks: lengths, the tree (an underspecified or overspecified
   tree is an error, except the single-entry case §3.2.1 allows), lookups.
6. The setup header, reading the fixture of 2211 end to end.
7. Make two or three more `.ogg` fixtures with a Vorbis encoder run as a
   black box (mono, stereo, a low and a high quality setting), with the
   fixture `README.md` saying how. Keep each under 100 KB.
8. In the completion notes, list the spec sections used.

## Acceptance criteria

- [ ] Every fixture's three headers parse, with the rate, channels, block
      sizes and comments its encoder was given.
- [ ] Each codebook in the fixtures decodes every one of its entries back
      from its own codeword (a test that encodes each entry's codeword
      by hand from the lengths, then decodes it).
- [ ] No input panics.
- [ ] The PR body says the code was written from the Vorbis I
      specification.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `BitReader`; each §9 helper; a hand-made codebook of each kind
  (ordered lengths, sparse, lookup 1, lookup 2).
- Property: for random valid length lists, the tree decodes each entry's
  codeword to that entry.
- Fuzz-style: random bytes as each header, and fixture headers with bits
  flipped, never panic (ADR-0007).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
