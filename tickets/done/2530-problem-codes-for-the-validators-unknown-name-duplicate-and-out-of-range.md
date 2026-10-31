---
id: "2530"
title: "Problem codes for the validators: unknown-name, duplicate and out-of-range in the keymap, the language packs and the maps"
type: feature
milestone: MVP
model: implementation
effort: medium
status: done
blocked_by: ["2509"]
nick_input: none
completed: 2026-10-21
---

# 2530 — Problem codes for the validators: unknown-name, duplicate and out-of-range

## Context

Ticket 2509 gave every error the contract gives a problem code
([ADR-0028](../../docs/adr/0028-problem-codes-a-stable-code-by-class-a-help-line-a-page-and-exit-code-3-for-the-setup.md),
[`docs/contract.md`](../../docs/contract.md), *Problem codes*): the
setup and operation codes, and the content codes `SH2001` to `SH2007`
on the format registry, the RON reader and the pack check. Its *Size*
rule moved the three codes inside the validators here, to keep its PR
near 600 lines: `SH2008` `unknown-name`, `SH2009` `duplicate` and
`SH2010` `out-of-range`.

Today a keymap's unknown action, a text key used twice or a map too
wide reaches `shuyi check` as a problem with no `code` (`null` in the
JSON). An agent can't branch on it, and gets no help line.

Adding a code where there was none is no break: `code` is in the
answer's shape already, so `contract::VERSION` doesn't change.

## Nick input

None.

## Scope

**In:**

- Three content codes in `shuyi_assets::error::codes`, after
  `PACK_DIFFERS`:

  | Code | Name | Constant | Given where |
  | ---- | ---- | -------- | ----------- |
  | `SH2008` | `unknown-name` | `UNKNOWN_NAME` | the keymap's unknown action, key chord or button (`validate_bindings`, `crates/assets/src/keymap.rs`, "unknown action", and the chord and button parse errors it pushes); a map's unknown terrain in its legend ("legend '…': unknown terrain") and a tile character not in the legend ("'…' is not in the legend", both `crates/assets/src/grid_map.rs`) |
  | `SH2009` | `duplicate` | `DUPLICATE` | a binding listed twice for an action or bound to two actions (`validate_bindings`, "is listed twice for", and the one bound to two actions next to it); a layout listed twice (`keymap.rs`, "layout … is listed twice"); a text key used twice in a language pack (`crates/assets/src/lang.rs`, both places, and `crates/assets/src/lang/lines.rs`) |
  | `SH2010` | `out-of-range` | `OUT_OF_RANGE` | the keymap's repeat interval and crank notch (`keymap.rs`, their range checks); a map too wide or too tall (`grid_map.rs`) |

  Find each site by the message quoted; the line numbers move.
- Each code's row in `TABLE` (`crates/cli/src/contract/codes.rs`), with
  a title and a help line, and its page `crates/cli/errors/SH2008.md` …
  `SH2010.md` in the form of the others (title, class and exit code,
  what it means, how to fix it, one example).
- Each site written `ContentError::new(…).with_code(codes::UNKNOWN_NAME)`
  (or `.at(…)` first), with the module, so the gate in
  `contract/codes/tests.rs` sees it.

**Out (do not do):**

- codes from a game's own validators, or a range of codes for a game;
- codes on `TargetProblem` and `FitProblem` (2509's *Out*);
- a code for every message of every decoder: only the sites above;
- changing any message's words.

## Implementation steps

1. Read ADR-0028, `crates/assets/src/error.rs` and
   `crates/cli/src/contract/codes.rs`.
2. Add the three constants to `shuyi_assets::error::codes`, with doc
   comments like the others.
3. Code each site in the table. Run `cargo test -p shuyi-assets` and
   the `no_std` build (`run-gates`).
4. Add the three rows and pages; run `cargo test -p shuyi-cli codes`
   (the gate).
5. Leave ADR-0028 as it is (an accepted ADR isn't edited; it already
   names this ticket's codes as a follow-up). `docs/contract.md` lists
   no codes, so it doesn't change either.

## Acceptance criteria

- [x] A keymap with an unknown action, read by `shuyi check` or
      `shuyi import`, gives a problem with `"code": "SH2008"` and its
      row's help.
- [x] A text key used twice gives `SH2009`; a crank notch out of range
      and a map too wide give `SH2010`.
- [x] `shuyi explain SH2009` prints its page.
- [x] The codes' gate test passes.
- [x] `shuyi-assets` still builds for `thumbv7em-none-eabihf`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-assets`): each coded site gives its code: a keymap's
  unknown action, unknown chord and unknown button; a binding listed
  twice and a layout listed twice; a text key used twice (`lang.rs` and
  `lang/lines.rs`); the repeat interval and the crank notch out of
  range; a map's unknown terrain, a tile not in the legend, a map too
  wide and one too tall. Extend the existing tests that check these
  messages with an assertion on `code`.
- The gate test (`contract/codes/tests.rs`), unchanged, passes with the
  new rows.

## Completion notes

- `UNKNOWN_NAME` (`SH2008`), `DUPLICATE` (`SH2009`) and `OUT_OF_RANGE`
  (`SH2010`) in `shuyi_assets::error::codes`, their rows in `TABLE` and
  their pages `crates/cli/errors/SH2008.md` … `SH2010.md`.
- Coded sites, as the table says: the keymap's unknown action and the
  chord and button parse errors, a binding listed twice or bound to two
  actions, a layout listed twice, the repeat interval and the crank
  notch; a text key used twice in English, in a pack's `ui.ron` and
  `data.ron`, and in a dialogue file; a map's unknown terrain, a tile not
  in the legend, a map too wide and too tall. No message changed.
- To code one message of several pushed by one closure, the language
  packs' `err` closures (`lang.rs`, `lang/lines.rs`) became `at`, which
  makes the error and leaves the push to the caller.
- Tests: each site's existing test asserts its code; a new test of a
  dialogue key used twice (there was none); a CLI test that a keymap's
  unknown action becomes a problem with `"code": "SH2008"` and its row's
  help, which is what `shuyi check` and `shuyi import` print.
- Not coded, as the ticket's *Out* says: the credits' and tips' "the id
  is used twice", and the keymap's other checks (a reserved key, too
  many bindings, a missing action, the stick thresholds). A later ticket
  can give them codes from the same table if an agent needs them.
- No API break: `code` was in the answer's shape already.
