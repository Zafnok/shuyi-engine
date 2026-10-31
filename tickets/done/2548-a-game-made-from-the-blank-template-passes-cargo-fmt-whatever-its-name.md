---
id: "2548"
title: A game made from the blank template passes cargo fmt whatever its name
type: bug
milestone: MVP
model: implementation
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-23
---

# 2548 — A game made from the blank template passes cargo fmt whatever its name

## Context

`shuyi new blank <dir>` fills `{{name}}` and `{{crate}}` in
`templates/blank/crates/game/src/lib.rs` (`crates/cli/src/contract/new.rs`
says how). Its `game()` is written on the template's lines:

```rust
    BasicGame::new("{{name}}", "{{crate}}", Layer::dir("files", &FILES), |_| {
        Box::new(title::Title)
    })
```

That layout is rustfmt's only while the two names are short. CI's
`templates` job names the game `demo` (`$RUNNER_TEMP/demo`), so it
passes; a game called `demo-blank` (found by ticket 2025 running the
template gate locally) fails its own `cargo fmt --all --check` out of the
box, because rustfmt breaks the call one argument per line. A game an
agent makes with any longer name starts with a red gate.

The `grid` template and the other `{{name}}` places weren't seen to fail,
but weren't tried with long names either.

## Nick input

None.

## Scope

**In:**

- Lay out every line of `templates/` that holds `{{name}}` or `{{crate}}`
  so rustfmt leaves it as it is for any name: put the names in their own
  `const` items, or write the call one argument per line, whichever
  rustfmt keeps for both a short and a long name.
- A check that would have caught it: the `templates` job, or a test in
  `shuyi-cli`, also makes each template under a long name (say
  `a-much-longer-game-name`) and runs `cargo fmt --all --check` there.

**Out (do not do):**

- Changing how `new` fills the placeholders, or the names it allows.
- Formatting the rest of `templates/` differently.

## Implementation steps

1. Make `blank` and `grid` with a short and a long name, and run `cargo
   fmt --all --check` in each to see every line that moves.
2. Lay those lines out so neither name moves them.
3. Add the long-name check: a second `new` and `cargo fmt --all --check`
   in CI's `templates` job (`.github/workflows/ci.yml`), or a `shuyi-cli`
   test if one can run rustfmt on the made files.
4. Run the `run-gates` skill's template gate for both templates.

## Acceptance criteria

- [x] `shuyi new blank` and `shuyi new grid` with a long name make games
      that pass `cargo fmt --all --check`.
- [x] CI checks a long name, so this doesn't come back.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the template gate with `demo` and with a long name, for
  `grid` and `blank`.

## Completion notes

- The only Rust lines with a placeholder were `blank`'s `game()` call
  in `crates/game/src/lib.rs` and `grid`'s `WINDOW` in
  `crates/game/src/main.rs` (grid's `STORAGE_NAME` was already a `const`).
  Each name is now the whole of its own `const` (`NAME`,
  `STORAGE_NAME`), and the calls name the consts, so rustfmt lays them
  out the same for any name. The other `{{name}}` places are Markdown,
  TOML and RON, which rustfmt doesn't read.
- Reproduced: the old `blank` moves under `a-much-longer-game-name`; the
  old `grid` line fit with that name by one column and moves under a
  longer one.
- CI's `templates` job makes each template a second time, named
  `A Much Longer Game Name Than Demo, To Test`, and runs `cargo fmt --all
  --check` on it; that name moves both old layouts.
- Checked locally: `blank` and `grid` made as `demo`,
  `a-much-longer-game-name` and a 41-character name all pass `cargo fmt
  --all --check`, and the long-name games pass the whole template gate.
  Of the engine's gates, ran fmt, clippy, `shuyi-cli`'s tests and the
  repository's checks; no dependency, crate or example changed.
- The `new.rs` module doc now says the names sit in their own consts.
- No break. No follow-up tickets.
