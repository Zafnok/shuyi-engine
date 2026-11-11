---
id: "2820"
title: "The tutorial's steps in the playground: each step's page edits that step's data files beside its game, and each before-and-after can be tried there"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2814", "2818", "3069"]
nick_input: none
completed: 2026-11-11
---

# 2820 — The tutorial's steps in the playground

## Context

Nick, 2026-11-01 ([`docs/decisions.md`](../../docs/decisions.md), *the
tutorial and the book teach from nothing*) asked that the playground wire
to the tutorial. 2814 builds the playground on Hello, Grid alone, and its
Out leaves "other examples" to a later ticket: this is that ticket for the
tutorial. 3069 gives a web build the files its page holds.

2818 gave each step's page tested before-and-after pictures, from
`site/tutorial/tries.toml`. Many of them edit a data file: a color in
`palette.ron`, a line of `en.ron`, a level's `.map`. Those a reader can
then try on the page, with no install.

This edits data files, not Rust. Hot reload (2905) doesn't change that: it
rebuilds on the developer's own machine, and a browser has no compiler.
Rust in the page is [2916](../open/2916-research-editing-a-games-rust-code-in-a-web-page.md)'s.

Nick, 2026-11-03 (*one book, read at the reader's level*): the tutorial
becomes the book's project (2824), its interactive parts come before
the engine goes public, and the book's exercises (2828) run in this
ticket's playground.

**From 2824 (done):** the steps' pages are the book's project,
`site/content/learn/book/box-pusher/`, rendered by
`site/templates/chapter.html`. Build the playground in its step block
(under `page.extra.step`, beside *Play this step*), and copy each step's
files to `/learn/book/box-pusher/<step>/`, where its `play/` is.

**From 2814 (done):** the playground is `site/templates/playground.html`
(the editors from the page's `extra.files`, the frame's address and the
files' folder, `data-files`, in the template) and `site/static/playground.js`
(it finds `.playground`, its `iframe`, its `textarea[data-file]` and its
buttons; no address is in the script). The script keeps a run file under
`shuyi-page-file:files/<name>` and the reader's typing under
`shuyi-playground-draft:files/<name>`. Every page of the site is one
origin, so every playground in a tab shares those keys: a step's game
would play Hello, Grid's edited `palette.ron`. Give each playground its
own keys (the game reads only `shuyi-page-file:files/…`, so a page clears
or swaps the others' before it runs), or say why sharing is right.
`copy-playground.sh` reads Hello, Grid's list from the page's front matter;
a step's copy reads its own.

## Nick input

None.

## Scope

**In:**
- A page's playground for each step that has data files worth changing,
  reusing 2814's template and `playground.js`, with the step's web build
  at `play/` in the frame: the step's `palette.ron`, `en.ron` and, from
  step 2, its levels, from `site/tutorial/<step>/crates/game/files/`.
- A step's web build gets 3069's file layer, as Hello, Grid's does.
- **An "Try this here" button on a `try_it`** whose edits are all in files
  under `files/`: it opens the page's playground with the edit made, so
  the reader sees the "after" live. A `try_it` that edits Rust keeps its
  picture and says it needs a rebuild on the reader's own machine.
- `site/README.md` and the step pages' wording on it.

**Out (do not do):**
- editing or compiling Rust in the browser (2916);
- changing 2814's Hello, Grid page;
- sharing edits by link, or saving them past the tab (2814's Out).

## Implementation steps

1. Read 2814's and 3069's completion notes, `site/templates/chapter.html`,
   the `try_it` shortcode and `tries.toml`.
2. Give the steps' web builds the file layer, and copy each step's files
   for the page as 2814's `copy-playground.sh` does.
3. Add the playground to the step template, and the button to `try_it`.
4. Try each step in the built-in browser.
5. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] On each step's page with data files, a reader changes a file, runs
      it, and sees the game change, with its problems shown when broken.
- [x] A `try_it` of a data file offers its edit in the playground, and
      running it gives what its "after" picture shows.
- [x] A `try_it` of Rust says it needs a rebuild, and offers no button.
- [x] The page works at 375 px with no sideways scroll, and loads nothing
      from another host.
- [x] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Site: the site's CI job checks each step's copied files equal its own.
- By hand in the browser, the criteria above, listed in the notes.

## Completion notes

What was done:

- **The steps' games read the page's files.** `shuyi_app::run_game` now
  lays the page's `files/` (3069's `page_layer`) over a short-path
  game's own when its data loads with them, and reports to the page
  either way (`lay_page`, with tests on the blank template's files: no
  files, files that load, broken files by file and line, files that
  can't be read). Every step is a `run_game` game, so no step's code
  changed, and every game made with `shuyi new` gets the same on the
  web. Off the web `page_layer` is `None`, so nothing changes there.
- **The playground is one macro**, `site/templates/playground-macros.html`
  (`script()`, `playground(game, frame, files_url, files, lazy)`),
  which Hello, Grid's `playground.html` and `chapter.html` both call.
  Hello, Grid's built page is byte-for-byte what it was.
- **Each step's page** (all ten: `extra.files` on each, `game` and
  `game_files` on the project's `_index.md`) has a playground folded in
  a `<details>` under *Play this step*, with a line saying what it is and
  that Rust needs a build on the reader's machine. Its frame is the
  step's `play/`, loaded only once the fold opens. Steps 1–5 hold
  `palette.ron`, `en.ron`, `keymap.ron` and, from step 2, `levels/1.map`;
  steps 6–10 also `levels.ron` and `levels/1.map`–`5.map`.
- **Each playground has its own keys** (the ticket's question):
  `playground.js` keeps a page's run files under
  `shuyi-playground-run:<page's path>:files/<name>` and its drafts under
  `shuyi-playground-draft:<page's path>:…`. As the script loads, in the
  page's head and before the frame, it empties the game's
  `shuyi-page-file:` keys and writes this page's run files there; on
  `pagehide` it empties them. So a game played on its own page (`play/`,
  `/examples/hello-grid/`) in the same tab plays its own files, and no
  playground sees another's.
- **`try_it`**: on its own step's page, a try whose edits are all of
  files that page's playground holds gets a hidden **Try this here**
  button with its edits as JSON. The script shows it once the files are
  fetched; it makes each edit to the game's own text of its file, opens
  the fold on the first file it edits, and runs. A try that edits a
  `.rs` file says it is Rust and needs `cargo run` on the reader's own
  machine, and gets no button, in the page and in its Markdown twin
  (`llms.rs`'s `TRY_RUST`, with a test). Ten tries got a button; nine
  say they need a build.
- **The copy and the checks**: `copy-playground.sh` copies Hello,
  Grid's files as before and each step's listed files to
  `site/static/learn/book/<project>/<step>/files/` (in `.gitignore`),
  beside its page and its `play/`. `check-build.sh` checks each step
  page with files loads `playground.js`, frames its `play/`, has an
  editor of each file, and that each copied file is `cmp`-equal to the
  step's own.
- Docs: `site/README.md` (*The playground* rewritten for both kinds,
  *What is where*, the rules on tries, previewing a step locally), the
  web shell's README and `docs/platform-backends.md` (`run_game` reads
  the page's files), and the book's *Assets and formats* chapter
  (*Layers*: `run_game` does it with no code, which is how the
  project's steps play in their pages).

Checked by hand in the built-in browser, on the site built locally with
steps 1, 2 and 6 and Hello, Grid built for the web and copied in, served
on `127.0.0.1:1111` (the pane was hidden, so frames were driven with
`wasm_exports.frame()`):

- step 2: the fold closed, the frame stayed blank (not loaded), and the
  editors held the step's four files. *Try this here* on `level-double`
  opened the fold on `levels/1.map`, ran, and Problems said "Loaded with
  your files."; after Enter the board's room was 14 × 10. On `box-blue`
  it opened on `palette.ron`, kept the reader's map, and the board's box
  was blue (28 pixels of `#5a9bff`, none of `#c89a5a`, read from the
  canvas). `"box": "blue"` and Run: Problems said
  `files/palette.ron:24: colour "box": "blue" must start with '#' (format "#RRGGBB")`.
  Reset all left no key;
- the keys: with step 2's palette run, `play/` on its own and Hello,
  Grid's playground saw no `shuyi-page-file:` key, and Hello, Grid's
  palette was its own; back on step 2, its key was back and its editor
  kept the edit;
- step 6: nine tabs; `levels-order` and `menu-words` ran and loaded;
- step 1 at 375 px: no sideways scroll (`scrollWidth` 375), the game
  above the files, the fold opened by hand loaded the game with none of
  step 6's files; two buttons shown, two Rust notes;
- Hello, Grid's playground: a broken `"hero"` showed its problem under
  its own keys, and Reset all left none.

`check-build.sh` passed (nothing from another host, every step's
files equal its own), and `cargo xtask llms` over the built site.

Deviations:

- The playground is folded on a step's page rather than open: the
  pages are long, and an open frame would load and run a whole game on
  every visit. A try's button opens it.
- The frame is `loading="lazy"`, and the script makes it eager when the
  fold opens: in a hidden pane a lazy frame never loads, and reloading a
  frame still blank cancels its load, so Run doesn't reload one that
  hasn't loaded.
- A try's button makes its edits to the game's own text of each file it
  edits, so its result is the after picture's; the reader's edits to
  other files stay, and are run with it.
- A try's file is named by its path after the first `files/`: a
  shortcode can't read the project's `game_files` (no `get_section`
  there).
- `keymap.ron` is in each step's playground too: `undo-key` edits it.
- The tries' pictures were not changed, so `tutorial-tries --check` was
  not run.

Book: the chapter *Assets and formats* (*Layers*) says `run_game` lays
the page's files with no code. No public API changed: `lay_page` is
private and `run_game` keeps its signature.

No break.

Follow-ups: none new. 2828 has a **From 2820 (done)** note (where its
check goes beside `lay_page`, the keys' pattern, the try button to
reuse), and 2830 one on how a second project's steps get a playground.

Nick input: none.
