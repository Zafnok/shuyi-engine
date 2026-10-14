---
id: "2201"
title: "Formats are plugins: asset kinds, the Format trait, the registry, and today's loaders as built-in formats"
type: feature
milestone: MVP
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["2002", "game#1131"]
nick_input: none
completed:
---

# 2201 — Formats are plugins: the registry

## Context

ADR-0007 asks for three things. Asset kinds are separate from file
formats. A format is a plugin (`Format`, registered in `Formats`).
Standard formats always load, and optimized ones are derived from text
sources and tested equal.

Nick asked for this so that "if we want to introduce a highly optimized
file format, this should be easier than ever. But also maintain broad
compatibility with normal, widely used file formats"
(`docs/decisions.md`).

After the carve (game 1103–1105) and the move (game 1131), every loader
in `shuyi-assets` is a pure parser (`from_source(file, text)`) plus a
`load()` that reads the bundle. This ticket turns those parsers into
`Format`s without changing what any of them accepts.

## Nick input

None. Nothing a player sees changes.

## Scope

**In:**
- In `shuyi-assets`:
  - the asset kinds listed in ADR-0007 §1, as plain types (most already
    exist after the carve; this names them);
  - `trait Format` and `trait Encode` as ADR-0007 §2 shows;
  - `Formats` registration with the conflict rule;
  - lookup by sniffing, then by extension;
  - `AssetPath`;
  - `DecodeCx` (limits, the bundle for references).
- The built-in formats: PNG (header, size, pixels as the frame needs
  them), the RON sidecars, `.dlg`, `.map`, language packs, cinematics,
  keymaps, WAV and OGG headers.
- Each built-in format has:
  - its existing tests, moved;
  - a fuzz-style property test (random bytes never panic);
  - a size limit.
- A game registers its own kinds and formats at start-up. The game crates
  (in the game repository) change only their `load()` calls, in a game
  PR that moves the pin (game ticket to be written by this one's PR).
- `docs/formats.md` covers how to write a format plugin, with a toy
  example (`*.txtimg`: a text image format used only in tests).

**Out (do not do):**
- new formats (2202, 2203);
- the optimized pack (2204);
- importers that 1123 and 1124 add (they will be written as `Format`s
  when unparked; update those tickets' text to say so).

## Implementation steps

1. Write the traits, the registry and its tests (conflicts, order, sniff
   before extension).
2. Wrap each existing parser as a `Format`. Every existing test passes
   unchanged.
3. Add the fuzz-style tests and the limits.
4. Write the docs page and the toy plugin. Update 1123 and 1124.
5. Write the game-side ticket in the game repository (`write-ticket`
   there): register the game's kinds, move the pin.

## Acceptance criteria

- [ ] A test registers the toy `txtimg` format and loads an image through
      it with no change to the code that asks for the image.
- [ ] Registering two formats for one extension fails with a message
      naming both, unless the second replaces the first.
- [ ] No existing test changes behavior. The game, built against this
      revision, passes its gates (checked in the game PR).
- [ ] Random bytes never panic any built-in format (property tests).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the registry.
- Property: decoders on random bytes.
- Integration: the toy plugin.

## Completion notes
