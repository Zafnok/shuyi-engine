---
id: "3338"
title: Each target says how far its support goes, and a Supported target has run on its device
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3338 — Each target says how far its support goes, and a Supported target has run on its device

## Context

ADR-0008 §1 says a profile's `status` "is `supported` once the target's
example runs on the device, and `planned` before that", and
`shuyi_assets::target::Status::Supported`'s doc says the same ("The
target's example runs on the device", `crates/assets/src/target.rs:87`).
The profiles break that rule:

- `targets/steam-deck.ron:8` and `targets/playdate.ron:5` are
  `status: Supported`.
- Neither has run on its device. `docs/targets/steam-deck.md` says "Not
  yet measured on a device. Ticket 3013 measures it on the device", and
  `docs/targets/playdate.md` says "Not yet measured on a device: ticket
  3011 measures it". Both device tickets are open.
- Ticket 3001 (done) set every profile but `switch`, `android` and `ios`
  to `Supported` before any device run. 3011 and 3013 each say to keep it
  so, because a demotion "would drop" the target from 3008's bench gate:
  `shuyi bench` gates only `Supported` profiles
  (`crates/cli/src/bench.rs:341`, ADR-0081 §5).
- The Steam Deck has one more gap. Deck Verified needs Steam's on-screen
  keyboard, and that needs Steamworks, which is the game's ticket 0903
  (3012's Context). 3037, the engine's Steamworks feature, is parked. So
  `Supported` there cannot mean "a game passes Deck Verified with the
  engine alone".

Nothing says what `Supported` promises: a build in CI, a launch in CI,
a run on the device, measured budgets, or a store's certification. The
README, the book and the website's "Cross platform" card will repeat
whatever the profiles say, and the first release (2005) will too. A
platform claim that fails on first contact costs the project more than a
modest one.

This ticket adds one level between the two, `Preview`, and makes
`Supported` mean what ADR-0008 already says. It is a different thing
from Nick's target **tiers** (decision of 2026-11-08: tier 1 is the
desktop and the web, tier 2 the devices, tier 3 PICO-8). A tier is the
order the work goes in. The status is how far that work has got. Keep
the two words apart in every text this ticket writes.

Source: the engine blind-spot audit (2026-11-23), finding "'Supported'
is not defined as a tier, and targets marked Supported have never run on
their device". Which devices the project buys or borrows is Nick's
question in 3527; this ticket only labels what has happened.

## Nick input

None.

## Scope

**In:**
- **Three statuses.** `Status` (`#[non_exhaustive]`, ADR-0039 kind 6)
  gains `Preview`, between `Supported` and `Planned`. The variant is
  additive, and a game's own profile that says `Supported` or `Planned`
  still reads. The three mean:
  - `Supported`: the target's example has run on the device, and the
    target's page records the run under `## On the device`: the device
    (and its model or revision), the OS or SDK version, the date, and
    the ticket or PR. Its budgets may still be provisional (*prov.*).
    It never promises a store's certification (Deck Verified, a
    console's lot check).
  - `Preview`: `shuyi export` builds it, and its example runs in CI or
    in the device's simulator, but nobody has run it on the device yet.
    Its budgets are provisional.
  - `Planned`: a profile and a ticket, nothing more.
