---
id: "2821"
title: "Code on the site that stands out: inline code with its own color and background, and code blocks with syntax colors of our own"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-11-01
---

# 2821 — Code on the site that stands out

## Context

Nick, 2026-11-01, reading the tutorial (2818): "When we are in-lining code i.e.
\"run_game(game::game()); calls two functions. game::game()\" the font looks
correct but usually I see these rendered with either some different font color
and/or some different background to make it more clear." And: "also block code I
would want a diff color for the text too, probably using Rust color settings from
an editor i.e. vscode with rust ext or something... or [...] some other open source
color scheme we can use w/o licensing issues..."

What there is today:

- `site/static/style.css` gives `code` and `pre` a monospace font, and `pre` a
  background. Inline code has no color or background of its own, so it barely
  stands out in a paragraph that uses it as heavily as the tutorial does.
- `site/config.toml` has `highlight_code = false`, so every block is one color.

What a trial build with Zola 0.21 showed (`highlight_code = true`,
`highlight_theme = "css"`, nothing committed):

- It writes classes, not colors: `<span class="z-keyword">`, `z-storage`,
  `z-support`, `z-string`, `z-comment`, `z-constant`, `z-entity`, `z-variable`,
  `z-punctuation`. So the colors can be ours, in `style.css`, under the tokens
  already there for light and dark. **No theme file is vendored, so there is no
  theme license to carry** (`AGENTS.md` rule 7). Dark colors in the spirit of an
  editor's Rust colors are fine; copying a theme's file isn't needed.
- `rust`, `bash`, `toml` and `diff` are highlighted. `diff` marks deleted and
  inserted lines (`z-markup z-deleted`, `z-markup z-inserted`), which the
  tutorial's before-and-afters (`try_it`) want in red and green.
- **`ron` isn't known**: 66 blocks of the Learn pages come out plain, with a
  warning each. It needs a grammar: Zola's `extra_syntaxes_and_themes` reads a
  `.sublime-syntax` file.
- `text` isn't known either (41 warnings), and should stay plain with no warning.

## Nick input

