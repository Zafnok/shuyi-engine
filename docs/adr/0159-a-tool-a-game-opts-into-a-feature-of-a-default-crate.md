# ADR-0159: A tool a game opts into: an off-by-default feature of the default crate it belongs beside, and of `shuyi-cli` for its command

- **Status:** Accepted
- **Date:** 2026-11-27
- **Related tickets:** 2259 (this: corner sheets made from one tile),
  2283 (the same sheet turned from 5 drawn pieces), 2260 (part sets,
  which weighs the same homes)
- **Extends:** ADR-0009 §4 (what a module is, and the two kinds of
  plugin that keep their own form)

## Context

Ticket 2259 makes a terrain's 15 corner pictures from its one tile at
tool time (and 2283 from 5 drawn pieces and their quarter turns). The result
is an ordinary PNG a game checks in, and a check keeps it equal to its
recipe. Hand-drawn corner layers stay the grid kit's way. A game opts
in, for one terrain or all of them, and may drop the recipe after one
run and keep the PNG.

The ticket asked for its home to be chosen against two goals that pull
against each other:

- **A clean default engine.** A game that doesn't opt in builds none of
  it, sees nothing change, and meets no new concept.
- **Modular.** The code is in one place that could be removed whole. It
  is opt-in, never the grid kit's main way, and has no more paperwork
  than the feature needs.

ADR-0009 §4 knows three homes for an optional part: a module crate, a
format (a feature of `shuyi-assets`, ADR-0007), and a platform backend.
`shuyi-assets` already has features that are not formats: `png` for
tools, and `golden`, `sample-actions` and `parse-count` for tests. No ADR
says when such a feature is the right home.

What the generator needs:

- pure, `no_std` with `alloc`, in integers, with its golden table on
  wasm32: `shuyi-assets`' tests already run in CI's `determinism` job;
- `shuyi_assets::image::Image`, `tileset::CornerLayer` and
  `ron::parse_ron`;
- a command and a check that read and write files: tool code, as
  `shuyi-cli`'s `frame-png --preview` and its `preview::check` are.

## Decision

1. **A tool a game opts into is an off-by-default cargo feature of the
   default crate its code belongs beside**, named for what it makes. It
   is not a module crate and not a format:
   - its code is one module behind the feature (`shuyi_assets::corners`
     behind `corners`);
   - it has no dependency of its own, and builds without `std` where
     its crate does;
   - its tests run with its crate's (the crate's dev-dependency on
     itself turns the feature on), so they run on wasm32 too.
2. **Its command is a feature of `shuyi-cli` of the same name**, off by
   default, which turns the first one on. The command (`corners`), the
   pure `make`, and the `check` a game's test calls are in one
   module behind it (`shuyi_cli::corners`). A game opts in by turning it
   on in its tools' dependency on `shuyi-cli`, and runs it as `cargo
   shuyi corners`.
3. **The command is always listed.** `shuyi-cli`'s usage and
   `COMMANDS` name it, so a skill or the docs may too. In a build
   without the feature, it runs in the game's tool program when the
   project has one, and otherwise says how to turn the feature on.
4. **No catalog row, no concern.** It answers no concern of
   `docs/modules.ron`: a game doesn't choose it in place of another way,
   it adds pictures to the default one. So `shuyi new` asks nothing new.
   Its docs are its rustdoc, the book's chapter on the frame and skins,
   and a line in the docs of what it serves (`CornerLayer`).
5. **When to use this rather than a module:** for code a game's tools
   run, not its shipped build, that is a small addition to a default
   crate's own types and has no dependency of its own. A part a shipped
   game runs, that answers a concern, or that pulls in a dependency or a
   heavy layer, is still a module (ADR-0009 §4).

## Consequences

- A game that doesn't opt in builds none of it, tools included: `cargo
  tree -p hello-grid --features tools -e features -i shuyi-assets` shows
  no `corners` (ticket 2259's notes).
- Removing it is one module and one feature in each of two crates, with
  their tests, a no-std build line and a book paragraph.
- A game that builds its layer in code with `corners::layer` as it runs
  turns `shuyi-assets`' feature on in its shipped build too. That is a
  few hundred lines of integer code, and no dependency. A game that
  names the sheet's 15 pictures in its `terrain-tiles` file needs
  nothing.
- The rustdoc of the default crates shows the module, marked as behind
  a feature. That is the one place the concept shows to a game that
  doesn't use it.
- `shuyi-cli`'s installed `shuyi` lacks the command unless built with the
  feature; a game runs it through its own tool program, as the template
  sets up.

## Alternatives considered

- **A module crate, `shuyi-corners`** (ADR-0009 §4). This is the
  cleanest separation. But it needs metadata, a README with its measured
  size, a skill, a catalog row and a `shuyi` feature. Its concern would
  add a question to `shuyi new`'s wizard about something no game must
  choose. It would also still need a `std` part for the command, or the
  command in `shuyi-cli` anyway. That is more paperwork than code.
- **Tool-only code in `shuyi-cli`.** A game's shipped build gains
  nothing. But the generator's tests and golden table don't run on
  wasm32 there: `shuyi-cli` isn't built for WASI. And the layout helpers
  a game's code may call would still need a home in a default crate.
- **Always on in `shuyi-cli`.** This is simpler: no feature on the
  command. But every game's tools would build the generator, which is
  what "a game that doesn't opt in builds none of it" rules out.
