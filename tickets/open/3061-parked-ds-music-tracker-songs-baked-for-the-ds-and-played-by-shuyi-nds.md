---
id: "3061"
title: "Parked: DS music: tracker songs baked for the DS and played by shuyi-nds, through 3027's song format and player or BlocksDS's sound library"
type: feature
milestone: "Parked: export targets"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3024"]
nick_input: none
completed:
---

# 3061 — DS music

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): a follow-up of 3024, the DS backend, which is parked, and a follow-up of parked work is parked, so it starts after 3024 is unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3024, the DS backend, plays sound effects only, through `libnds`'s
sound calls on BlocksDS's stock ARM7 core. It leaves music out: it
"may reuse 3027's format and mixer, or BlocksDS's own sound library if
3018 found its license permissive". 3024 first promised music as "a
follow-up ticket written when this lands". This is that ticket,
written now so the work isn't lost when 3024 lands. 3024 and 3027
name it.

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md):

- §3: the game runs on the ARM9 in Rust over `libnds`, and the ARM7
  runs BlocksDS's stock core, in C, unchanged. The ARM7 does the sound.
- §4: music is baked to a tracker-style format the backend plays,
  under a permissive license (ADR-0002 §8). 3027 picks it for the GBA.
- §7: the `nds` budget is 700 k emulator cycles a frame on the ARM9
  (prov.), counted by the emulator runner (3020).

[ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md):
the source stays a standard tracker module, and the baked song is
derived from it and tested equal (§4), by a decoder that never panics
on any input (§5).

The DS is not the GBA. The GBA has two sample channels, so 3027 mixes
in software. The DS has 16 hardware channels (3017's provisional
`channels`, which 3018 re-checks in `docs/targets/nds.md`). Each plays
a sample at its own rate, volume and pan, and the hardware mixes them.
So a tracker player on the ARM9 can play each of a song's channels on
a hardware channel, through the same `libnds` calls as 3024's sound
effects, with no software mix. BlocksDS also ships a sound library of
its own, which 3018 lists with its license.

When this starts, 3024 is done, and so is what 3024 waits for: the
`nds` and `dsi` profiles with their `audio` (3017), BlocksDS's parts,
their licenses and the emulator (3018), `shuyi bake --target nds`
(2204's target hook, which 3019 wired for `nds`), the emulator runner
(3020) and the bench scenarios (3008).

## Nick input

None.

## Scope

**In:**
- **The choice: how the DS plays a song.** There are two ways:
  - **3027's player**: `GbaSong` (or a DS form of it), played on the
    ARM9 by 3027's tracker player, which steps rows and effects once
    per frame. Each of the song's channels plays on a hardware channel
    through `libnds`'s sound calls (`soundPlaySample`, `soundSetFreq`,
    `soundSetVolume`, `soundSetPan`, `soundKill`; check the names in
    BlocksDS's docs on the day). 3027's integer mixer feeding one
    channel is a fallback, only if the hardware channels can't play
    what the format needs;
  - **BlocksDS's sound library**, on two conditions: 3018 found its
    license permissive (ADR-0002 §8), and BlocksDS's stock ARM7 core
    plays it as it is.

  Weigh them on: the licenses; the ARM7 core staying BlocksDS's stock
  one; the source staying a standard tracker module that we bake
  (ADR-0007); whether CI can check what a song plays (*CI*, below);
  the ARM9 cycles a song costs against the `nds` budget; and whether
  the songs and 3024's sound effects share the 16 channels without
  taking each other's. **Either order with 3027:** if 3027 is done,
  its ADR chose the tracker source format, its reader and the effects
  kept, `GbaSong` and its tracker player exist, and `hello-grid` has a
  song: keep that format and that song, weigh reusing `GbaSong` and
  its player against BlocksDS's sound library, pick one and record
  why; if not, use BlocksDS's sound library if it meets its two
  conditions, and choose the source format, its reader and the effects
  kept here, by the rules of 3027's *The music format*, in an ADR (the
  `write-adr` skill); 3027 then keeps that choice and `hello-grid`'s
  song, and runs this ticket's music scenario on `gba` (its clause
  says so). If 3027 isn't done and the library fails a condition, a
  player of our own would repeat most of 3027 here: add 3027 to
  `blocked_by`, say why in the PR that does it, and stop.

  Record the choice, and the license of every part it links or ships,
  in `docs/targets/nds.md`. A new crate or a new baked format also
  gets an ADR, or extends 3027's. If 3027's player is picked and it
  lives in `shuyi-gba`, move it and `GbaSong` into a small crate both
  backends use (as 3024 shares the 2D code), with its line in
  `tools/layers.ron` (ADR-0032); 3027's host tests pass unchanged.
