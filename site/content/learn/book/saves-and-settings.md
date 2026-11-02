+++
title = "Saves and settings"
description = "Saves and settings go through the player's storage, in the profile in use; a save names its format's version."
weight = 70

[extra]
chapter = 7
+++

A player closes the game and comes back: their garden should be as they left it, and
the music as quiet as they set it. That means keeping something after the program
stops.

On a computer, you'd write a file. But a browser has no files for you to write, a
console has its own system, and a test shouldn't touch your disk at all. So a game never
opens a file. Its saves and the player's settings go through **storage**, which the
platform hands it: files on desktop, the browser's `localStorage` on the web, a
console's own save files, and memory in tests. The same code saves on every target,
and a test needs no disk.

## Storage

`Storage` keeps small named texts: `read`, `write`, `delete` and `list`, under keys of
lowercase letters, digits, `_` and `-`, at most 64 long. A screen reaches it through
`ctx.storage`.

`Storage` is a **trait**, a list of functions, and each platform has a type that keeps
that promise in its own way.

## Profiles

`ctx.storage` is the storage of the **profile** in use. A profile holds one player's
settings, keys and saves, so the people who share a computer each keep their own, on the
engine's profiles screen (`shuyi::ui::screens::profiles`). Each key keeps the name it
would have without profiles: the game writes `garden`, and the profile keeps it apart
from another player's.

## A save

A game changes between versions. What should a new build do with a save from an old one,
whose shape has changed? Reading it anyway can give nonsense without any error. So a
save names the version of its format. A build reads only saves of its own version, and
refuses another's with a message that says so, rather than reading it wrong:

{{ snippet(file="src/saves.rs", anchor="save") }}

> **Rust:** there's a lot of error handling here, since saving is where things go wrong
> for real. A disk can be full.
>
> `storage: &mut dyn Storage` means "lend me some type with the `Storage` trait, which
> I may change". So this same function works with whatever the platform has. `save` gives
> back `Result<(), StorageError>`: fine, or what went wrong. `load` gives back
> `Result<Option<Garden>, LoadError>`, three answers in one: a garden, or `Ok(None)` for
> no save yet, which is no problem at all on a first run, or what went wrong.
>
> `garden.clone()` makes a copy, because `garden` is only borrowed (`&Garden`) and the
> `Save` needs to own its own. `ron::to_string` writes the `Save` as text, using what
> `Serialize` derived, and `.map_err(...)` turns its kind of error into this function's
> kind. The `?` then hands an error back at once.
>
> `let Some(text) = ... else { return Ok(None); };` says: this must be a `Some`, and name
> what's inside `text`. Or else, do this.

`SaveHeader` reads the version alone from any save, even one whose other fields no longer
parse, so the refusal can say "another version" rather than "damaged". Raise `VERSION`
whenever a saved type changes shape or meaning. What a save holds, and when the game
writes it, are the game's: a turn game's rules state is plain data
([chapter 6](@/learn/book/rules-as-commands.md)), so saving it is one line.

{{ snippet(file="src/saves.rs", anchor="test_save") }}

A record or a save may also say what made it, with `shuyi_core::Made`: the engine's
version, the game's word for its build, and a fingerprint of the files it ships.
`made()` in a new game's `lib.rs` makes it.

## Settings

The player's settings are the engine's (text speed, full screen, the color theme, the
music and sound volumes, the frame cap, the layout of keys) and the game's own.
`ctx.settings()` reads them, and `ctx.change_settings` changes them and saves them in
the profile:

{{ snippet(file="src/saves.rs", anchor="settings") }}

> **Rust:** `|settings| settings.music_volume = 40` is a small nameless function:
> `change_settings` hands it the settings to change, and then saves them. So you can't
> forget the save. `.unwrap()` stops the test if the save failed.

`ctx.preview_settings` changes them without saving, for an options screen that shows a
change before the player keeps it. The engine's options screen
(`shuyi::ui::screens::options`) takes the game's rows, and the key bindings are saved
apart, by the Key bindings screen. A game with settings of its own writes a
`GameSettings` type and its own `GameData`
([chapter 2](@/learn/book/games-and-screens.md#when-a-game-outgrows-the-short-path)).

## When it goes wrong

**Forgetting the copy.** Write `garden,` in place of `garden: garden.clone(),`, and the
compiler says what's wrong, and how to fix it:

```text
error[E0308]: mismatched types
46 |         garden,
   |         ^^^^^^ expected `Garden`, found `&Garden`
help: consider using clone here
46 |         garden: garden.clone(),
```

**Forgetting to raise `VERSION`.** No tool can catch this one. Add a field to `Garden`
without raising it, and an old save has the right version but doesn't parse, so `load`
says `Damaged` to a player whose save was fine. Whenever a saved type changes, raise it.

**A bad key.** A storage key takes only lowercase letters, digits, `_` and `-`, at most
64 long. `storage.write` answers with an error for any other. So a player's name is no
key: name your keys in code, and keep the name inside the save.

**Two games with one id.** The player's saves are kept under the game's id
([chapter 9](@/learn/book/targets-and-export.md#who-the-game-is)), so two games left on
the placeholder `com.example.…` id share them. `cargo shuyi check --target` warns of it.

**On the web.** A player who clears their browser's site data loses their saves, as
`localStorage` belongs to the browser. So on the web, as anywhere, `load` must handle
`Ok(None)`.
