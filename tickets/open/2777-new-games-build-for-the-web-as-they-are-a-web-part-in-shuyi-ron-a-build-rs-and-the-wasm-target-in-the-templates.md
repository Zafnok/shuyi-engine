---
id: "2777"
title: "New games build for the web as they are: a web part in shuyi.ron, a build.rs and the wasm target in the templates"
type: feature
milestone: "Engine: public"
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2777 — New games build for the web as they are

## Context

A game made with `shuyi new` lists the `web` target in its `shuyi.ron`
but can't build for it: `cargo shuyi web` answers `SH1002` ("shuyi.ron
has no `web`"). Found while writing the book (2802): its quick start
couldn't end on `cargo shuyi web`, so it points at chapter 9, which
teaches the two files a game adds by hand. A session on 2802 added them
to a new `blank` game and `cargo shuyi web` built it (Windows, GNU
toolchain), so nothing in the engine is missing, only the template's
files:

- a `web` part in `shuyi.ron` (`package`, `bin`, `title`, `dist`), as
  `examples/hello-grid/shuyi.ron` has;
- `crates/game/build.rs`, which links the binary with
  `--allow-undefined` on `wasm32` (`crates/cli/web/README.md`, *The link
  argument a game's binary needs*; Hello, Grid's `build.rs`);
- `wasm32-unknown-unknown` among `rust-toolchain.toml`'s `targets`, so
  rustup fetches it with the pinned Rust.

The end goal (`docs/decisions.md`, 2026-10-19) is a game an agent makes
"with a web ver". 2524's eval measures what that costs an agent today
and left this change to "a ticket of its own, if the numbers say so";
the book's quick start is reason enough to make it now.

## Nick input

None.

## Scope

**In:**
- `templates/shuyi-default/base/shuyi.ron`: a `web` part,
  `(package: "{{crate_name}}", bin: "{{crate_name}}", title: "{{name}}",
  dist: "dist/web")`, with a comment in the file's style. `dist/` is
  already in the template's `.gitignore`.
- `templates/shuyi-default/base/crates/game/build.rs`: Hello, Grid's,
  with its doc comment. The `grid` stage's game crate is the same crate
  (`stages/top-down-grid` adds files to it), so both templates get it.
- `templates/shuyi-default/base/rust-toolchain.toml`: `targets =
  ["wasm32-unknown-unknown"]`. `cargo xtask check-toolchain` and the
  `bump-rust` skill keep the channel in step; check they still pass
  with a `targets` line.
- CI's `templates` job (`.github/workflows/ci.yml`): after the new
  game's tests, `cargo shuyi web` in it, and a check that its `dist/web/`
  holds `index.html`, one `.wasm` and `NOTICES.txt`, for both
  templates.
- The new game's README (`templates/shuyi-default/base/README.md`): a
  line for `cargo shuyi web` in its command block, and `crates/game/build.rs`
  in its layout table.
- `docs/templates.md` if it lists the base part's files.
- **The book** (`site/content/learn/book/`, 2802 is done):
  - the quick start's *In the browser* section builds the new game's
    page with `cargo shuyi web` (serve `dist/web/` over HTTP), and keeps
    Hello, Grid's link;
  - chapter 9's *The web build* says a new game has its `web` part and
    `build.rs` already, and shows them as what they are and why
    (its `snippet`s of `site/snippets/project/shuyi.ron` and
    `site/snippets/build.rs` stay).
- **Either order with 2524:** if 2524 is done, its tic-tac-toe
  reference (`evals/references/tic-tac-toe/`) drops the `web` entry,
  `build.rs` and the toolchain's target it carries over the template,
  and its PR's number of lines over the template is written again in
  2524's place for it (its README, or wherever 2524 keeps it); if not,
  nothing to do (2524 writes its reference from the template as it is
  then).
- **Either order with 2528:** if 2528 is done, the quick start's line
  is `cargo shuyi web --serve --open`; if not, nothing to do (2528
  changes the book's web steps, as its clause says).

**Out (do not do):**
- a `web` part in Hello, Grid (it has one) or in the engine's root
  `shuyi.ron` (the engine isn't a game);
- `shuyi export web` (3004) and serving the page (2528);
- the link over the web build (2522).

## Implementation steps

1. Read `crates/cli/web/README.md`, `crates/cli/src/web.rs` (`Web`,
   `usage`) and `examples/hello-grid/{shuyi.ron,build.rs}`.
2. Add the three template files' changes. Run `cargo shuyi new` into a
   folder outside the repository with `--template blank` and with
   `--template grid`, both with `--engine-path` at this checkout, and
   `cargo shuyi web` in each.
3. Update the plans golden (`crates/cli/src/template/plans.golden`) and
   any test that lists a new game's files.
4. Add the `templates` job's step.
5. Update the game's README, `docs/templates.md` and the book.
6. Run the gates.

## Acceptance criteria

- [ ] In a game fresh from `shuyi new --template blank` and from
      `--template grid`, `cargo shuyi web` exits 0 and writes
      `dist/web/index.html`, the game's `.wasm` and `NOTICES.txt`.
- [ ] CI's `templates` job builds each template's game for the web.
- [ ] The quick start tells a reader to run `cargo shuyi web` on their
      new game.
- [ ] The book's chapter is updated (2802), or the completion notes say why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The `templates` job's web build of each template's game.
- The template's own tests, as they are, still pass on a new game.

## Completion notes

*(Filled in by the session that completes the ticket.)*