- **The song, baked.** `shuyi bake --target nds` turns the standard
  tracker module into the form the chosen player reads: `GbaSong`, a
  DS form of it whose samples are in a format the hardware channels
  play, or the library's own song format (made by its converter, an
  installed tool the exporter names as it names `ndstool`, or by a
  writer of ours). Either way:
  - an effect the DS's player doesn't play is a bake error naming the
    song, the pattern and the effect, never a silent drop;
  - a song with more channels than music has (*Channels*, below) is a
    bake error naming the song and both counts;
  - **equality** (ADR-0007 §4): decoding the baked song equals
    converting the decoded source with the same settings, on every
    fixture. For the library's format, a small reader of ours does the
    decoding, in the tests only;
  - a new baked format is a `Format` (ADR-0007), with a test that
    arbitrary bytes never panic its decoder (§5).
- **Channels.** Split the 16 hardware channels once: a fixed number
  for music (8, unless the choice needs another), and the rest for
  sound effects. Add `music_channels: Option<u8>` to the profile's
  `audio` (3017's `AudioCaps` in `crates/assets/src/target.rs`), `None`
  on every profile but `nds` and `dsi`, so the bake knows the limit.
  `channels` on `nds` and `dsi` becomes what is left for sound
  effects, since the cue rules' voice limit reads it (3017): a sound
  effect never takes a song's channel. If `AudioCaps` is
  `#[non_exhaustive]` (ADR-0039), the field is additive; if not, the
  PR has a `Breaking:` line (ADR-0038).
- **Playing songs in `shuyi-nds`.** Play, stop and fade a song at the
  frame's volume for its cue, as 3027 does on the GBA. A `Load` reads
  the baked song from the pack in the ROM's file system (3024's
  export), and a `Start` plays it, looped if its cue says so. A `Gain`
  sets every channel of the song to its new volume. A `Stop` stops
  every one of them and frees the song. Closing the lid (3024's
  `SystemEvent::Suspend`) holds the song, and opening it goes on from
  the same row.

  **From 3029 (done):** `shuyi-nds`'s sound effects play under the shared
  player (3024 puts them there): add the songs the same way, as the shared
  backend trait's methods for a track (`load_music`, `poll_music`, `play`,
  `set_volume`, `stop`), so the player keeps the cue volumes, the fades and
  the music clock, and fill in the provided methods the song player can
  play (a pause, say). **Either order with 2342:** if 2342 is done, a song plays at the
  frame's `gains.music(cue)`; if not, at its `music_volume`, and 2342
  moves it onto the gains (its clause says so).
- **The profiles.** Check `targets/nds.ron`'s and `targets/dsi.ron`'s
  `audio` against the choice: `channels` and `music_channels` as
  above, `music` (`Mixed`, as 3017 wrote it, unless the choice streams
  a song), and `features`, which gains each `AudioFeature` the songs
  now play (`Pause`, say) and nothing they can't.
