---
id: "2837"
title: "The code pad: one Codespace of the book's project, every step built and prebuilt once, opened from each step's page"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2094"]
nick_input: setup
completed: 2026-11-17
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
  `apt`; the script now tries `apt-get` three times (2094, done), and the
  book's container runs the same script.
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

- [x] The book's container builds every step into one target folder, and
      the check fails on a missing or extra step, or a drifted image or
      setup.
- [x] A Codespace from the link opens at `site/tutorial` with step 2's web
      build served on a private forwarded port; its time and disk are in
      the notes.
- [x] Each step's page links the book's Codespace and names its folder,
      and the twin says so.
- [x] Nick's prebuild is set as *Nick input* says, within his included
      storage, or the notes say why not.
- [x] The book's chapter is updated (2802): the quick start and the
      project's front page.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (xtask): the check, on a fixture tree with a missing and an extra
  step and a drifted container.
- By hand: steps 2 and 5, in the notes.

## Completion notes

**What was done:**

- `.devcontainer/book/devcontainer.json`, "The book's project (Box
  Pusher)": the root's image, setup and `remoteEnv`, opened at
  `site/tutorial` (`/workspaces/${localWorkspaceFolderBasename}/site/tutorial`,
  which Codespaces honours), no MCP server, `CARGO_TARGET_DIR` at
  `/home/vscode/book-target` (in `containerEnv`, so the reader's own
  builds use it too). `.devcontainer/book/build.sh <step>…` runs each
  step's `cargo shuyi web` into it and prints each step's time.
