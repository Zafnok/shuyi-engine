---
id: "2841"
title: "The step's Rust in the playground's tabs, in the book served from its Codespace: edit, Run, and the game rebuilt beside the page"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2840"]
nick_input: none
completed: 2026-11-18
---

# 2841 — The step's Rust in the playground's tabs

## Context

Nick asked (2026-11-17, `docs/decisions.md`, *the book's Codespace hosts
the book*) that the book's Codespace host the book and "unlock new tabs
for the rust code like we have tabs for the .ron files". 2840 serves the
book from the Codespace (`cargo xtask book-pad`,
`crates/xtask/src/book_pad.rs`), frames each step's own build there, and
rebuilds a step when its files change. This ticket adds the Rust: in a
page served from the Codespace, the playground has a tab for each of the
step's Rust files the page teaches, and **Run** writes them and plays the
rebuilt game, as it does the data files today.

The public site keeps its data tabs only: a page there can't build Rust
(ADR-0126 §3), and it says where to go to edit the Rust.

What there is:

- The playground (ticket 2820, `site/README.md`, *The playground*):
  `templates/playground-macros.html` and `static/playground.js`, a tab and
  an editor per file of the page's `extra.files` (from the project's
  `extra.game_files`), drafts kept under `shuyi-playground-draft:<page's
  path>:files/<name>`, **Run** writing the changed files to the game's
  `shuyi-page-file:` keys and reloading the frame, **Reset this file** and
  **Reset all**, and Problems. A `.ron` editor is painted by
  `static/ron-paint.js`; a `.map` or `.dlg` one stays plain.
- After 2840: `GET /pad/status` answers only in the Codespace;
  `GET /pad/build/<step>` gives a step's build number, state and cargo's
  output; the page polls it, reloads the frame on a new good build and
  shows a failed one in Problems. `/pad/…` answers only `GET`, and checks
  the `Host`.
  **From 2840 (done):** `/pad/status` answers `{"pad":true}` (add `rust`
  beside it); `/pad/build/<step>` answers `number` (builds since the
  server started), `good` (the last good one's number), `state` (`none`,
  `building`, `ok`, `failed`) and `output`. The polling is its own script,
  `site/static/book-pad.js`, loaded by every step page (`chapter.html`),
  which finds the playground's frame and Problems in the page: **Run**'s
  wait for the next good build can read the same route, or hand off to
  it. A step's `files/` is served from the step's
  `crates/game/files/` on disk, so the playground's own text is the
  reader's. The server answers every method but `GET` with 405
  (`Server::answer`): the `POST` routes go before that refusal.
- Each step's page names its files in a table (*Is it new?*); its
  Rust is under `site/tutorial/<step>/crates/game/src/`.

**Either order with 2838:** if 2838 is done, a `code` exercise on a page
served from the Codespace shows **Check my work**, which asks the server
(`POST /pad/check/<step>/<check>`, added here) to run `cargo xtask
exercise <step> <check>` and shows its `Done.` or `Not yet.` and text as
the page's other exercises do (a compile error in Problems); if not, 2838
adds it (its clause says so).

## Nick input

None.

## Scope

**In:**
- **`extra.rust`** on a step's page: the paths of the Rust files its
  playground edits, from the step's folder, in their tabs' order, each
  under `crates/game/src/` and there; `crates/xtask/src/book.rs` checks
  them (a missing file, a path outside `crates/game/src/`, a page with
  `extra.rust` and no `extra.files` or no `extra.step`). Steps 2 to 10
  list the Rust files their table names as new or changed.
- **The server** (`book_pad.rs`):
  - `GET /pad/status` says the Rust tabs are on (`{"rust": true}`);
  - `GET /pad/file/<step>/<path>`: the file as it is on disk;
  - `POST /pad/file/<step>/<path>` with the text (UTF-8, at most 256
    KiB): writes it, which 2840's watcher rebuilds; `POST
    /pad/reset/<step>/<path>`: puts back the file at `HEAD` (`git show
    HEAD:<path>`);
  - a `POST` is refused (403) unless its `Origin` is the served origin
    (the forwarded address, or `http://127.0.0.1:4000`) and its path is
    one of that step's page's `extra.rust` (read from
    `site/content/learn/book/box-pusher/<step>.md` at start): no other
    file in the repository can be written from the page.
