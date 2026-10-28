---
id: "2032"
title: "deny.toml's license allow list and credits::LICENSES kept equal by a test"
type: infra
milestone: After MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2032 — `deny.toml`'s license list and `credits::LICENSES` kept equal

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). ADR-0002 §8's permissive list of code and font licenses is
written by hand in two places:

- **`deny.toml`** (lines 30–47): `[licenses] allow` holds 15 ids, the
  14 code ids of ADR-0002 §8 and `OFL-1.1` (line 46, with the comment
  `# only used if a font crate needs it`). Its header (line 3) says the
  list "is the permissive list of ADR-0002 §8". `cargo deny` reads it for
  the crates the engine ships (rule 7).
- **`shuyi_assets::credits::LICENSES`** (`crates/assets/src/credits.rs`,
  line 84): `[&str; 15]`, the same 15 ids in the same order. The credits
  file's entries are checked against it (`from_source`, line 290).

Nothing ties them: no `.rs` file reads `deny.toml`. If one of them gains
or loses an id, the other doesn't follow, and no gate notices.

Two open tickets add a third copy beside them: 2223 and 2526 each add
`credits::CODE_LICENSES`, the 14 code ids (`LICENSES` without
`OFL-1.1`). They agree with each other on it, but neither checks it
against `deny.toml`.

`audio::LICENSES` (`crates/assets/src/audio.rs`, line 30) is another
list: art and audio (`CC0-1.0`, `CC-BY-3.0`, `CC-BY-4.0`, `Own`). It
has no counterpart in `deny.toml`, which checks crates only.

## Nick input

None.

## Scope

**In:**
- A unit test in `crates/assets/src/credits.rs`'s tests module that
  reads `deny.toml` at compile time (`include_str!`, as
  `crates/assets/src/pack/tests.rs` reads `targets/playdate.ron`), takes
  the ids of `[licenses] allow`, and checks that they are the same set as
  `credits::LICENSES`. On a difference it fails with the ids that are only
  in `deny.toml` and the ids that are only in `LICENSES`.
- The reader of the allow list is a small private function in the tests
  module, line by line like `cargo xtask check-toolchain` reads TOML
  (`crates/xtask/src/toolchain.rs`): after the `[licenses]` header, from
  the `allow = [` line to the `]` line, every quoted string on a line,
  with anything after a `#` dropped. No new dependency.
- One sentence in `LICENSES`'s doc comment: it is the same list as
  `deny.toml`'s `[licenses] allow`, and a test keeps them equal. One
  comment line under `deny.toml`'s header saying the same, naming
  `shuyi_assets::credits::LICENSES`.
- **Either order with 2223:** if 2223 is done, `credits::CODE_LICENSES`
  exists: the test also checks that `CODE_LICENSES` plus `OFL-1.1` is
  the same set as `deny.toml`'s allow list, and the sentence goes in the
  doc comment 2223 wrote for `LICENSES`; if not, nothing more to do
  (whichever of 2223 and 2526 adds `CODE_LICENSES` extends this test).
- **Either order with 2526:** if 2526 is done, `credits::CODE_LICENSES`
  exists: the test also checks that `CODE_LICENSES` plus `OFL-1.1` is
  the same set as `deny.toml`'s allow list; if not, nothing more to do
  (whichever of 2223 and 2526 adds `CODE_LICENSES` extends this test).
- **For the game:** no API change. The test and the comments add no
  public item (ADR-0038 §6: no `Breaking:` line).

**Out (do not do):**
- Changing any list: no id is added to or removed from `deny.toml` or
  `LICENSES`.
- `audio::LICENSES`, and the art list 2223 adds (`ART_LICENSES`): art
  and audio lists, with nothing in `deny.toml` to match.
- `deny.toml`'s `[licenses.exceptions]` and `[[licenses.clarify]]`: an
  exception for one crate is not the allow list.
- ADR-0002 §8's list: an ADR is a record, not checked by a gate.
- The `game ADR-0013` wording in `LICENSES`'s doc comment and in the
  credits errors: 2223 rewrites those.
- An xtask check instead of the test: `xtask` doesn't depend on
  `shuyi-assets`, and the test runs in the `test` job already.

## Implementation steps

1. In `crates/assets/src/credits.rs`'s `mod tests`, add
   `const DENY: &str = include_str!("../../../deny.toml");` and a private
   `fn allow_list(toml: &str) -> Vec<&str>` that returns the ids of
   `[licenses] allow` as described in Scope.
2. Add a test of `allow_list` on a short sample: a `[licenses]` header,
   `allow = [`, two ids, one with a `# comment` after it, `]`, and an
   `allow = ["X"]` line under another header that must not be read.
3. Add the test `licenses_are_deny_tomls_allow_list`: sort and dedupe
   both `allow_list(DENY)` and `LICENSES`, and `assert_eq!` them with a
   message naming the ids only in one side.
4. Settle the 2223 and 2526 clauses above with what is done.
5. Add the sentence to `LICENSES`'s doc comment and the comment line to
   `deny.toml`.
6. Run the gates (`cargo deny check` still passes: only a comment
   changed in `deny.toml`).

## Acceptance criteria

- [ ] `cargo test -p shuyi-assets credits` runs
      `licenses_are_deny_tomls_allow_list` and it passes.
- [ ] Adding an id to `deny.toml`'s allow list by hand (not committed)
      makes that test fail with a message naming the id; so does
      removing one from `LICENSES`.
- [ ] `grep -n "credits::LICENSES" deny.toml` finds the new comment.
- [ ] `git diff main -- deny.toml` shows comment lines only.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-assets`): `allow_list` on the sample of step 2;
  `licenses_are_deny_tomls_allow_list` on the real files.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
