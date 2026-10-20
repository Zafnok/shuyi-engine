---
id: "2211"
title: "Parked: In-house sound: our own Ogg container reader: pages, CRC, packets and granule positions"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["2210"]
nick_input: none
completed:
---

# 2211 — In-house sound: our own Ogg container reader

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): sound goes in-house, "with our own
OGG/WAV readers". An `.ogg` music file is a Vorbis stream inside the Ogg
container. This ticket is the container only. The Vorbis decoder is
2212–2214 and reads the packets this ticket gives it.

Written from RFC 3533 (*The Ogg Encapsulation Format Version 0*), never
from the source of the `ogg` crate, libogg or any other reader
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§1).

## Nick input

None.

## Scope

**In:**
- **`shuyi_sound::ogg`**, in the crate from 2210, `no_std` with `alloc`:
  - `Page`: the header fields of RFC 3533 §6 (version, header type flags,
    granule position, serial number, sequence number, CRC, segment table)
    and the body;
  - the CRC-32 the RFC specifies (polynomial 0x04C11DB7, no reflection,
    initial value 0, no final XOR), with a table built at compile time by
    a `const fn`;
  - `PacketReader` over a byte slice: yields whole packets of one logical
    stream across page boundaries (segments of 255 continue the packet),
    with each packet's end granule position when its page ends it;
  - capture-pattern resync: after a bad page (wrong CRC, truncated), it
    finds the next `OggS` and carries on, and reports how many bytes it
    skipped;
  - the first stream's serial number is used, and pages of other streams
    are skipped (chained and multiplexed files: see *Out*).
- **Errors** name the offset and the problem.

**Out (do not do):**
- Vorbis (2212–2214).
- Chained streams (one file, several streams one after another) and
  multiplexed streams: write a ticket if a game brings one.
- Writing Ogg.
- Seeking by granule position: 2214 needs only the last page's granule
  for the length. Write a ticket if streaming needs seeking.

## Implementation steps

1. Add `src/ogg.rs`, `src/ogg/crc.rs`, `src/ogg/page.rs`,
   `src/ogg/packets.rs`.
2. The CRC first, tested against values computed by hand for short
   inputs and against the CRC field of the fixture pages.
3. Page parsing, then the packet reader, then resync.
4. A test helper that writes pages (with a correct CRC) from a list of
   packets, so tests build any layout: a packet spanning three pages, a
   packet ending exactly on a 255 boundary (followed by a 0 lacing
   value), a page with no packet ending (granule `-1`).
5. One real `.ogg` fixture, made by us with a Vorbis encoder run as a
   black box (ADR-0021 §1), under `crates/sound/tests/fixtures/ogg/`
   with a `README.md` saying how it was made.
6. In the completion notes, list the documents used, by name and link.

## Acceptance criteria

- [ ] The fixture's packets come out whole, in order, with the granule
      positions its pages carry.
- [ ] A page with a wrong CRC is skipped and reported, and the next good
      page is read.
- [ ] No input panics.
- [ ] The PR body says the code was written from RFC 3533.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: CRC values; each header flag (continued, first, last); the
  lacing edge cases in step 4.
- Property: any list of packets written by the helper reads back equal,
  whatever the page sizes.
- Fuzz-style: random bytes, and valid files with random bytes flipped,
  never panic (ADR-0007).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
