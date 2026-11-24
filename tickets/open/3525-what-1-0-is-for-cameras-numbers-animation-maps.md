---
id: "3525"
title: "Nick decides what 1.0 is for: world space, a numeric core, animation, map import, a real-time core, shaders, smooth 2D, positional sound, script-written rules and reference games"
type: decision
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 3525 — Nick decides what 1.0 is for

## Context

The blind-spot audit of 2026-11-23 looked at the engine as a stranger
choosing a 2D engine would. It found that the first release (2005)
would be a kit for grid and turn games, not a general 2D engine. Most
of the gap is work Nick parked on purpose:

- **2026-10-20** (`docs/decisions.md`, "a second comparison, with Unity
  and Unreal"): the engine is built for Visions of Shuyi first, and
  real-time work waits until the game launches, unless the game needs
  it.
- **2026-10-30** ("real demos"): he unparked one chain (snake, the maze
  chase and the A/B) and kept the rest of real time parked: fixed-point
  numbers, collision, cameras, the brick-breaker and the shooter.
- **2026-11-14** ("no run of breaking versions after the MVP"): parked
  work that would break games stays parked, and its ticket says whether
  it needs a breaking release when unparked.

The audit found that some of this parked work isn't really real time.
Turn games need it too, and some of it gets more expensive the longer
it waits: once 1.0 ships, adding it can break every game. These are
product choices: what the engine is for, and in what order. So each
one is asked here, not decided by a session.

The game asks for none of it. A code search of the game for
`engine#1120` and `engine#1119` finds nothing (the game pins engine
`50472bbc`).

Each "yes" in this ticket amends the 2026-10-20 rule for that piece
alone, as 2026-10-30 did for the demo chain.

## Nick input

**Decision:** eleven questions, asked with the `ask-nick` skill, one at
a time. Each says what exists today, the choices, and what each answer
starts. Questions 1 to 10 offer the same three answers unless they say
otherwise:

- **Before 1.0:** the work joins the first release. It is unparked,
  gets milestone `MVP`, and 2005 waits for it. The release comes later.
- **After 1.0:** it is unparked with milestone `After MVP`. It doesn't
  hold up the release.
- **No:** it stays parked under the 2026-10-20 rule.

1. **Cameras, scrolling worlds and draw order.**
   - Today everything is drawn in fixed screen positions, on two
     layers. There is no camera, no zoom between whole steps, no
     scrolling world and no "draw what is lower on screen in front".
     Every other 2D engine starts here. Large maps, isometric tactics
     maps and cutscene pans in turn games need it too.
   - Ticket 1120 plans it, and is parked as real-time work. Only part
     of it is real time: a camera that smoothly follows a moving player.
     The audit suggests splitting 1120: cameras, draw order and screen
     regions as general work, and smooth following left parked.
   - A "yes" splits 1120 that way and unparks the general part. It
     also unparks 3343 (skip what the camera can't see, and load big
     maps in pieces) when question 4 is a yes too.

2. **If cameras wait until after 1.0, should 1.0 still leave room for
   them?** (Asked only if question 1's answer is "after 1.0" or "no".)
   - Today the way a screen draws, and the way each platform shows the
     picture, assume one fixed grid of cells. 1120 says adding cameras
     later "would change the frame's API and every snapshot".
   - Yes: a small ticket, before the release, shapes the picture so
     that layers can be added later without breaking games. The release
     waits a little longer.
   - No: adding cameras later is a breaking release (a new major
     version), which the 2026-11-14 rule allows for parked work.

3. **One kind of number for positions.**
   - The engine bans fractions in game rules, so that replays and online
     play always agree. But it gives a game no exact fractional number,
     no position or rectangle type, no angle and no square root. Each
     game, and each module, would invent its own, and they wouldn't fit
     together. The one planned in 1119 can't measure a distance
     past about 2,900 pixels, less than one wide screen.
   - 2139 brings the pieces planned in 1119, 2701, 3101 and 2335 into
     one number type, with no drawing work. It doesn't make the engine
     real-time; the real-time features built on it stay parked.
   - A "yes" unparks 2139. "Before 1.0" means the type is fixed before
     people build on it.

4. **Sprite animation, animated tiles and particles.**
   - None exists. They are parked as real time, but each frame is
     worked out from the clip and the time alone, like the tweens that
     already shipped (2109). A turn game's idle units, water tiles and
     spell sparks need them.
   - The tickets: 2231 (animation clips), then 1123 (Aseprite and
     TexturePacker sheets into clips), 2359 (animated tiles and unit
     frames), then 2103 (particles). Animation state machines (2104)
     and skeletons (2105) stay parked either way.
   - A "yes" unparks 2231, 1123, 2359 and 2103, in that order. 2103
     also needs question 3's number type, or is reworked to do without
     it.

5. **Maps from Tiled and LDtk, as the level editor until our own.**
   - There is no way to build a level visually. The editor is parked
     (2506 to 2508), and the engine can't yet write its own map files.
     Tiled and LDtk are free map editors most 2D developers know.
   - 2274 reads their tile layers into the engine's grid maps, with
     objects at whole tiles, and Aseprite sheets into the sprite pages.
     It needs neither cameras nor the number type. The full import with
     free object positions (1124) stays parked.
   - A "yes" unparks 2274.

6. **A small real-time core.**
   - Snake and the maze chase run on fixed steps, but on a grid. There
     is no free movement, collision with walls, or camera that follows
     the player.
   - Yes: unpark free movement and collision (1122, and the queries in
     2124), with cameras (question 1), the number type (question 3) and
     animation (question 4), plus one small top-down example. Rollback
     for online play (2604) and the faster internet path (2605) ask the
     same question, and unpark with a "yes" here only if Nick says
     online real-time play is in it too.
   - No: the release says plainly that 1.0 is for turn-based and grid
     games, so nobody is misled.

7. **Game shaders and screen effects before the renderer is replaced.**
   - Games can't have their own shaders, screen effects (CRT, bloom,
     vignette) or render to a picture. All of it (2326, 2327, 2328,
     2331) waits for the new renderer (3325), which waits for the MVP
     and for the old sound library's removal.
   - Today's renderer can already draw them: 2337 does tints and blends
     on it now.
   - Yes: 3237 builds effects and one material on today's renderer, as
     files a game ships, so they move to the new renderer unchanged.
   - No: effects wait for the new renderer, and 3237 is closed.

8. **Pixel art only, or smooth 2D too?**
   - Today every picture is drawn sharp-edged, zoom is whole steps,
     rotation snaps to pixels, and fonts are drawn at one size. Hand-
     drawn, high-resolution and vector-style games would look wrong.
   - Answers: "pixel art only" (the release says so); "smooth 2D too,
     researched now" (3236 researches it, before or after 1.0); "smooth
     2D later" (it stays parked).
   - "Smooth 2D too" unparks 3236, a research ticket (a smooth 2D tier
     beside the pixel tier) whose report asks Nick again before
     anything is built. "Pixel art only" closes 3236 as not planned.
     "Smooth 2D later" leaves it parked.

9. **Sounds heard from their side of the screen.**
   - The mixer already sets each sound's left and right volume. What is
     missing is a sound placed in the world, louder and to the side it
     is on. 2363 plans it, and is parked as real time. It works over
     today's grid view, without question 1's cameras.
   - A "yes" unparks 2363.

10. **Gameplay written by people who don't write Rust.**
    - Scripts can add small pieces (abilities, events) to a game, but a
      game's rules are written in Rust (`crates/script/README.md`).
      A designer or modder who writes no Rust can't write a game.
    - Yes: 2146 lets a game's whole rules be a script. Script reload
      (2145) is already open, and visual scripting's order is asked in
      3526.
    - No: rules stay Rust-only, and 2146 is closed.

11. **Two small reference games with 1.0.**
    - Today there is one example game beside the demos in progress
      (snake, the maze chase). A stranger sees few kinds of game.
    - The audit suggests a brick-breaker (2738, which needs question
      6's free movement and collision) and a match-3 on the turn path
      (2736, which needs only the random-number helpers, 2114).
    - Answers: both, the match-3 alone (it needs no real-time work), or
      neither (they stay parked with the gallery of small games).

## Scope

**In:**
- Ask the eleven questions and record each answer in Nick's words in
  `docs/decisions.md` (the `ask-nick` skill), with the rules that
  follow from it.
- Carry out each answer on the tickets it names:
  - **unparked:** `status: todo`, `Parked:` dropped from the title, the
    milestone the answer gives, and `"3525"` dropped from `blocked_by`.
    "Before 1.0" adds the ticket to 2005's `blocked_by`;
  - **no:** the ticket stays parked, and its parked note names the
    answer. A parked ticket is never started (`AGENTS.md`, rule 3), even
    once this ticket is done. A ticket the answer makes unwanted (2146,
    3237, 3236) is closed as its note says;
  - **a split** (question 1's 1120): the general part becomes a new
    ticket (`write-ticket`) and 1120 keeps the rest.
- For question 2's "yes", write the ticket that shapes the frame for
  layers, with 2005 waiting for it.
- If question 6 is "no", a ticket for the release's line saying what
  1.0 is for (README and the book), or a line added to 2843 if it is
  still open.
- Update `docs/ROADMAP.md` for every ticket that changes status.

**Out (do not do):**
- The editor's questions (the toolkit, visual scripting's order,
  groundwork before the release, the accessibility tree and text size):
  3526.
- The order of visual scripting and whether it is built from
  turn-game pieces (1135, 2508): 3526 asks it. The graph does not wait
  for script-written rules (2146).
- Platform and service scope (signing, Steam services, phone browsers,
  online services, web HTTP, devices): 3527.
- The release's guards: 3528.
- Any technical choice inside the unparked tickets. Their sessions make
  those.
- Unparking networking as a whole. Question 6 touches only 2604 and
  2605, and only if Nick says so.

## Implementation steps

1. Read this ticket's Context and each named ticket's Context, so the
   evidence can be shown if Nick asks.
2. Ask with `ask-nick`, one question at a time, in the words above.
   Skip question 2 if question 1 is "before 1.0".
3. Record each answer in `docs/decisions.md`.
4. Edit the named tickets as the Scope says, and write the new ones.
5. Run `cargo xtask ticket-lint`.

## Acceptance criteria

- [ ] `docs/decisions.md` has an answer to each asked question in
      Nick's words, and the rules from it.
- [ ] Every ticket named in the answers has the status, title, milestone
      and `blocked_by` the answer gives, and 2005's `blocked_by` holds
      each "before 1.0" ticket.
- [ ] No ticket left parked by a "no" lost its `Parked:` prefix.
- [ ] `docs/ROADMAP.md` matches the tickets' new status.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (a decision). `cargo xtask ticket-lint` passes after the edits.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
