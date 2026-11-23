---
id: "2786"
title: A cookbook of about 25 small tested examples, one task each, and shuyi-audio and shuyi-save skills in the pack
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2786 — A cookbook of about 25 small tested examples, one task each, and shuyi-audio and shuyi-save skills in the pack

## Context

Found by the engine blind-spot audit of 2026-11-23 (finding "No small
single-topic examples"). Someone new to the engine asks "how do I play a
sound" or "how do I save the game", and today the answer is a whole game
or a book chapter.

What there is (read on 2026-11-23):

- `examples/` holds one crate, `examples/hello-grid`: 9,759 lines of
  `.rs` (`find examples -name '*.rs' | xargs wc -l`). It is a whole game,
  not a page per task.
- The book (`site/content/learn/book/`, ticket 2802) teaches a concept
  per chapter, with tested code in `site/snippets/src/` (12 files, 2,481
  lines, crate `book-snippets`). Its code is one game, Moon Garden, cut
  into chapters by anchor. It does teach sounds:
  `games-and-screens.md` shows `site/snippets/src/screens.rs`'s chime,
  which calls `ctx.audio.add_sound` and `ctx.audio.play_sound`. But no
  file there is one task a reader can copy whole.
- The tutorial (`site/tutorial/`, `site/exercises/`, ticket 2803) is one
  project in steps.
- The other example-like files are backend probes
  (`crates/app/examples/render_card.rs`, `sound_probe.rs`) and
  `crates/cli/examples/toy.rs`.
- The planned examples are whole genre games, and most are parked
  (1125, 1126, 1127, 2704 and 2735 to 2738, among others).
- The skill pack (`skills/`, ticket 1134) has 18 skills. None is about
  sound or saves: `ls skills/` has no `shuyi-audio` or `shuyi-save`,
  and a grep of `tickets/open` for `shuyi-audio` and `shuyi-save`
  finds nothing.

The engine already has what most recipes need, headless:
`BasicGame::harness()` (`crates/platform/src/basic.rs`) gives a
`Harness` (`crates/platform/src/harness.rs`) with `keys`, `act`, `pad`,
`type_text`, `drop_file`, `wait`, `steps`, `top_screen`, `sounds`,
`audio_log`, `music_log` and `into_storage`. So each recipe can be a
program that opens a window and a test that plays it with no window.

A cookbook is a different shape from the book: one file per task, under
about 120 lines, which a reader or an agent copies whole. 3412 indexes
`examples/` for search, and 2843 can show these files to people as
how-to pages.

## Nick input

None.

## Scope

**In:**
- A new workspace crate, `examples/cookbook/` (package `cookbook`,
  `publish = false`, licence from the workspace). It depends on `shuyi`
  by path, as a game does, and on nothing else but `serde` and `ron`
  where a recipe saves. It is caught by the workspace's `examples/*`
  member glob, so no root `Cargo.toml` change.
- About 25 recipes. Each is one Cargo example,
  `examples/cookbook/examples/<task>.rs`:
  - a `//!` doc comment whose first line is the task as a reader asks
    it ("Play a sound when a key is pressed."), then two or three lines
    on what it shows;
  - its game, `fn game() -> BasicGame<…>`, and `fn main()` calling
    `run_game(game())`, so `cargo run -p cookbook --example <task>`
    opens it in a window;
  - a `#[cfg(test)] mod tests` that plays it through `game().harness()`
    and checks what happened by the scene, the state, the screens or the
    sounds asked for, never by cells (`AGENTS.md` rule 9);
  - at most 120 lines, the test included, counted with `wc -l`;
  - actions, never keys, in the game's code (`AGENTS.md` rule 8). The
    harness scripts in tests may name keys.
- Each recipe declared in `examples/cookbook/Cargo.toml` as
  `[[example]] name = "<task>"` with `test = true`, so
  `cargo test --workspace` runs its tests (Cargo builds an example by
  default but runs its tests only with `test = true`).
- Shared files in `examples/cookbook/files/` (English, keymap, palette,
  font), and a recipe's own files in `files/<task>/`, embedded with
  `shuyi::files!`.
- An index, `examples/cookbook/README.md`: a table of task, file and the
  engine's types it uses, grouped (screens and text, input, sound,
  saves and settings, rules and time, files and tools).
- A test, `examples/cookbook/tests/index.rs`, that fails when:
  - a file in `examples/` has no row in the README, or a row names a
    missing file;
  - a file is over 120 lines;
  - a file has no `//!` first line;
  - a file has no `[[example]]` entry with `test = true`.
