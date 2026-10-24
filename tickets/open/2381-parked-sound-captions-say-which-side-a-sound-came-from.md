---
id: "2381"
title: "Parked: Sound captions say which side a sound came from: a side mark on each caption line, from the pan positional sound resolves"
type: feature
milestone: "Engine: accessibility"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["2363", "2366"]
nick_input: none
completed:
---

# 2381 — Sound captions say which side a sound came from

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after
> MVP/parity) tickets for networking, styles for later", for 2366; and
> 2026-10-20 for 2363, as its note quotes, `docs/decisions.md`): a
> follow-up of the parked sound captions (2366) and of the parked
> positional sound (2363), and a follow-up of parked work is parked.
> So it starts after both are unparked and done. If the game asks for
> 2363 and it is unparked early, this stays parked: the game hasn't
> asked for sound captions. Nobody starts it until Nick unparks it.
> Whoever unparks it sets `status: todo` and drops `Parked:` from the
> title.

## Context

2366 adds sound captions: a line of text on the screen for each sound
cue that has a caption key, for a player who can't hear it. 2363 gives
a sound a place in the world, and `Game::collect_audio` resolves it,
by where it is from a listener, into a `PlaySound` with a volume and a
`pan` from -1 (left) to 1 (right). Neither shows which side a
captioned sound came from. Each leaves that to this ticket, in its
**Out**:

- 2366: which way a sound came from needs positional sound (2363), so
  2381 does it, blocked by both;
- 2363: showing where a sound came from, in captions, is 2381's.

Why it matters:

- A hearing player hears a door open on the left and turns that way.
  A player who reads "[Door opens]" knows it opened, not where. So
  does a player who hears with one ear, or plays with one speaker.
- Minecraft's subtitles put a `<` or a `>` on a line when its sound
  came from that side. The Game Accessibility Guidelines' "ensure no
  essential information is conveyed by sounds alone", which 2366
  quotes, covers the side as much as the sound.
- For any 2D game with positioned sounds: a foe's steps behind a wall,
  an alarm off screen, a door in a top-down town.

What it builds on. Both tickets are open today, so read their
completion notes for what changed from their plans:

- **From 2363** (`crates/ui/src/audio.rs`, `crates/ui/src/game.rs`):
  - `AudioRequest::PlaySound { cue, volume, pan }`. `play_sound`,
    `play_sound_at` and the menu sounds send a `pan` of 0;
  - `Game::collect_audio` turns each `PlaySoundFrom` into a
    `PlaySound`, or drops it at or past the cue's `silent` distance.
    The pan comes from how far across the view the sound is, times the
    cue falloff's `pan` (0 to 100), so a cue whose `pan` is 0 always
    plays centred;
  - the toy game's `door` cue has a falloff, and a test screen in
    `crates/ui/tests/it/audio.rs` sets a listener and plays it.
- **From 2366** (`crates/ui/src/sound_captions.rs`):
  - `SoundCaptions`, the log. `feed` adds a line for a `PlaySound`, or
    a `MusicCommand::Start`, of a cue with a caption. The same key
    while its line shows adds one to the count and restarts the hold;
  - `SoundCaptionsView { lines }` and `SoundCaptionLine { text, count
    }`, plain data, painted by `sound_captions::paint` or by the
    game's own `GameData::paint_sound_captions`;
  - `Game` feeds the log at the end of `collect_audio`, with this
    frame's `audio_out` and `music_out`;
  - `sound_captions::check` reports a caption too wide for the
    console, counting ` ×99` and a panel cell on each side;
  - the text key `sound_captions.repeat`, the toy's `cc.click` caption
    and a Harness test of the strip.

One thing depends on which of the two landed second: the order inside
`collect_audio`. The log must be fed after the resolve. If it were fed
first, it would see `PlaySoundFrom`, which it ignores, and no
positioned sound would ever get a line.

## Nick input

None.

## Scope

