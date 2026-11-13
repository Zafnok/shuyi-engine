---
id: "2832"
title: The playground's editors color RON as the page's code blocks do
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-12
---

# 2832 — The playground's editors color RON as the page's code blocks do

## Context

Nick's feedback on the book (2026-11-12): the playground folded at the top
of a step's page is good, but its editors show plain text, while every
`ron` code block on the same page is colored. The two should look alike.

Why they differ: the page's code blocks are colored at build time by Zola
(`site/config.toml`, `highlight_theme = "css"`, with our own
`site/syntaxes/ron.sublime-syntax`), which writes `<span class="z-…">`
around each token, and `site/static/style.css` colors those classes in
light and dark. The playground's editors (ticket 2814, 2820;
`site/templates/playground-macros.html`) are plain `<textarea>`s that
`site/static/playground.js` fills at run time, so nothing colors them.

The files the editors hold are `.ron` (`palette.ron`, `keymap.ron`,
`en.ron`, `levels.ron`), `.map` and `.dlg` (`extra.files` of the
playground page and the Box Pusher steps). The book shows `.map` and
`.dlg` text in `text` code blocks, which are plain (`text.sublime-syntax`).

## Nick input

None.

## Scope

**In:**
- A `.ron` file in a playground editor is colored token by token with the
  same classes, so the same colors, as a `ron` code block on the page, in
  light and dark, and stays colored as the reader types.
- `.map` and `.dlg` files stay plain, as their `text` blocks on the page are.
- Done with the overlay technique, in our own code: the `<textarea>` stays
  the thing the reader types in (selection, undo, paste, the keyboard,
  screen readers and `spellcheck="false"` keep working), with its text
  transparent and its caret visible, over an `aria-hidden` `<pre><code>`
  that holds the same text colored. No editor library: a hand-written
  tokenizer, a few dozen lines.
- The tokenizer follows `site/syntaxes/ron.sublime-syntax`'s rules and gives
  each token the classes Zola writes for that scope (the scope's dotted
  parts, each `z-` prefixed, e.g. `comment.line.ron` →
  `z-comment z-line z-ron`). Read the classes off a built page
  (`zola build`, a `ron` block) rather than guess.

**Out (do not do):**
- No editor library (CodeMirror, Monaco, Prism, highlight.js…): a new
  dependency for one grammar is not worth its weight or license check.
