---
id: "2594"
title: "check-keys and lint read a game's private files when the checkout is there"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2594 — check-keys and lint read a game's private files when the checkout is there

## Context

ADR-0111 (ticket 2590) lays a game's private files, a checkout of its
private repository in `private/files/`, over `crates/game/files/`
whenever the checkout is there. `cargo shuyi check` reads the game's
bundle, so it checks them. `cargo shuyi check-keys` (and `lint`, which
runs it) scans the folders `shuyi.ron`'s `check_keys.assets` lists from
disk, and `files_under` (`crates/cli/src/check_keys.rs`) fails a folder
that isn't there: a game can't list `private/files`, which a fork or a
clone without access lacks. So a key named in a private text file is
never found (ADR-0111, *Consequences*).

The game does the same for its content (game ADR-0117 §5:
`crates/xtask/src/content_layer.rs` scans the content too).

## Nick input

None.

## Scope

**In:**
- `check-keys` scans `<private_assets.checkout>/files/`
  (`private_assets::PRIVATE_FILES`) beside each folder of
  `check_keys.assets` when the project has `private_assets` and the
  folder is there, and leaves out the paths `not_text` names there at
  the same path under it (the keymap). A finding names the file as
  `private/files/<path>`.
- Without the checkout, nothing changes; the answer's `limits` says the
  private files weren't scanned.

**Out (do not do):**
- `PrivateAssets` and `CheckKeys` gain no field.
- Anything in the game repository.

## Implementation steps

1. `crates/cli/src/check_keys.rs`: the extra folder, and the limit.
2. Tests: a project with a fixture checkout whose text names a key; the
   same without the checkout.
3. `docs/templates.md` (*The private files*) and the template's private
   part's `AGENTS.md` say `check-keys` reads them.

## Acceptance criteria

- [ ] A key named in `private/files/en.ron` is a finding of `cargo shuyi
      check-keys` naming `private/files/en.ron` and its line.
- [ ] Without the checkout, `check-keys` passes and its limits say the
      private files weren't scanned.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the folders scanned, with and without the checkout.
- Snapshot / integration: `check-keys --json` on a fixture project.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
