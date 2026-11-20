---
id: "3505"
title: The web page takes the keys inside a site's frame without a click, after the site shows it and after the site's Fullscreen button
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-19
---

# 3505 — The web page takes the keys inside a site's frame

## Context

1. **The game ticket:** `game#0933`
   ([`tickets/open/0933-the-web-game-takes-the-keys-without-a-click.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0933-the-web-game-takes-the-keys-without-a-click.md)).
   Its owner played its web build on itch.io and had to click the game
   before the keys worked, after the site's Run button and again after the
   site's Fullscreen button.
2. **The job:** a keyboard game in a site's frame gets the keys without
   the player clicking it first.
3. **Friction numbers:** no friction review: a bug.
4. **The generic shape:** the web shell (`crates/cli/web/index.html`)
   focuses its canvas once, when its script runs. The keys go to the frame
   that has focus, and a site that frames the page keeps focus on its own
   page:
   - when it shows the frame only after the page has loaded (it is
     `display: none` meanwhile), the canvas has no box to take focus with
     when the script runs, and focus stays on the site's Run button;
   - its Fullscreen button fullscreens the frame from outside, and focus
     stays on that button;
   - the bundle reads the keys on the canvas, so a frame that has focus on
     its `<body>` loses them too.
5. **A game that isn't tactics:** any keyboard game on itch.io, Newgrounds
   or a game portal that frames it: a platformer, a puzzle game, a typing
   game.

## Nick input

None. Nick asked for the game's fix (2026-11-20).

## Scope

**In:**
- `index.html` takes focus (the window, then the canvas) when its script
  runs, at `load`, when the frame is first given a size after it had no
  focus yet (shown late), and when it comes to fill the screen without
  focus (a fullscreen from outside). Whenever the page has focus, the
  canvas has it too.
- It never takes focus on any other resize, so a text box on the site
  keeps it while the player types and the browser window changes size.
- The shell's `README.md` says so; a test in `web.rs` holds the pieces.

**Out:**
- Mouse support, a "click to play" sign, a pause sign while unfocused.
- The API: unchanged (a page of the shell only). The game's PR that moves
  the pin follows this one.

## Implementation steps

1. Reproduce in a real browser: a page on one origin frames the build on
   another (as itch.io does), with a Run button that adds the frame
   (optionally shown 4 s later) and a Fullscreen button that calls
   `requestFullscreen()` on the frame or its container.
2. The listeners in `index.html`.
3. The test and the README.

## Acceptance criteria

- [x] In Chrome, a build framed cross-origin: after Run, keys reach the
      game without a click, with the frame shown at once or 4 s later.
- [x] After the parent fullscreens the frame (or its container), keys
      reach the game without a click.
- [x] A resize that isn't a fullscreen never takes focus from the parent.
- [x] Edge and Firefox recorded in the Completion notes.
- [x] The gates pass.

## Tests required

- `web.rs`: `index.html` takes focus at load, on a resize to the full
  screen or the first size, and puts it on the canvas whenever the window
  gets it. The browser results in the Completion notes (no browser in CI).

## Completion notes

- `index.html` takes focus (window, then canvas) when its script runs, at
  `load`, on the first resize before it ever had focus (a frame shown
  late), and on a resize that fills the screen without focus (a fullscreen
  from outside), one animation frame later so the canvas has its new box.
  Whenever the window has focus the canvas takes it. Any other resize
  leaves focus where it is.
- `check.py frame` (CI's `web-render`): a site on `localhost` frames Hello,
  Grid's page on `127.0.0.1` (shown at once, shown 1.5 s late,
  fullscreened) and presses a key with no click; plus a copy without the
  fullscreen focus, which must fail. Headless Chromium gives its pages
  focus, so it can't fail the late-shown case; that one was checked by
  hand. A string test in `web.rs` holds the pieces.
- Checked by hand on Windows 11 with the game's web build (`game#0933`),
  framed cross-origin by a test page whose Fullscreen button calls
  `requestFullscreen()`, keys sent over the DevTools protocol to a real
  desktop window:

  | Browser | Shell | Run, shown at once | Run, shown 4 s late | Fullscreen (frame) | Fullscreen (container) |
  | --- | --- | --- | --- | --- | --- |
  | Chrome | before | keys reach the game | keys go to the site | keys go to the site | keys go to the site |
  | Chrome | after | keys reach the game | keys reach the game | keys reach the game | keys reach the game |
  | Edge | before | keys reach the game | keys go to the site | not checked | not checked |
  | Edge | after | keys reach the game | keys reach the game | not checked | not checked |
  | Headless Chromium 156 | before / after | pass / pass | pass / pass | **fail** / pass | — |

  Edge's window wouldn't go fullscreen under the DevTools protocol, so its
  fullscreen cells are unchecked (same engine as Chrome). Firefox isn't
  installed on the machine and Playwright's Firefox wouldn't start there:
  unchecked.
- No API change.

Game: game#0933 moves the pin and adopts it.