- **The playground** (`playground.js`, `playground-macros.html`): when
  `/pad/status` says `rust`, a tab per `extra.rust` file after the data
  files' tabs, its text from `/pad/file/…`, plain (no Rust painting),
  its draft kept as the data files' are (`…:rust/<path>`). **Run** posts
  each Rust file whose draft differs from the file, then waits for the
  step's next build (2840's polling) before reloading the frame, with
  *Building…* shown; the data files run as today. **Reset this file** on
  a Rust tab posts its reset; **Reset all** resets the Rust tabs too.
  A failed build shows cargo's output in Problems, the frame keeping the
  last good build. Without `rust`, the page is as today, and its fold's
  line says the Rust can be changed in the book's Codespace (the
  button above).
- **The twin** (`llms.rs`): a step page with `extra.rust` names the files
  its playground edits in the book's Codespace.
- **`check-build.sh`:** a step page with `extra.rust` lists them in its
  playground's markup (`data-rust`), which the script reads.
- **Measure** in a Codespace of the branch: the time from **Run** after a
  one-line Rust edit in a tab to the frame showing it, and that a compile
  error shows in Problems with the last build still playing.
- **Docs:** `site/README.md`, *The playground* and *The code pad*; the
  docs map's row of the book's project; the project's front page says
  once that in the Codespace the playground edits the Rust too.

**Out (do not do):**
- serving the book and rebuilding (2840);
- Rust tabs on the public site, or building there (ADR-0126 §3);
- painting Rust in the editor (its own ticket, if wanted: `ron-paint.js`
  paints RON only);
- a Rust file a page doesn't list, or a new file from the page;
- running a `code` exercise's check (the clause above says who).

**Estimated size:** about 800 lines (code 300, tests 280, templates and
pages 100, docs 120).

## Implementation steps

1. Read 2840's completion notes, `book_pad.rs`, `playground.js` and
   `site/README.md` (*The playground*, *The code pad*).
2. `extra.rust` in `book.rs`, then the steps' pages.
3. The server's routes and their checks, with tests.
4. The playground's Rust tabs, **Run**, the resets; the fold's line on
   the public site; `check-build.sh`; the twin.
5. Push the branch, make a Codespace of it from
   `codespaces.new/Zafnok/shuyi-engine/tree/<branch>?devcontainer_path=.devcontainer/book/devcontainer.json&machine=basicLinux32gb`,
   open step 3's page in its preview, change a line of its rules in a
   Rust tab, **Run**, and play the change; then a line that doesn't
   compile. Note the times, then delete the Codespace and any scratch
   branch.

## Acceptance criteria

- [x] In the book served from a Codespace, a step page's playground has a
      tab for each `extra.rust` file, and **Run** after an edit plays the
      rebuilt game in its frame; a compile error shows in Problems and
      the last good build keeps playing.
- [x] **Reset this file** and **Reset all** put a Rust file back as
      `HEAD` has it, and the step is rebuilt.
- [x] The server writes no file but a step's `extra.rust`, and refuses a
      `POST` from another origin; tests show both.
- [x] On the public site the playground has no Rust tab, and its line
      points to the Codespace.
- [x] `book.rs` fails a wrong `extra.rust`, and the twin names the files.
- [x] The book's chapter is updated (2802): the project's front page.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (xtask): `extra.rust` checks; the routes: a read, a write, a reset
  (on a fixture repository with git), a path not listed, a `..`, a body
  over the limit and a foreign `Origin`, each refused.
- By hand: step 5, in the notes.

## Completion notes

**What was done:**

