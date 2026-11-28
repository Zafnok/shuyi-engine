---
id: "2285"
title: "JPEG XL on wasm32: a frame whose section sizes add past 32 bits panics jxl-frame; it becomes a decode error"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2285 — JPEG XL sizes past 32 bits panic on wasm32

## Context

- CI's `determinism` job (the tests on wasm32 under wasmtime) failed on
  PR #642's commit `7697f3ed` (run 46084868389, job 140797313473):
  `shuyi-assets`' property test `format::builtin_tests::no_bytes_panic_a_codec`
  fed random bytes after the JPEG XL signature (`0xff 0x0a`) to
  `JpegXl`, and `jxl-frame` 0.13.3 (under `jxl-oxide` 0.12, the
  `jpeg-xl` feature) panicked at `src/data/toc.rs:217`, "attempt to add
  with overflow". On wasm32 a panic aborts the whole test binary.
- The line is `acc += size as usize`, summing a frame's table of
  contents: each size is a `u32` read from the file, of up to about
  4.2 million (`4211712 + U(30)`), and `usize` is 32 bits on wasm32. A
  few large sizes overflow it; on 64-bit targets they never do. A test
  build checks overflow; a release build wraps, and what the decoder
  then does with the wrapped offsets is unknown.
- So a damaged or hostile `.jxl` can crash a game's web build (a debug
  one for certain), while ADR-0007 asks every format to refuse bad bytes
  with an error. The property test draws its bytes from a random seed,
  so it finds such a header only now and then: this was its first
  sighting (no entry in `tools/flaky.ron`).
- Rule 12 (`AGENTS.md`): the fix is ours, a workaround or a local
  patch, never a PR or an issue upstream.

## Nick input

None.

## Scope

**In:**
- `JpegXl` and `JpegXlInfo` (`crates/assets/src/image/codecs.rs`) refuse
  such a file with a decode error on every target, in debug and release
  builds. The way is the implementer's, written in the completion notes,
  in this order of preference:
  1. a check of the frame's table of contents before `jxl-oxide` reads
     it, if the header can be read that far without a JPEG XL decoder
     of our own;
  2. a local patch of `jxl-frame`'s `toc.rs` (`[patch.crates-io]` in the
     root `Cargo.toml`, as `alsa-sys` has) that adds with `checked_add`
     and returns its error, kept until a release of `jxl-frame` fixes
     it;
  3. no other: turning overflow checks off for the crate hides the
     panic and leaves the wrapped offsets.
- A regression test with a fixed input that reaches `toc.rs:217` on
  wasm32 (sizes that add past `u32::MAX`), in the formats' tests, run by
  the `determinism` job.

**Out (do not do):**
- Pinning the property test's seed: it is meant to keep finding new
  inputs.
- Other decoders: their own tickets if they panic.

**Estimated size:** about 250 lines (code 80, tests 120, docs 50).

## Implementation steps

1. Build a JPEG XL codestream whose frame's table of contents holds
   sizes adding past `u32::MAX`, and check it panics `JpegXl` on
   wasm32 (`cargo test --target wasm32-wasip1 -p shuyi-assets`, the
   `run-gates` skill's *Golden tables* section says how to install the
   target and wasmtime).
2. The fix, as the Scope ranks them.
3. The regression test; `docs/formats.md`'s JPEG XL row if it says what
   the format refuses.

## Acceptance criteria

- [ ] The regression input decodes to an error with `JpegXl` and
      `JpegXlInfo` on wasm32 and natively, and panics nothing.
- [ ] `no_bytes_panic_a_codec` still runs with a random seed.
- [ ] `cargo deny check` passes, and a local patch, if it is the way,
      says in `Cargo.toml` why and when it goes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the regression input, natively and on wasm32 (the
  `determinism` job runs `shuyi-assets`' tests there).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
