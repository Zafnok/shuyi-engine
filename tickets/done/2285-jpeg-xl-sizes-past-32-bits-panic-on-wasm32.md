---
id: "2285"
title: "JPEG XL on wasm32: a frame whose section sizes add past 32 bits panics jxl-frame; it becomes a decode error"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
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

- [x] The regression input decodes to an error with `JpegXl` and
      `JpegXlInfo` on wasm32 and natively, and panics nothing.
- [x] `no_bytes_panic_a_codec` still runs with a random seed.
- [x] `cargo deny check` passes, and a local patch, if it is the way,
      says in `Cargo.toml` why and when it goes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] The `run-gates` skill's fast gates pass, and the touched crate's clippy and tests natively and on wasm32; CI runs the rest.

## Tests required

- Unit: the regression input, natively and on wasm32 (the
  `determinism` job runs `shuyi-assets`' tests there).

## Completion notes

- **The way: the Scope's first, a check before `jxl-oxide` reads the
  file.** `JpegXl` and `JpegXlInfo` walk the codestream's frames first
  (`sections_fit` and its `sections` module, in
  `crates/assets/src/image/codecs.rs`): the container's boxes, the image
  header, its ICC profile if any, then each frame's header and table of
  contents, all with `jxl-oxide`'s own parsers (`jxl-bitstream`,
  `jxl-coding`, `jxl-color`, `jxl-oxide-common`, already in the build at
  the same versions). It adds the sizes in a `u64` and refuses a frame
  whose sections end past `u32::MAX` bytes from the codestream's start,
  with "not a valid JPEG XL file: its frame's sections end past 4 GiB
  from its start", on every target. It steps over each frame's sections
  to the next, and stops at the last frame, at one cut short and at bytes
  it can't read, which `jxl-oxide` then refuses itself, and at a frame
  `Frame::parse` refuses for its size before its table of contents (over
  2³⁰ px a side or 2⁴⁰ px), whose groups `FrameHeader::num_groups`
  can't count without overflowing: the first walk called it unchecked,
  and the property test found that on wasm32 within three runs. No
  decoder of our own: nothing is decoded, only read as `Toc::parse`
  reads it.
- **Why not the patch (the second way):** a `[patch.crates-io]` applies
  only to the workspace that declares it, so a game, which depends on
  the engine by git revision and builds its own workspace, would have
  kept the panic; `shuyi-app` turns `jpeg-xl` on for every game. The
  check needs no patch, so nothing in `Cargo.toml` has to go; the
  dependencies' comment says they go once a release of `jxl-frame`
  refuses such a file itself, and Dependabot moves the `jxl-*` crates in
  one group now (`.github/dependabot.yml`).
- **The bound counts from the codestream's start**, where `jxl-frame`
  counts from the frame's (or the codestream's, for a preview frame): it
  refuses a little more than what overflows, only files whose sections
  claim more than 4 GiB, which no 32-bit target can hold.
- **Step 1:** `jpeg_xl_past_4_gib` (`format/codec_tests.rs`), 25 bytes:
  a 512 × 256 px codestream, every header default, five sections at
  their most. Before the fix it trapped the wasm32 test run at
  `panic_const_add_overflow` (`cargo test --target wasm32-wasip1 -p
  shuyi-assets --lib`); natively `jxl-oxide` refused it with "failed to
  allocate 1077953535 byte(s)".
- **Tests:** the regression input through `JpegXlInfo`, `JpegXl` and the
  standard registry, natively and on wasm32; the walk on the two
  fixtures (their one frame ends at their last byte), in a container, at
  its bound (an end at `u32::MAX` passes, a byte more doesn't) and over
  a table of contents with an order (a hand-made one-symbol prefix code,
  an empty order); and two frames too big to count (2³⁰ × 2³⁰ px, and
  2³¹ × 2³⁰ px), each of which overflowed `num_groups` without the size
  check, natively too. `no_bytes_panic_a_codec` is unchanged and keeps its
  random seed; after the size check it passed 40 runs of 256 cases each
  on wasm32 under wasmtime.
- **Docs:** `docs/formats.md`'s JPEG XL bullet says what it refuses and
  why, and its decoders' row names the four crates; the formats' rustdoc
  says so too.
- No upstream PR or issue (`AGENTS.md` rule 12).
- No break: no public item changed (`cargo xtask api`).
- Behaviour: shuyi-assets: `JpegXl` and `JpegXlInfo` refuse a JPEG XL whose frame says its sections end past 4 GiB from the codestream's start, with "not a valid JPEG XL file: its frame's sections end past 4 GiB from its start", on every target; on wasm32 it panicked, elsewhere it was refused with another message or decoded as a file cut short.
- Game: nothing to adopt, the game names no JPEG XL file or format (`gh search code --repo Zafnok/visions-of-shuyi 'jxl'` and `'JpegXl'`: no hits); its next pin bump brings the check.
- Book: none, no public item changed; the book's formats chapter names JPEG XL only as a format a game may use.
- Skills: none, no schema, template, format or problem code a game writes changed; only what one decoder refuses.
