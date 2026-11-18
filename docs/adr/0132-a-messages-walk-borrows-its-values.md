# ADR-0132: A message's walk borrows its values, lent through a callback

- **Status:** Accepted
- **Date:** 2026-11-18
- **Related tickets:** 2268; 2266 (the stored parse), 3219 (the plain check), 3221 (a frame's allocations pinned)
- **Amends:** ADR-0131 (a fill copies none of its values; its last consequence is done)

## Context

ADR-0131 took the parse out of every frame, and left what a fill still
copied: `Message::walk` looks each value up as an owned `Arg`, so
`ToArg::to_arg` copies a text into a `String`; `Choice::pick` compared a
select's value by `arg.to_string()`; and `text::fill_form` wrote each
value with `.to_string()`, a `#` with `n.to_string()`, and a key's name
through a joined `String` whose spaces were no-break spaces. A select on
`device`, in every hint of a screen, made two `String`s a frame for a
value that is a `&'static str`.

A value can't always lend a `&str` it holds: a `char` and an integer too
big for an `i64` have no text until they are written.

## Decision

1. **`ArgRef<'a>`** (`shuyi_assets::message`, `#[non_exhaustive]`,
   ADR-0039 kind 6 as `Arg` is): `Int(i64)` or `Text(&'a str)`, `Copy`.
   `Arg::as_arg_ref` borrows an `Arg`; `ArgRef::with_str(f)` hands `f`
   an `Int`'s digits written into a stack buffer (40 bytes, an `i128`'s)
   or the text; `From<ArgRef> for Arg` copies it back.
2. **`ToArg::with_arg_ref(&self, lend: &mut dyn FnMut(ArgRef<'_>))`**, a
   provided method that calls `lend` once. Its default lends
   `to_arg()`'s result, so a game's `ToArg` that implements only
   `to_arg` compiles and fills as before (and allocates as before).
   Overridden, with no allocation, for `str`, `String`, `char` (encoded
   on the stack), every integer type (an `i64`, else its digits on the
   stack), `Arg` and `&T`. A callback, not a return value, because a
   `char` and a big integer have nothing to borrow from.
3. **`Message::walk_lent(lang, value: impl Fn(&str) -> Option<&'v dyn
   ToArg>, each)`**, beside `walk`, whose signature stays. One walk
   serves both: a lookup in continuation style (`&dyn Fn(&str, &mut dyn
   FnMut(Option<ArgRef<'_>>))`), each choice walking its branch inside
   the continuation, where the value is lent. A `with_arg_ref` that never
   calls `lend` is read as no value (`other`), and a second call is
   ignored, so a branch is walked once.
4. **A select compares a value as text without a `String`:** a
   `Text` by `&str`, an `Int` by its digits on the stack. An `Int`
   matches a branch by its digits: `7` picks `7`, not `007`.
5. **`text::fill_form` takes its values as `Option<&dyn ToArg>`** and
   walks with `walk_lent`: a value is pushed as it is lent, a `#` writes
   its digits from the stack, and a key's name is pushed in pieces
   between its spaces with a `NO_BREAK_SPACE` between them (the span
   joins them), so no copy of it is made. `Ctx::fill_styled` hands
   `device` and `target` as `&dyn ToArg`.

## Consequences

- A select on `device` and a `{name}` filled with text, a number, a
  character or an integer of any size allocate nothing while a message
  is walked: `crates/assets/tests/walk_alloc.rs` counts the heap over
  ten walks with `shuyi-alloc-count`'s global counter. What a fill still
  allocates is its `StyledText`, the output (3221 pins a frame's
  allocations).
- Every text of the toy's and Hello, Grid's languages fills as it did,
  checked against a copy of the fill before this change
  (`text/plain_tests.rs`), and `walk_lent` hands out what `walk` does
  for every message of the message tests and for random ones.
- A game's own `ToArg` keeps compiling (the method is provided: an
  addition, no break). It allocates in a walk until it overrides
  `with_arg_ref`.
- The walk recurses through a closure per choice: a stack frame more per
  nesting level, which `MAX_DEPTH` (8) bounds.

## Alternatives considered

- **`fn arg_ref(&self) -> ArgRef<'_>`, a return value:** a `char` and an
  integer too big for an `i64` have no `str` to borrow; they would need
  an `ArgRef` variant per kind, or an allocation.
- **Change `walk`'s lookup to borrowed values:** a break for every
  caller of `walk` and `format`, for nothing a new method can't give.
- **A `Cow<str>` in `Arg`:** the lookup still returns an owned `Arg` per
  choice per frame, and a `char` still allocates.
- **Two walks, one per lookup type, generic:** the same code twice in
  the binary (the Playdate counts its bytes), where one walk behind a
  `dyn` lookup costs an indirect call per choice.
