---
id: "3258"
title: "Shared test doubles in ui, platform and app: one failing Storage, a row reader, Toy overrides, and the tests copied when code moved"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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
- **`GlyphBuffer::row_text(y) -> String`.** This is additive public API,
  listed by the `api` job. Delete the local row helpers, and keep
  `.trim()` at the call sites that trim.
- **`toy::Toy` overrides** (`with_palette`, `with_lang`,
  `with_key_caps`, `without_themes`), defaulting to today's behaviour.
  `Pale`, `Bare` and `InJapanese` become `Toy::new().with_…()`. `Capped`
  keeps a small wrapper, or a `key_text` hook.
- **The copied tests:**
  - drop cli's hand-checked frame (keep its PNG-atlas and bad-atlas
    tests);
  - put the template fixtures in one `include!` file the four tests
    share;
  - make the lint-copy test one xtask check over every crate whose
    `[lints]` is not `workspace = true`.

**Out (do not do):**
- Changing what a test asserts. The error texts tests assert ("down",
  "stopped") stay settable.
- The cli's scratch and argv helpers: 3546.

**Estimated size:** about 900 lines changed (about 650 removed, 250
added).

## Implementation steps

1. Write `FaultyStorage` with its unit tests, then move each double
   onto it, one file at a time.
2. Add `row_text` and replace the helpers.
3. Add the `Toy` builder methods, and replace the wrappers.
4. Remove the copied tests, and add the shared fixture file and the
   xtask lint-copy check.

## Acceptance criteria

- [ ] `grep -rnE "impl Storage for" crates` finds at most 3 test
      doubles outside `FaultyStorage`.
- [ ] `grep -rn "fn row(buf" crates/ui` finds none.
- [ ] Every ui, platform, app, cli and alloc-count test passes with the
      same names and assertions, apart from the deleted copies.
- [ ] A crate whose `[lints]` is not `workspace = true` and that lacks
      the lint copy fails the new xtask check.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `FaultyStorage`'s knobs, `row_text` (an in-bounds row, an
  out-of-bounds row), and the `Toy` defaults matching `Toy::new()`.
- The existing tests are the regression suite.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
