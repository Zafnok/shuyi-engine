---
id: "2837"
title: "The code pad: a Codespace per step of the book's project, opened from the step's page, with the step built and its web build forwarded"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2094"]
nick_input: none
completed:
---

# 2837 — A Codespace per step of the book's project

## Context

Nick wants a code pad (2026-11-01 and 2026-11-03, `docs/decisions.md`): a
reader of the book edits a step's Rust and sees its game change. 2916
compared five ways (`docs/research/2916-code-pad.md`) and chose a
Codespace a step, on the reader's own GitHub account, free to Nick
([ADR-0126](../../docs/adr/0126-the-code-pad-a-codespace-per-step-not-a-build-server.md) §1).

What there is:

- `.devcontainer/devcontainer.json`: the image
  `mcr.microsoft.com/devcontainers/base:ubuntu-24.04`, a
  `postCreateCommand` running `bash scripts/setup-linux.sh` and `cargo run
  -q --bin shuyi -- mcp install --yes …`, rust-analyzer. It opens at the
  repository's root.
- The book's quick start links `codespaces.new/Zafnok/shuyi-engine?quickstart=1`
  (`site/content/learn/book/quick-start.md`, *Try it with nothing
  installed*), and so does `README.md`.
- Each step is `site/tutorial/<step>/`, its own workspace, its web build
  `cargo shuyi web` into `dist/web/` (its `shuyi.ron`). Its page is
  `site/content/learn/book/box-pusher/<step>.md`, laid out by
  `site/templates/chapter.html`, which shows *Play this step* and the
  playground for a page with `extra.files`.
- Measured by 2916 on a new 2-core Codespace: the setup 40.5 s, then
  step 2's first builds (the research's table); a `dev` rebuild after an
  edit, 1.3 s. A Codespace has no display, so the game shows as its web
  build on a forwarded port, private by default.
- 2916 found the root's container's first boot failing on a race in
  `apt` (2094, this ticket's blocker): a step's container runs the same
  script.
- 2916 tried `codespaces.new` links: `machine=` works; a
  `devcontainer_path` that doesn't exist falls back to the one container
  without a word, so the parameter is confirmed only once a second
  container exists (step 4 below).

**Either order with 2528:** if `shuyi web --serve` is done, the container
runs it (it serves, rebuilds and reloads on save); if not, it serves
`dist/web/` with a static server the image has (`python3 -m http.server`)
and the page tells the reader to run `cargo shuyi web` and reload after
an edit, and 2528 then changes the containers.

## Nick input

None. Prebuilds stay off: they cost Nick money and are his decision
(`docs/decisions.md`, 2916's answer).

## Scope

**In:**
- **A container a step:** `.devcontainer/<step>/devcontainer.json` for
  each of `site/tutorial/*` (ten), named after the step's title ("Box
  Pusher, step 2: a level on the board"). The root's file but for:
  `workspaceFolder` the step's folder; a `postCreateCommand` that runs the
  setup, then `cargo shuyi web` in the step; `forwardPorts` the serving
  port, with `portsAttributes` opening it in a preview, private; a
  `postAttachCommand` (or `postStartCommand`) that serves `dist/web/`.
- **A test that keeps them in step:** `cargo xtask check-devcontainers`
  (or a case of an existing xtask check), failing when a step has no
  container, a container names a step that doesn't exist, or one differs
  from the root's in anything but those fields.
- **The link on each step's page:** in `chapter.html`, beside *Play this
  step*, for a page with `extra.step`: "Change its Rust in a Codespace",
  to `https://codespaces.new/Zafnok/shuyi-engine?devcontainer_path=.devcontainer/<step>/devcontainer.json&machine=basicLinux32gb`,
  with a line saying it is a computer in the browser, paid from the
  reader's own GitHub account's free hours. The twin says the same.
- **The book:** the quick start's *Try it with nothing installed* says a
  step's page opens a Codespace at that step; the project's front page
  (`box-pusher/_index.md`) says how to use it once.
- **Docs:** `site/README.md` (*The code pad*), the docs map's row of the
  book's project.

**Out (do not do):**
- a command that runs an exercise's check, and exercises that edit Rust
  (2838);
- Codespaces prebuilds;
- a build server, or editing Rust in the page (ADR-0126 §3).

**Estimated size:** about 700 lines (data 200, xtask check and tests 300,
templates and book 200).

## Implementation steps

1. Read ADR-0126, the research's *A Codespace*, and 2528's state.
2. Write step 2's container, push the branch, and open
   `codespaces.new/…?devcontainer_path=…&ref=<branch>` (or the creation
   page's own list) to make one on the branch; check it opens at the step,
   builds it, and forwards the game, which plays in the preview. Time it
   from the link to the game, and note it.
3. Write the other nine, and the check.
4. The page's link: confirm that the creation page picks the container a
   `devcontainer_path` names (it lists two now), and that `machine=`
   picks 2-core.
5. The book and docs, then delete every Codespace made.

## Acceptance criteria

- [ ] Every step of `site/tutorial/` has a container, and the check fails
      on a missing, extra or drifted one.
- [ ] A Codespace from step 2's link opens at the step, with its web build
      built and served on a private forwarded port, timed in the notes.
- [ ] Each step's page links its own container, and the twin says so.
- [ ] The book's chapter is updated (2802): the quick start and the
      project's front page.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (xtask): the check, on a fixture tree with a missing, an extra and
  a drifted container.
- By hand: step 2 and step 4, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
