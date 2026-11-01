---
id: "2821"
title: "Code on the site that stands out: inline code with its own color and background, and code blocks with syntax colors of our own"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
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

- [ ] Inline code stands apart from the text around it, in light and dark, in a
      paragraph, a link, a table, a heading and a blockquote.
- [ ] A Rust block has separate colors for at least keywords, types, strings,
      comments and numbers, in light and dark.
- [ ] A RON block has colored comments, strings and numbers.
- [ ] A `try_it` diff shows its removed and added lines in different colors, told
      apart by more than color (its `-` and `+` stay).
- [ ] Every color pair meets 4.5:1, listed in the completion notes.
- [ ] `zola build` prints no highlight warning.
- [ ] No page has sideways scroll at 375 px, and `check-build.sh` passes.
- [ ] No theme file is added, and the grammar is our own.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:`
      line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Site: `check-build.sh` and `zola check`, and a new line in `check-build.sh` that
  fails if a built page has a Rust block with no `z-keyword`, so highlighting can't
  be turned off by accident.
- By hand in the browser, the criteria above, listed in the completion notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
