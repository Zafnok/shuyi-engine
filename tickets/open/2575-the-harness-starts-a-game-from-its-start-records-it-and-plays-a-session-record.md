---
id: "2575"
title: "The Harness starts a game from its real Start, records it, and plays a session record to any frame"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2518"]
nick_input: none
completed:
---

# 2575 — The Harness starts a game from its real Start, records it, and plays a session record

## Context

2518 built session records
([ADR-0072](../../docs/adr/0072-session-records-app-notes-what-the-platform-gives-a-hash-per-frame-of-what-happened-and-ron-never-migrated.md)):
`App::record`, `shuyi_platform::session` (`SessionRecord`, `prepare`, `Stamps`,
`Playback`, `FrameDifference`) and `HeadlessPlatform::from_session` and
`push_frame`. It tested all of them on `App` directly. Its size clause (about
600 changed lines) sent the Harness's part here. So today only code that drives
`App` by hand can record or play one, and a game's test can't write
`h.play_to(frame)`.

Why the Harness needs more than a call to `Playback`:

- `Harness::frame(events, dt)` drives `App::frame_after` with `f32` seconds
  (`crates/platform/src/harness.rs`, `frame`), which stops a recording, and whose
  `Clock::after` rounds differently from the whole milliseconds of `Clock::at`,
  so the fixed steps would differ.
- `Harness::new` and `on` start from a running game (`App::running`), skipping the
  load, whose seed comes from the clock.
- `Harness::frame`'s bookkeeping calls `screens()`, which panics while the game
  hasn't loaded, as it hasn't on the record's first frames.

2519 (`shuyi run`, `scene` and MCP), 2357 (rewind) and 2029 wait for this. 2519's
contract answers carry `Played::limits`.

## Nick input

None.

## Scope

**In:**
- **`Harness::start(start: Start<G>) -> Result<Self, String>`:** the game from its
  real `Start`, as a device runs it: the card frame (at 0 ms) and the load frame
  (one `FRAME_DT` later) run at once, and the load's errors come back as the
  `Err`. Its frames go through `App::frame` on a clock of whole milliseconds the
  Harness keeps in microseconds, rounded with integers (no `f64::round`: the
  `no_std` build has none), so `FRAME_DT` frames come out as 16, 17 and 17 ms. It
  doesn't hold back undeclared actions, as a device doesn't (ADR-0072 §6;
  `Harness::new` and `on` still do). The scripts (`keys`, `pad`, `wait`, `system`,
  …) work as before.
- **`Harness::start_recording(start)`:** the same, recording from the first frame;
  `Harness::session()` gives the record so far. Its docs say what isn't recorded:
  changes made around the trait (`act`, `ctx_mut`, `screen_mut`).
- **`Harness::from_session(start: Start<G>, record: &SessionRecord, stamps: Stamps)
  -> Result<Self, SessionError>`:** through `session::prepare`, on
  `HeadlessPlatform::from_session`; plays nothing yet.
- **`Harness::play_to(&mut self, frame: u32) -> &Played`** plays the record's
  frames up to `frame` (or its end) through `session::Playback`, with the Harness's
  per-frame bookkeeping, and stops at the first frame whose hash differs, so the
  game is left on that frame. `Harness::play_all()` plays to the end. It answers
  `session::Played { frames: u32, of: u32, first_difference:
  Option<FrameDifference>, limits: Vec<String> }` (new, `#[non_exhaustive]`,
  ADR-0039), `limits` from `prepare`.
- **After the record's last frame,** the Harness goes on with its own scripts and
  its clock from `Playback::now`. The music then comes from the headless speaker
  (`Playback` already ends the session on the platform), and `limits` says so once
  a script's frame has run.
- **The bookkeeping:** `Harness::frame` and `play_to` share it (the audio log, the
  screen log, the quit frame), and it must not panic before the game has loaded.
  The screen log starts with no screens at frame 0.
- **Docs:** the book's testing chapter (the book's chapter 8) gets a section on
  recording a session in a test and playing it to a frame, with its code in
  `site/snippets/`; the `shuyi-test` skill names it. `docs/platform-backends.md`'s
  *Session records* links them.

**Out (do not do):**
- Anything of the record's format, capture or `Playback` (2518, done).
- `shuyi run --session`, MCP and golden session files: 2519.
- Rewind: 2357.
- A `Harness::on`-like start from a `Start` on a platform a test made: a test that
  needs a store or a system keyboard records on `App` and a `HeadlessPlatform`
  directly, as 2518's tests do. A ticket if tests keep needing it.

## Implementation steps

1. Read ADR-0072, ADR-0024 §3, ADR-0025, `crates/platform/src/session.rs` and its
   tests, and `harness.rs`.
2. Add the Harness's clock: `Option<u64>` microseconds, `None` for `Harness::new`
   and `on`, which stay on `frame_after`.
3. Move `Harness::frame`'s bookkeeping to a private method that doesn't need a
   running game.
4. Add `Harness::start`, `start_recording`, `session`, then `Played`,
   `from_session`, `play_to` and `play_all`.
5. Write the tests below on the toy game, and the book's section.

## Acceptance criteria

- [ ] For random scripts on the toy game (keys, pad buttons, waits of 0 to 100 ms
      frames, a suspend and a resume), a session recorded with
      `Harness::start_recording` and played with `Harness::from_session` has no
      differing frame, ends on the same screens, and leaves the same storage
      (property test).
- [ ] Removing one key press from a recorded toy session at frame N makes
      `play_all` report its first difference at frame N, and leaves the game on
      that frame.
- [ ] `FRAME_DT` frames on a `Harness::start` come out as 16, 17 and 17 ms in the
      record.
- [ ] A `Harness::start` of a game whose load fails is the `Err` with its errors.
- [ ] A `Harness::start` doesn't hold back an undeclared action; `Harness::new`
      still does.
- [ ] `Stamps::Ignore`, a start without a stamp, a truncated record and an
      incomplete store each show in `Played::limits`; another `Made` and another
      version are `Err` from `from_session`.
- [ ] After `play_all`, `keys` runs on from the record's last time, and `limits`
      says the music is the headless speaker's.
- [ ] `play_to` before the game has loaded leaves the Harness usable: a test can
      read `app()` and `screen_log()` on the card's frame.
- [ ] The book's chapter is updated, or the completion notes have a `Book:` line
      saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the Harness's clock (16, 17, 17 ms); `play_to` past the end; `play_to`
  stopping at a difference; the screen log and audio log over the card and load.
- Property: record then play gives no difference (above).
- Integration: the divergence frame; scripts after the record's end.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
