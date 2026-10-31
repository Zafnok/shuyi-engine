---
id: "2410"
title: "PadKind::Playdate follows the Playdate profile: A is East and B is West in its docs, its button names and its tests"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2410 — `PadKind::Playdate` follows the Playdate profile: A is `East`, B is `West`

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). The engine says two different things about which binding
position the Playdate's A and B buttons report:

- **`East` and `West`** in the data and the ADR:
  - `targets/playdate.ron`, lines 14–21: "A (`East`) and B (`West`)",
    `buttons` lists `"East", "West"` and the chord is
    `("East", "West")`;
  - ADR-0008 §1, the table of profiles: "d-pad, A, B (`East`, `West`)";
  - hello-grid's `targets."playdate"` table
    (`examples/hello-grid/files/keymap.ron`, line 41 on): Confirm on
    `East`, Cancel on `West`;
  - the controls-fit tests (`crates/ui/src/input/fit/tests.rs`), which
    read `targets/playdate.ron` with `include_str!`.
- **`South` and `East`** in `shuyi-ui`'s input code:
  - `PadKind::Playdate`'s doc (`crates/ui/src/input/pad.rs`, lines
    47–50): "reports A at the binding position `South` and B at `East`,
    so the default bindings make A Confirm and B Cancel", citing
    `docs/design/playdate.md`, which doesn't exist in this repository
    (there is no `docs/design/`; the path was carved from the game);
  - `PadKind::button_name` (`pad.rs`, line 103 on) names a Playdate's
    buttons with the Xbox names, so `South` is `A`, `East` is `B` and
    `West` is `X`; its doc says so (lines 97–101);
  - the test `a_playdates_a_confirms_and_its_b_cancels_named_as_printed_on_it`
    (`crates/ui/src/input/pad/tests.rs`, line 161 on), with the same
    dead citation;
  - `chord_tests.rs` (`crates/ui/src/input/chord_tests.rs`): its module
    doc says "`A` (`South`) confirms, `B` (`East`) cancels", and
    `playdate_keymap` ("A Playdate's buttons, as `docs/design/playdate.md`
    has them", line 34) binds Confirm to `South`, Cancel to `East` and
    End turn to `South`+`East`.

This is a bug, not only words: once a backend reports the Playdate's A
at `East` and B at `West`, as the profile says, the help text names A
"B" and B "X". 3010 (the Playdate backend) noticed the two rules and
says "pick one rule and write it in the docs"; the engine's data, the
ADR and 3010's own **Input** bullet already pick `East` and `West`, so
this ticket makes `pad.rs` and its tests follow them.

## Nick input

None.

## Scope

**In:**
- **The rule:** on a Playdate, A is the binding position `East` and B
  is `West` (ADR-0008 §1, `targets/playdate.ron`). `PadKind::position`
  doesn't change: no swap, so a game binds them in its keymap's
  `targets."playdate"` table, as hello-grid does; the default bindings
  (Confirm on `South`, Cancel on `East`) don't fit a Playdate.
- **`PadKind::button_name`:** on `PadKind::Playdate`, `East` is `"A"`
  and `West` is `"B"`; every other button keeps the Xbox name it has
  today (the D-pad arrows, the crank's names, the menu items). Its doc
  says so.
- **The docs:** `PadKind::Playdate`'s doc states the rule above and
  cites `targets/playdate.ron` and ADR-0008 §1 instead of
  `docs/design/playdate.md`. The same dead citation goes from
  `pad/tests.rs` (line 164) and `chord_tests.rs` (line 34).
- **The tests:** `pad/tests.rs`'s Playdate test and `chord_tests.rs`
  move to `East` and `West`, and a new test ties `pad.rs` to the
  profile file (below).
- **From 3010 (done):** the rule is kept: A is `East` and B is `West`,
  as `targets/playdate.ron` says. `PadInput` in
  `crates/playdate/src/input.rs` reports `East` and `West`, with a test
  (`every_button_is_one_the_profile_lists`) against the profile.
  `PadKind::Playdate`'s names are not changed: this ticket makes
  `pad.rs`'s docs, `button_name` and tests follow the rule, as above.
  The backend's test `the_first_frames_help_names_the_playdates_buttons`
  (`crates/playdate/src/driver/tests.rs`) compares the first frame's
  help with `PadKind::Playdate`'s own names, so it passes before and
  after this ticket. Until then the help says A as `B` and B as `X`.
- **For the game:** no signature changes, so CI's `api` job finds
  nothing, but `button_name`'s answer changes for `PadKind::Playdate`.
  Write a `Breaking:` line for it (ADR-0038 §6: a break the job can't
  see), e.g. `Breaking: shuyi-ui: PadKind::button_name: on
  PadKind::Playdate, East is named A and West B; a front end that
  reports A at South and B at East moves to East and West`. The game's
  own Playdate front end reports A at `South` and B at `East` (3010's
  Context, the game's `src/pads.rs`), and its `playdate` table uses
  them (3002's completion notes); the game's PR that moves the pin
  moves both to `East` and `West`.

**Out (do not do):**
- The Playdate backend and its fake-SDK test of which buttons A and B
  report: 3010.
- `PadKind::SteamDeck` (3012) and `PadKind::Touch` (2407, parked): new
  kinds that add their own arms to `button_name`, whichever lands first.
- `pad.rs`'s "several pads drive the game as one" rule and its tests:
  2406 (parked).
- The other `docs/design/…` citations in `crates/` (`controls.md`,
  `title-screen.md`, and `playdate.md` in
  `crates/ui/src/frame/onebit.rs`): 2034's. **Either order with
  2034:** if 2034 is done, the three Playdate citations above are gone
  already: check that `pad.rs`'s doc cites `targets/playdate.ron` and
  ADR-0008 §1 and states the rule this ticket sets; if not, remove
  them here, and 2034 does the rest.
- Changing `targets/playdate.ron`, ADR-0008, hello-grid's keymap or the
  controls-fit tests: they already say `East` and `West`.

## Implementation steps

1. In `crates/ui/src/input/pad.rs`:
   - rewrite `PadKind::Playdate`'s doc: the Playdate's own buttons (game
     ADR-0070): the D-pad, A and B; its front end reports A at the
     binding position `East` and B at `West` (`targets/playdate.ron`,
     ADR-0008 §1), with no swap, so a game binds them in its keymap's
     `targets."playdate"` table;
   - in `button_name`, add two arms before the Xbox arm:
     `Self::Playdate if button == Button::East => "A"` and
     `Self::Playdate if button == Button::West => "B"`, and rewrite the
     doc's Playdate sentence to say A is `East` and B is `West`, every
     other name the Xbox one.
2. In `crates/ui/src/input/pad/tests.rs`, rewrite
   `a_playdates_a_confirms_and_its_b_cancels_named_as_printed_on_it`:
   `position` is still the identity; `button_name(East)` is `"A"` and
   `button_name(West)` is `"B"`; the D-pad names and the `"D-pad"`
   cursor name stay. Its `actions_of` part used the default bindings;
   check instead that a keymap with Confirm on `East` and Cancel on
   `West` gives Confirm for `East` and Cancel for `West` on a Playdate,
   or drop that part if `actions_of` can't take a keymap (the chord
   tests below cover it). Replace the doc's citation.
3. Add the test tying `pad.rs` to the profile, in `pad/tests.rs`: read
   `targets/playdate.ron` as `fit/tests.rs`'s `profile` does
   (`include_str!("../../../../../targets/playdate.ron")` and
   `shuyi_assets::target::TargetProfile::from_source`), and check that
   `PadKind::Playdate.button_name` of the profile's buttons other than
   the D-pad and the crank is `["A", "B"]`, and that its one chord is
   named `A+B` (`chord_name` with `"+"`).
4. In `crates/ui/src/input/chord_tests.rs`, move the Playdate to `East`
   and `West`:
   - the module doc: `A` (`East`) confirms, `B` (`West`) cancels;
   - `playdate_keymap`: Confirm on `East`, Cancel on `West`, End turn
     on `pair(East, West)`, Previous and Next unit on
     `pair(West, DpadLeft)` and `pair(West, DpadRight)`; its doc says
     "as `targets/playdate.ron` has them";
   - in every test that uses `playdate_keymap()`, `playdate()` or
     `PLAYDATE`, each `South` meaning A becomes `East` and each `East`
     meaning B becomes `West`. A `West` used there as a button in no
     chord (`km.chord_wait(West)`, line 93; `pair(South, West)`,
     line 111) becomes `South`, and the `West.into()` bound beside the
     chord at line 464 becomes `North`, its expected `"A+B/X"` then
     `"A+B/Y"`;
   - tests on keymaps of their own stay as they are: `lone` and `pc`
     (lines 95–103), the default bindings in
     `chords_bound_by_the_player_work_and_save` (line 290 on), and the
     PlayStation and Xbox names (lines 466–470).
   The expected names (`A+B`, `B+←`, `B+→`, `A`) and the actions don't
   change: that is what the move checks.
5. Run the gates.

## Acceptance criteria

- [ ] `PadKind::Playdate.button_name(Button::East)` is `"A"` and
      `button_name(Button::West)` is `"B"` (a test).
- [ ] A test reads `targets/playdate.ron` and fails if the profile's A
      and B are not the buttons `PadKind::Playdate` names `A` and `B`.
- [ ] `grep -rn "docs/design/playdate" crates/ui/src/input` finds
      nothing.
- [ ] `PadKind::Playdate`'s doc and `button_name`'s doc name the same
      two positions for A and B as `targets/playdate.ron`'s `buttons`.
- [ ] Every test in `chord_tests.rs` passes with `playdate_keymap` on
      `East` and `West`, and its expected help names are unchanged.
- [ ] The completion notes have the `Breaking:` line of the Scope.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-ui`, `pad/tests.rs`): the Playdate's names and no swap;
  the profile test of step 3.
- Unit (`shuyi-ui`, `chord_tests.rs`): the existing chord tests on the
  moved keymap.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
