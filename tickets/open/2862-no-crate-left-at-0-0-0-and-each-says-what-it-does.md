---
id: "2862"
title: "No shuyi crate left at 0.0.0 after the release, and each crate's page says what it does today, thin ones included"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2859", "2861", "2005"]
nick_input: none
completed:
---

# 2862 — No crate left at 0.0.0, and each says what it does

## Context

The engine holds its crates' names on crates.io as empty 0.0.0
placeholders until the first release (`docs/decisions.md`, 2026-10-21,
2026-10-22, 2026-11-01, 2026-11-24, 2026-11-26). Each held name is a
workspace crate, and 2005 publishes the workspace's engine crates (3510:
the publish list is the crates `is_engine` names), so after the release
none should stay empty.

The heat check of 2859 found two ways it can still read as squatting on
r/rust and Hacker News, where readers open crates.io to check:

- a crate left at 0.0.0 by a release that stopped part way, or left out
  of the list;
- a published crate whose page says nothing, or more than it does: today
  `shuyi-net` and `shuyi-playdate` are early and narrow, and their README
  must say so.

crates.io's usage policy (RFC 3463,
<https://rust-lang.github.io/rfcs/3463-crates-io-policy-update.html>) is
aimed at names held without real work. 2859 says no launch post goes out
before this ticket is done, and what to do with a crate a release leaves
out.

## Nick input

`None.` If 2859's answer is "hand the name back" for a crate the release
left out, that is Nick's step on crates.io, listed in the PR.

## Scope

**In:**
- A check, `cargo xtask crates-io-state` (in `crates/xtask/`): for each
  engine crate, the newest version on crates.io, failing on any at
  0.0.0 or missing; run in the release workflow after the publish, and by
  hand.
- Each engine crate's README (the one crates.io shows) opens with one
  line saying what it does today, and, for an early one, what it doesn't
  yet ("LAN rooms only; no matchmaking yet"), with its ROADMAP status.
- A test that every engine crate has a README its `Cargo.toml` names, and
  that its first paragraph has no word from 2861's list of words to keep
  out.
- The FAQ line on the honest page (2861): "Yes, the names were held
  before the first release; every one is a crate you can use, and here is
  what each does", linking a table of the crates.

**Out (do not do):**
- Holding any new name.
- The release itself (2005) and the publish list (3510, done).
- Any claim that a crate does more than its code does.

## Implementation steps

1. Write `crates-io-state` over the crates.io API (a recorded answer in
   tests; the network only when run).
2. Add it to the release workflow after the publish step.
3. Write or fix each engine crate's README opening; add the README test.
4. Add the FAQ line and the crates table to the honest page.
5. Run it against crates.io after the release and note the result.

## Acceptance criteria

- [ ] `cargo xtask crates-io-state` fails a recorded answer with a crate at 0.0.0, and passes one with every crate past it.
- [ ] Run after the first release, it passes (output in the completion notes).
- [ ] Every engine crate's README opens with what it does today; the early ones say what they don't do yet.
- [ ] The README test passes and fails a crate with no README.
- [ ] The honest page has the FAQ line and the crates table.
- [ ] The completion notes have `Book:` and `Skills:` lines saying why neither changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `crates-io-state` over recorded answers (all published; one at
  0.0.0; one missing).
- Unit: every engine crate names a README, and its first paragraph has no
  word from the keep-out list.

## Completion notes

*(Filled in by the session that completes the ticket.)*
