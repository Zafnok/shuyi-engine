---
id: "2504"
title: "Targets in the wizard: pick several export targets, check the picked modules' needs against them, and give the new game a CI job per target"
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
[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§5 sets the rules:

- the picked targets fill the manifest's `targets` table (3001);
- the wizard runs the capabilities check (`capabilities_fit`, ADR-0012
  §1, the parked 3017) on the picked modules' `needs` against each
  picked target, and refuses a game that doesn't fit;
- the new game's CI checks and exports each picked target (3004);
- a target with its own duties brings a part (the Playdate's 1-bit
  patterns and controls check).

2502 asks the targets question with `desktop` and `web` only until 3001
exists. This ticket widens it to every target profile and adds the
checks.

**Either order with 3017** (parked): if 3017 is done, call its
`capabilities_fit` as below. If not, skip that check, add a `limits`
entry ("modules not checked against targets: 3017 isn't built"), and
3017 wires it in when it lands (its scope says so). Never add a second
per-module list of targets: `needs` is the one place a module says what
hardware it wants.

## Nick input

None.

## Scope

**In:**
- **The targets question** lists every profile (3001) with its status.
  `planned` profiles (`switch`, `android`, `ios`, and `gba`, `nds` and
  `dsi` once 3017 adds them) are shown with their ticket and can't be
  picked (ADR-0013 §6).
- **The check**, in the composer, if 3017 is done: for each picked
  target, `capabilities_fit(profile, needs)` over the picked modules'
  `needs` (and the template parts' stage needs, if a part declares any).
  Any problem is `ComposeError::DoesNotFit { target, problems }`, which
  prints 3017's own error text. It runs before anything is written.
  `left_out` entries are printed as warnings.
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
- **The JSON** of `--questions` (2502) gives each module's `needs` and
  each target's capabilities, if 3017 is done, so an agent can explain a
  refusal before it happens.

**Out (do not do):**
- new exporters or backends (3004, 3010, 3012);
- choosing frame sizes for the developer beyond the profile's default;
- parked targets (3014–3016, 3021, 3024) beyond showing them as planned;
- the capabilities types and the check themselves (3017).

## Implementation steps

1. Widen the targets question to the profiles.
2. Add the check to the composer, before writing, or the `limits` entry
   if 3017 isn't done.
3. Write the manifest entries and the CI matrix in the base part.
4. Write the Playdate and Steam Deck parts, with what 3002, 3006 and
   3012 give if they are done.

## Acceptance criteria

- [ ] `shuyi new demo --no-input --targets desktop,web,playdate` makes a
      game whose manifest lists three targets and whose CI has three
      matrix entries.
- [ ] If 3017 is done: a fixture module with `needs.three_d =
      "software"` picked with `--targets playdate,web` passes, and with
      a fixture profile of `three_d: none` fails before writing, with
      3017's error text. If 3017 isn't done: the `limits` entry is
      there.
- [ ] A planned target by flag (`--targets switch`) fails with its
      status and ticket 3014.
- [ ] The new game made with `desktop,web` passes `shuyi check --target
      all` in CI.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the needs × target check (or its `limits` entry); manifest
  entries from profiles.
- Integration: the three-target game above; the refusal; the CI job
  builds the `desktop,web` game and runs its checks.

## Completion notes
