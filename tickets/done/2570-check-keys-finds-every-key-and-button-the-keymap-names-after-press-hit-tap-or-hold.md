---
id: "2570"
title: "check-keys finds every key and button the keymap names after press, hit, tap or hold"
type: bug
milestone: After MVP
model: implementation
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-28
---

# 2570 — check-keys finds every key and button the keymap names

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). `check-keys` (`crates/cli/src/check_keys.rs`) fails when a
game's text tells the player to press a key instead of naming it through
the keymap (`AGENTS.md` rule 8). It knows the keys by a list of its own,
and the keymap format knows them by another:

- **`KEY_WORDS`** (`check_keys.rs:42-44`): eight words kept by hand,
  `Space`, `Esc`, `Escape`, `Enter`, `Shift`, `arrows`, `WASD`, `wasd`.
  Besides them, a single letter and `F1`–`F12` (`is_function_key`) count
  as keys.
- **`Key::ALL` and `Button::ALL`** (`crates/assets/src/keymap.rs`): every
  key a keymap file can name (`Key`'s `keys!` table, lines 64-154: `Tab`,
  `Backspace`, `Insert`, `Delete`, `Home`, `End`, `PageUp`, `Up`, `Kp0`,
  …) and every controller button (`buttons!`, lines 291-326: `South`,
  `Start`, `Select`, `LeftShoulder`, `DpadUp`, …). `Key::from_name` and
  `Button::from_name` look a name up exactly.

So the two places that read keys in text miss most of them:

- `prose_names_key` (`check_keys.rs:226-257`, for `.ron` strings and
  `.dlg` lines) flags a press verb (`press`, `hit`, `tap`, `hold`)
  followed by a letter, a `KEY_WORDS` word, a `Shift+` chord or a
  function key. "Press Tab to switch", "press Start" and "hold
  Backspace" pass.
- `names_key` (`check_keys.rs:275-298`, for string literals in Rust)
  flags a `KEY_WORDS` word anywhere, `press` followed by a letter, a
  help-bar letter (`f select`) and `[F]`. "Press Tab to switch" passes
  there too.

`KEY_WORDS` is right as a list of *bare* words for `names_key`: those
are the names a help bar shows alone ("Space end turn"). Most keymap
names are ordinary words in a label ("Start", "Select", "Up", "End",
"Home", "Delete"), so they can't be flagged alone. After a press verb
they can: that is an instruction to press a key.

`shuyi check-keys` on this repository finds no such text today
(searched 2026-10-28: no press, hit, tap or hold followed by a keymap
name outside `tickets/`), so the root's gates stay green.

## Nick input

None.

## Scope

**In:**
- **A keymap name after a press verb is a key** in both scans. A word
  after `press`, `hit`, `tap` or `hold` (any case) names a key when,
  after the trailing punctuation is trimmed as today, it is:
  - what it is today: a single letter, a `KEY_WORDS` word (any case),
    a `Shift+` chord or a function key; or
  - a keymap name of two or more ASCII letters and digits that starts
    with a letter: `Key::from_name(word)` or `Button::from_name(word)`
    finds it. (The numpad's `Kp+`, `Kp-`, `Kp*`, `Kp/` and `Kp.` are
    left out: they aren't letters and digits, and the punctuation trim
    cuts the last character of all but `Kp+`.)
    These match exactly as the keymap writes them (`Tab`, `Start`,
    `PageUp`), so "Hold up!", "hold down the fort" and "press on" stay
    prose.
- **One verb rule for both scans.** Move the press-verb loop out of
  `prose_names_key` into a function both use. `names_key` runs it after
  its own checks, so every message either scan gives today stays the
  same, and a new hit reads `tells the player to press "Tab"` as
  `prose_names_key`'s do.
- The module's doc comment and `KEY_WORDS`' doc say which words are
  bare words and which come from the keymap.
- **Either order with 2543:** if 2543 is done, the new hits come out of
  the scans as `Hit`s like the rest: add a `lint` test that a `.ron`
  string "Press Tab to switch" is a finding with its file and line; if
  not, nothing more to do (2543 carries every `check-keys` finding, these
  too).
