---
id: "2213"
title: "Parked: In-house sound: Vorbis, part 2: audio packets: modes, floors, residues and channel coupling"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2212"]
nick_input: none
completed:
---

# 2213 — In-house sound: Vorbis, part 2: floors, residues and coupling

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the sound research (2902), which may keep, change or drop it.

## Context

The second of three Vorbis tickets (2212 lists them). 2212 left the
headers and codebooks as plain data. This ticket decodes one audio packet
into each channel's spectrum, ready for 2214's inverse MDCT.

Written from the Xiph *Vorbis I specification*, §4.3 (audio packet
decode), §6 (floor 0), §7 (floor 1) and §8 (residues), never from another
decoder's source
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§2).

## Nick input

None.

## Scope

**In:**
- **`shuyi_sound::vorbis::packet`**: from an audio packet and the setup,
  produce, per channel, the spectrum of `blocksize / 2` values, plus the
  packet's block size and window flags (spec §4.3.1):
  - the packet type bit, the mode number, the block flag, and the
    previous and next window flags of long blocks;
  - **floor 1** (§7.2): decode the curve's points, then synthesize it with
    `render_line` from 2212, and the "unused" case that zeroes a channel;
  - **floor 0** (§6): decode and synthesize it as the spec says. Encoders
    have not written floor 0 for many years, so if no fixture uses it,
    keep it tested by a hand-made packet only;
  - **residues 0, 1 and 2** (§8), with their classification passes and
    the "do not decode" vector for unused channels (§4.3.4);
  - **inverse coupling** (§4.3.5), magnitude and angle, in reverse order
    of the mapping's steps;
  - the dot product of floor and residue (§4.3.6).
- **The end-of-packet rule** (§2.1.4): a packet cut short decodes as far
  as it goes, and the rest of its spectrum is zero.

**Out (do not do):**
- The MDCT, windows, overlap-add and output samples (2214).
- Speed work, past keeping the work proportional to the packet's size.

## Implementation steps

1. `src/vorbis/packet.rs`, `src/vorbis/floor0.rs`, `src/vorbis/floor1.rs`,
   `src/vorbis/residue.rs`, `src/vorbis/coupling.rs`.
2. Floor 1 first, since every modern fixture uses it.
3. Residues 0, 1 and 2, sharing the classification pass.
4. Coupling, then the dot product.
5. Floor 0 last.
6. A debug-only dump (behind a test helper, not the public API) of a
   packet's floor and residue, to compare against the spec's worked
   steps while writing.
7. In the completion notes, list the spec sections used.

## Acceptance criteria

- [ ] Every audio packet of 2212's fixtures decodes without error.
- [ ] Inverse coupling turns a hand-made magnitude and angle pair into the
      values the spec's table (§4.3.5) gives, for each of its four cases.
- [ ] A packet cut at any byte decodes without panicking.
- [ ] The PR body says the code was written from the Vorbis I
      specification.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: floor 1 on hand-made points; each residue type on a hand-made
  codebook; coupling's four cases; floor 0 on a hand-made packet.
- Property: decoding a packet never writes past `blocksize / 2` per
  channel, for random packets with a fixture's setup.
- Fuzz-style: random packets with each fixture's setup never panic
  (ADR-0007).
- Snapshot: a checksum of each fixture's spectra (`insta`), so 2214's
  work can't change them silently.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
