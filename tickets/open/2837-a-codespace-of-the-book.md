---
id: "2837"
title: "The code pad: one Codespace of the book's project, every step built and prebuilt once, opened from each step's page"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2094"]
nick_input: setup
completed:
---

# 2837 — A Codespace of the book's project

## Context

Nick wants a code pad (2026-11-01 and 2026-11-03, `docs/decisions.md`): a
reader of the book edits a step's Rust and sees its game change. 2916
compared five ways (`docs/research/2916-code-pad.md`) and Nick chose a
Codespace on the reader's own GitHub account, with one prebuild kept
within his plan's included storage (decision of 2026-11-17,
[ADR-0126](../../docs/adr/0126-the-code-pad-a-codespace-of-the-book-not-a-build-server.md) §1).

What there is:

- `.devcontainer/devcontainer.json`: the image
  `mcr.microsoft.com/devcontainers/base:ubuntu-24.04`, a
  `postCreateCommand` running `bash scripts/setup-linux.sh` and `cargo run
  -q --bin shuyi -- mcp install --yes …`, rust-analyzer. It opens at the
  repository's root.
- The book's quick start links `codespaces.new/Zafnok/shuyi-engine?quickstart=1`
  (`site/content/learn/book/quick-start.md`, *Try it with nothing
  installed*), and so does `README.md`.
- Each step is `site/tutorial/<step>/`, its own workspace with its own
  lock (the ten locks list the same crates and versions), its web build
  `cargo shuyi web` into `dist/web/` (its `shuyi.ron`). Its page is
  `site/content/learn/book/box-pusher/<step>.md`, laid out by
  `site/templates/chapter.html`, which shows *Play this step* and the
  playground for a page with `extra.files`.
- Measured by 2916 on a new 2-core Codespace of the root's container:
  40 s to *Available*; the setup 40.5 s; the engine's tools for the MCP
  server 136 s; step 2's first `cargo shuyi web` about 6.5 minutes in
  `dev` (its tool program about 5, the web build 1.5); then 4.9 s an edit;
  4.8 GB of disk at the end, 1.6 GB of it step 2's target. The game
  showed from a private forwarded port after GitHub's sign-in hop. A
  Codespace has no display.
- 2916 found the root's container's first boot failing on a race in
  `apt` (2094, this ticket's blocker): the book's container runs the same
  script.
- 2916 tried `codespaces.new` links: `machine=` works; a
  `devcontainer_path` that doesn't exist falls back to the one container
  without a word, so the parameter is confirmed only once a second
  container exists (step 5 below).

**Either order with 2528:** if `shuyi web --serve` is done, the container
serves with it (it rebuilds and reloads on save); if not, it serves a
step's `dist/web/` with a static server the image has (`python3 -m
http.server`), the page tells the reader to run `cargo shuyi web` and
reload after an edit, and 2528 then changes the container.

## Nick input

**Setup**, once the container is on `main` (the session gives the exact
clicks with the numbers): in the repository's *Settings → Codespaces →
Set up prebuild*, the configuration `.devcontainer/book/devcontainer.json`
on `main`, **one region** (the one nearest most readers; US East if
unsure), **1 version** kept, trigger **"Configuration change"**, failure
notifications to himself. Then he reads its size under *Codespaces* in
his billing page and tells the session.

It must stay within his plan's included 20 GB-month, shared with his own
Codespaces (decision of 2026-11-17). If the session's measure says it
won't, it is not turned on, and he is asked again with the number.

## Scope

**In:**
- **The book's container:** `.devcontainer/book/devcontainer.json`, named
  "The book's project (Box Pusher)": the root's image and setup, with
  - `workspaceFolder` `site/tutorial`;
  - an `onCreateCommand` (which a prebuild runs; `postCreateCommand`
    doesn't) running the setup, then each step's `cargo shuyi web` in
    `dev` with one `CARGO_TARGET_DIR` for all ten (on the Codespace's
    own disk, not in the repository's tree), so the engine is built once;
  - no MCP registration: a reader of the book doesn't need it (the
    root's container keeps it);
  - `forwardPorts` the serving port, `portsAttributes` private, opened in
    a preview, and a `postAttachCommand` that serves the step a file
    names (step 2's by default) and says how to serve another.
- **Measure** in a Codespace of the branch, before the prebuild: the time
  from the link to step 2's game with no prebuild, the disk used (the
  prebuild's size), and whether the shared target folder is reused across
  steps (each step after the first should build only its own crate). If
  ten steps make a Codespace without the prebuild much slower than one
  (the ADR's *Consequences*), build only step 2 there and the rest when
  the reader first runs a step; say what was chosen.
- **A test that keeps it in step:** a case of an xtask check (or `cargo
  xtask check-devcontainers`) that fails when the book's container names a
  step that doesn't exist, misses one, or differs from the root's in the
  image or the setup command.
- **The link on each step's page:** in `chapter.html`, beside *Play this
  step*, for a page with `extra.step`: "Change its Rust in a Codespace",
  to `https://codespaces.new/Zafnok/shuyi-engine?devcontainer_path=.devcontainer/book/devcontainer.json&machine=basicLinux32gb`,
  with a line saying it is a computer in the browser, paid from the
  reader's own GitHub account's free hours, and the step's folder to open
  and the two commands (rebuild, serve). The twin says the same.
- **The book:** the quick start's *Try it with nothing installed* names
  the book's Codespace for the project; the project's front page
  (`box-pusher/_index.md`) says once how to use it.
- **Docs:** `site/README.md` (*The code pad*), the docs map's row of the
  book's project.

**Out (do not do):**
- a command that runs an exercise's check, and exercises that edit Rust
  (2838);
- a container per step (ADR-0126's alternatives: ten prebuilds);
- turning the prebuild on (Nick's, in *Nick input*), or anything that
  bills his account;
- a build server, or editing Rust in the page (ADR-0126 §3).

**Estimated size:** about 600 lines (data 100, xtask check and tests 300,
templates and book 200).

## Implementation steps

1. Read ADR-0126, the research's *A Codespace*, and 2528's state.
2. Write the container, push the branch, and make a Codespace of it from
   a `codespaces.new/…?devcontainer_path=…` link on the branch; check it
   opens at `site/tutorial`, builds the steps, forwards the game, which
   plays in the preview. Measure as the scope says, and note it.
3. The check, then the page's link, the book and the docs.
4. Delete every Codespace made.
5. After merge: confirm the creation page picks the container a
   `devcontainer_path` names (it lists two now) and that `machine=` picks
   2-core. Then give Nick the prebuild's setup with the measured size, as
   *Nick input* says, and record in the notes what he set.

## Acceptance criteria

- [ ] The book's container builds every step into one target folder, and
      the check fails on a missing or extra step, or a drifted image or
      setup.
- [ ] A Codespace from the link opens at `site/tutorial` with step 2's web
      build served on a private forwarded port; its time and disk are in
      the notes.
- [ ] Each step's page links the book's Codespace and names its folder,
      and the twin says so.
- [ ] Nick's prebuild is set as *Nick input* says, within his included
      storage, or the notes say why not.
- [ ] The book's chapter is updated (2802): the quick start and the
      project's front page.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (xtask): the check, on a fixture tree with a missing and an extra
  step and a drifted container.
- By hand: steps 2 and 5, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