**In:**
- **The side, in the log** (`crates/ui/src/sound_captions.rs`):
  - `SoundSide { Left, Right }`: `Debug`, `Clone`, `Copy`,
    `PartialEq`, `Eq`, and `#[non_exhaustive]` (ADR-0039 kind 6: above
    and below may join it);
  - `SIDE_PAN: f32 = 0.25` (*tunable*), and `side_of(pan: f32) ->
    Option<SoundSide>`: `Left` at or below `-SIDE_PAN`, `Right` at or
    above `SIDE_PAN`, `None` between them and for a NaN. It is two
    comparisons, so it needs nothing from `std` and gives mirrored
    answers for mirrored pans;
  - `SoundCaptionLine` gains `side: Option<SoundSide>`;
  - `feed`: a `PlaySound`'s line takes `side_of(pan)`. A repeat of the
    same key sets the line's side to the newest sound's, `None`
    included: the line says where the last one came from. A music
    line, and any other line `feed` adds, has no side;
  - `view` copies each line's side.
- **Fed after the resolve** (`Game::collect_audio`,
  `crates/ui/src/game.rs`): check that the log is fed this frame's
  `audio_out` after 2363's resolve has turned each `PlaySoundFrom`
  into a `PlaySound` or dropped it. If the feed comes first, move it
  after. So a positioned sound's line gets its side, and a sound
  dropped at or past `silent` adds no line: no player hears it, so
  there is nothing to caption. A comment there says why the order
  matters.
- **The painter** (`sound_captions::paint`):
  - a `Left` line shows its mark, then a space, then its text; a
    `Right` line shows its text, then a space, then its mark. The
    count stays with the text (`< [Door] ×2`, `[Door] ×2 >`);
  - the marks are the text keys `sound_captions.left` (`"<"`) and
    `sound_captions.right` (`">"`), through `Ctx::text`. They are
    plain ASCII, so any font a game draws its text with has them, and
    a game whose font has arrows may change them in its own text. A
    mark is a character, never a key's name;
  - the whole line, mark included, is centred and gets its panel cells
    as 2366 paints them, in the same colours as the text: the side is
    said by a character, never by colour alone;
  - a line wider than the buffer cuts its text, never its mark: the
    mark and its space stay whole at the line's side, at the buffer's
    edge;
  - a line with no side paints exactly as 2366 paints it.
- **The text:** the two keys go in the same English file or files as
  2366's `sound_captions.repeat`. The `cc.` keys don't change.
- **The check** (`sound_captions::check`):
  - its width rule counts the mark: a caption's English with ` ×99`
    after it, plus one space and the wider of the two marks' English,
    plus a panel cell on each side, must fit `cols`;
  - when any cue has a caption and `lang` lacks `sound_captions.left`
    or `sound_captions.right`, it reports each missing key once (the
    painter would fail on it). The message is for the game's
    developer: `// check-text: not player text`, as 2366's are.
- **The toy** (`crates/ui/src/toy.rs`): its `door` cue gets the
  caption `cc.door` ("[Door]") in its `ENGLISH` array, set the way
  2366 sets `click`'s.
- **Docs:** the *Sound captions* section of `docs/accessibility.md`
  (2366's) gains a paragraph: a positioned sound off centre shows a
  mark on its side; `SIDE_PAN`, and that a cue whose falloff `pan` is
  under 25 never reaches it; the two keys, and that a game may change
  them. The module docs of `sound_captions` say the same in a line.
- **Public types (ADR-0039):** `SoundSide` is new and
  `#[non_exhaustive]`. `SoundCaptionLine` is a view a skin reads
  (kind 3). If 2366 made it `#[non_exhaustive]` with a `new`, the
  field is additive and `new` leaves `side` at `None`. If 2366 left it
  exhaustive, it moves under kind 3 here, with a `Breaking:` line in
  2010's form.
- **For the game:** no manifest change, so `pack::VERSION` stays and
  no pack is baked again. A game that paints the strip itself
  (`GameData::paint_sound_captions`) keeps compiling and shows no mark
  until it reads `side`. The game's PR that moves the pin follows this
  one.

**Out (do not do):**
- above and below, or an arrow that points at the place: 2363's pan
  comes from across the view alone, and a direction needs more;
- how far away a sound is (a fainter mark for a far sound);
- sides for music and voices: they play centred (2363);
- a looped sound's side: looped sounds have no place (2363's Out);
- the side in what a screen reader is told: the mark is painted, not
  part of the line's text;
