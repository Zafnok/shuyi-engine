---
id: "2835"
title: The playground's editors color .map and .dlg files too
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-13
---

# 2835 — The playground's editors color .map and .dlg files too

## Context

Ticket 2832 colored the playground's editors as the page's `ron` code
blocks are colored, with `site/static/ron-paint.js` following
`site/syntaxes/ron.sublime-syntax`. Only `.ron` editors are painted in
color: `site/templates/playground-macros.html` sets `data-lang="ron"` on
a file ending in `.ron` alone. A reader sees a level's `.map` file and
Hello, Grid's `intro.dlg` in one color, although:

- a `.map` file is RON, a `---` line, then rows, and the RON grammar
  already reads it (its `rows` context), so only the template leaves it
  out;
- `.dlg` (`shuyi_assets::dialogue`, ADR-0004) has no grammar at all,
  for the editors or for a `dlg` code block.

The playgrounds edit `.ron`, `.map` and `.dlg` files and nothing else
(`files = [...]` in `site/content/learn/`).

## Nick input

None.

## Scope

**In:**
- The template sets `data-lang="ron"` on `.ron` and `.map` files and
  `data-lang="dlg"` on `.dlg` files.
- A `site/syntaxes/dlg.sublime-syntax` grammar, so a `dlg` code block in
  the book is colored and the editors have a grammar to follow. It colors
  `# comments`, `@directives`, narration's `>`, an option's `*` and its
  tone, a speaker and their `[expression]`. The text a line says stays
  plain.
- `ron-paint.js` paints by either grammar, chosen by `data-lang`.
- `site/tests/ron-paint.mjs` checks both languages against Zola: the
  `.ron`, `.map` and `.dlg` sample files and cases of its own for each.

**Out (do not do):**
- Coloring a `.map` file's rows: they stay plain, as in a `ron` block.
- Checking that a `.dlg` file is valid: the grammar only colors.
- Renaming `ron-paint.js` (the CI step and 2828's notes name it).

## Implementation steps

1. `site/templates/playground-macros.html`: the `data-lang` by the
   file's ending.
2. `site/syntaxes/dlg.sublime-syntax`: one `main` context of line rules.
3. `site/static/ron-paint.js`: the grammars by language (`GRAMMARS`),
   `tokenize(text, lang)`, `paint` coloring any language it has, and
   `languages` in its API.
4. `site/tests/ron-paint.mjs`: a sample set per language, the Zola site
   built per language, the built site's `ron` and `dlg` blocks checked,
   and a failure if the painter has a language the test has no samples
   of.

## Acceptance criteria

- [x] On the playground page and in each book step, every editor has a
      `data-lang`: `ron` for `.ron` and `.map`, `dlg` for `.dlg`.
- [x] `intro.dlg` and `stage.map` editors are painted with colored spans.
- [x] `node site/tests/ron-paint.mjs <built site>` passes on every
      `.ron`, `.map` and `.dlg` sample, its cases, and every `ron` and
      `dlg` block of the built site.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: `site/tests/ron-paint.mjs`, against Zola's own coloring
  of the same text, for both languages.

## Completion notes

**What was done.** The template marks `.map` editors as RON and `.dlg`
editors as dialogue. A new `site/syntaxes/dlg.sublime-syntax` colors
`# comments` (`comment.line.dlg`), `@directives` (`keyword.control.dlg`),
narration's `>` and an option's `*` (`keyword.operator.dlg`), an option's
tone and a speaker (`entity.name.tag.dlg`), and a speaker's
`[expression]` (`constant.other.dlg`), each rule anchored to the line's
start but the expression's. `ron-paint.js` holds both grammars by
language and paints by `data-lang`. `ron-paint.mjs` now builds one Zola
site per language, samples `.map` files with the RON ones and `.dlg`
files from Hello, Grid, the tutorial and the skill pack, has four
dialogue cases (directions, expressions, choices with indented
reactions, continuation lines, comments with colons), and fails if the
painter colors a language it has no samples of. 156 blocks pass against
a full local build of the site. On the served playground page, all five
editors, `stage.map` and `intro.dlg` included, are painted.

**Deviations.** None. The work was done before the ticket was written,
at Nick's request in the session; the ticket records it.

Book: no chapter changes: the book doesn't describe the editors' colors.