- `site/tutorial/serve.sh [step]` serves a step's `dist/web/` on port
  4000 with Python's server, uncached, `.wasm` as `application/wasm`: the
  step whose folder it runs in, the one named, or the last served
  (`~/.book-step`, step 2 at first). It stops the server it started
  before, and says so when the new one doesn't start. Port 4000 is
  forwarded (private, GitHub's default) with `onAutoForward:
  openPreview`.
- `cargo xtask check-devcontainers` (`crates/xtask/src/devcontainers.rs`,
  in CI's `tickets` job and the `tickets' checks` gate): the steps the
  `onCreateCommand` and `updateContentCommand` build, against the folders
  of `site/tutorial` with a `shuyi.ron` (missing, extra, twice); the image,
  the `remoteEnv`, the folder, and the root's setup (its
  `postCreateCommand` up to `. "$HOME/.cargo/env"`, each `bash <script>`
  read from its container's folder). It reads JSON with comments. 18 unit
  tests, three on a fixture tree (one with a missing and an extra step and
  a drifted image), and two in `tests/cli.rs`.
- Each step's page (`chapter.html`): **Change its Rust in a Codespace**
  beside *Play this step*, to `config.toml`'s `extra.codespace`, and a
  line: a computer in the browser, paid from the reader's own free hours,
  the step's folder, its `src/`, `cargo shuyi web` and `bash ../serve.sh`.
  The playground's note says Rust builds there too. The twin says the
  same (`llms.rs`, `codespace`; a test keeps its URL equal to the
  config's). `check-build.sh` fails a step page without the link or its
  folder.
- The book: the quick start's *Try it with nothing installed* links the
  book's Codespace; the project's front page says once how to use it.
  Docs: `site/README.md` (*The code pad*), the docs map's row of the
  book's project, the root README's row of the dev container, the
  run-gates skill's list.

**Measured** (on 2-core Codespaces, `basicLinux32gb`, US East, from the
branch at `d60152a6` and `8ff8eba4`):

| What | Time |
| ---- | ---- |
| Every step in the `onCreateCommand` (first container): the link to *Available* | 12 min 43 s |
| Step 2 alone, from an empty target folder | 404 s (358 s on the second Codespace) |
| Then step 1 | 73 s |
| Then steps 3–6, each | 9–10 s |
| Then step 7 | 239 s |
| Then steps 8–10, each | 10–11 s |
| All ten (`build.sh`, timed) | 786 s |
| Step 2 in the `onCreateCommand`, the rest in `updateContentCommand` (second container): the link to *Available* | 7 min 36 s |
| An edit to step 2's Rust, then `cargo shuyi web` | 8.1 s |

- **The shared target folder is reused**, but not as one: the ten locks
  resolve three sets of the engine's dependencies' features (step 1;
  steps 2–6, which add `serde` with its default features; steps 7–10,
  which add `ron` with its default features), and each set builds much
  of the engine again. Each step in a set after its first builds only
  its own crate, about 10 s.
- **Disk:** the target folder 2.7 GB with every step built (3.5 GB once
  rust-analyzer has run, its `flycheck0` folder), the container's disk
  4.8 GB in all (`du -sx /`: the image, Rust 1 GB, the target folder),
  the repository 0.14 GB. A prebuild is that disk: about **5 GB**, so
  5 GB-month of the 20 Nick's plan includes, one region and one version.
- **Memory:** 3.2 GB of 8 used with rust-analyzer indexing the ten
  workspaces.
- The game showed from the private forwarded port
  (`https://<codespace>-4000.app.github.dev/`) after GitHub's sign-in hop,
  and played: title, then step 2's level.
- The creation page from
  `codespaces.new/Zafnok/shuyi-engine/tree/<branch>?devcontainer_path=.devcontainer/book/devcontainer.json&machine=basicLinux32gb`
  picked "The book's project (Box Pusher)" and *2-core* (the two
  containers are on the branch; nothing was created from it).

**Chosen:** ten steps took twice step 2's time (786 s against 404 s), so a
Codespace without the prebuild (before Nick turns it on, and in every
region it isn't in) builds step 2 first and opens after it: the
`onCreateCommand` sets up, adds Python, builds step 2 and serves it; the
`updateContentCommand` builds the other nine; `"waitFor":
"onCreateCommand"`. A prebuild runs both, so one made from it opens with
every step built. Behind the open editor, the nine take about 6 minutes,
shown in the Codespace's own creation terminal, and a build the reader
starts meanwhile waits for the target folder's lock (`serve.sh` says so).
2839 makes the ten locks resolve one set of features, after which all ten
cost about step 2's time and go back in one command.

**Deviations:**

- The image has no Python (2916 assumed `python3 -m http.server`): the
  `onCreateCommand` installs `python3` after the setup, and `serve.sh`
  runs Python's server with no caching rather than `-m http.server`.
- The steps are split between two commands (above), not all in the
  `onCreateCommand`, so the check reads both and no longer asks for
  them in order.
- The server for step 2 starts in the `onCreateCommand` too: the
  `postAttachCommand` runs only after the `updateContentCommand`, so
  without it a Codespace with no prebuild would show no game for its
  first minutes. A process started there survives into the Codespace
  (checked: it served while the nine built).
- *Opened in a preview:* `openPreview` didn't open the port by itself in
  the second Codespace, whose server was already listening as the editor
  attached; the Ports tab and the forwarded address open it, and
  `serve.sh` and the pages say so.
- Tried and dropped: `git.openRepositoryInParentFolders: "always"`, which
  traded a notification for a modal trust prompt that came back.
- No ticket 2528 change: `shuyi web --serve` isn't done; its clause now
  says what to change in `serve.sh`.

**Codespaces made:** `special-journey-r4g6gjjggp9cpw5j` and
`ideal-robot-4j7v7xx779vf77rq`, both deleted; the scratch branch
`scratch-2837-book-codespace` they were made from, deleted.

**Follow-up:** 2839 (the steps resolve one set of features, so the
engine is built once). 2528 and 2838 say what they take from here.

Size: 997 counted lines against the 600 estimated: the check's fixture
tests and the split build. It would split cleanly at the check (`crates/xtask/`).

Book: the quick start and the project's front page say how to use the
book's Codespace, and each step's page links it.

Game: nothing to adopt, the change is the site, a dev container and an
xtask check; no crate changed.

No break. No `Behaviour:` line: no crate changed.

**The prebuild isn't set yet:** GitHub makes it from a branch's
container, and Nick's is on `main`, so it waits for this PR's merge. Its
size, measured above, fits his included storage.

**Nick input, setup** (once this is on `main`): the prebuild, measured
at about 5 GB, so 5 of his 20 included GB-month. In
<https://github.com/Zafnok/shuyi-engine/settings/codespaces>, *Set up
prebuild*:

1. Branch **main**; configuration **`.devcontainer/book/devcontainer.json`**.
2. Region: **US East** only (untick the others).
3. Prebuild triggers: **Configuration change**.
4. Template history: **1** version.
5. Failure notifications: himself.
6. **Create**. Its first run takes about 15 minutes; then, in his
   billing page (*Codespaces*, storage), its size, which he tells the
   session that works 2839 or a later one: past 15 GB with his own
   Codespaces, it is turned off and he is asked again.