- A new skill, `skills/shuyi-audio/`: `SKILL.md` (when to use it and
  when not, steps, the problem codes it meets) and a worked example
  `example.ron` run by `cargo xtask skill-examples`. It covers a sound
  made or added at run time (`ctx.audio.add_sound`), `play_sound` and
  `play_sound_at`, music where it plays (`play_music`,
  `play_music_with` and `MusicSwitch`, `stop_music`), the player's
  volumes in the settings, and a test that reads `Harness::sounds` and
  `Harness::audio_log`. The example checks `/result/sounds` from
  `cargo shuyi run`.
- A new skill, `skills/shuyi-save/`: `SKILL.md` and `example.ron`. It
  covers the game's `Storage`, profiles (`Profiles`,
  `crates/ui/src/profiles.rs`), a save type with `SaveFormat` and
  `SaveHeader` (`crates/core/src/save.rs`), settings, files the player
  keeps, bundles, and what to do when a save of another version is
  read. Its example writes a save type, saves and loads it in a test,
  and runs it with a `Cargo(args: ["test", …])` step.
- Both skills listed in `skills/README.md`, and pointing at the
  cookbook's recipes for their task.
- `docs/README.md` (the docs map, the `document` skill) gets a row for
  the cookbook: what it is for, where it lives, who reads it.

**Out (do not do):**
- Whole genre games: the example tickets (1125, 1126, 1127, 2704, 2739,
  2776, and the parked 27xx ones) build those.
- A recipe for a feature the engine doesn't have yet (Tiled maps, a
  tween library, a timer API in `BasicGame`, and so on). The README
  lists it under "Not yet" with the ticket that builds it. Its ticket
  adds the recipe.
- Doc examples on the entry-point types and the `shuyi` crate root's
  game: 2842.
- Publishing the recipes as how-to pages on the website, and site
  search: 2843's job.
- **Either order with 3412:** if 3412 is done, add a row to
  `site/data/how-do-i.toml` for each recipe (`go =
  "example/cookbook/examples/<task>.rs"`, with the task phrases from its
  `//!` line), and run its how-do-I table test; if not, nothing to do
  for the rows (3412 adds a row for each cookbook example).
- **Either order with 2225:** if 2225 is done, `shuyi-audio` and a
  `sound_from_files` recipe use a sound named in the game's
  `audio.ron` on the short path; if not, both use a sound added at run
  time, and `shuyi-audio` says that files on the short path wait for
  2225 (2225 adds that section and the recipe).
- **Either order with 2384:** if 2384 is done, `shuyi-audio` and a
  `music` recipe play music from `audio.ron`'s cues on the short path;
  if not, `shuyi-audio` teaches music only for a game that writes its
  own `GameData` with its manifest (as Hello, Grid does) and says that
  music on the short path waits for 2384 (2384 adds that section and
  the recipe).
- **Either order with 2110:** if 2110 is done, `shuyi-save` teaches its
  migration chain and golden fixtures, and a `save_migrate` recipe
  shows one step; if not, `shuyi-save` says a save of another version is
  refused today and that migrations wait for 2110 (2110 adds that
  section and the recipe).
- No change to any engine crate's public API. A recipe that finds a gap
  writes a ticket (the `write-ticket` skill) and is left out.

**Estimated size:** about 4,400 lines (code 2,600, tests 900, docs 900).
The 120-line cap bounds the recipes at 3,000; the rest is doubled.

**Split:** past 900 lines, ship steps 1–4 (the crate, the index test and
the first ten recipes) and split steps 5–6 (the other recipes) into one
new ticket and step 7 (the two skills) into another.

## Implementation steps

1. **The crate.** `examples/cookbook/Cargo.toml` as
   `site/snippets/Cargo.toml` is written (workspace version, edition,
   rust-version, licence, `[lints] workspace = true`), with `shuyi` by
   path. `src/lib.rs` holds only the crate doc pointing at the README
   (a Cargo package needs a target; no shared code, so each recipe
   reads alone). `files/` with `en.ron`, `keymap.ron`, `palette.ron`
   and a font, copied from `site/snippets/files/`.
2. **The index test** (`tests/index.rs`), as the Scope lists, reading
   `examples/`, `Cargo.toml` and `README.md` from
   `env!("CARGO_MANIFEST_DIR")`. Write it first, so each recipe is
   checked as it lands.
