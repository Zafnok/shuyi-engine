---
id: "2841"
title: "The step's Rust in the playground's tabs, in the book served from its Codespace: edit, Run, and the game rebuilt beside the page"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: in-progress
blocked_by: ["2840"]
nick_input: none
completed:
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

- [ ] In the book served from a Codespace, a step page's playground has a
      tab for each `extra.rust` file, and **Run** after an edit plays the
      rebuilt game in its frame; a compile error shows in Problems and
      the last good build keeps playing.
- [ ] **Reset this file** and **Reset all** put a Rust file back as
      `HEAD` has it, and the step is rebuilt.
- [ ] The server writes no file but a step's `extra.rust`, and refuses a
      `POST` from another origin; tests show both.
- [ ] On the public site the playground has no Rust tab, and its line
      points to the Codespace.
- [ ] `book.rs` fails a wrong `extra.rust`, and the twin names the files.
- [ ] The book's chapter is updated (2802): the project's front page.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (xtask): `extra.rust` checks; the routes: a read, a write, a reset
  (on a fixture repository with git), a path not listed, a `..`, a body
  over the limit and a foreign `Origin`, each refused.
- By hand: step 5, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
