---
id: "2831"
title: "The site's pages get their top and bottom space back, and the book's actions look like buttons"
type: bug
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-10
---

# 2831 — The site's pages get their top and bottom space back, and the book's actions look like buttons

## Context

Nick, reading the book on the website (2026-11-10), found three faults:

1. The reader's filter bar ("Notes for: New to programming, New to making
   games …", ticket 2826) touches the site's menu bar (Learn, Playground,
   News …) with no space between them.
2. The pager at the end of a page (← previous, next →) touches the footer
   ("Shuyi Engine is free and open source …") with no space between them.
3. Controls that act on the page are drawn as links: a newcomer passage's
   "hide" and its stub's "show", the bar's "Show all" and "Start over", and
   the welcome popup's "Skip: show me the whole book". They are `<button>`s
   in the HTML, but underlined accent text with no box, so a reader takes
   them for links that go somewhere.

He asked that the fix follow the web's usual design rules.

The cause of 1 and 2 is in `site/static/style.css`, and it hits every page,
not only the book's: `main` sets `padding-top: 2rem` and `padding-bottom:
3rem`, but `<main>` also has the class `wrap`, and `.wrap { padding: 0 1rem }`
is the more specific rule, so its shorthand sets both to 0. The bar is the
first thing in `<main>` on a book page and the pager the last, so they meet
the header and the footer.

The site's design is ADR-0048 (our own templates, one stylesheet) and the
reader's filter ADR-0090 §6.

## Nick input

None.

## Scope

**In:**
- `.wrap` sets only its sides (`padding-inline`), so `main`'s top and bottom
  padding apply on every page.
- One small-button look in `style.css` for an action in running text or a bar
  (a border, a rounded box, the accent color, a hover and a focus ring, at
  least 24 × 24 px to click, WCAG 2.2 SC 2.5.8), used by the five controls
  above. Links stay links: an `<a>` that goes to a page keeps the link look,
  and a `<button>` never wears it.
- The stub of a hidden passage is one `<button>` already: its "show" is drawn
  as that button's visible label box, not as underlined text.
- `site/README.md`'s `style.css` row names the small-button look.

**Out (do not do):**
- No change to `book-filter.js`'s behaviour, the bar's toggles (already
  pill buttons with `aria-pressed`), the popup's answer buttons, the
  playground's buttons, or the `.button` links ("Play this step", the home
  page's calls to action), which are links that go to a page.
- No new colors: the button uses the existing tokens, so light and dark keep
  their contrast.
- No redesign of the header, the footer or the bar's layout.

## Implementation steps

1. In `site/static/style.css`, change `.wrap`'s `padding: 0 1rem` to
   `padding-inline: 1rem`. Check that `.nav` and `.footer-grid`, the other
   `.wrap`s, keep their own top and bottom padding.
2. Add a `.button-small` rule (and its `:hover`, `:focus-visible`), and give it
   to `.newcomer-hide`, `.book-bar-button` and `.book-welcome-skip button`
   (by class in `book-macros.html` and `book-filter.js`, or by the selectors in
   the stylesheet). Remove their `text-decoration: underline` and `padding: 0`
   / `border: 0` / `background: none`.
3. Draw `.newcomer-stub-show` as the same box (it is a span inside the stub's
   button, so the box is a look only, and the stub keeps its hover).
4. Update `site/README.md`.

## Acceptance criteria

- [x] On a book page, the bar's top is 2rem below the header's bottom, and
      the footer's top is 3rem below the pager's bottom (`main`'s padding),
      at 1024 px and 375 px wide.
- [x] Every other page (home, News, Examples, Community, Sponsor, Learn, the
      playground) gets the same space at its top and bottom.
- [x] "hide", "show", "Show all", "Start over" and "Skip: show me the whole
      book" are drawn as bordered boxes, not underlined, at least 24 px tall,
      with a visible focus ring, in light and dark.
- [x] No `<a>` on the site is drawn as the small button, and no `<button>`
      is drawn as a link.
- [x] `site/README.md` names the small-button look.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit / property: none; the site has no CSS tests.
- Integration: `bash site/check-build.sh` passes. The spacing and the buttons
  are checked in a browser against `zola serve`, light and dark, at 1024 px
  and 375 px, and the measures go in the completion notes.

## Completion notes

- `.wrap` sets `padding-inline: 1rem` instead of the `padding: 0 1rem`
  shorthand, so `main`'s own `padding-top: 2rem` and `padding-bottom: 3rem`
  apply on every page. `.nav` and `.footer-grid` set their own top and bottom
  and are unchanged.
- One small-button rule in `style.css`: a 1 px accent border, a 6 px radius,
  the accent text, filled with the accent on hover, `min-height: 24px`, no
  underline. It covers a passage's "hide", the stub's "show", the bar's Show
  all and Start over, and the popup's Skip. Their old link-look rules are
  gone; their existing `:focus-visible` rings stay.
- Measured against `zola serve` on *Actions, never keys*: at 1024 px and at
  375 px, the bar starts 32 px below the header and the footer 48 px below the
  pager (both were 0). "hide" and "show" are 24 px tall, Start over 25.8 px.
  Looked at in dark at 1024 px and light at 375 px; the home page and the
  playground were checked too.
- Deviation: step 2 named a `.button-small` class. The rule uses a selector
  list of the five controls instead, which the step allowed, so
  `book-macros.html` and `book-filter.js` are unchanged.
- `site/README.md`'s `style.css` row says that an action is a button and
  looks like one, and that a `.button` link goes to another page.
- No follow-up tickets.

Book: none, only the site's stylesheet changed; no chapter's text or code.
