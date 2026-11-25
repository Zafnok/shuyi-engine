---
id: "3353"
title: "A game on the short path runs on the Playdate: BasicGame's one-bit look and its pack from the .pdx, and Snake's Playdate build"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2739"]
nick_input: none
completed:
---

# 3353 — A short-path game on the Playdate, and Snake's build

## Context

The end goal (Nick, 2026-10-19, `docs/decisions.md`): an agent makes a
snake game "with a web build, a Playdate build and the other targets".
2739 wrote `examples/snake` on the short path (`BasicGame`, ADR-0036);
its 50 × 15 console fits the Playdate's screen, its keymap has the
Playdate's buttons, its `palette@onebit.ron` passes `check --target
playdate`, and `shuyi.ron` lists `playdate`. It doesn't build for the
Playdate: the short path can't run there.

What a game adds for the Playdate (`docs/targets/playdate.md`, *What a
game adds*, 3010): a library without `std` that reads a baked pack from
`shuyi_playdate::pack()`, `shuyi_playdate::main!(start)`, and a look with
`Look::with_one_bit`. Hello, Grid does it all by hand (`src/playdate.rs`,
`Hello::one_bit_ctx`, about 300 lines of its `src/lib.rs`). The short
path has none of it:

- `BasicGame::start`'s look is `Look::new(&data.font, &data.images)`
  (`crates/platform/src/basic.rs`), with no one-bit part, so the
  Playdate's backend shows "This game has no one-bit look"
  (`crates/playdate/src/display.rs`);
- `BasicData` (`crates/ui/src/basic.rs`) holds no pattern map and no
  font's one-bit rows (`font/atlas_1bit.bin`);
- a short-path game embeds its files with `shuyi::files!`, which needs
  `std`.

## Nick input

**Sign-off:** run `cargo shuyi export playdate --run` in `examples/snake`
with the Playdate SDK installed, and say whether the snake plays in the
Simulator.

## Scope

**In:**
- **`BasicData`'s one-bit look**: when its bundle has
  `palette@onebit.ron` and `font/atlas_1bit.bin`, it reads the pattern
  map and the font's one-bit rows, and `BasicGame`'s look adds
  `Look::with_one_bit` from them. A game without them is as today.
- **`BasicGame` from a pack, without `std`**: a way to build a
  `BasicGame` over the layer of a baked pack (`shuyi_playdate::pack()`'s)
  that builds without `std`, and `BasicGame::start` loading in steps
  (`Start::in_steps`, ADR-0097) a file a step.
- **What a short-path game adds** in `docs/targets/playdate.md`: a
  `playdate` feature, its `src/playdate.rs` of a few lines, and its
  one-bit files.
- **Snake's Playdate build**: its `playdate` feature and `src/playdate.rs`,
  `font/atlas_1bit.bin` (made by `cargo shuyi font-atlas`), `shuyi export
  playdate` building it; CI's `no-std` gate builds its library for
  `thumbv7em-none-eabihf` as Hello, Grid's is
  (`crates/xtask/src/gates.rs`, the `no-std` gate's steps, and ci.yml's
  job); its README's *Targets* says it runs there. The eval's
  `unmeasured` Playdate entry for snake (`evals/tasks/snake.ron`) points
  here: drop it if an export of it can be checked, else keep it.
- A `frame-png --target playdate` snapshot of snake's title.

**Out (do not do):**
- The template's own Playdate part (2504).
- Sound on the Playdate for a short-path game, beyond what
  `GameTools::audio` already gives: a ticket if the export warns.

**Estimated size:** about 1,300 lines (code 700, tests 350, docs 250).

**Split:** past 1,000 lines, split off snake's build (its feature, its
`src/playdate.rs`, the gate and the README) into a new ticket.

## Implementation steps

1. Read `docs/targets/playdate.md`, ADR-0062, ADR-0097, Hello, Grid's
   `src/playdate.rs` and `Hello::one_bit_ctx`, and `BasicGame` and
   `BasicData`.
2. `BasicData`'s one-bit part and `BasicGame`'s look; tests with a
   bundle that has the files and one that doesn't.
3. `BasicGame` over a pack without `std`, loaded in steps; a test that
   the steps' data equals a load at once.
4. Snake's build, the gate, the docs.
5. Run `shuyi export playdate` (and `--run` where the SDK is), and the
   gates.

## Acceptance criteria

- [ ] A game on the short path with the one-bit files paints its frames
      on a one-bit display (`frame-png --target playdate` and the
      Playdate backend's display test).
- [ ] `cargo build -p snake --lib --no-default-features --features
      playdate --target thumbv7em-none-eabihf` passes in the `no-std`
      gate.
- [ ] `shuyi export playdate` builds snake's `.pdx`; the PR says whether
      it plays in the Simulator.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `BasicData`'s one-bit part read or absent; the look with it.
- Integration: snake's one-bit title as a snapshot; its data from a
  baked pack equals its data from `files/`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