3. **Screens and text** (each in `examples/<task>.rs`, with its row):
   `hello_screen` (a screen that prints a line, `print_centred`),
   `text_from_files` (player text by key from `en.ron`),
   `menu` (a list picked with `UiAction`), `pause_overlay` (a screen
   pushed over another with `Transition::Push`, popped with a value,
   `Screen::uncovered`), `type_a_name` (`TextAsk` and the answer).
4. **Input:** `move_with_actions` (an `ActionSet` and its keymap),
   `pad_and_keys` (the same game played with `Harness::pad`),
   `pointer_click` (`crates/ui/src/input/pointer.rs`), `drop_a_file`
   (`DroppedFile`, `Harness::drop_file`), `rebind_keys` (the player's
   keymap changed and saved).
5. **Sound, saves and settings:** `play_sound` (`add_sound` and
   `play_sound`, tested by `Harness::sounds`), `sound_volume` (the
   player's volume in the settings), `save_and_load` (a `SaveFormat`
   type written to `Storage` and read back after `into_storage`),
   `profiles` (two profiles, each its own slot), `settings_screen` (a
   setting changed and kept).
6. **Rules and time, files and tools:** `turn_rules` (a `TurnRules`
   game: `check` and `apply`), `undo` (the history,
   `crates/core/src/history.rs`), `tick_game` (`TickSim`,
   `crates/core/src/tick.rs`), `random_from_a_seed` (`SimRng`, same seed
   same rolls), `ease_a_move` (`crates/core/src/ease.rs`),
   `find_a_path` (`crates/core/src/path.rs`), `grid_map_from_a_file`
   (`GridMap`, `crates/assets/src/grid_map.rs`), `camera_follow`
   (`crates/ui/src/grid/camera.rs`), `talk` (a `.dlg` scene played by
   `DialoguePlayer`), `a_bot_plays` (`shuyi::core::bots::play` and a
   `Bot`), `replay_a_session` (`Harness::start_recording` and
   `play_to`). Read each type's docs first: a recipe uses the API as it
   is and adds none. If one doesn't fit in 120 lines, it is two recipes
   or it is left out with a "Not yet" row.
7. **The skills.** `skills/shuyi-audio/` and `skills/shuyi-save/`, each
   modelled on `skills/shuyi-dialogue/` (frontmatter `name`,
   `description` with when to use and when not, `license`,
   `metadata.version`). Their `example.ron` runs on a game from `shuyi
   new` (`template: "blank"`), writes its files from `example/`, and
   checks with `Shuyi(args: ["run", …])` and `Cargo(args: ["test", …])`
   steps, as `skills/shuyi-debug-replay/example.ron` does. Run
   `cargo shuyi check-skills skills` and `cargo xtask skill-examples`.
   Add both to `skills/README.md`'s list. Check whether `shuyi new`
   copies a fixed list of skills (`crates/cli/src/contract/new.rs`); if
   it does, add them there.
8. **The docs map row,** as the Scope says, and the book's
   `games-and-screens.md` and `saves-and-settings.md` each end with a
   line pointing at the cookbook's recipes for their topic.
9. Write the engine lint's rule into the recipes: units spelled out in
   strings, no lone letter that reads as a key. Run `cargo xtask gates`.

## Acceptance criteria

- [ ] `examples/cookbook/` is a workspace member with at least 20
      recipes in `examples/`, each at most 120 lines, each with a `//!`
      task line and a test.
- [ ] `cargo test -p cookbook` runs every recipe's test (the count of
      tests run is at least the count of recipes).
- [ ] `tests/index.rs` fails when a recipe has no README row, when a row
      names a missing file, when a file is 121 lines, and when an
      `[[example]]` has no `test = true` (a unit test of its check over
      a fixture of each case).
- [ ] No recipe's game code names a key, and the engine lint (`cargo xtask gates --fast`) passes.
- [ ] No recipe's test reads cells or colours.
- [ ] `cargo run -p cookbook --example play_sound` opens a window and
      plays the sound (checked by hand once, said in the completion
      notes).
- [ ] `skills/shuyi-audio/` and `skills/shuyi-save/` pass
      `cargo shuyi check-skills skills`, and their worked examples pass
      `cargo xtask skill-examples`.
- [ ] `skills/README.md` lists both skills, and `docs/README.md` has the
      cookbook's row.
- [ ] Each either-order clause above is settled in the completion notes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the index test's checks, each over a small fixture that breaks
  it once.
- Integration: one headless test per recipe, through
  `BasicGame::harness()`.
- Snapshot / integration: the two skills' worked examples in CI's
  `skills` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
