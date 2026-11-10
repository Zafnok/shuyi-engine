---
id: "3304"
title: "The bake takes an edition's content root, so a Playdate edition ships only its own files"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3304 — The bake takes an edition's content root

## Context

3082 added editions to `shuyi.ron`
([ADR-0100](../../docs/adr/0100-editions-named-builds-in-shuyi-ron-a-content-root-handed-to-the-build-and-default-features-always-said.md)):
`export <target> --edition <name>` builds a public demo from its own folder
of files (`content`), handed to the desktop and web builds as
`SHUYI_CONTENT`, which the template's `build.rs` points `include_dir!` at.

The Playdate can't do that yet. Its pack is baked by `contract::bake` through
the game's tools (`GameTools::bake` on the tool program's own bundle,
`crates/cli/src/export/playdate.rs`, `prepare`), and the tool program embeds
`files/` whatever the edition says. So `playdate.rs::edition_fits` refuses an
edition with a `content` root (and one with `default_features: true`, which
stays refused: the library never keeps them). A Playdate demo today can only
leave out code, by features, not files.

## Nick input

None.

## Scope

**In:**
- The Playdate export of an edition with a `content` root bakes the pack from
  that folder: the files the pack holds are the edition's folder's, never
  `files/`.
- The way the bake is told the folder is one a later packed desktop or web
  export (3300) can use too. **Either order with 3300:** if 3300 is done, its
  release bake of an edition takes the same folder through this ticket's way;
  if not, nothing to do here for it (3300 uses this ticket's way when it
  lands).
- `edition_fits` stops refusing `content` on the Playdate; it keeps refusing
  `default_features: true`.
- `exposure-report.json`'s `inputs.content_root` names the folder the pack was
  baked from, as it names the program's on desktop and web.

**Out (do not do):**
- Changing the pack's format.
- Editions per target (ADR-0100, *Alternatives*).

## Implementation steps

1. Read `contract::bake` (`prepared`, `GameBake`) and how the tool program
   gets its bundle (`BasicTools`, the template's `examples/tools.rs`). Decide
   between (a) building the tool program with `SHUYI_CONTENT` set, as the
   game's build is, and (b) the bake laying the edition's folder over the
   bundle's `files` layer as `run --watch` lays `assets` over it (the watch's
   layer code in `crates/cli/src/watch.rs`). (b) must replace the layer, not
   lay over it, or the full game's files stay in the pack.
2. Hand the choice to `PlaydateExporter::prepare` for `cx.edition`'s
   `content()`; write an ADR amending ADR-0100 §7.
3. Update `docs/targets/playdate.md` (the paragraph on `--edition`), the
   book's *A public demo* and the `shuyi-ship` skill.

## Acceptance criteria

- [ ] A Playdate export of an edition whose `content` folder lacks a file of
      `files/` has no pack entry of that file, and its canaries pass.
- [ ] The same export with `content` naming `files/` holds it.
- [ ] An edition with `default_features: true` is still refused on the
      Playdate.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the bake input for an edition with and without a content root.
- Integration: a stand-in game's bake through the Playdate exporter's
  `prepare` with two content roots, its pack's entries read back (no SDK:
  stop before `pdc`, as `export/playdate/tests.rs` does).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
