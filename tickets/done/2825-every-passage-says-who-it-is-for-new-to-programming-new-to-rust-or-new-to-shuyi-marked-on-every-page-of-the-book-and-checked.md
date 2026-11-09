---
id: "2825"
title: "Every passage says who it is for: new to programming, to making games, to Rust or to Shuyi, marked on every page of the book and checked in CI"
type: feature
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2823", "2824"]
nick_input: none
completed: 2026-11-09
---

# 2825 — Every passage says who it is for

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): the book filters by "familiarity with
general programming, familiarity with Rust, familiarity with Shuyi", so
that a reader new to programming, a reader new to Rust and an experienced
developer who wants "to see how Shuyi Engine differs" each read it at their
level. Before a page can be filtered (2826), each passage must say who it
is for.

**From 2823 (done):** [ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md) §4 and §5 decide the mark:

- **The audiences:** exactly four, an unmarked passage being for
  everyone:
  - `programming` ("New to programming?"): what a program, a terminal,
    a file or a variable is;
  - `games` ("New to making games?"): how games are made whatever the
    engine, such as the game loop and frames, state, sprites, data kept
    apart from code, and ECS. Nick added it on 2026-11-08
    (`docs/decisions.md`, *the book's project folds into its chapters,
    and a fourth kind of reader*);
  - `rust` ("New to Rust?");
  - `shuyi` ("New to Shuyi?").
- **A `games` passage says how games are made, not how Shuyi makes
  them.** "A game draws a frame many times a second" is `games`; "a
  screen draws into the glyph buffer" is `shuyi` or unmarked.
- **A `shuyi` mark is narrow.** A passage about the engine that a reader
  who knows Shuyi still needs stays unmarked: a line to type, a rule
  the engine checks, a gotcha, or what an operation does that the page
  relies on. Only an introduction to an idea a veteran already has is
  `shuyi`.
- **The shortcode:** `` {% newcomer(to="rust", topic="`match`") %} `` …
  `{% end %}`.
  - `topic` is required: a few words, which name the stub 2826 leaves.
  - A mark holds whole blocks only (paragraphs, lists, code,
    `snippet`s, `try_it`s, headings), on its own lines.
  - Marks don't nest.
- **Its HTML:** `<aside class="newcomer" data-newcomer="…">`, then a
  `newcomer-label` paragraph with the label and the topic. The topic goes
  through `markdown(inline=true)`.
- **A page that is all one audience:** `[extra] newcomer = "…"`. 2824 set
  it on *Before you start*, and this ticket makes its top line say so.
- **The twin:** a blockquote. Its first line is `` > **New to Rust?**
  `match` ``, and the passage follows with `> ` in front of each line.
- **`llms-full.txt`:**
  - it leaves out the `programming`, `games` and `rust` passages, and
    the pages marked whole for any of them;
  - it keeps the `shuyi` passages;
  - it still leaves out the project's pages;
  - its first lines say what it leaves out.

The ADR's trial ran the body shortcode around a `snippet` on Zola 0.21.
2824 put every page in the one book.

