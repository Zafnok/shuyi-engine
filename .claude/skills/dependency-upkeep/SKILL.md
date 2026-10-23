---
name: dependency-upkeep
description: Work one Dependabot PR or one GitHub issue labelled advisory (a RustSec advisory from the weekly check) to a merge. Use when pointed at a Dependabot PR, a dependabot/... branch, an advisory issue, or asked to bump, upgrade or audit the dependencies or the actions' pins.
---

# Dependency upkeep

A Dependabot PR, or one `advisory`-labelled GitHub issue, is the ticket
for one PR: an exception to "work one ticket at a time from
`tickets/open/`" (`AGENTS.md` hard rule 1,
[ADR-0037](../../../docs/adr/0037-dependency-upkeep-dependabot-a-weekly-advisory-check-workflow-audits-and-bot-work-as-its-own-ticket.md) §5).
There is no ticket file to write or move.

`work-ticket` still governs everything this skill doesn't change: scope,
the layers, nothing of the game, licenses, gates, never weakening a gate,
one PR, then stop.

Never use Dependabot's comment commands: each starts with a mention of
the bot (`AGENTS.md` rule 11). An `ignore` entry in
`.github/dependabot.yml` does the same job.

## A Dependabot PR

1. **Read what moves:** `gh pr view <n> --json files,body`. It is only
   `Cargo.lock`, a requirement in a `Cargo.toml`, or action pins. A PR
   that moves a `rust-toolchain.toml` (2012's update) is worked with the
   `bump-rust` skill instead. Check out its branch
   (`gh pr checkout <n>`); push your fixes onto it.
2. **The `backend` group** (macroquad, miniquad, `quad-*`, `sapp-jsutils`,
   `gilrs*`):
   - re-vendor the JS as [`crates/cli/web/README.md`](../../../crates/cli/web/README.md)
     says, its local patch to `quad-storage.js` included;
   - search the repository for each old version string and update every
     place that names it (`THIRD_PARTY_ASSETS.md`, that README, any table
     in code);
   - keep `quad-snd` in `crates/app/Cargo.toml` equal to the one
     macroquad's `audio` feature uses;
   - run `cargo deny check advisories`, and remove an ignore in
     `deny.toml` that no longer matches anything;
   - build Hello, Grid for the web (`cd examples/hello-grid && cargo shuyi
     web --release`) and open it.
3. **A requirement change:**
   - make the templates' requirement follow (`templates/*/Cargo.toml`;
     `cargo xtask check-pins` fails until it does);
   - build the game against the branch (`run-gates`, *The game at the
     next pin*);
   - if the public API changes (the engine's API shows a type of the
     crate that moved), it is step 6: the game must follow (`AGENTS.md`,
     *Working with the game repository*), so it is a ticket of its own.
     CI's `api` job fails a Dependabot PR that breaks the API (ADR-0038
     §5: no break without a ticket), and its summary shows the change.
4. **Actions:** if Dependabot left a template's pin behind
   (`cargo xtask check-pins` names it), push the bump onto its branch.
   An action keeps one SHA across the repository.
5. **Read every check on the PR** (`gh pr checks <n>`), CodeQL's
   `codeql-result` and the code scanning alerts too. When all are green,
   leave one comment that lists what was checked, then merge as any PR
   is merged.
6. **An upgrade that needs real work:** write a ticket (`write-ticket`),
   close the PR with a comment that names it, and add an `ignore` entry
   for that version to `.github/dependabot.yml` if Dependabot opens it
   again.

## An `advisory` issue

The weekly check (`.github/workflows/security.yml`) opens it, or comments
on it, when `cargo deny check advisories` fails.

1. Read the run it links. For each advisory, if a fixed version exists,
   move to it (`cargo update -p <crate> --precise <version>`, or the
   requirement in a `Cargo.toml`, then step 3 above).
2. If none exists, add an `ignore` to `deny.toml` with a reason that ends
   "revisit on `<crate>` upgrade", the shape of the ignores there.
3. Work on an `advisory/<id>` branch (`ticket-lint` ignores it). The PR
   title is `[advisory #<n>] <what moves>`, and its body says
   `Closes #<n>`.
4. Replacing an unmaintained crate is a ticket of its own
   (`write-ticket`), never this PR.

Gates: the `run-gates` skill, as for any PR.
