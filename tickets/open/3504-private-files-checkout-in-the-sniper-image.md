---
id: "3504"
title: "A game with private files exports for Linux: the sniper image gets the ssh its private files' checkout needs"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3504 — A game with private files exports for Linux: the sniper image gets the ssh its private files' checkout needs

## Context

Found by 3406's scratch release (2026-11-18, `Zafnok/scratch-release-3406`,
run 44412606416, since deleted): the first tag release of a game made
with `shuyi new --template grid --github` (private files on) failed in
`export (linux)`, at the step *The private files*:

> Unable to locate executable file: ssh.

The base's `export.yml` builds the `linux` package in Valve's sniper
image (`container: ${{ inputs.container }}`, `export::SNIPER`,
ADR-0058), and the private files' action
(`templates/shuyi-default/private-files/.github/actions/private-files/action.yml`)
checks them out with `actions/checkout`'s `ssh-key`, which runs `ssh`.
The sniper image has none. Every release of a game with private files
fails there (the other legs are cancelled by the matrix's fail-fast), a
dry run too, since the key is there. The engine's CI never saw it: it
exports Hello, Grid, which has no private files, and `ci.yml`'s gates run
on `ubuntu-latest`.

3406's scratch run patched its own copy with a step that installs
`openssh-client` in the container before the action
(`apt-get update && apt-get install -y --no-install-recommends
openssh-client`); see 3406's completion notes for whether that worked and
whether the image has `openssl` (3406's seal step needs it).

## Nick input

None.

## Scope

**In:**
- The private files' checkout works in a container job: `ssh` present
  before `actions/checkout` runs with the key, in the sniper image and
  any other image the plan names. Choose where (a step in `export.yml`
  before the action, or a step in the action itself, run only when `ssh`
  is missing) and say why in the completion notes.
- The engine's `.github/workflows/export.yml` stays a byte-equal copy.
- A test in `crates/cli/src/contract/new/tests.rs` that reads the
  workflows with `serde_norway`: in a job with a `container:` and the
  private files' action, a step that makes `ssh` available runs before
  it.

**Out (do not do):**
- Anything else of the release (3406).
- A different image for the Linux export.

## Implementation steps

1. Reproduce: in a scratch game with private files, or by reading
   the sniper image (`docker run --rm <SNIPER> sh -c 'command -v ssh'`).
2. Add the step, guarded so it runs only when `ssh` is missing.
3. Copy `export.yml` to the engine's, bless `plans.golden`
   (`SHUYI_BLESS=1 cargo test -p shuyi-cli --lib template`).
4. The test.

## Acceptance criteria

- [ ] A private-files game's `export (linux)` checks out its private files
      in the sniper image.
- [ ] The test fails without the step.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `crates/cli/src/contract/new/tests.rs`: the step runs before the
  private files' action in every container job that has it.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