- **Budgets.** A bench scenario (3008's) with a song and four sound
  effects at once runs in the emulator runner (3020) against the `nds`
  `frame_cycles` budget. Its bench ROM plays through `shuyi-nds`'s
  audio, not the headless backend's silent one, so the song's ARM9
  cycles count. If 3027 is done, its music scenario is that scenario:
  run it on `nds` too, rather than writing another.
- **On the device.** **Either order with 3026:** if 3026 is done, its
  report says music isn't measured: check that `shuyi export nds
  --bench` offers the music scenario in its menu, and write a ticket,
  parked as this one is, to run it on a DS and add its numbers to
  `docs/targets/nds.md` (Nick's setup as in 3026's steps 5 and 6); if
  not, nothing to do (3026 runs the music scenario with the others).
- **CI.** A fixture song plays in the emulator 3018 chose, and the ROM
  prints a hash, through the emulator's debug output, of what the song
  did over its first 600 frames:
  - with 3027's player, the sound calls it made each frame (the
    channel, the sample, the rate, the volume and the pan). The player
    is a pure function of its state, so the hash equals a host run's;
  - with the library, its position (the pattern and the row) each
    frame, read through its own calls, against a checked-in value.
- **`hello-grid`** plays a song on the DS: 3027's, if 3027 is done,
  or the one this ticket adds (*The choice*): one short tracker module
  of our own (`CreditRef::Own`). The desktop and the web builds keep
  passing their tests: the song is silent there, or left out of their
  manifest; say which in the PR. `hello-grid`'s CI export for `nds`
  (3024) carries it.
- **Docs.** `docs/targets/nds.md`: the choice and why, every license,
  the tracker formats and effects the DS plays, the channel split, and
  how to add a song to a game. `crates/nds/README.md`: the songs, and
  a link there.

**Out (do not do):**
- the GBA's audio (3027);
- changing the ARM7 core, or writing our own: it stays BlocksDS's
  stock one (3024);
- streaming long recorded music from the ROM: write a ticket if a game
  needs it;
- two songs sounding at once (a crossfade): a song's `Start` stops the
  song before it; write a ticket when a DS game asks;
- MIDI on the DS (ADR-0014 §3: the GBA and the DS play tracker music
  instead);
- the microphone and DSi-only sound;
- measuring on the device: 3026 does it, or the ticket the clause
  above writes;
- anything of Nintendo's in this repository (3024's rule).

## Implementation steps

1. Read `docs/targets/nds.md` (BlocksDS's sound library, its license,
   whether the stock ARM7 core plays it, the emulator), `crates/nds`
   (its `sys.rs`, its fake hardware and its sound effects), and
   `crates/assets/src/target.rs`. Check whether 3027, 3029, 2342 and
   3026 are done (the either-order clauses).
2. Make the choice. Write it, with the licenses, in
   `docs/targets/nds.md`, and the ADR if one is needed.
3. Add `music_channels`, and fix the `nds` and `dsi` profiles.
4. Write the bake for `nds`, its bake errors and the equality test,
   with fixture songs under `crates/nds/tests/fixtures/` (or where the
   format's crate keeps its fixtures).
5. Add the bindings the player needs to `crates/nds/src/sys.rs`, each
   naming its `libnds` header, and teach the fake hardware to record
   them.
6. Write the song player (or wire the library), and play songs from
   the frame's music, with the lid's hold. Test against the fake.
7. Add the bench scenario, the fixture song's run in the emulator with
   its hash in CI, and `hello-grid`'s song.
8. Write the docs (the `document` skill). If 3026 is done, write the
   device ticket its clause asks for.

## Acceptance criteria

- [ ] The choice, and the license of every part it links or ships, are
      in `docs/targets/nds.md`; a new crate or format has its ADR.
- [ ] A fixture song plays in the emulator, and CI checks the hash the
      ROM prints (*CI*).
- [ ] An effect the DS's player doesn't play fails the bake, naming
      the song, the pattern and the effect; a song with more channels
      than `music_channels` fails, naming the song and both counts.
- [ ] The equality test passes on every fixture song.
- [ ] Host tests with the fake hardware:
  - [ ] a `Start` plays the song on at most `music_channels` channels;
  - [ ] a gain of one half halves the volume of every channel the song
        holds, and a `Stop` stops all of them;
  - [ ] with every sound-effect channel busy, the song keeps its own;
  - [ ] closing the lid holds the song, and opening it goes on from
        the same row;
  - [ ] the music clock (`music_playing`, or 3029's) gives the song's
        cue and the time since it started.
- [ ] The music scenario stays under the `nds` `frame_cycles` budget
      in the emulator runner.
- [ ] `hello-grid` exported for `nds` plays its song in the emulator.
- [ ] The ARM7 core is BlocksDS's stock one, unchanged.
- [ ] `cargo deny check` and `cargo xtask check-layers` pass.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (host, fake hardware): starting, fading and stopping a song;
  the channel split; the lid; the music clock; each effect the player
  plays.
- Property: random songs never panic the bake or the player, and a
  frame never uses more than `music_channels` channels for a song.
- Format: the equality test on every fixture, and arbitrary bytes into
  a new decoder never panic (ADR-0007 §5).
- Integration: the fixture song and `hello-grid` in the emulator, the
  hash checked in CI; the music scenario in the emulator runner.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