- No coloring of `.map` or `.dlg` (no grammar for them exists; a new
  grammar is its own ticket, for the page's blocks and the editors alike).
- Ticket 2828 (exercises in a step's playground) is not this ticket's job.
  It also edits `playground.js`; whichever lands second keeps the other's
  behavior, and an exercise that sets an editor's text must repaint it as
  step 4 below says.
- No change to what the editors hold, keep or send (2814, 2820).

## Implementation steps

1. In `site/templates/playground-macros.html`, wrap each `<textarea>` in a
   `<div class="playground-code">` with an `aria-hidden="true"`
   `<pre class="playground-paint"><code></code></pre>` before it, and mark
   the textarea's language (`data-lang="ron"` when the name ends in `.ron`,
   else none).
2. In `site/static/style.css`, stack the two in the same grid cell with the
   same font, size, line height, padding, border width, `white-space: pre`
   and `tab-size` (the textarea is `wrap="off"`); the textarea's
   `color: transparent`, `caret-color` the text color, background
   transparent; the `<pre>` painted below it. Selection must stay visible.
3. In `site/static/playground.js`, a `paint(area)` that tokenizes RON into
   spans with the Zola classes (escape `<`, `&`), writes them into the
   `<code>`, and adds a trailing newline so the last line's height matches.
   Plain files get their text escaped, uncolored.
4. Call `paint` on load (after the files arrive), on `input`, and wherever
   the script sets a textarea's value itself: **Reset this file**, **Reset
   all**, a kept edit restored from `sessionStorage`, and the "Try this
   here" button (2820). Mirror `scroll` (top and left) from the textarea to
   the `<pre>`.
5. The tokenizer, by the grammar: `//` line and `/* */` block comments,
   `"…"` strings with `\` escapes, `'c'` chars, numbers, `true`/`false`/
   `None`/`Some` and the grammar's other constants, `Name(` type names and
   `key:` field names as the grammar scopes them, and punctuation. Anything
   it doesn't know is left plain, never dropped.

## Acceptance criteria

- [x] On the playground page and a Box Pusher step's playground, `palette.ron` in the editor shows the same colors as a `ron` block on the page, in light and in dark (screenshots of both in the PR).
- [x] Typing, pasting, undo, selecting, scrolling a long file sideways and down keep the colored text exactly under the typed text (no drift by the last line or the last column).
- [x] **Reset this file**, **Reset all**, a reload with a kept edit and "Try this here" all leave the editor colored.
- [x] `.map` and `.dlg` editors look as before: plain text, no layout change.
- [x] With no JavaScript the page is as before (the editors stay hidden).
- [x] A screen reader reads the textarea once, not the painted copy (`aria-hidden` on the `<pre>`).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- A test of the tokenizer. No JavaScript runs in the site's CI today
  (2026-11-12: only `sound-parity.yml` uses Node), so put the tokenizer
  in its own `site/static/ron-paint.js` that `playground.js` loads, add
  `site/tests/ron-paint.mjs` (plain Node, no packages), and run it with
  `node` in the workflow job that builds the site (Node is on GitHub's
  runners). It checks that RON samples from
  `examples/hello-grid/files/*.ron` give the same class sequence as
  Zola's own output for that text, read from a built page.
- Manual, in the built-in browser on `zola serve`: the criteria above, with
  screenshots in the PR.

## Completion notes

**What was done.** Each playground editor is now a `<textarea>` over an
`aria-hidden` `<pre class="playground-paint"><code>` in one grid cell
(`site/templates/playground-macros.html`), with the same box, font, line
height, padding, border, `white-space: pre` and `tab-size`
(`site/static/style.css`). `site/static/ron-paint.js` cuts RON into runs
by `site/syntaxes/ron.sublime-syntax`'s rules (line by line, the earliest
match of the top context's rules winning, the first rule on a tie, as
syntect reads a grammar) and writes Zola's classes for each scope, nested
as Zola nests them. `playground.js` paints on load, on `input`, and
through one `set(area, text)` that **Reset this file**, **Reset all**, a
kept edit and "Try this here" all use, and mirrors the textarea's scroll.
The textarea's text and background turn transparent only once painted
(`.painted`), so an editor that never paints still shows its text.
`.map` and `.dlg` editors are painted too, escaped and uncolored.

`site/tests/ron-paint.mjs` (plain Node) builds a small Zola site of its
own with the website's grammars and settings, holding Hello, Grid's
`.ron` files, the book's project's and nine cases of its own (nested and
multi-line block comments, escapes, chars, every number form, key
strings, an unclosed string, a `.map`'s `---` rows), and checks that
every character gets the classes Zola gives it; given a built site it
checks each of its `ron` blocks too. CI's `site` job runs it after its
build (120 blocks pass locally). Breaking a rule on purpose (no escape
rule; `false` dropped) fails it, naming the line and column.

**Deviations.**
- `ron-paint.js` is loaded by the `script()` macro just before
  `playground.js`, not by `playground.js` itself: a second synchronous
  `<script>` is the plain way for one to find the other.
- Step 5 lists `None`/`Some` among the constants and names punctuation;
  the grammar scopes `None` and `Some` as `entity.name.type.ron` (any
  capitalized name) and has no punctuation scope but `---`. The tokenizer
  follows the grammar, as the Scope says, so the editors match the
  blocks exactly.
- `.map` and `.dlg` editors are the same size as before (340.6 × 569.1 px
  measured), but the 6.6 px under each editor that an inline textarea's
  line box left is gone, now that the textarea is a grid item: the
  buttons sit that much higher, for every file alike.
- The test builds its own Zola site rather than only reading the
  website's build: Hello, Grid's files aren't in any page's `ron` block
  whole.

**Manual check**, in the built-in browser (Chromium) on `zola serve`,
on this branch's tree over `main` at `c508197c` (the tip when checked):
- `palette.ron` colored as the page's `ron` block, light and dark, on
  `/learn/playground/` and on Box Pusher's step 1 (screenshots in the
  PR).
- Typing (real keys) and Ctrl+Z repaint; the painted text equals the
  textarea's after each. A real Ctrl+V didn't reach the textarea in the
  pane (no clipboard there), so paste was not tried by hand; it fires the
  same `input` event typing does.
- With the typed text shown in red over the painted copy: crisp single
  glyphs at the last line (tabs included) and the last column of a
  200-character line; the two scroll offsets equal to the subpixel at
  both ends.
- A selection shows its highlight over the colored text.
- A kept edit after a reload, Reset this file, Reset all and Try this
  here (step 1, `en.ron`) all leave the editor painted, and the try
  focuses it.
- No JavaScript: the served HTML still has `<div class="playground-editors" hidden>`
  around the editors, as before.
- Every painted `<pre>` has `aria-hidden="true"`. The pane's page tree is
  built from the DOM (it lists a `hidden` element too), so it cannot show
  what a screen reader skips; no screen reader was run.

Follow-ups: none. **From 2832 (done):** 2828's step 5 now says an exercise
sets an editor's text with `set(area, text)`.

Book: none, the editors look as the book's code blocks already do;
nothing a reader does or reads about changes. `site/README.md`'s *The
playground* says how the overlay and its test work.

No break: no crate changes. No `Behaviour:` line: the website only; a game
sees nothing. Nick input: none.
