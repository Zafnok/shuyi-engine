---
id: "2814"
title: "The playground: Hello, Grid playing in a Learn page beside its data files, which a reader edits and runs again, with its problems shown"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["3069", "2808"]
nick_input: none
completed:
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
  concept of the book: `stage.map` (a map in a text format, chapter 5),
  `palette.ron` (the skin's colours, chapter 4), `keymap.ron` (actions
  and keys, chapter 3), `intro.dlg` (dialogue) and `en.ron` (the screen
  text).
- 2808 makes a Markdown twin of every Learn page and lists Learn in
  `llms.txt`, so a playground under Learn is there for agents too, with
  what it is and which files it edits.
- The site loads nothing from another host (`site/README.md`, *The
  rules*), and has no JS framework or editor library.

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
  ticket, with its own tab set;
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

- [ ] `/learn/playground/` shows Hello, Grid playing beside five tabs,
      one per file, each holding the game's own text.
- [ ] A changed `stage.map` and **Run** play the changed map, and the
      problems line says the files loaded.
- [ ] A broken `palette.ron` and **Run** show its problem with its file
      and line, and the game still plays.
- [ ] **Reset this file** and **Reset all** put the game's text back.
- [ ] A reload of the page keeps the edits.
- [ ] A message from another origin, or from a window other than the
      frame, is ignored (checked in the browser's console by posting
      one; say so in the completion notes).
- [ ] At 375 px the page has no sideways scroll, and the tabs work with
      the keyboard.
- [ ] The page loads nothing from another host (`check-build.sh`
      passes), and the top menu and Learn link it.
- [ ] `llms.txt` lists the page, and its twin says what the playground
      is and which files it edits.
- [ ] The book's chapter is updated (2802), or the completion notes say
      why not.
- [ ] All gates in the `run-gates` skill pass.

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

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