- **`extra.rust`** (`crates/xtask/src/book.rs`): read from a page's front
  matter as `files` is (`page_rust`), and checked by `cargo xtask
  check-book` (`rust_problems`, from `run`, with the step's folder
  `site/tutorial/<page's name>/`): each path a `.rs` file under
  `crates/game/src/` (`rust_path_problem`: no empty, `.` or `..` segment,
  no `\` or `:`), there, named once; a page with it and no `extra.step` or
  no `extra.files` fails. Steps 2 to 9 list the Rust files their table
  names as new or changed, in the table's order (step 5's
  `lib.rs`, `title.rs`, `stages.rs` from its `UiAction` row).
- **The server** (`crates/xtask/src/book_pad.rs`): `Request` reads
  `Origin`, `Content-Length` and a body of at most 256 KiB
  (`MAX_BODY`; a longer one isn't read); `rust_tabs` reads each step's
  `extra.rust` from `site/content/learn/book/box-pusher/<step>.md` as it
  starts; `/pad/status` answers `{"pad":true,"rust":true}`;
  `GET /pad/file/<step>/<path>` reads a listed file (another is a 404);
  `POST /pad/file/…` writes the body (UTF-8, else 400; over the limit
  413) and `POST /pad/reset/…` puts back `HEAD`'s text (`git cat-file
  --filters HEAD:./<step>/<path>`, so with the checkout's line ends),
  each answering the file's text and writing nothing when the text is the
  file's already (no build for nothing). A `POST` is refused 403 unless
  its `Origin` is the site's base or the loopback's
  (`Server::own_origin`) and its file is listed (`Server::rust_file`); a
  `..` is 400 as before; any other `POST` is still 405. The `GET /pad/…`
  routes now read the decoded segments.
- **The playground** (`templates/playground-macros.html`,
  `static/playground.js`, `templates/chapter.html`): with `rust`, `pad`
  and `step`, the markup holds a hidden tab and a plain editor
  (`data-rust`) per file after the data files'. The script asks
  `<pad>status`; where it says `rust` it reads each file
  (`<pad>file/…`), shows the tabs, and keeps drafts under
  `…:rust/<path>`; elsewhere it removes them. **Run** posts each Rust tab
  whose text differs from its file's, says *Building…* in Problems, and
  leaves the frame to `book-pad.js` (reloaded on the next good build; a
  failed build's output in Problems); the frame's load takes *Building…*
  away, since a game given no data file reports nothing. **Reset this
  file** on a Rust tab and **Reset all** post the reset, and a run that
  rebuilds when a file changed. The fold's line, on a step with
  `extra.rust`, says the Rust tabs are in the book's Codespace (the
  button above), or, there, how they work (`pad-away` and `pad-here`).
- **The twin** (`llms.rs`): a step's twin with `extra.rust` names its
  files (`<step>/crates/game/src/…`) after the Codespace's paragraph.
- **`check-build.sh`:** each file a step's `rust` names is in its folder,
  and the page has its `data-rust` editor.
- **Docs:** `site/README.md` (*The playground*: the Rust tabs, and what
  `check-build.sh` checks; *The code pad*: the routes, the `POST`s and
  their refusals, the tests), the docs map's row of the book's project,
  the project's front page (one sentence), and the comments of the two
  templates and both scripts.
- **Tests:** 2 in `book/tests.rs` (`extra.rust` read and passing; eight
  wrong paths and a page with no `files` or no `step` failing, each on its
  line); 6 in `book_pad/tests.rs` (a `POST` read with its origin and body,
  one over the limit unread, one cut short or with a bad length refused;
  a listed file read and others 404, the status's `rust`; a write, the
  same text again leaving the file's time alone, a write from the
  forwarded address; a reset on a fixture repository with git; twelve
  refusals — three foreign origins and none, four unlisted files, two
  `..`, a body over the limit, bytes that aren't UTF-8 — writing nothing,
  and another `POST` still 405); 1 in `llms/tests.rs` (the twin names the
  files; a step without names none).

**Checked locally** (Windows, at `1288616f` on `main` at `c0da2ec7`): the
site built at `http://127.0.0.1:4000/`, step 3 built, `book-pad
--no-build` serving it in the browser pane. Step 3's page showed
`rules.rs` and `board.rs` after the four data tabs, with their text; an
edit to `board.rs`'s player glyph in its tab, **Run**: the file written,
the frame reloaded after 8.8 s; a line that doesn't compile: cargo's error
in Problems 3.1 s after **Run**, the frame kept; **Reset this file**: the
file as `HEAD` has it (`git status` clean), the frame reloaded after
6.9 s; **Reset all** after a Rust and a data edit: both back, the frame
reloaded after 8.2 s. The same pages served by a plain static server (no
`/pad`, as the public site): the four data tabs only, no Rust editor in
the page, and the fold's line pointing to the Codespace.

**Measured** in a Codespace of the branch
(`urban-space-rotary-phone-r4g6gj9jj56hg7x`, 2 cores, `basicLinux32gb`,
no prebuild, made from the scratch branch `scratch-2841-rust-tabs` at
`1288616f`, the commit above; `main` hasn't moved since), on step 3's
page at the forwarded port, in the browser pane:

| What | Time |
| ---- | ---- |
| `gh codespace create` to *Available* | 10 min 41 s (21:47:35 to 21:58:16) |
| A one-line edit of `rules.rs`'s `is_wall` in its tab (walls no longer block), **Run** to *Building…* | 1.8 s |
| the same, **Run** to the frame reloaded with it, the first build since the server started | 31.7 s |
| a second one-line edit, **Run** to the frame reloaded | 8.0 s (*Building…* at 2.3 s) |
| a line that doesn't compile, **Run** to cargo's error in Problems, while rust-analyzer's first check held the target folder | 113.7 s |
| the same, quiet | 1.6 s |
| **Reset all**: `rules.rs` back as `HEAD` has it, the step rebuilt, the frame reloaded | 8.4 s |

Each failed build left the frame on the last good build (`play/`
unchanged, no reload), and `pad/build/03-walk` said so (`good` 2 while
`number` went to 5). The `POST`s came from the forwarded origin
(`https://…-4000.app.github.dev`), so Codespaces' port forwarding passes
`Origin` as the browser sends it.

- The pane was hidden throughout, so the game didn't draw: the
  frame's reload with each new build, the file on disk (`pad/file`) and
  the build's state were read, not the game played by hand. The change
  that plays is the build's, which `book-pad.js` reloads (2840 played
  that by hand).
