---
id: "2814"
title: "The playground: Hello, Grid playing in a Learn page beside its data files, which a reader edits and runs again, with its problems shown"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["3069", "2808"]
nick_input: none
completed: 2026-11-09
---

# 2814 — The playground

## Context

Nick, 2026-10-28 ([`docs/decisions.md`](../../docs/decisions.md),
*pictures in the book, a playground, and a site that keeps up with the
tickets*): the website gets a playground. The decision says what it is:
a game playing in the page beside its data files, which a reader edits
and runs again with no install. It edits files, not Rust code:
compiling Rust in the browser needs a build server, which costs money.

What there is once the blockers are done:

- Hello, Grid's web build plays on the site at `/examples/hello-grid/`
  (`pages.yml` copies it there, ticket 2801).
- With 3069, that build lays the files its page holds in
  `sessionStorage` (`shuyi-page-file:<path>`) over its own, and sends
  its problems to its parent frame with `postMessage({ kind:
  "shuyi-report", text }, origin)`, an empty text when it loaded.
- Hello, Grid's text files (`examples/hello-grid/files/`) are each a
  concept of the book: `stage.map` (a map in a text format, chapter 3),
  `palette.ron` (the skin's colours, chapter 2), `keymap.ron` (actions
  and keys, chapter 4), `intro.dlg` (dialogue) and `en.ron` (the screen
  text).
- 2808 makes a Markdown twin of every Learn page and lists Learn in
  `llms.txt`, so a playground under Learn is there for agents too, with
  what it is and which files it edits.
- The site loads nothing from another host (`site/README.md`, *The
  rules*), and has no JS framework or editor library.

Nick, 2026-11-03 (*one book, read at the reader's level*): the book and
the tutorial fold into one book (2824), and its interactive parts come
before the engine goes public, so this ticket does too.

**From 3069 (done):** a key's `<path>` is the file's path in Hello,
Grid's project (`shuyi-page-file:files/stage.map`); Hello, Grid reads
the keys under `files/` and ignores the rest. Its problems are one a
line as `cargo shuyi check` prints them (`files/stage.map:11:6: 'X' is
not in the legend`). When the page holds no file under `files/` the
game reports nothing at all, not the empty text: a reset that clears
the keys clears the page's problems itself. A file over 256 KiB, or
over 1 MiB in all, is reported as one line naming it, and the game
plays its own files. Checked by hand in the built-in browser, with the
page in a same-origin frame: the parent got the message at its origin.

**From 2824 (done):** Learn's landing page (`site/content/learn/_index.md`)
has one way in, the book, with a list beside it of the API docs and the
problem codes; the book's front page is its intro and its contents
(`site/templates/book.html`). Link the playground from both, in that list
and under the contents' links to the problem codes and the API docs, and
from the top menu.

## Nick input

None.

## Scope

**In:**
- **The page**, `site/content/learn/playground.md`, at
  `/learn/playground/`, with `template = "playground.html"`. Its
  Markdown is the text above and below the playground: what it is, that
  it edits Hello, Grid's data and not its code (and why, in a line),
  and for each file, a line on what it is with a link to its chapter.
  The Learn landing page and the top menu (`site/templates/base.html`)
  link it, as "Playground".
- **The template** (`site/templates/playground.html`, over `learn.html`
  or `base.html`, whichever the Learn pages use):
  - beside the text, the game in an `<iframe>` of
    `/examples/hello-grid/` (through `get_url`), at the size its web
    build asks for, with a `title`;
  - a tab per file (a `role="tablist"` of buttons, the arrow keys
    moving between them), each showing a `<textarea>` in the monospace
    font, `spellcheck="false"`, with a `<label>` naming the file;
  - **Run**: writes each file whose text differs from the game's to
    `sessionStorage` under `shuyi-page-file:files/<name>`, removes the
    rest, and reloads the frame. **Reset this file** and **Reset all**
    put the game's text back and run;
  - **Problems**: a `<pre role="status">` under the editors. It shows
    each `shuyi-report` message whose `event.origin` is the page's own
    and whose `event.source` is the frame, or "Loaded with your files."
    for an empty one. It is cleared on each run;
  - the edits stay in `sessionStorage` while the tab is open, so a
    reload of the page keeps them;
  - under 900 px, the game above the editors; the whole page works at
    375 px with no sideways scroll;
  - the frame has focus when the reader clicks it, and keys typed in an
    editor never reach the game (they are in another document).
- **The script**, `site/static/playground.js`, plain JS with no build
  step and nothing from another host; the stylesheet's rules for the
  page in `site/static/style.css`, light and dark.
- **The game's text** reaches the page as the build copies it:
  `site/copy-playground.sh` copies the five files above from
  `examples/hello-grid/files/` to `site/static/playground/files/`, which
  goes in `.gitignore`. `pages.yml` runs it before `zola build`, and so
  does the CI job that builds the site. The script fetches them from
  there (same host).
- **Locally**: `site/README.md` says the playground needs
  `bash copy-playground.sh` and Hello, Grid's web build copied to
  `site/static/examples/hello-grid/` (or says what `zola serve` shows
  without it), and lists the page under *What is where*.
- **The twin** (2808): the page is a Learn page, so its twin and its
  `llms.txt` line come with no change, unless the page uses a shortcode
  or a front-matter key `cargo xtask llms` refuses. Then this PR adds
  what the tool needs, with its test.
- **The document skill** for the new page.

**Out (do not do):**
- editing or compiling Rust in the browser, or any build server;
- other examples or templates in the playground. One more is a later
  ticket, with its own tab set: 2820 does it for the tutorial's steps;
- sharing edits by link, saving them past the tab, or downloading them;
- a code editor library, syntax colours or completion;
- pictures or sounds edited in the page (3069 hands over text only);
- the web shell's own errors (a build that doesn't start): 3052.

