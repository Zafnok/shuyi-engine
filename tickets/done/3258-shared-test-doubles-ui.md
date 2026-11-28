---
id: "3258"
title: "Shared test doubles in ui, platform and app: one failing Storage, the present hook, and Toy overrides"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-28
---

# 3258 — Shared test doubles in ui, platform and app

## Context

The redundancy sweep of 2026-11-28 found test scaffolding written again
in each module. The verifier's counts:

- **Storage doubles.** 11 hand-written `impl Storage` doubles of 14–21
  lines each (`grep -rnE "impl Storage for" crates`):
  - ui: `ctx/tests.rs:27` Failing and `:924` NoWrites,
    `profiles/tests.rs:210` Stopping, `screens/options/tests.rs:763`
    ReadOnly, `storage/bundle/tests.rs:443` Unlistable and `:543`
    FailingWrite, and `storage.rs:199` Broken;
  - platform: `session/tests.rs:305` Broken and `app/tests.rs:510`
    Shared;
  - app: `tests/crash_hook.rs:37` Shared.

  The platform also has `RecordLook` (`app/tests.rs` about 365–410),
  which forwards eight methods only to watch `present()`.
  `HeadlessPlatform::set_present_hook` (`headless.rs` about 824) already
  does that. About 120 lines go.
- **Row readers.** About 11 local `fn row(buf, y) -> String` helpers
  over a `GlyphBuffer` (`debug/tests.rs:20`, `frame/draw/tests.rs:14`,
  `dialogue/text_box/glyph/tests.rs`, `widgets/ask/tests.rs`,
  `widgets/menu/glyph.rs`, `screens/{key_bindings,layout_picker,profiles}/glyph*`).
  Each is four or five lines.
- **Pass-through GameData wrappers.** `Pale` (`ctx/tests.rs` about
  78–104), `Capped` (`game/key_caps_tests.rs` about 20–64), `Bare` and
  `InJapanese` each implement the whole of `GameData` to override one or
  two methods. About 90 lines go with the row readers.
- **Tests copied when code moved.** About 175 lines:
  - cli's `frame_png/tests.rs:66-149` (`tiny_atlas`, `painter`,
    `hand_checked_frame`) repeats ui's `frame/raster/tests.rs:119-203`;
  - the blank template's five `include_bytes` constants and its `GOOD`
    table are in four test files (`crates/cli/src/basic/tests.rs`,
    `crates/platform/src/basic/tests.rs`, `crates/ui/src/basic/tests.rs`,
    `crates/platform/tests/it/basic_reload.rs`);
  - the lint-copy test is nearly the same in `crates/app/src/lib.rs`
    (about 1495–1535) and `crates/alloc-count/src/lib.rs` (about 189–220).

That is about 400 lines saved, all tests apart from one small public
method.

## Nick input

None.

## Scope

**In:**
- **A `FaultyStorage` in `shuyi_ui`** behind its `toy` feature (or a new
  `test-support` feature). It is configurable: which operations fail,
  the error text, a write budget, and a shared `Rc` handle. Replace the
  simple doubles with it. Keep `storage/power_cut.rs`'s `Failing`, which
  models a power cut. Keep `crash_hook.rs`'s `Shared` if it does not fit.
- **`RecordLook` rewritten on `set_present_hook`**, and `Wanting` too
  where it fits.
- **`toy::Toy` overrides** (`with_palette`, `with_lang`,
  `with_key_caps`, `without_themes`), defaulting to today's behaviour.
  `Pale`, `Bare` and `InJapanese` become `Toy::new().with_…()`. `Capped`
  keeps a small wrapper, or a `key_text` hook.

**Out (do not do):**
- `GlyphBuffer::row_text` and the local row helpers: 3204, which plans
  the same method (its scope lists the twelve copies this sweep found).