- The server answered only once the folder was trusted: the Codespace
  opened in Restricted Mode, the port answered 502 for nearly four minutes after
  *Available*, and `/pad/status` answered within seconds of **Trust Folder &
  Continue**. 2840 saw the trust question when a terminal was first
  opened; a reader who never opens one may need the same click. No ticket:
  the trust prompt is VS Code's, about the reader's own Codespace, and the
  project's front page already sends the reader to the Ports tab when the
  preview isn't open.

**Deviations:**

- Step 10 has no `extra.rust`: its page has no table of files, and its
  only Rust changes are `app_info.rs`, which the page says not to edit
  by hand, and two anchor lines. Steps 2 to 9 do.
- Step 1 has none either: its table names the folder `crates/game/src/`,
  not a file (the ticket named steps 2 to 10).
- The Rust tabs' markup is rendered by Zola, hidden, and removed by the
  script outside the Codespace, rather than built by the script: the
  tab's id and label come from the template as the data files' do, and
  `check-build.sh` can read them (`data-rust`).
- `GET /pad/file/<step>/<path>` reads only a listed file too, not any
  file of the step: the reads and writes have one rule.
- A `POST` whose `Origin` is `http://localhost:4000` is accepted too, as
  `Host` is: the loopback's two names.
- A `POST` of the text the file already has writes nothing, so a
  **Reset all** of unchanged files builds nothing.
- 2838 isn't done: its clause now says what holds (**From 2841
  (done)**), with where its `POST /pad/check/…` goes in the server.

No ticket: nothing is left to do after this one.

Size: 1,005 counted lines, past the 1,000 planned for (code about 380,
tests 300, pages and templates 110, docs 120), not split. It would split cleanly between the server
(its routes, refusals and tests, with `extra.rust`) and the playground's
tabs, twin and docs, but neither does anything a reader sees alone.

Book: the project's front page says that in the Codespace the playground
edits the step's Rust too; each step's fold says so.

Game: nothing to adopt, the change is `xtask`, the site's pages and
scripts; no crate the game uses changed.

No break. No `Behaviour:` line: no crate the game uses changed.

**Nick input:** none needed.
