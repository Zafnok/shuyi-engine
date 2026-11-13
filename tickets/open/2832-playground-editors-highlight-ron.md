---
id: "2832"
title: The playground's editors color RON as the page's code blocks do
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] On the playground page and a Box Pusher step's playground, `palette.ron` in the editor shows the same colors as a `ron` block on the page, in light and in dark (screenshots of both in the PR).
- [ ] Typing, pasting, undo, selecting, scrolling a long file sideways and down keep the colored text exactly under the typed text (no drift by the last line or the last column).
- [ ] **Reset this file**, **Reset all**, a reload with a kept edit and "Try this here" all leave the editor colored.
- [ ] `.map` and `.dlg` editors look as before: plain text, no layout change.
- [ ] With no JavaScript the page is as before (the editors stay hidden).
- [ ] A screen reader reads the textarea once, not the painted copy (`aria-hidden` on the `<pre>`).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

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

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
