---
id: "2504"
title: "Targets in the wizard: pick several export targets, check the picked modules against them, and give the new game a CI job per target"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2502", "3001", "3004"]
nick_input: none
completed:
---

# 2504 — Targets in the wizard

## Context

Nick, 2026-10-17 (`docs/decisions.md`): the wizard asks "what export
targets they want to support".
[ADR-0011](../../docs/adr/0011-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§5 sets the rules:

- the picked targets fill the manifest's `targets` table (3001);
- each module says which targets it builds for, and the wizard refuses a
  module a picked target doesn't take;
- the new game's CI checks and exports each picked target (3004);
- a target with its own duties brings a part (the Playdate's 1-bit
  patterns and controls check).

2502 asks the targets question with `desktop` and `web` only until 3001
exists. This ticket widens it to every target profile and adds the
checks.

## Nick input

None.

## Scope

**In:**
- **Module metadata gains `targets`** (ADR-0009 §4, under
  `[package.metadata.shuyi]`): a list of target ids, or `["*"]`. 2003's
  `ModuleInfo` reads it; a module without it means `["*"]`.
  `check-modules` fails on an id that isn't a target profile.
- **The targets question** lists every profile (3001) with its status.
  `planned` profiles (`switch`, `android`, `ios`) are shown with their
  ticket and can't be picked (ADR-0011 §6).
- **The check**, in the composer: for each picked module and each picked
  target, the module must list the target. Otherwise
  `ComposeError::TargetUnsupported { module, target }`, worded as "rigid-
  body physics isn't built for `playdate`". It runs before anything is
  written.
- **The manifest:** for each picked target, a `targets` entry with the
  profile's default frame size and scale (3001).
- **The new game's CI** (`.github/workflows/ci.yml` in the base part)
  gets a matrix over the picked targets, each running `shuyi check
  --target <id>` and `shuyi export <id>` (3004). A target whose exporter
  needs a tool CI can't have (the Playdate SDK) runs the check only, and
  the workflow says why in a comment.
- **Target parts** in the built-in template:
  - `target-playdate`, when `playdate` is picked: if 3006 is done, a
    palette pattern map with the template's roles; if 3002 is done, its
    check in CI. If either isn't done, the part leaves it out, and that
    ticket adds it (their scopes say so);
  - `target-steam-deck`, when `steam-deck` is picked: if 3012 is done,
    its export in the matrix; if not, 3012 adds it (its scope says so).
- **The JSON** of `--questions` (2502) gives each module's targets, so an
  agent can explain a refusal before it happens.

**Out (do not do):**
- new exporters or backends (3004, 3010, 3012);
- choosing frame sizes for the developer beyond the profile's default;
- parked targets (3014–3016) beyond showing them as planned.

## Implementation steps

1. Add `targets` to `ModuleInfo` and `check-modules`, with fixtures.
2. Widen the targets question to the profiles.
3. Add the check to the composer, before writing.
4. Write the manifest entries and the CI matrix in the base part.
5. Write the Playdate and Steam Deck parts, with what 3002, 3006 and
   3012 give if they are done.

## Acceptance criteria

- [ ] `shuyi new demo --no-input --targets desktop,web,playdate` makes a
      game whose manifest lists three targets and whose CI has three
      matrix entries.
- [ ] A fixture module with `targets = ["desktop"]` picked with
      `--targets web` fails before writing, naming both.
- [ ] `check-modules` fails on a module listing an unknown target.
- [ ] A planned target by flag (`--targets switch`) fails with its
      status and ticket 3014.
- [ ] The new game made with `desktop,web` passes `shuyi check --target
      all` in CI.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the module × target check; manifest entries from profiles.
- Integration: the three-target game above; the refusal; the CI job
  builds the `desktop,web` game and runs its checks.

## Completion notes