The pages mark some passages already, by hand, as blockquotes:
`> **Rust:**`, `> **Shuyi:**` and `> **Game idea:**` (104 on
2026-11-08: 75, 21 and 8, most of them in the project's steps). They
become `rust`, `shuyi` and `games`. Many more explain a word in a sentence of
an ordinary paragraph ("A **crate** is Rust's word for one package of
code"), and those are unmarked.

## Nick input

None.

## Scope

**In:**
- **The mark:** the ADR's shortcode, `site/templates/shortcodes/newcomer.md`,
  and its look in `site/static/style.css`: a label naming the audience
  and the topic, and a rule at the passage's side, light and dark,
  readable at 375 px. The page-level key also goes in `chapter.html`'s
  top line. With no JavaScript every passage shows, labelled.
- **The twins:** its expansion in `crates/xtask/src/llms.rs` (a shortcode
  added to Learn needs one, `site/README.md`, *The docs for agents*), and
  `llms-full.txt` as the ADR says.
- **The check:** a page that uses a mark with an audience not among the
  ADR's four, an unclosed mark, a mark inside a mark, or a mark with no
  `topic`, fails the site's build checks (`check-build.sh`, or a `cargo xtask`
  command if it reads the Markdown better). So does a hand-written callout
  left in the old form (`> **Rust:**`).
- **Every page marked:** each existing callout becomes the ADR's mark, and
  every passage that only a newcomer needs is marked: the sentence that
  explains a word of Rust, the paragraph that says what a frame is, the
  part of the quick start on what a terminal is. A passage that both
  teaches a newcomer and says something everyone needs is split, so the
  part everyone needs stays unmarked. Read each page with each audience
  hidden in mind: what is left must still read as whole sentences, and
  the code it shows must still be explained for that reader.
- **The glossary:** each word names its audience.
- ***Before you start*'s legend** of the three kinds of note becomes a
  few lines on the four marks: their labels, and that the bar folds them
  (2826).
- **The rules** in `site/README.md`: how to mark a passage, what to leave
  unmarked, and that a new passage that teaches a newcomer is marked
  (the `document` skill).

**Out (do not do):**
- hiding anything: the filter is 2826's, and this ticket's pages show
  every passage;
- new teaching beyond the splits and the joins the marks need;
- the version marks (2829);
- quick checks and exercises (2827, 2828).

## Implementation steps

1. Read ADR-0090, 2824's completion notes, `site/README.md`, the
   shortcodes and `llms.rs`'s shortcode expansions.
2. Write the shortcode, its styles, its expansion in `llms.rs` and the
   check, with their tests.
3. Mark the pages in the book's order: *Before you start*, then each
   chapter with the steps after it, then the appendices. For each page,
   read it once with each audience left out, and fix what no longer
   reads.
4. Build the site and look at three pages in the built-in browser, light
   and dark, at 375 px.
5. Run `cargo xtask llms`, read a twin with marks, then the `document`
   skill and the gates.

## Acceptance criteria

- [x] Every page of the book uses only the ADR's marks; no callout of the
      old form is left, and the check fails one put back.
- [x] A page with a mark of an unknown audience, or an unclosed mark, fails
      the build checks.
- [x] Each marked passage shows its label with no JavaScript, in light and
      dark.
- [x] A twin shows each marked passage with its audience, and
      `llms-full.txt` is as the ADR says.
- [x] For three pages (a chapter, a step and an appendix), the completion
      notes quote what a Rust programmer from another engine would read
      with the `programming`, `games` and `rust` passages left out, and it
      reads as whole sentences.
- [x] The book's chapter is updated (2802): this ticket is that change.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`llms.rs`): the mark's expansion in a twin, and in
  `llms-full.txt`: a `programming` or `rust` passage left out, and a
  `shuyi` one kept.
- The check: a fixture page for each failure above, and the site passing.
- By hand: step 4, and step 3's read-throughs, in the notes.

## Completion notes

**What was done.**

- **The mark:** `site/templates/shortcodes/newcomer.md` writes ADR-0090
  §5's `<aside class="newcomer" data-newcomer="…">` with its
  `newcomer-label` paragraph. The topic goes through
  `markdown(inline=true)`. The template throws on an unknown `to` or an
  empty `topic`, so `zola build` fails on them too.
- **Its look:** `style.css` gives the passage a 3 px rule at its side and
  a label in a new `--newcomer` colour, light and dark. Its contrast on
  the page's background is about 5.3:1 light and 8.6:1 dark.
- **A page marked whole:** `book-macros.html`'s `where` adds "· For
  readers new to programming" (and the other three audiences) to the
  top line, and throws on an unknown audience.
- **The check:** `cargo xtask check-book`, a new module
  (`crates/xtask/src/book.rs`), runs in CI's `site` job before
  `check-build.sh`. It reads every Markdown file under
  `site/content/learn/`. It fails:
  - an audience not among the four, a missing or empty `topic`, or an
    argument it doesn't know;
  - an unclosed mark, a mark inside a mark, or an `{% end %}` with no
    mark open;
  - a mark not on lines of its own at column 0, or with no blank line
    before or after it;
  - a page-level `newcomer` that isn't an audience;
  - a blockquote callout of the old form (`> **Rust:**`, `> **Shuyi:**`,
    `> **Game idea:**`).

  It reads the Markdown, so it is an `xtask` command rather than a
  `check-build.sh` grep. It reuses `llms.rs`'s shortcode parser (`call`,
  `arguments`).
