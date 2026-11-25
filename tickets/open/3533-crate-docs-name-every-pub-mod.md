---
id: "3533"
title: "Each crate's doc names every pub mod, checked by cargo xtask check-docs"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3533 — Each crate's doc names every `pub mod`, checked by `cargo xtask check-docs`

## Context

Found by the docs audit of 2026-11-25. The docs map's first row
(`docs/README.md`) says what a crate, a module or a public item does
lives in rustdoc, and "the crate doc (`//!` in each crate's
`src/lib.rs`) names every `pub mod`". Nothing checks it, and it has
drifted:

- from the last 60 PRs: `shuyi-app`'s `log_sink` (3041, 3324) and
  `shuyi-cli`'s `rust_lines` (2512);
- older: `shuyi-assets`' `fnv`, `import_settings` and `message`;
  `shuyi-cli`'s `codec`, `toy` and `watch`; `shuyi-core`'s `path` and
  `rng` (spoken of only as "the seeded RNG", though 2114 grew it);
  `shuyi-platform`'s `dropped` and `text_input`; `shuyi-playdate`'s
  `heap` and `text`.

`shuyi-app`'s crate doc has two more slips: it names the keyboard
(`keys`) and controllers (`pads`) but not the mouse and touches it reads
since 2413, and it points at `run_game_checked` and `page::page_check`
(about lines 34–36), which 2098 made `#[doc(hidden)]`.

## Nick input

None.

## Scope

**In:**

- **The check**, in `cargo xtask check-docs` (`crates/xtask/src/docs.rs`):
  for each workspace crate's `src/lib.rs`, every `pub mod <name>` not
  marked `#[doc(hidden)]` is named in the crate's `//!` doc, as an
  intra-doc link (`[`name`]`) or in backticks. A miss names the crate,
  the module and the file. It reads the files as text, as the other docs
  checks do; no `syn` dependency unless the xtask already has it.
- **The crate docs**: each missing module named in a line of its crate
  doc, in the doc's own order and voice; `shuyi-app`'s pointer input and
  its two hidden items fixed.
- The map's first row names the check.

**Out (do not do):**

- naming every public item, or `pub use` re-exports;
- a module's own `//!` doc (a module without one is clippy's
  `missing_docs`, already on).

## Implementation steps

1. Read `crates/xtask/src/docs.rs` and how `check-docs` reports.
2. Write the check and its tests; run it and list what it finds (the
   list above, or more).
3. Fix each crate doc.
4. `cargo doc` with the gates' flags, so each new link resolves.
5. Run the gates.

## Acceptance criteria

- [ ] `cargo xtask check-docs` fails a crate whose doc misses a
      `pub mod`, and passes a `#[doc(hidden)]` one.
- [ ] Every crate passes it.
- [ ] `shuyi-app`'s doc names its pointer input and no hidden item.
- [ ] `docs/README.md`'s first row names the check.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the check on a crate doc that names a module, one that misses
  it, one with `#[doc(hidden)]`, and a `pub mod` inside a `cfg`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