- The tests copied when code moved (cli's hand-checked frame, the
  template's fixtures, the lint-copy test): 3261.
- Changing what a test asserts. The error texts tests assert ("down",
  "stopped") stay settable.
- The cli's scratch and argv helpers: 3546.

**Estimated size:** about 900 lines changed (about 650 removed, 250
added).

## Implementation steps

1. Write `FaultyStorage` with its unit tests, then move each double
   onto it, one file at a time.
2. Rewrite `RecordLook` and `Wanting` on `set_present_hook`.
3. Add the `Toy` builder methods, and replace the wrappers.

## Acceptance criteria

- [x] `grep -rnE "impl Storage for" crates` finds at most 3 test
      doubles outside `FaultyStorage`.
- [x] Every ui, platform and app test passes with the same names and
      assertions.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `FaultyStorage`'s knobs, and the `Toy` defaults matching
  `Toy::new()`.
- The existing tests are the regression suite.

## Completion notes

What was done:

- `shuyi_ui::storage::FaultyStorage` (`crates/ui/src/storage/faulty.rs`,
  behind `toy` and `cfg(test)`): a `MemoryStorage` behind an `Rc` that
  a clone shares, failing reads, writes, deletes or lists, every
  operation on one key, the writes past a budget, or the nth write once,
  or dropping writes, with one settable error text (`"down"` at first).
  Nine unit tests, the conformance suite with no faults among them.
- It replaces nine doubles: ui's `Failing`, `NoWrites`, `Stopping`,
  `ReadOnly`, `Unlistable`, `FailingWrite` and `Broken`, and platform's
  session `Broken` and app `Shared`. `grep -rnE "impl Storage for"
  crates` now finds three test doubles beside it: power_cut's `Failing`
  and `MemoryRig` (kept, as the scope says), and `crash_hook.rs`'s
  `Shared`, kept because a panic hook needs a `Send` storage
  (`Arc<Mutex<…>>`), which an `Rc` isn't.
- Platform's `RecordLook` and app's `Wanting` are a `HeadlessPlatform`
  with a present hook.
- `Toy` gains `with_palette`, `without_themes`, `with_key_caps` and
  `with_key_text` (crate-private, `cfg(test)`, like its `with_lang`),
  starting as the toy always was (a unit test checks it). `Pale`,
  `InJapanese` and `Capped` became `Toy::new().with_…()`.

Deviations:

- Split: this PR stopped at 1,032 counted lines before step 2 of the
  ticket as written, and steps 2 and 4 could land apart, so they left.
  `GlyphBuffer::row_text` and the twelve row helpers went to 3204, which
  already planned the same method (the sweep didn't name it); its scope
  now lists the twelve copies. The copied tests (cli's hand-checked
  frame, the template's fixtures, the lint-copy test) went to 3261,
  written here, with a clause in both it and 2335, whose step 8 named
  the cli's copy of the test.
- `Bare` stays: it tests `GameData`'s own default methods
  (`debug_lang`, `voices`, `keys_before_profiles`, `old_layout_key`),
  and a `Toy` that overrides them can't reach those defaults.
- `with_lang` was there already (its closure gets the English map), so
  `InJapanese` uses it; no new one.
- The ticket's scope and acceptance criteria were rewritten for the
  split, and its title shortened to what this PR does.
- storage.rs's `Broken` also accepted any key; `FaultyStorage` with
  `dropping_writes` checks keys as `MemoryStorage` does. The suite
  still fails at its first `assert_eq` (a read after a write), as the
  test asserts.

No break. `FaultyStorage` is new public API behind the `toy` feature;
nothing else public changed. A search of the game for `FaultyStorage`
and for the `toy` feature (`gh search code --repo Zafnok/visions-of-shuyi
'FaultyStorage'`, `'features = ["toy"]'`) found nothing.

Book: none, `FaultyStorage` is a test double behind the engine's own `toy` feature, which games don't turn on.

Skills: none, no schema, template, format or problem code changed.