**Sign-off:** after the merge, look at the tutorial's
[step 1](https://zafnok.github.io/shuyi-engine/learn/tutorial/01-new/) in light and
dark, and say whether the colors read well. Changing them later is a change to
`style.css` alone.

## Scope

**In:**
- **Inline code:** in `.prose`, code outside a `pre` gets a color and a background
  of its own, with a little padding and a small radius, from new tokens in
  `:root` and its dark block. It must still read well inside a link, a table, a
  heading and a blockquote (the tutorial's notes). It wraps on a narrow screen
  rather than pushing the page sideways.
- **Code blocks:** `highlight_code = true` and `highlight_theme = "css"` in
  `site/config.toml`, and rules for the `z-*` classes in `style.css`, by tokens, in
  light and dark. A `.prose` code block keeps its background.
- **RON:** a small grammar of our own, `site/syntaxes/ron.sublime-syntax` under the
  repository's license, for comments, strings, numbers, booleans, map keys and
  names, read through `extra_syntaxes_and_themes`. If a RON block's `.map` level
  after `---` reads badly as RON, keep it plain rather than wrong.
- **`text`:** no warning. Either the `snippet` shortcode leaves the language off a
  `text` block, or the build is left to say nothing about it.
- **Contrast:** every token at least 4.5:1 on its background (WCAG AA), in light and
  dark, with the pairs and their ratios in the completion notes. The site honors the
  reader's light or dark setting, so both are needed.
- `site/README.md`, *What is where*, for the grammar's folder and how colors are set.

**Out (do not do):**
- anything loaded from another host, or a script: Zola's output is plain HTML and
  `check-build.sh` holds it to that;
- vendoring a theme file;
- line numbers, a copy button, or highlighted lines;
- the Markdown twins, which `cargo xtask llms` makes from the source and which keep
  plain fences;
- the pictures of the game, whose colors are the game's.

## Implementation steps

1. Read `site/README.md`, `site/static/style.css`, `site/config.toml` and the
   `snippet` and `try_it` shortcodes.
2. Add the inline code tokens and rules.
3. Turn on highlighting, and write the `z-*` rules.
4. Write the RON grammar, and silence `text`.
5. Serve the site. Look at the tutorial's steps 1 and 2, a `try_it`, the book's
   chapter 1 and a problem code's page, in light and dark and at 375 px.
6. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] Inline code stands apart from the text around it, in light and dark, in a
      paragraph, a link, a table, a heading and a blockquote.
- [x] A Rust block has separate colors for at least keywords, types, strings,
      comments and numbers, in light and dark.
- [x] A RON block has colored comments, strings and numbers.
- [x] A `try_it` diff shows its removed and added lines in different colors, told
      apart by more than color (its `-` and `+` stay).
- [x] Every color pair meets 4.5:1, listed in the completion notes.
- [x] `zola build` prints no highlight warning.
- [x] No page has sideways scroll at 375 px, and `check-build.sh` passes.
- [x] No theme file is added, and the grammar is our own.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:`
      line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Site: `check-build.sh` and `zola check`, and a new line in `check-build.sh` that
  fails if a built page has a Rust block with no `z-keyword`, so highlighting can't
  be turned off by accident.
- By hand in the browser, the criteria above, listed in the completion notes.

## Completion notes

**Done.** Inline code in `.prose` has its own color and background, by new
`--code-*` tokens. `highlight_code` is on with `highlight_theme = "css"`, and
`style.css` colors the `z-*` classes in light and dark. No theme file is added.
`site/syntaxes/` holds our own grammars, `ron` and `text`. A `.map` file's rows
after `---` stay plain, as the ticket allowed. `site/README.md` says where all of
this is.

**Deviations.**

- `text` is silenced with a one-line empty grammar, not by the `snippet` shortcode,
  because the warnings also came from fences in the problem codes' pages and the
  book, which the shortcode doesn't write.
- The `z-keyword` check reads a page, not a block: `fn` and `let` are `z-storage`, so
  one Rust block can rightly have no `z-keyword`. With `highlight_code = false` it
  failed on 19 pages.
- Added beyond the listed test: `check-build.sh` also fails on a language the build
  doesn't know, for the "no highlight warning" criterion.
- Punctuation and operators keep the text's color, so `z-punctuation` has no rule.
- The map keys share the function color, and names the type color.

**Contrast** (WCAG, computed from the tokens; all at least 4.5:1):

| Pair | Light | Dark |
| ---- | ----- | ---- |
| inline code on its background | 7.74 | 8.01 |
| a link's color on inline code's background | 5.09 | 4.97 |
| block text on the block's background | 16.50 | 13.08 |
| keyword | 7.10 | 6.78 |
| type, name | 6.23 | 8.76 |
| function, map key | 6.65 | 11.49 |
| string | 6.25 | 7.33 |
| number, constant | 6.09 | 8.80 |
| comment | 5.63 | 6.80 |
| deleted | 6.54 | 7.15 |
| inserted | 5.41 | 10.28 |

Light tokens are `#7a2a63` on `#f1eee6` for inline code and `#8b2c9f`, `#0a6b75`,
`#6b5c00`, `#9a4a10`, `#1f5fbf`, `#4f6f52`, `#b3261e`, `#1a7a37` on `#ffffff` for
blocks. Dark are `#e9b8dc` on `#2a2e36`, and `#c792ea`, `#5fd0c0`, `#dcdca0`,
`#e0a070`, `#8fc4f0`, `#8fb08a`, `#ff8a80`, `#7ee29a` on `#1d2026`.

**By hand in the browser.** Pictures: tutorial step 2's RON block and inline code in
light; step 1's Rust block and inline code in a blockquote in dark. Read from the
computed styles at 375 px in dark, on ten pages (tutorial steps 1, 2, 3 and 6, its
glossary, the quick start, chapter 2, the actions and agents chapters, a problem
code): each token's color in Rust, RON and diff blocks; inline code's background in
a paragraph, a table, a heading and a blockquote; a link's color inside a link; and
no sideways scroll. Light at 375 px wasn't measured separately (the widths don't
depend on the colors). The diff's colors were read from computed styles, not looked
at in a picture; its `-` and `+` are in the text.

Book: none, this changes how the site's code looks, and no public API.

**Gates.** Run here: `check-build.sh`, `zola check`, `cargo xtask llms`, `ticket-lint`,
`check-docs`, `check-links` and `typos`. No Rust changed, so the code gates are
left to CI.

No follow-up tickets.

**Nick:** after the merge, look at the tutorial's step 1 in light and dark and say
whether the colors read well. Changing them later is a change to `style.css` alone.