- **For the game:** no API change. `check_keys`' public functions keep
  their signatures and the words they flagged; they flag more. The
  game's next pin bump runs `check-keys` on its text, and a hit there is
  fixed in the game's text (a game ticket), not here.

**Out (do not do):**
- Flagging a keymap name as a bare word (without a press verb) in
  either scan: "Start", "Select" and "End" are labels.
- Digits and punctuation keys (`press 1`, `press ;`): "hold 2 cards"
  is prose.
- Matching keymap names in any case, or by a list copied out of
  `keymap.rs`.
- Changing what `check_keys::run` returns, or `lint`'s answer: 2543's
  job.
- The brace scanning in `check-keys` and `check-text`, which 2390 leaves
  alone too.

## Implementation steps

1. Read `crates/cli/src/check_keys.rs` (`KEY_WORDS`, `prose_names_key`,
   `names_key`, `is_function_key`, the tests) and `Key::from_name` and
   `Button::from_name` in `crates/assets/src/keymap.rs`.
   `crates/cli/Cargo.toml` already depends on `shuyi-assets`, and the
   root `shuyi.ron` lists `crates/cli/src/check_keys.rs` among its
   `check_keys.key_files`, so the file may name `Key` and `Button`.
2. Write the tests below first and see the new ones fail.
3. Add `fn is_keymap_name(word: &str) -> bool` (two or more chars, all
   ASCII letters and digits, a letter first, `Key::from_name` or
   `Button::from_name` is `Some`) with
   `use shuyi_assets::keymap::{Button, Key};`.
4. Move the verb loop of `prose_names_key` into
   `fn press_instruction(words: &[&str]) -> Option<String>`, with
   `is_keymap_name` added to its `is_key`. `prose_names_key` calls it
   where the loop was; `names_key` calls it on its whitespace-split
   words after the help-bar loop, before the bracket check.
5. Update the doc comments. Settle the 2543 clause above.
6. Run `cargo shuyi check-keys` at the root (no hits expected), then
   the gates.

## Acceptance criteria

- [x] `prose_names_key` flags "lord: Press Tab to switch.", "lord: press
      Start!", "lord: Hold Backspace to clear." and "lord: Tap DpadUp.";
      `names_key` flags "Press Tab to switch".
- [x] "lord: Hold up!", "lord: Hold down the fort.", "lord: Press on,
      press forward." and "lord: We hold the North." still pass, and
      `names_key` passes "Start game", "Select a file" and "End turn".
- [x] Every existing test in `check_keys.rs` passes unchanged.
- [x] `grep -n '"Tab"\|"Start"\|"Backspace"' crates/cli/src/check_keys.rs`
      finds them only in tests: the names come from `keymap.rs`.
- [x] `cargo shuyi check-keys` at the root passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`check_keys.rs`): the flagged and passing lines above, for
  `prose_names_key` and `names_key`; a test that every `Key::ALL` name
  of two or more ASCII letters and digits starting with a letter, and
  every `Button::ALL` name, is flagged after "press " in
  `prose_names_key`, so a key added to `keymap.rs` is found with no
  change here.
- Unit: `scan_dlg` on "lord: Press Start to begin." gives one hit on
  its line, reading `tells the player to press "Start"`.
- The existing tests, `run_reports_hits_in_a_repo_tree` among them,
  pass unchanged.

## Completion notes

Both scans now flag a keymap name (`Key`/`Button`, two or more ASCII
letters and digits, exact case) after `press`, `hit`, `tap` or `hold`,
through one `press_instruction` used by `prose_names_key` and `names_key`.
The module and `KEY_WORDS` docs say which words are bare and which come
from the keymap. Tests added for each acceptance line, the all-keys sweep
and `scan_dlg`.

Deviation: the ticket said the root has no such text, but the shared loop
also flagged "hold a brace" in two Rust strings (single letter after
`hold`). `names_key` therefore calls the loop without single letters
(it already flags "press f"); `.ron` and `.dlg` keep them as before.

2543 was not done: its ticket now says it adds the `lint` test for a
`.ron` "Press Tab to switch" finding.

No break. `check_keys`' public functions keep their signatures.
The game's next pin bump may see new `check-keys` hits in its text.