- a colour for each side, and a setting to hide the mark: the mark
  shows whenever sound captions are on;
- positioned sounds in hello-grid: it plays none, so its captions keep
  no mark;
- a new field in `shuyi run` or the Harness: `run` lists each sound's
  pan already (2363), and the frame text shows the mark;
- the game's own cues and screens.

## Implementation steps

1. Read 2363's and 2366's completion notes, then
   `crates/ui/src/audio.rs` (`PlaySound`'s `pan`, `PlaySoundFrom`),
   `Game::collect_audio` in `crates/ui/src/game.rs`,
   `crates/ui/src/sound_captions.rs`, the toy (`crates/ui/src/toy.rs`),
   `crates/ui/tests/it/audio.rs` (2363's listener screen) and 2366's
   Harness test of the strip, and `docs/accessibility.md`.
2. Check the order in `collect_audio`. Move the feed after the resolve
   if needed, with its comment, and write the test that a sound past
   `silent` adds no line.
3. Add `SoundSide`, `SIDE_PAN`, `side_of` and the line's `side`, and
   set it in `feed`. Unit and property tests in the module's tests.
4. Paint the marks, with the cut that keeps them, and add the two
   text keys.
5. Count the mark in `check`'s width rule, and report a missing mark
   key, with a test for each.
6. Caption the toy's `door`, and write the Harness test and its glyph
   snapshot beside 2366's.
7. Write the docs paragraph and the module docs line, and run the
   gates, the `no_std` build of `shuyi-ui` among them. Expect about
   300 changed lines, not counting the snapshot.

## Acceptance criteria

- [ ] `side_of` gives `Left` at -1 and at `-SIDE_PAN`, `None` at 0, just
      inside `±SIDE_PAN` and for a NaN, and `Right` at `SIDE_PAN` and
      at 1.
- [ ] A Harness test on the toy with sound captions on: a screen sets
      a listener and plays `door` at the right edge of the view. The
      log has one line, `cc.door`, with side `Right`. Within
      `HOLD_SECS` it plays `door` at the left edge: the same line, count
      2, side `Left`. Then under the listener: count 3, no side.
- [ ] A `door` played past `silent` adds no line.
- [ ] The toy's `click` and its `theme` music show no side.
- [ ] An `insta` glyph snapshot of the strip with `[Click]` and no
      mark, and `< [Door] ×2` on the line below it, is committed.
- [ ] A painter unit test: a `Right` line wider than the buffer ends
      with ` >` at the buffer's edge, its text cut before it; a `Left`
      one starts with `< `.
- [ ] With sound captions off, every existing snapshot is unchanged.
      With them on, 2366's snapshots of unpositioned sounds are
      unchanged.
- [ ] `check` reports a caption that fits the console without a mark
      but not with one, and a missing `sound_captions.left` once.
- [ ] `docs/accessibility.md` has the paragraph on the side mark.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - `side_of` at the edges above;
  - `feed`: a positioned sound's side, a repeat from the other side, a
    repeat from the centre, a music start with no side;
  - `Game::collect_audio`: a sound past `silent` adds no line;
  - the painter: each mark's place, centring with the mark, the cut
    that keeps it, and a line with no side as before;
  - `check`: the width with the mark, and a missing mark key.
- Property:
  - for any finite pan, `side_of(-pan)` is the mirror of
    `side_of(pan)`;
  - for random sounds with random keys and pans fed over random
    frames, each line's side is `side_of` the pan of the last sound
    with its key.
- Snapshot / integration: the Harness test on the toy, reading
  `Game::sound_captions()` first (what happened) and then the strip's
  cells (the look, `AGENTS.md` rule 9), with its glyph snapshot.

## Completion notes

*(Filled in by the session that completes the ticket.)*