- **The twins:** `llms.rs`'s `twin` splits a page at its marks with
  `book::parts`. Each passage becomes a blockquote,
  `` > **New to Rust?** `match` ``, with its snippets expanded. A page
  marked whole has `> **New to programming?** This whole page.` under its
  `Source:` line. `llms-full.txt` makes each page's twin again without
  the `programming`, `games` and `rust` passages, and keeps the `shuyi`
  ones. Its first lines, and `llms.txt`'s, say so. On this branch it is
  219 043 bytes, down from 237 028.
- **Every page marked:** 232 marks (163 `rust`, 25 `games`, 24
  `programming`, 20 `shuyi`) over 22 pages.
  - All 104 old callouts are gone. A callout about several things became
    one mark per thing.
  - A callout, or part of one, that a reader who knows the audience
    still needs became unmarked text. Examples: what `Dir`, `ActionId`,
    `ScreenResult::take` or `SaveHeader` do; the key check's `key_files`
    rule; the id that must never change; the `// check-text` comment the
    text check reads.
  - Definitions inside ordinary paragraphs were split into marked
    paragraphs of their own (a crate, a frame, a commit, CI, a hash, a
    bool, a panic, the state), with the smallest joins.
  - The steps' *What you learned* lists became an "In Rust: …" and an
    "In making games: …" paragraph, each marked, and an unmarked "In
    Shuyi: …", as step 10 has.
  - *Before you start* (marked whole for `programming`) also marks its
    game loop `games` and its glyphs `shuyi`. Its first lines now send a
    programmer to the quick start, which installs what they need (ADR-0090
    §5).
- ***Before you start*'s legend** names the four labels, says this page
  is one, and says the bar folds the notes a reader doesn't need.
- **The glossary** has a *New to* column on every row: *Making games*,
  *Shuyi* or *Rust* by its table, and *Programming* for the general words
  (compiler, hash, property test, golden table, headless, gate, float,
  semver, linker).
- **Docs:**
  - `site/README.md` has a new *Marking a passage*, the rule in *The
    rules*, and the twins and the check in *The docs for agents* and
    *The checks*;
  - the docs map's row for the book says where the marks live, and that
    a new passage that teaches a newcomer is marked;
  - `cargo xtask`'s usage text lists `check-book`.

**Read-throughs.** Each page was read once with each audience left out
(a script that drops the marked blocks). Below is what a Rust programmer
from another engine reads, with the `programming`, `games` and `rust`
passages left out.

- ***Rules as commands*** (chapter 5), its start:

  > Most of what's hard about a game comes later: undo, saving a game in
  > progress, a replay of how someone got that bug, a bot that plays it,
  > a test that doesn't need a screen, online play. […] You get all of
  > them from one rule, if you keep it from the start.
  >
  > A game's rules are a **state**, and that state changes **only
  > through commands**. A **command** is plain data saying what a player
  > asks for: "plant a seed here". […]
  >
  > The chain is short, and worth learning by heart. A key press becomes
  > an **action** […]

  Left out between those: "The **state** is everything that's true right
  now and could change." Further down, *Determinism and the hash* goes
  straight from its question to `shuyi_core::state_hash`, without the
  sentence on what a hash is. The test's `left == right` sentence is
  left out too, and `ActionId(2)`'s meaning stays.