- **The profiles.** `steam-deck` and `playdate` become `Preview`, unless
  their device ticket landed first (the clauses below). `desktop` and
  `web` stay `Supported`, and their pages get an `## On the device`
  section that records the runs that back it. For the desktop that is
  one line for each of Windows, Linux and macOS. Find each run in
  `tickets/done/` and the CI workflows (Linux under Xvfb in CI, Windows
  on the owner's machine). An OS with no recorded run says so, and
  names 3336 (CI launches packages on Windows and macOS). CI's `launch`
  cells (3336) launch the Windows and macOS packages and compare their
  windows with the raster, on Mesa llvmpipe and the macOS runner's GL.
  That is CI evidence, not a device run. For the web it is headless
  Chromium in CI. Firefox and WebKit are 3507's job.
- **The bench gate keeps them.** `shuyi bench` gates every target that
  is not `Planned`: `Supported` and `Preview`. A `Preview` target's
  limits line says its budgets are provisional. Then setting the Deck
  and the Playdate to `Preview` does not drop them from 3008's gate,
  which was 3011's and 3013's reason to keep them `Supported`.
- **`shuyi check --target`** pushes one `limits` line for a `Preview`
  target: `"{id} is a preview target: it hasn't run on its device yet
  (docs/targets/{id}.md)"`. The `TargetReport` contract shape doesn't
  change, so no schema moves.
- **Store certification on the Deck's page.** `docs/targets/steam-deck.md`
  says that `Supported` won't mean Deck Verified. The on-screen keyboard
  Verified asks for comes from Steamworks: the game's 0903, or 3037 once
  Nick unparks it.
- **A short ADR** (the `write-adr` skill). It amends ADR-0008 §1 (the
  three statuses and what each promises) and ADR-0081 §5 (the bench
  gates every status but `Planned`). Both old ADRs' `Status:` lines
  name it, and the index gets its row.
- **The texts that make the claim:** the README's **Targets**
  paragraph, the book's *Targets and export* chapter
  (`site/content/learn/book/targets-and-export.md`), the "Cross
  platform" card in `site/data/features.toml`, and
  `docs/targets/README.md`. Today `docs/targets/README.md` says the
  bench checks "each `Supported` target"; that becomes every target not
  `Planned`. Each text says which status each engine target has, or
  links the table that does.
- **No break.** The new variant is additive. The completion notes get
  `Behaviour:` lines: a profile may say `Preview` (`shuyi-assets`);
  `shuyi bench` gates `Preview` targets, and `check --target` gives a
  limits line for one (`shuyi-cli`). The game uses no `target::Status`
  in code (`gh search code --repo Zafnok/visions-of-shuyi
  'target::Status'`: no hits; the game pins engine
  `50472bbcd3084555a61728a5d07518f720bd6514`). A game whose `shuyi.ron`
  lists `playdate` or `steam-deck` is still gated there.

**Either order with 3011:** if 3011 is done, the Playdate has run on its
device and `docs/targets/playdate.md` records it: keep `playdate`
`Supported`, and move that record under an `## On the device` heading if
it sits elsewhere; if not, set `playdate` to `Preview`, add an
`## On the device` section that says it hasn't run there yet, and 3011
sets it `Supported` when it records the run.

**Either order with 3013:** if 3013 is done, keep `steam-deck`
`Supported`, its run recorded under `## On the device`; if not, set it
to `Preview`, and 3013 sets it `Supported` when it records the run.

**Either order with 2504:** if 2504 is done, `Planned` carries its
tickets (`Planned(tickets: ["3014"])`): give `Preview` the same field,
read the same way, with a bare `Preview` still reading (`playdate`
names 3011, `steam-deck` names 3013); if not, `Preview` is bare, and
2504 gives it the field when it gives `Planned` one.

**From 2502 (done):** `shuyi new --targets` takes the engine's
profiles through `engine_targets()` in
`crates/cli/src/contract/new/wizard.rs`, a `Target { id, ready }` per
profile; a planned one is refused by flag. Take a `Preview` profile
there like a supported one.

**Either order with 3424** (the questions as data): if 3424 is done,
its targets question offers a `Preview` profile with "preview: not yet
run on its device" beside it, and never offers a `Planned` one; if not,
nothing to do (3424 does it, its clause says so).

**Either order with 3507:** if 3507 is done, `docs/targets/web.md`'s
`## On the device` names headless Firefox and headless WebKit in CI (the
`web-browsers` job) beside Chromium; if not, nothing to do (3507 adds
those two lines).

**Out (do not do):**
- running anything on a device, or setting a budget from a measurement
  (3011, 3013; desktop and web frame times are 3340's);
- launching exported packages in CI on Windows and macOS (3336);
- the web in Firefox and WebKit (3507);
- the Deck Verified checks themselves (3012), and Steamworks (game 0903,
  parked 3037);
- Nick's tiers: they stay as decided, and no text renames them;
- a status for a store's certification: a store's page or ticket says
  that, not the profile.

**Estimated size:** about 400 lines (code 80, tests 120, docs 200).

## Implementation steps

1. `crates/assets/src/target.rs`: add `Status::Preview` with its doc
   (the meaning above). Rewrite `Supported`'s doc to say the run is
   recorded on the target's page. Update the module's tests and
   `crates/assets/tests/it/constructors.rs` if a list of statuses is
   asserted there.
2. `targets/steam-deck.ron`, `targets/playdate.ron`: `status: Preview`
   (unless the clauses above say otherwise). The comment above each
   `status` names the ticket that makes it `Supported`.
3. `crates/cli/src/bench.rs`: gate a target whose status isn't
   `Planned`, not only `Supported`. Keep the "isn't supported yet: not
   gated" line for `Planned`, and add a limits line for `Preview`
   ("{id} is a preview target: its budgets are provisional").
4. `crates/cli/src/contract/check.rs`: in the `--target` loop, push the
   `Preview` limits line.
5. `docs/targets/*.md`: give `desktop`, `web`, `steam-deck` and
   `playdate` an `## On the device` section, as the Scope says. Write the
   Deck Verified note on `steam-deck.md`.
6. The ADR (`write-adr`), and the two `Status:` lines.
7. The README, the book chapter, `site/data/features.toml` and
   `docs/targets/README.md`, as the Scope says.

## Acceptance criteria

- [ ] `Status::Preview` exists, documented, and `ron` reads `Preview`; a
      profile with `Supported` or `Planned` still reads.
- [ ] Unless 3011 or 3013 landed first, `targets/playdate.ron` and
      `targets/steam-deck.ron` are `status: Preview`, and `desktop` and
      `web` are `Supported`.
- [ ] Every engine profile that is `Supported` has a page whose
      `## On the device` section names a run (a test checks it).
- [ ] `cargo shuyi bench --target playdate` on Hello, Grid still gates
      against the `playdate` budgets, and so does a run with no
      `--target` whose `shuyi.ron` lists `playdate`.
- [ ] `shuyi check --target steam-deck` lists the preview limits line;
      `shuyi check --target desktop` doesn't.
- [ ] The new ADR amends ADR-0008 §1 and ADR-0081 §5, and both
      `Status:` lines name it.
- [ ] No text this ticket writes calls a status a "tier".
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/assets`): a profile with `status: Preview` decodes to
  `Status::Preview`; `Supported` and `Planned` still decode.
- Unit (`crates/cli/src/contract/target/tests.rs`): beside the test that
  lists the planned profiles, one that asserts the preview list
  (`["playdate", "steam-deck"]`, or less after 3011 or 3013) and the
  supported list. Another one reads each supported engine profile's page
  with `include_str!` (as `catalog.rs` reads `docs/modules.ron`) and
  asserts an `## On the device` section that doesn't begin "Not yet".
- Unit (`crates/cli/src/bench.rs`): the target picker gates a `Preview`
  profile with its limits line, and skips a `Planned` one unless asked.
- Unit (`crates/cli/src/contract/check.rs`): `check --target` on a
  `Preview` profile gives the limits line, and on a `Supported` one
  doesn't.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
