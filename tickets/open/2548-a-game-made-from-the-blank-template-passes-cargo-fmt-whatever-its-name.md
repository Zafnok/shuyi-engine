---
id: "2548"
title: A game made from the blank template passes cargo fmt whatever its name
type: bug
milestone: MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `shuyi new blank` and `shuyi new grid` with a long name make games
      that pass `cargo fmt --all --check`.
- [ ] CI checks a long name, so this doesn't come back.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the template gate with `demo` and with a long name, for
  `grid` and `blank`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
