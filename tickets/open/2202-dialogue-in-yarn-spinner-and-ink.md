---
id: "2202"
title: "Dialogue in widely used formats: Yarn Spinner and Ink, beside our .dlg"
type: feature
milestone: After MVP
model: opus-5.5
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
`DialogueScene` kind. The work is mapping each format's features onto our
scene model, or saying clearly what isn't supported.

## Nick input

None.

## Scope

**In:**
1. **Research first**, written in `docs/formats/dialogue.md`. For each
   format, list its features and how each maps to our scenes:
   - lines with speakers;
   - choices;
   - variables and conditions;
   - commands (our `@music`, portraits);
   - line ids for voice and translation (game ADR-0045, game ADR-0046).

   Each feature is supported, supported with a limit, or refused with a
   clear error. For Ink, decide between two routes:
   - read compiled JSON with our own runtime subset;
   - embed an existing Rust Ink runtime, if one exists under an allowed
     license, builds for the web build, and is maintained.

   Check its license from its repository, not from memory.
2. **The Yarn Spinner format** (`.yarn`): a `Format` for `DialogueScene`.
   It ships with fixtures from Yarn Spinner's own sample scripts, if their
   license allows copying them, or with our own fixtures otherwise.
3. **The Ink format**, by the route chosen in step 1.
4. **The text tools work on imported scenes:** line ids,
   `shuyi check`, and the language status.

**Out (do not do):**
- an editor;
- converting the game's `.dlg` files;
- Twine or Twee (write a ticket if the research finds demand).

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
- [ ] Each refused feature gives a clear error with the file and line.
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Fixtures and error tests per format.
- Integration: a Harness playthrough.

## Completion notes