- **Step 3, *Walking***, at *The rules*:

  > Now the rules themselves. `TurnRules` is the engine's trait for a
  > game that moves in turns. […] It's short, and there's a lot to see
  > in it.
  >
  > `Outcome` is how a game ends. For now, nothing ends it, so it's `()`.
  >
  > **`check`** answers "would this command be allowed?", without
  > changing anything. […] `self.player.step(dir)` is the engine's way
  > of saying "one tile that way from here".
  >
  > **`apply`** does it. It calls `check` first, and that `?` is doing a
  > job here: if `check` refuses, `apply` returns the refusal at once,
  > before anything has changed. […]

  The `type` lines, `Ok(())`, `&mut self`, the iterator chain, `*` and
  `is_none_or` are left out. What each engine call does stays, and so
  does "Stepping off the map counts as a wall." The *What you learned*
  section keeps only its "In Shuyi: …" line.
- ***Fast compiles*** (appendix B), its start:

  > You change one line, you want to see it, and you wait.
  >
  > A rebuild is the wait you'll meet a hundred times a day, so it's the
  > one that matters. A game made with `shuyi new` rebuilds in about a
  > second, on the stable Rust it pins: […]
  >
  > A **clean build** compiles everything, the engine included, as your
  > first build does.
  >
  > Linux and Windows (MSVC) are GitHub's 4-core runners; […]

  Left out: what a rebuild and linking are, `cargo check`, Windows' two
  toolchains, and what nightly is.

**By hand in the built-in browser** (`zola serve`, 375 px wide):

- step 3 in light, its `shuyi` mark around a whole section, heading
  included;
- *Testing* in dark, a `games` mark beside a code block;
- *Fast compiles* (appendix B) in dark;
- *Before you start* in light, its top line "The book · For readers new
  to programming".

Each label showed with no JavaScript, and no page scrolled sideways.

**Twins:** `cargo xtask llms` passed over a build of this branch. In
`rules-as-commands/index.md`, the `shuyi` opening and the `games` line on
the state are blockquotes under their labels. `llms-full.txt` holds 6
`New to Shuyi?` blockquotes and none for the other three.

**Tests:**

- `book.rs`: each failure has a fixture page (an unknown audience, no
  `topic` or no `to`, an unknown argument, unclosed, nested, a stray
  `{% end %}`, not on its own line, an indent, no blank line around it,
  an old callout, a page-level `newcomer` that isn't an audience);
- also the split, and an escaped mark read as text;
- and the site's own pages passing.
- `llms.rs`: a twin with all four marks as blockquotes and a snippet
  inside one; `llms-full.txt` keeping only the `shuyi` passage; a
  whole-page label; an unclosed mark failing the twin.
- The two `llms` snapshots changed by the new header sentences.

**Gates run:**

- `cargo fmt --all --check`;
- `cargo clippy -p xtask --all-targets -- -D warnings`;
- `cargo test -p xtask`;
- `cargo xtask check-book`, `check-docs`, `check-links`, `check-pins`,
  `ticket-lint` and `changelog`;
- `typos`;
- `zola check`;
- `site/check-build.sh`;
- `cargo xtask llms`;
- the engine's own `shuyi lint`.

Only `xtask` changed among the crates, so the workspace-wide gates are
CI's.

**Deviations:**

- The check is `cargo xtask check-book` in CI's `site` job, not
  `check-build.sh`, which the ticket allowed.
- *Before you start*'s first lines changed. They said a programmer could
  skip to its *Install the tools*. ADR-0090 §5 has every reader but a
  newcomer install in the quick start, so they now send a programmer
  there.
- The *What you learned* lists were rewritten as paragraphs, since a mark
  can't hold part of a list.
- Short glosses inside a list item or a table cell, which a mark can't
  hold, stay unmarked. Examples: "floats" in chapter 5's contract list,
  `PATH` in the quick start's table, "a **plugin**, something you add…".

**Handed over:**

- 2826 now has a "From 2825 (done)" note: the passage's HTML, the label
  and topic its stub repeats, the page-level key, and the `--newcomer`
  colour.
- 2827 and 2829 now say that `check-book` takes every `{% end %}` as a
  mark's close, so a `check` or a `since` with a body extends
  `book.rs`'s `kind()` and `parts()`, and `llms.rs`'s twin.

**Follow-up tickets:** none.

No break: no public API, format or golden value changed.

Book: this ticket is the book's change.
