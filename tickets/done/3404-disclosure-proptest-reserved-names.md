---
id: "3404"
title: "The disclosure report's property test leaves out Windows' reserved device names"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-15
---

# 3404 — The disclosure report's property test leaves out Windows' reserved device names

## Context

`export::disclosure::tests::the_same_files_give_the_same_report`
(`crates/cli/src/export/disclosure/tests.rs`) generates file paths with
the regex `[a-z]{1,6}(/[a-z]{1,6})?` and writes each one to a temporary
folder. On Windows a path component named `con`, `prn`, `aux` or `nul`
is a reserved device name, whatever its case, so `fs::write` fails with
"The system cannot find the path specified" (os error 3) and the test
panics. CI's `test (windows-latest)` failed this way on PR #446 (ticket
2070, unrelated), run 43780343934, attempt 1, with the minimal input
`nul/a`. The test is flaky on Windows, not the code it tests: a game's
package never ships such a name, since it couldn't be made on Windows
either.

The other property tests that generate `[a-z]` names were checked:
`crates/assets/src/pack/tests.rs` (`pack_files`) builds a pack in memory,
`crates/platform/src/session/tests.rs`, `crates/ui/src/profiles/tests.rs`,
`crates/ui/src/storage.rs` and `crates/ui/src/storage/bundle/tests.rs`
use keys in memory (`MemoryStorage`), and the property tests in
`crates/cli/src/font_atlas.rs`, `crates/cli/src/template/tests.rs` and
`crates/platform/tests/it/session.rs` generate no file names. None
writes a generated name to disk, so this test is the only one to fix.

## Nick input

None.

## Scope

**In:**
- The property test leaves out any generated path with a component that
  is a Windows reserved device name, case-insensitively.
- A plain unit test showing `nul/a` is left out and an ordinary path is
  kept.

**Out (do not do):**
- No change to the disclosure code itself, or to what a report holds.
- No `cfg!(windows)`: the names are left out on every system, so the
  test generates the same cases everywhere.

## Implementation steps

1. In `crates/cli/src/export/disclosure/tests.rs`, add a helper
   `writable_everywhere(path: &str) -> bool` that is false when any
   `/`-separated component equals (ASCII case-insensitive) `con`, `prn`,
   `aux`, `nul`, `com1`–`com9` or `lpt1`–`lpt9`.
2. Filter the generated map's keys with it, beside the existing filter
   that drops a path which is also a folder's name.
3. Add the unit test of step 1's helper.

## Acceptance criteria

- [x] `writable_everywhere("nul/a")`, `("a/CON")` and `("aux")` are false;
      `("a/b")` and `("nully")` are true.
- [x] `the_same_files_give_the_same_report` never writes a path with a
      reserved component.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the helper's cases above.
- Property: the existing test, with the filter.

## Completion notes

- `writable_everywhere` in `crates/cli/src/export/disclosure/tests.rs`
  drops a generated path with a component Windows reserves (`con`,
  `prn`, `aux`, `nul`, `com1`-`com9`, `lpt1`-`lpt9`, any case), beside
  the existing folder-name filter; the unit test
  `a_device_name_on_windows_is_left_out_of_the_generated_paths` covers
  the acceptance cases and the near misses (`nully`, `com0`, `lpt10`).
- Deviation: the unit test checks a few more names than the acceptance
  list (`x/lpt1`, `Com9/b`, `com`, `com0`, `lpt10`).
- The other property tests with `[a-z]` names were checked (Context):
  none writes a generated name to disk.
- No change outside a test: no break, no behaviour change.
- Docs: none, a test's generator only.
- Book: none, no public API changed.
