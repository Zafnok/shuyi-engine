# ADR-0164: The short path reloads its own files, and starts over at its first screen on a new own part

- **Status:** Accepted
- **Date:** 2026-11-27
- **Related tickets:** 2269 (this), 2220 (hot reload), 2905 (keeping a
  screen's place across a reload), 3414 (the MCP session's disk layer)
- **Amends:** ADR-0071 §4 (what a reload hook changes) and ADR-0040 §3
  (what a hook may change), for a file reload in a build with debug tools

## Context

ADR-0071 made any asset kind reload: a `Hot<T>` remembers what its decode
read, and a module with `Hook::Reloaded` refreshes it. But a value reloads only
if a module of the game's keeps it as a `Hot`. The short path (ADR-0036) had
none: `BasicData::load` read `en.ron`, `keymap.ron`, `palette.ron` and the
game's own part (`Own<T>`, a plain `fn(&Bundle)`) once, and `BasicGame` laid
the watched folder over the files with no module to read them again. Under
`cargo shuyi run --watch` only a changed picture's pixels showed (2242); a line
of text, a colour, a key or a map showed after a restart. Every game made with
`shuyi new` would have written the same module.

Two rules stood in the way of the engine writing it:

- ADR-0071 §4 keeps what a reload hook changes inside ADR-0040 §3: the module's
  own state and the context's UI side (the palette).
- ADR-0040 §3: a hook never changes the game's data or a stage's state, which
  change only through commands or inputs per step (ADR-0002 §3).

The text, the keymap and the palette are read by screens each frame, so a new
value shows with nothing else to do. The own part is different: the template's
stage builds its `Stage` from the map when it opens, and a screen built from the
old map can't be patched in place, as its pieces may stand on a wall the new map
has. Rebuilding a screen and replaying it to the same moment is research of its
own (2905).

## Decision

### 1. `BasicData` remembers what each value read, and reads it again

`BasicData::load` runs the decode of each of the English, the keymap, the
palette and the own part inside `shuyi_assets::bundle::recording`, and keeps
the paths each read. `BasicData::reload(&mut self, changed)` reads again each
value one of whose paths is among `changed`, with `BasicData::formats()` and the
checks `load` makes: `check_key_placeholders` for the English, and the palette
format's engine colours (`PaletteRon::requiring`), which refuses a palette that
lacks one. A good decode replaces the value and its paths; a failed one keeps
the value and adds the failed decode's paths, as `Hot::refresh` does. It answers
`Vec<(&'static str, Reload)>`: each value read again, in that order, named by
its file (`ENGLISH_RON`, `KEYMAP_RON`, `PALETTE_RON`) or by `OWN_PART`; a value
none of whose files changed is not listed.

It keeps plain values, not `Hot` ones: the own part is a `fn(&Bundle)`, not an
asset kind, and the English has a check of its own after the decode. The font
atlas and the picture table stay read once: a changed picture redraws (2242),
and a new picture or a new atlas needs a restart. Without `std`, nothing is
recorded and nothing reloads.

### 2. A built-in module hands the context what reloaded

`shuyi_ui::basic::BasicReload`, named `basic_reload`, declares
`Hook::Reloaded`, calls `BasicData::reload` and applies each good reload:

- the English: the languages' wide sets again (`Ctx::set_widths`'s work, with
  the widths it has); the frame drawn next shows the new text;
- the palette: `ctx.palette = Palette::new::<EngineColor>(…)`;
- the keymap: `ctx.refresh_keymap()`, so the layout in use and the player's own
  bindings stay. A layout the player changed keeps all its keys, as
  `PlayerKeys` always has; the file's new keys show in the layouts they didn't
  change, and its buttons while the player's buttons are the defaults;
- the own part: a restart (§3).

Every outcome goes to `Ctx::note_reload` under its name, so a broken file shows
its errors over the frame (ADR-0071 §5) while the old value stays.

`shuyi_platform::BasicGame::game` installs it before the game's own modules
when `shuyi_ui::DEBUG_TOOLS` and `ctx.debug_tools` are both on. A build that
ships lists no new module, and `Game::reloaded` is only called with debug tools
or on a folder the platform watches in every build (ADR-0095).

### 3. A new own part starts the game over at its first screen

`Ctx::restart(first)` asks the game to start over at the screen `first` makes,
on the same context. `Game::reloaded` honours it once every module's hook has
run, once: every screen leaves, top first, each with its sounds
(`ScreenStack::clear_in`), no screen told it is covered or uncovered; `first`
makes the only screen, and the frame is drawn again. The settings, the profile,
the layout, the player's bindings and the modules' states stay. Asked anywhere
else (a screen's update), it is honoured at the start of the next frame, before
the update; asked twice before then, the game starts over once, at the last
one's screen.

`BasicReload` holds the game's first screen (`BasicGame`'s `First<A, T>`) and
asks for it when the own part reloads. It is the one rule every own part can
obey: a map, a dialogue file, a table of the game's.

`Game::reloaded` also hands a keymap a hook changed to the input before the
frame's update (`sync_keymap`), so a new key works on the frame that told of
the change.

### 4. What this amends

ADR-0071 §4 and ADR-0040 §3 hold for every module, but for a file reload in a
build with debug tools, where a hook may also:

- replace the game's data read from its files (`ctx.game`'s values, through
  `BasicData::reload` or a game's own `GameData`), never its state of play;
- ask for a restart, which drops every screen's state rather than changing it.

Neither is an input: ADR-0071 §7 holds, and a session record warns of the frames
after the first that named files (2518).

## Consequences

- A game made with `shuyi new` reloads its text, colours, keys and stage map
  under `cargo shuyi run --watch` with no code of its own. The template's
  `AGENTS.md`, `docs/formats.md`, `docs/templates.md` and the book say so.
- A changed map loses the player's place in the stage: they start again from
  the first screen. Keeping it is 2905's research.
- `Game::modules` of a short-path game in a debug build lists `basic_reload`
  first; a test that listed the game's modules alone lists it too.
- A game off the short path (its own `GameData`) still writes its module, as
  Hello, Grid's `hot_palette` does; it can ask `Ctx::restart` too.
- `BasicData` holds the paths its values read, a few short strings each.

## Alternatives considered

- **`Hot` values for the three standard files.** The English's placeholder check
  runs after the decode, and the own part is a function, not an asset kind, so
  it needed `recording` anyway: one way for all four is simpler.
- **Patching the screens in place for a new own part.** No rule fits every
  game: a screen built from the old map can't know what of its state still
  holds. 2905 studies rebuilding and replaying.
- **A restart honoured the moment it is asked.** A module after the one asking
  would then run its hook on a stack the first had changed, and a screen asking
  from its update would drop itself mid-update.
- **Refusing a restart asked outside a reload.** A screen with a "start again"
  of its own would need its own way to clear the stack; honouring it at the next
  frame's start costs one check a frame.
- **Installing `BasicReload` in every build.** A shipped build never hears of a
  changed file (ADR-0071 §1), and would list a module that does nothing.