## Implementation steps

1. Read 3069's completion notes and `crates/app/src/page.rs`,
   `site/templates/` (`base.html`, `learn.html`), `site/static/style.css`,
   `site/README.md`, `pages.yml`, the CI job that builds the site, and
   `crates/xtask/src/llms.rs`.
2. Write `site/copy-playground.sh` and run it in both workflows.
3. Write the page, the template, the script and the styles.
4. Build the site with Hello, Grid's web build copied in, serve it, and
   try it in the built-in browser: change the stage's map and run;
   break the palette and run; reset; reload the page; at 375 px.
5. Check `cargo xtask llms` on the built site, and the page's twin.
6. Run the `document` skill, then the gates. After the merge, try the
   live page.

## Acceptance criteria

- [x] `/learn/playground/` shows Hello, Grid playing beside five tabs,
      one per file, each holding the game's own text.
- [x] A changed `stage.map` and **Run** play the changed map, and the
      problems line says the files loaded.
- [x] A broken `palette.ron` and **Run** show its problem with its file
      and line, and the game still plays.
- [x] **Reset this file** and **Reset all** put the game's text back.
- [x] A reload of the page keeps the edits.
- [x] A message from another origin, or from a window other than the
      frame, is ignored (checked in the browser's console by posting
      one; say so in the completion notes).
- [x] At 375 px the page has no sideways scroll, and the tabs work with
      the keyboard.
- [x] The page loads nothing from another host (`check-build.sh`
      passes), and the top menu and Learn link it.
- [x] `llms.txt` lists the page, and its twin says what the playground
      is and which files it edits.
- [x] The book's chapter is updated (2802), or the completion notes say
      why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Site: in the site's CI job, a step checks that
  `public/playground/files/` holds the five files, each the same as
  Hello, Grid's (`cmp`), and that `/learn/playground/index.html` names
  `playground.js` and the frame's address.
- `llms.rs`: no new test unless the page needed a change there (then
  a fixture test of it).
- By hand in the browser, the acceptance criteria above, listed in the
  completion notes.

## Completion notes

What was done:

