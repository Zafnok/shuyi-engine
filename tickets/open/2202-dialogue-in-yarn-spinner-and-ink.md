---
id: "2202"
title: "Dialogue in widely used formats: Yarn Spinner and Ink, beside our .dlg"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: todo
blocked_by: ["2201"]
nick_input: none
completed:
---

# 2202 — Dialogue in Yarn Spinner and Ink

## Context

Nick named dialogue scripts as the first place for "broad compatibility
with normal, widely used file formats" (`docs/decisions.md`). Writers
coming from other engines use two common formats:

- **Yarn Spinner**, line-based, made for games, MIT;
- **Ink**, inkle's narrative language, MIT. It is compiled to JSON by
  inklecate.

Our `.dlg` (game ADR-0011, game ADR-0055) stays the engine's own source
format. ADR-0007 says a second format plugs in as a `Format` for the
`DialogueScene` kind. **From 2201 (done):** that kind is
`shuyi_assets::dialogue::DialogueFile<D>`, the scenes of one file
(`docs/formats.md`, "The kinds of ADR-0007 §1"). Its format `Dlg<D>` is
generic over the game's own directives `D`. A format reads one file
(ADR-0019 §5), and `dialogue::scripts_from_parsed` checks the scenes
against the game's rules after loading. The work is mapping each
format's features onto our scene model, or saying clearly what isn't
supported.

## Nick input

None.

## Scope

**In:**
1. **Research first**, written in `docs/formats/dialogue.md`. For each
   format, list its features and how each maps to our scenes:
   - lines with speakers;
   - choices;
   - variables and conditions;
   - commands (our `@music`, portraits). For Ink, write the rule that
     tells a tag that is a command from any other tag (metadata, say);
   - line ids for voice and translation (game ADR-0045, game ADR-0046);
   - plural, ordinal and select markup (Yarn's `[plural]`, `[ordinal]`
     and `[select]`). 2348 gives screen and data text these choices as
     ICU messages, but dialogue lines don't take that syntax. Refuse the
     markup with a clear error naming the file and line. If the research
     finds the markup in common use, write a follow-up ticket that brings
     2348's syntax to dialogue lines.

   Each feature is supported, supported with a limit, or refused with a
   clear error. For Ink, decide between two routes:
   - read compiled JSON with our own runtime subset;
   - embed an existing Rust Ink runtime, if one exists under an allowed
     license, builds for the web build, and is maintained.

   Check its license from its repository, not from memory.
2. **The Yarn Spinner format** (`.yarn`): a `Format` of
   `DialogueFile<D>`, generic over `D` as `Dlg<D>` is.
   It ships with fixtures from Yarn Spinner's own sample scripts, if their
   license allows copying them, or with our own fixtures otherwise.
3. **The Ink format**, by the route chosen in step 1: a `Format` of
   `DialogueFile<D>` too. `shuyi_assets::ink` is taken (the one-bit
   inks), so it goes under `dialogue` (`dialogue::ink`, say).
4. **The text tools work on imported scenes:** line ids,
   `shuyi check`, and the language status.
5. **Either order with 2721** (the visual novel's stage directions,
   parked): if 2721 is done, its directions work in both formats.
   `<<show …>>` in Yarn and `# show …` as an Ink tag reach the scene as
   commands, by the Ink rule of item 1. Add their rows to
   `docs/formats/dialogue.md` with one fixture per format, and check
   that one scene of 2721's fixture story, in Yarn and in Ink, gives
   the same `VnState` hashes as in `.dlg`. If not, nothing to do: 2721
   adds them when it lands.
6. **Either order with 2726** (parked: the visual novel example): if
   2726 is done, its story is in `.dlg` only. Write one of its scenes
   in Yarn Spinner and one in Ink, so each format's stage directions
   run in CI, and drop the line in its README that says `.dlg` only.
   If not, nothing to do (2726 does it).

**Out (do not do):**
- an editor;
- converting the game's `.dlg` files;
- Twine or Twee (write a ticket if the research finds demand);
- the visual novel's stage directions (`show`, `hide`, `bg`, and the
  rest of ADR-0016 §2), unless 2721 is done (In, item 5). Otherwise a
  command reaches the game as a `.dlg` `@` line does today, through its
  directives `D`.

## Implementation steps

1. Write the research page, and get the routes and limits on record in
   it.
2. Write the Yarn format with fixtures, plus error tests for each
   refused feature.
3. Write the Ink format the same way.
4. Run the Harness on a toy game that plays one scene from each format.

## Acceptance criteria

- [ ] The research page exists, with the mapping tables and the sources
      for each license.
- [ ] A `.yarn` and an Ink scene play in a Harness test of the toy game,
      with choices.
- [ ] If 2721 is done, its stage directions play in Yarn and Ink, with
      the same `VnState` hashes as in `.dlg`.
- [ ] Each refused feature gives a clear error with the file and line.
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Fixtures and error tests per format.
- Integration: a Harness playthrough.

## Completion notes
