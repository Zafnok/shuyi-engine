---
id: "2840"
title: "The book served from its Codespace: the reader follows the book there, and each step's game is rebuilt when its files change"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: in-progress
blocked_by: ["2837"]
nick_input: none
completed:
---

# 2840 — The book served from its Codespace

## Context

2837 (done) made the book's Codespace (`.devcontainer/book/`, ADR-0126
§1): each step's page links it, and it opens VS Code at `site/tutorial/`
with every step's web build made and step 2's served on port 4000
(`site/tutorial/serve.sh`). The reader edits a step's Rust in the editor,
runs `cargo shuyi web`, and reloads the game.

Nick, seeing it (2026-11-17, `docs/decisions.md`): "it opens a vscode
session and we cant follow along the book now? seems weird", and asked
that the Codespace host the book, with the step's Rust in the page's tabs
as the `.ron` files are. This ticket is the first half: the book itself
served from the Codespace, each step's *Play this step* and playground
framing the step's own build there, rebuilt when the reader changes the
step's files. 2841 adds the Rust tabs to the playground on top of it.

The public site doesn't change: it has no build server (ADR-0126 §3), and
a page there can't reach the reader's Codespace (its port is private to
the reader's signed-in browser, and making it public opens it to anyone).

What there is:

- `.devcontainer/book/devcontainer.json`: the `onCreateCommand` sets up,
  installs Python, builds step 2 (`.devcontainer/book/build.sh 02-board`)
  and runs `serve.sh`; the `updateContentCommand` builds the other nine;
  `waitFor: onCreateCommand`; the `postAttachCommand` runs `serve.sh`;
  port 4000 forwarded with `onAutoForward: openPreview`.
  `CARGO_TARGET_DIR=/home/vscode/book-target`.
- The site is built in CI (`.github/workflows/pages.yml`): `cargo xtask
  site-data --site site`, `bash site/errors.sh`, `bash
  site/copy-playground.sh`, `bash site/copy-previews.sh`, then `zola build
  --base-url "$BASE_URL"`, with Zola v0.21.0 from
  `.github/actions/setup-zola/action.yml` (downloaded, its SHA-256
  checked). Each step's web build is copied to
  `/learn/book/box-pusher/<step>/play/`.
- Each step's page (`site/templates/chapter.html`) has *Play this step*
  (`<permalink>play/`), the Codespace's button and its line, and, with
  `extra.files`, the playground framing `play/`
  (`templates/playground-macros.html`, `static/playground.js`). The
  twin's line is `codespace` in `crates/xtask/src/llms.rs`.
- Measured (2837's notes): a step's rebuild after an edit 8.1 s on 2
  cores; the Codespace's disk 4.8 GB with every step built.

**Either order with 2528:** if 2528 is done, `serve.sh` (which 2528
changed to run `cargo shuyi web --serve` in the served step's folder) is
replaced by this ticket's server all the same, which rebuilds every step
on change, and the pages' line that the game reloads on save stays true;
if not, `serve.sh` is replaced as it is, and nothing is left for 2528 to
change in the Codespace.

## Nick input

None. (Nick asked for it: `docs/decisions.md`, 2026-11-17, *the book's
Codespace hosts the book*. Its prebuild must still fit his included
storage: the measure below says.)

## Scope

**In:**
- **Zola in the container**, at CI's version and checksum, from one place:
  `scripts/setup-zola.sh` downloads Zola v0.21.0, checks its SHA-256 and
  installs it; `.github/actions/setup-zola/action.yml` runs it, and so
  does the book's `onCreateCommand` (after the setup). Zola is MIT
  (`getzola/zola`'s `LICENSE`); a tool, not shipped.
- **The book's server**, `cargo xtask book-pad`
  (`crates/xtask/src/book_pad.rs`, `std::net` alone, no new dependency):
  - it builds the site once as CI does, into a folder outside the
    repository's tree (`/home/vscode/book-site`), with the forwarded
    address as its base URL
    (`https://${CODESPACE_NAME}-4000.${GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN}`,
    read at start: a prebuild's name isn't the reader's), or
    `http://127.0.0.1:4000` outside a Codespace;
  - it binds `127.0.0.1:4000` and serves that folder, with
    `/learn/book/box-pusher/<step>/play/…` answered from
    `site/tutorial/<step>/dist/web/…` (the reader's own build, not CI's),
    `/` redirected to the last step page served (step 2's at first), no
    caching, `.wasm` as `application/wasm`;
  - **rebuilding:** once a second it reads the modification times under
    each step's `crates/` and its `Cargo.toml`; a step that changed is
    built again (`cargo shuyi web` in its folder), one build at a time, a
    change during a build queuing one more. Each step has a build number
    and a state (`building`, `ok`, `failed` with cargo's output);
    `GET /pad/build/<step>` answers them as JSON. A failed build keeps
    the last good one served;
  - **the page follows:** when the server answers `GET /pad/status`, the
    step's page (`chapter.html`'s script, or `playground.js`) polls
    `/pad/build/<step>` while it is open, says *Building…* beside *Play
    this step*, reloads the playground's frame on a new good build, and
    shows a failed build's output where the playground shows Problems.
    On the public site `/pad/status` is a 404, and nothing changes;
  - `/pad/…` answers only `GET`; a request whose `Host` isn't
    `127.0.0.1:4000`, `localhost:4000` or the forwarded host is refused
    (403).
- **The container:** `serve.sh` goes; the `onCreateCommand` adds Zola and
  builds `xtask` into the shared target folder; the `postAttachCommand`,
  and the `onCreateCommand`'s last part as `serve.sh` was (so a Codespace
  without the prebuild shows the book while the other steps build),
  start `cargo xtask book-pad` in the background, once (a second start
  finds the port taken and says the book is already served). Port 4000
  stays forwarded, private, `openPreview`. `cargo xtask
  check-devcontainers` keeps passing (it checks the steps and the root's
  setup, not this).
- **The pages:** the Codespace's button on each step's page says the
  Codespace opens this book, with the step's own build in *Play this
  step* and the playground, rebuilt when its files change; the line under
  it names the step's folder, and that a Codespace that has just opened
  may still be building the other steps. The twin (`codespace` in
  `llms.rs`) says the same. The project's front page and the quick start
  say the book opens in the Codespace's preview, or in a browser tab from
  its Ports tab.
- **Measure** in a Codespace of the branch: the time from the link to the
  book in the preview, the disk with the site and `xtask` built (the
  prebuild's size: it must stay within Nick's included 20 GB-month with
  his own Codespaces; if it won't, say so in the notes and ask him with
  the number), and the time from an edit saved in the editor to the
  page's frame showing it.
- **Docs:** `site/README.md`, *The code pad* (the server, its routes,
  the rebuilding) and *The playground* (its frame in the Codespace); the
  docs map's row of the book's project; the container's comments.

**Out (do not do):**
- the step's Rust as tabs in the playground, and writing files from the
  page (2841);
- anything on the public site beyond the button's text and line;
- a build server, or reaching the Codespace from the public site
  (ADR-0126 §3);
- `shuyi web --serve` itself (2528; the clause above);
- the steps' features (2839).

**Estimated size:** about 900 lines (code 350, tests 300, scripts and
container 80, templates and book 70, docs 100).

## Implementation steps

1. Read ADR-0126, 2837's completion notes, `site/README.md` (*The code
   pad*, *The playground*), and `pages.yml`'s site steps.
2. `scripts/setup-zola.sh`, and the action running it; check CI's `site`
   job still passes.
3. `book_pad.rs`: the site build, the routes, the watcher and the build
   queue behind a small builder trait (`build(&mut self, step) ->
   Result<(), String>`), so tests drive it with a fake; the
   `xtask book-pad` command and its usage line.
4. The page's polling (`chapter.html` or `playground.js`), off unless
   `/pad/status` answers.
5. The container: Zola, `xtask`, the server; `serve.sh` gone and its
   mentions (`chapter.html`, `llms.rs`, the book, `site/README.md`)
   changed.
6. Push the branch, make a Codespace of it from
   `codespaces.new/Zafnok/shuyi-engine/tree/<branch>?devcontainer_path=.devcontainer/book/devcontainer.json&machine=basicLinux32gb`,
   measure as the Scope says, and follow the book there: open step 3,
   change a line of its Rust in the editor, and see its *Play this step*
   and playground show it. Note it, then delete the Codespace and any
   scratch branch.

## Acceptance criteria

- [ ] In a Codespace of the branch, the preview shows the book; a step's
      *Play this step* and playground play that step's build from the
      Codespace, not CI's.
- [ ] An edit saved under a step's `crates/` rebuilds that step, and its
      page's frame shows the change without a reload by hand; a failed
      build shows cargo's output in the page and keeps the last good one.
- [ ] On the public site nothing changes but the button's text and line
      (`/pad/status` is a 404 there).
- [ ] Zola's version and checksum are in one place, which CI and the
      container both use.
- [ ] The notes have the time to the book, the time from a save to the
      frame, and the disk; the prebuild's size fits Nick's included
      storage, or the notes say why not and he is asked.
- [ ] The book's chapter is updated (2802): the quick start and the
      project's front page.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (xtask): the routes (a step's `play/` from its `dist/web/`, the
  site's files, `/` redirected, a `..` path refused, a wrong `Host`
  refused), the watcher (a change under one step's `crates/` queues that
  step only; a change during a build queues one more), and the build
  states, with a fake builder.
- By hand: step 6, in the notes.

## Completion notes

**What was done:**

- `scripts/setup-zola.sh`: Zola v0.21.0, its SHA-256 checked, installed
  in `/usr/local/bin` (kept when that version is there). The action
  `.github/actions/setup-zola/` runs it, and so does the book's
  `onCreateCommand`: the version and checksum are written there only.
- `cargo xtask book-pad [--out <dir>] [--no-build]`
  (`crates/xtask/src/book_pad.rs`, `std::net` and `std::thread`, no new
  dependency): binds `127.0.0.1:4000` first (taken: it says the book is
  served already, and exits 0), builds the site as CI does (`site-data`
  in-process, `errors.sh`, `copy-playground.sh`, `copy-previews.sh`,
  `zola build --force`) into `~/book-site` at the forwarded address read
  at start (`https://${CODESPACE_NAME}-4000.${GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN}/`,
  or `http://127.0.0.1:4000/`), then serves it, a thread a connection,
  `Connection: close`, `Cache-Control: no-store`, `.wasm` as
  `application/wasm`. Routes: a step's `play/…` from its `dist/web/`,
  `/` a 302 to the last step page served (step 2's at first),
  `/pad/status` (`{"pad":true}`), `/pad/build/<step>` (`number`, `good`,
  `state`, `output`), the site's files, its `404.html`. `GET` only (405
  otherwise), a `Host` other than `127.0.0.1:4000`, `localhost:4000` or
  the forwarded host refused (403), a `..`/`.` segment, `\`, `:`, NUL or
  bad escape refused (400).
- The watcher (`Watcher::tick`, once a second on its own thread): each
  step's files under `crates/` and its `Cargo.toml` by path, time and
  length (a new or removed file counts), a step queued once, one build a
  tick behind a `Builder` trait (`cargo shuyi web` in the step's folder;
  a fake in the tests), a change during a build queued again at the next
  tick. A failed build keeps its output and the last good `dist/web/`
  (the web build replaces it only whole).
- `site/static/book-pad.js`, loaded by every step page: asks
  `pad/status` at the site's base; served from the Codespace it swaps the
  Codespace's button and line (`pad-away`) for a line saying the reader is
  in it (`pad-here`), and polls `pad/build/<step>` each second while the
  page is shown: *Building…* beside *Play this step*, the playground's
  frame reloaded on a new good build (if it has loaded), and a failed
  build's output in Problems (or in `pad-output` on a step without a
  playground). On the public site the 404 leaves the page as it is.
- The container: Python and `serve.sh` gone; the `onCreateCommand` adds
  Zola after the setup, builds step 2 and runs `.devcontainer/book/pad.sh`,
  which builds `xtask` into the shared target folder and starts `cargo
  xtask book-pad` in the background (`setsid nohup`, log
  `~/.book-pad.log`), printing the log if it stopped at once; the
  `postAttachCommand` runs `pad.sh` too. Port 4000 stays forwarded,
  private, `openPreview`, labelled "The book". `check-devcontainers`
  passes unchanged.
- The pages: **Open this book in a Codespace**, and its line (the book in
  the Codespace, *Play this step* and the playground playing the reader's
  own build of the step's folder, built again on each save, its Rust's
  folder, the other steps maybe still building); the twin says the same
  (`codespace` in `llms.rs`, its test). The quick start and the project's
  front page say the book opens in the preview, or from the Ports tab.
- Docs: `site/README.md` (*Preview it*: the script; the files' table;
  *The playground*: its frame and files in the Codespace; *The code pad*:
  the server, its routes, the rebuilding, the page), the docs map's row
  of the book's project, the container's comments, `xtask`'s usage.
- Tests: 15 unit tests in `crates/xtask/src/book_pad/tests.rs`: the
  options, the address, a step's `play/` and `files/` from its folder, the
  site's files uncached, a folder without its `/`, `/` to the last step
  page, nine paths that leave their folder (400), a wrong or missing
  `Host` and a `POST` refused, the pad's routes, a request read from bytes
  (and three that aren't one), and with a fake builder: a change under
  one step's `crates/` builds that step only (a new file and a manifest
  count, a file outside `crates/` doesn't), a change during a build builds
  it once more, changes queue each step once in order, a failed build
  keeps the last good one and says why, a build made another way is a
  good one and its own isn't counted twice; and one over TCP.

**Checked locally** (Windows, at `a500dbdb`): the site built at
`http://127.0.0.1:4000/`, step 3 built, `book-pad --no-build` serving it in
the browser pane: a compile error saved in `03-walk/crates/game/src/title.rs`
showed cargo's error in Problems 2 s later, the frame kept playing; the
file put back, the line said *Building…*, and the frame reloaded 7 s after
the save, Problems emptied.

MEASURED

**Deviations:**

- Each step's playground `files/` is served from the step's
  `crates/game/files/` too (the ticket named `play/` only): its frame
  plays the reader's files, so **Reset** and the editors' own text must
  be the reader's, not CI's copy.
- The build state has a fourth value, `none`, for a step not built yet
  (a Codespace without the prebuild serves the book before the other
  nine are built); the page says so, and a `dist/web/index.html` that
  appears (the container's `build.sh`, or the reader's own `cargo shuyi
  web`) counts as a good build, so the frame follows those too.
- The page's script is its own file, `static/book-pad.js`, loaded by
  every step page (`chapter.html`), not `playground.js`: steps without a
  playground follow their builds too. Served from the Codespace, the
  Codespace's button and line give way to a line saying the reader is in
  it.
- `--out` and `--no-build`, for serving a site built elsewhere (how it was
  checked on Windows, where `bash` from a program is WSL's).
- The pad's routes are found at the site's base (`get_url(path='pad')`),
  so on the public site the 404 is the site's own, not the host's root.
- The server shuts the socket's writing side and drains it before
  closing: Windows resets a socket closed with unread bytes, which the
  TCP test met about one run in four.
- 2528 isn't done: `serve.sh` was replaced as it was, and 2528's clauses
  on the Codespace now say nothing is left there for it.