- `site/content/learn/playground.md` at `/learn/playground/`: the text
  above the playground (what it is; data, not code, and why) and below it
  (each file with its chapter; how Run, the storage and Reset work). Its
  `extra.files` lists the five files in their tabs' order.
- `site/templates/playground.html`: the page's text before
  `<!-- more -->` above the playground, the rest below; the frame of
  `/examples/hello-grid/` (the example page's permalink, so the trailing
  slash the build's relative files need is there) at 384 × 384, the web
  target's frame; a `tablist` of the files, a labelled monospace
  `textarea` each; Run, Reset this file, Reset all; Problems, a
  `<pre role="status">`. With no JavaScript the editors stay hidden and a
  `<noscript>` says the game plays its own files.
- `site/static/playground.js`: fetches the game's text from
  `/playground/files/`; arrow keys, Home and End move between the tabs;
  Run writes each changed file to `shuyi-page-file:files/<name>`, removes
  the rest and reloads the frame; Problems shows each `shuyi-report` from
  the frame at the page's origin, "Loaded with your files." for an empty
  one, and is cleared on each run.
- `site/copy-playground.sh` copies the files the page lists to
  `site/static/playground/files/` (in `.gitignore`); `pages.yml` and the
  `site` job run it before `zola build`, and `check-build.sh` runs it too
  and checks the build: the five files `cmp`-equal to Hello, Grid's, an
  editor of each, the page loading `playground.js` and framing
  `/examples/hello-grid/`.
- Links: the top menu ("Playground", after Learn; `nav_item` gained an
  `except` so Learn isn't also marked current there), Learn's landing
  page, the book's contents beside the problem codes and the API docs, and
  chapter 3's *Layers*, which describes such a page.
- Docs: `site/README.md` (*The playground*, *What is where*, the rules'
  two scripts, how to try it locally, the checks), the docs map's
  website row, and Hello, Grid's README.

Deviations:

- The reader's typing is kept under `shuyi-playground-draft:files/<name>`
  as it is typed, beside the run files, so a reload of the page keeps
  edits not yet run too; the game never reads those keys.
- The script isn't deferred: from the cache the game can report before a
  deferred script runs. Its listener is there first and keeps the last
  report for the page.
- The game's web shell focuses its canvas as it starts, which took the
  focus into the frame on a load and after Run. The script gives it back
  (to the page on its first load, to the pressed button after a run), so
  the frame has the focus only when the reader clicks it.
- `zola serve` can't put the game's build in the frame: the README says
  to build, copy it into `public/` as `pages.yml` does, and serve that.

Checked by hand in the built-in browser, on a build with Hello, Grid's web
build copied in, served on `127.0.0.1:1111` (the pane was hidden, so a
report came late, and once frames were driven by hand):

- five tabs, each the game's own text, beside the game playing;
- the stage's two top inner rows of walls cleared and Run: Problems said
  "Loaded with your files." and the stage played without them;
- `"hero": "blue"` in `palette.ron` and Run: Problems showed
  `files/palette.ron:24: colour "hero": "blue" must start with '#' (format "#RRGGBB")`,
  and the game played its title;
- Reset this file put the palette back and removed its key; Reset all
  put every file back and left no key;
- a reload of the page kept both edits and showed the palette's problem;
- in the console: a `shuyi-report` posted by the page itself, one from a
  sandboxed frame (an opaque origin), and one dispatched with the frame
  as its source but another origin were all ignored; the same with the
  frame and the page's origin was shown;
- at 375 px no sideways scroll (`scrollWidth` 375), the game above the
  editors; the tabs by the arrow keys, End and wrapping, and Tab into the
  editor; light and dark;
- `check-build.sh` passed (nothing from another host), and
  `cargo xtask llms` listed the page in `llms.txt` with its twin, with no
  change to `llms.rs`.

The live page is to be tried after the merge.

The book: chapter 3's *Layers* now links the playground. No public API
changed.

No break.

Follow-ups: none new. 2820 has a **From 2814 (done)** note: every
playground in a tab shares the page-file keys.

Nick input: none.
