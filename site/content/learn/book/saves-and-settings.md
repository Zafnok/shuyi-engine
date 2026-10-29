+++
title = "Saves and settings"
description = "Saves and settings go through the player's storage, in the profile in use; a save names its format's version."
weight = 70

[extra]
chapter = 7
+++

A game never opens a file. Its saves and the player's settings go
through **storage**, which the platform hands it: files on desktop, the
browser's `localStorage` on the web, a console's own save files, and
memory in tests. So the same code saves on every target, and a test
needs no disk.

## Storage

`Storage` keeps small named texts: `read`, `write`, `delete` and `list`,
under keys of lowercase letters, digits, `_` and `-`, at most 64 long.
A screen reaches it through `ctx.storage`.

## Profiles

`ctx.storage` is the storage of the **profile** in use. A profile holds
one player's settings, keys and saves, so the people who share a
computer each keep their own, on the engine's profiles screen
(`shuyi::ui::screens::profiles`). Each key keeps the
name it would have without profiles: the game writes `garden`, and the
profile keeps it apart from another player's.

## A save

A save names the version of its format. A build reads only saves of its
own version, and refuses another's with a message that says so, rather
than reading it wrong:

{{ snippet(file="src/saves.rs", anchor="save") }}

`SaveHeader` reads the version alone from any save, even one whose
other fields no longer parse, so the refusal can say "another version"
rather than "damaged". Raise `VERSION` whenever a saved type changes
shape or meaning. What a save holds, and when the game writes it, are
the game's: a turn game's rules state is plain data, so saving it is
one line.

{{ snippet(file="src/saves.rs", anchor="test_save") }}

A record or a save may also say what made it, with `shuyi_core::Made`:
the engine's version, the game's word for its build, and a fingerprint
of the files it ships. `made()` in a new game's `lib.rs` makes it.

## Settings

The player's settings are the engine's (text speed, full screen, the
color theme, the music and sound volumes, the frame cap, the layout of
keys) and the game's own. `ctx.settings()` reads them, and
`ctx.change_settings` changes them and saves them in the profile:

{{ snippet(file="src/saves.rs", anchor="settings") }}

`ctx.preview_settings` changes them without saving, for an options
screen that shows a change before the player keeps it. The engine's
options screen (`shuyi::ui::screens::options`) takes the game's rows, and
the key bindings are saved apart, by the Key bindings screen. A game
with settings of its own writes a `GameSettings` type and its own
`GameData` ([chapter 2](@/learn/book/games-and-screens.md#when-a-game-outgrows-the-short-path)).
