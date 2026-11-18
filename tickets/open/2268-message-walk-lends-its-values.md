---
id: "2268"
title: "A message's walk borrows its values: a select and a fill copy no value into a String"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2266"]
nick_input: none
completed:
---

# 2268 — A message's walk borrows its values

## Context

Split from 2266 at its **Split** point (2266 grew past 1,000 lines with
its stored parse alone). 2266 made a language learn each text's form
when it loads (`Lang::form_in`, `shuyi_assets::message::Form`), so a
`Ctx` fill parses nothing in a frame (ADR-0130). What is left is the
copies a fill still makes of its values, which 3219 found and left
(its scope's two items, handed to 2266 and now here):

- `Message::walk` (`crates/assets/src/message.rs`) takes
  `value: impl Fn(&str) -> Option<Arg>`: each lookup is an owned `Arg`,
  and `ToArg::to_arg` copies a text value into a `String`. `Ctx`'s
  `fill_styled` (`crates/ui/src/ctx.rs`) hands `device` to every select
  by device as `device.to_arg()`, a `String` each time.
- `Choice::pick` compares a select's value by `arg.to_string()`, a
  `String` per select walked.
- `text::fill_form` (`crates/ui/src/text.rs`) writes each argument with
  `push_plain(&plain.to_string())`, a number with `n.to_string()`, and
  a key's name through `joined`, a new `String` whose spaces are
  `NO_BREAK_SPACE`.

The job in one sentence: a select on `device` and a `{name}` filled
with text cost no allocation while a message is walked. A farming
game's shop hint, `{device, select, playdate {Crank} other {Turn}} to
browse · {coins} coins`, filled each frame, gains the same.

A design that compiled and passed the tests during 2266 (left out at
the split), for the session to take or improve:

- `ArgRef<'a>` (`#[non_exhaustive]`, `Int(i64)`, `Text(&'a str)`,
  `Copy`), `Arg::as_arg_ref`, and `ArgRef::with_str(self, f)`, which
  hands `f` an `Int`'s digits written into a stack buffer (40 bytes, an
  `i128`'s) or the text.
- `ToArg::with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>))`, a
  provided method calling `lend` once; its default lends `to_arg()`'s
  result, so a game's own `ToArg` keeps compiling. Overridden for `str`
  and `String` (borrowed), `char` (`encode_utf8` into a stack buffer),
  the integers (an `i64`, else digits on the stack), `Arg` and `&T`. A
  callback, not a return value, because a `char` and a big integer have
  no `str` to borrow from.
- `Message::walk_lent<'v>(&self, lang, value: impl Fn(&str) ->
  Option<&'v dyn ToArg>, each)`. Inside, one walk serves both `walk`
  and `walk_lent` by a lookup in continuation style, `&dyn Fn(&str,
  &mut dyn FnMut(Option<ArgRef<'_>>))`, each choice recursing into its
  branch inside the continuation (a flag calls it with `None` if a
  `with_arg_ref` never lends).

## Nick input

None.

## Scope

**In:**
- **`ArgRef` and `ToArg::with_arg_ref`** (`shuyi_assets::message`), as
  above: kind 6 of ADR-0039 for `ArgRef`; the trait method provided, so
  no break (`api: added`).
- **`Message::walk_lent`**, beside `walk`, which keeps its signature.
- **A select compares without a `String`:** `Choice::pick` compares an
  `ArgRef::Text` by `&str`, and an `ArgRef::Int` by its digits written
  into a stack buffer. An `Int` still matches a branch by its digits:
  `7` matches `7`, not `007`.
- **An argument is written once:** `text::fill_form` takes its values
  as `Option<&dyn ToArg>` and pushes each lent value with no
  `.to_string()`; a `#` writes its digits from the stack. A key's name
  is pushed with its spaces as `NO_BREAK_SPACE`, in pieces between the
  spaces, with no `joined` `String`. `Ctx::fill_styled` hands `device`
  and `target` as `&dyn ToArg`.
- An amendment to ADR-0130 (the `write-adr` skill): arguments are lent
  to the walk.
- Docs: rustdoc of `ArgRef`, `with_arg_ref` and `walk_lent`.

**Out (do not do):**
- The stored parse and `Lang::form_in`: 2266 (done).
- Any change in what a text fills to.
- The `StyledText`'s own `String`s (a fill's output allocates): 3221
  pins a frame's allocations.
- **Either order with 3221:** if 3221 is done, lower the counts pinned
  in `examples/hello-grid/tests/frame_alloc.rs` to what it measures (it
  fails until you do); if not, nothing to do (3221 pins the counts
  after this).

**Estimated size:** about 500 lines (code 250, tests 200, docs 50).

## Implementation steps

1. Read `message.rs` (`walk`, `Choice::pick`, `ToArg`), `text.rs`'s
   `fill_form` and `ctx.rs`'s `fill_styled`, and ADR-0130.
2. `ArgRef`, `Arg::as_arg_ref`, `ArgRef::with_str`, the stack digits.
3. `ToArg::with_arg_ref` and its overrides.
4. The walk on lent values; `walk_lent`; `pick` on `ArgRef`.
5. `fill_form` and `Ctx::fill_styled` on lent values; `joined` gone.
6. The tests, the ADR amendment, the docs, the gates.

## Acceptance criteria

- [ ] A select on `device` and a `{name}` filled with a text value
      allocate nothing while the message is walked with `walk_lent`
      (an allocation count with `shuyi-alloc-count`'s global counter,
      alone in its test binary).
- [ ] `walk_lent` gives what `walk` gives for every message test of
      `message/tests.rs`, with the same values.
- [ ] Every text of the toy's and Hello, Grid's languages fills the
      same as before, byte for byte (2266's equality test passes
      unchanged).
- [ ] A game's own `ToArg` that implements only `to_arg` compiles and
      fills as before (a test type in the tests).
- [ ] An `Int` matches a select's branch by its digits: `7` picks `7`,
      not `007`; `u128::MAX` picks its digits' branch.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `with_arg_ref` of each type lends what `to_arg` gives; a
  select by an `Int`, a `char` and a big integer.
- Property: for random messages and values, `walk_lent` and `walk`
  hand out the same pieces.
- Integration: the allocation count above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
