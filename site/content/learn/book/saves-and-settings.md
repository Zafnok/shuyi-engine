+++
title = "Saves and settings"
description = "Saves and settings go through the player's storage, in the profile in use; a save names its format's version."
weight = 70

[extra]
part = "chapters"
chapter = 7
+++

{% newcomer(to="games", topic="keeping a game after it closes") %}
A player closes the game and comes back: their garden should be as they left it, and
the music as quiet as they set it. That means keeping something after the program
stops.
{% end %}

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

{% newcomer(to="rust", topic="a trait") %}
`Storage` is a **trait**, a list of functions, and each platform has a type that keeps
that promise in its own way.
{% end %}

## Profiles

`ctx.storage` is the storage of the **profile** in use. A profile holds one player's
settings, keys and saves, so the people who share a computer each keep their own, on the
engine's profiles screen (`shuyi::ui::screens::profiles`). Each key keeps the name it
would have without profiles: the game writes `garden`, and the profile keeps it apart
from another player's.

## A save

{% newcomer(to="games", topic="why a save names its version") %}
A game changes between versions. What should a new build do with a save from an old one,
whose shape has changed? Reading it anyway can give nonsense without any error.
{% end %}

A save names the version of its format. A build reads only saves of its own version, and
refuses another's with a message that says so, rather than reading it wrong:

{{ snippet(file="src/saves.rs", anchor="save") }}

{% newcomer(to="rust", topic="`dyn Storage` and a `Result` of an `Option`") %}
There's a lot of error handling here, since saving is where things go wrong
for real. A disk can be full.

`storage: &mut dyn Storage` means "lend me some type with the `Storage` trait, which
I may change". So this same function works with whatever the platform has. `save` gives
back `Result<(), StorageError>`: fine, or what went wrong. `load` gives back
`Result<Option<Garden>, LoadError>`, three answers in one: a garden, or `Ok(None)` for
no save yet, which is no problem at all on a first run, or what went wrong.
{% end %}

{% newcomer(to="rust", topic="`clone`, `map_err` and `?`") %}
`garden.clone()` makes a copy, because `garden` is only borrowed (`&Garden`) and the
`Save` needs to own its own. `ron::to_string` writes the `Save` as text, using what
`Serialize` derived, and `.map_err(...)` turns its kind of error into this function's
kind. The `?` then hands an error back at once.
{% end %}

{% newcomer(to="rust", topic="`let ... else`") %}
`let Some(text) = ... else { return Ok(None); };` says: this must be a `Some`, and name
what's inside `text`. Or else, do this.
{% end %}

`SaveHeader` reads the version alone from any save, even one whose other fields no longer
parse, so the refusal can say "another version" rather than "damaged". Raise `VERSION`
whenever a saved type changes shape or meaning. What a save holds, and when the game
writes it, are the game's: a turn game's rules state is plain data
([chapter 5](@/learn/book/rules-as-commands.md)), so saving it is one line.

{{ snippet(file="src/saves.rs", anchor="test_save") }}

A record or a save may also say what made it, with `shuyi_core::Made`: the engine's
version, the game's word for its build, and a fingerprint of the files it ships.
`made()` in a new game's `lib.rs` makes it.

## Settings

The player's settings are the engine's (text speed, full screen, the color theme, a
volume for each sound group, whether voices play, the frame cap, the layout of keys) and
the game's own. The sound groups are `master` over everything, `music`, `sfx` and
`voice` under it, and `ui`, the menus' sounds, under `sfx`; each volume goes from 0 to
100 on a loudness curve, so every step sounds like the same change. `ctx.volume(MUSIC)`
reads one.

A game adds groups of its own in its audio manifest (`audio.ron`), each under a parent
(`master` if it names none) and with the volume a new player starts at; an entry may set
a built-in group's starting volume too. A cue plays in the group its `group` names:

```ron
groups: [
    (id: "ambience", parent: "sfx", volume: 80),
    (id: "music", volume: 90),
],
sounds: [
    (id: "rain", files: ["sfx/rain.ogg"], volume: 60, credit: Own, group: "ambience"),
],
```

A `ducks` list lowers a group while a voice plays, so the words are heard over the
music: down to a position on the same loudness curve (80 is 12 dB down) over `attack_ms`,
and back over `release_ms` once the voice has ended. Only `voice` can lower another group
for now, and a voice the player turned down to 0 lowers nothing:

```ron
ducks: [
    (when: "voice", lower: "music", to: 80, attack_ms: 100, release_ms: 500),
],
```

The game's options screen shows the group's volume with a row of its own, named by its
id: `ctx.volume("ambience")` reads it, and `ctx.preview_volume("ambience", v)` sets it.
`ctx.settings()` reads them, and `ctx.change_settings` changes them and saves them in
the profile:

{{ snippet(file="src/saves.rs", anchor="settings") }}

{% newcomer(to="rust", topic="a closure and `.unwrap()`") %}
`|settings| settings.volumes.set(MUSIC, 40)` is a small nameless function:
`change_settings` hands it the settings to change, and then saves them. So you can't
forget the save. `.unwrap()` stops the test if the save failed.
{% end %}

`ctx.preview_settings` changes them without saving, for an options screen that shows a
change before the player keeps it. The engine's options screen
(`shuyi::ui::screens::options`) takes the game's rows, and the key bindings are saved
apart, by the Key bindings screen. A game with settings of its own writes a
`GameSettings` type and its own `GameData`
([chapter 6](@/learn/book/games-and-screens.md#when-a-game-outgrows-the-short-path)).

What the game does while its window has no focus is the game's choice, not the player's:
a constant of its `GameSettings`. By default it runs on with its sound. A game that
pauses, or goes quiet, or both, says so in one line of its `impl GameSettings`:
`const BACKGROUND: Background = Background::KEEP_RUNNING.paused().muted();`.
Paused, the game isn't updated and shows its last frame until the player comes back, and
the time away makes no steps. Muted, its music keeps its place in silence. `Background`
is in `shuyi::ui::settings`.

## Files the player keeps

{% newcomer(to="games", topic="why a player wants their save as a file") %}
Storage is the game's: on the web it sits inside the browser, where the player can't see,
copy or move it. A player who wants to back up their garden, move it to another computer
or send it to a friend needs it as a file of their own.
{% end %}

A game hands the player a file with `ctx.offer_file(name, text)`, and asks them for one
with `ctx.ask_for_file(&[".ron"])`. Both happen after the frame. On the web the file is a
download, and the browser's file picker opens; on a desktop the file is written into an
`exports` folder beside the saves, as `garden (2).ron` if a `garden.ron` is there, never
over it:

{{ snippet(file="src/saves.rs", anchor="files") }}

The answers come back to the game's system hook (`Start::on_system`) in a later frame:
`SystemEvent::FileOffered` says where the file went (downloaded, written at a path, or
why not), and `SystemEvent::FileChosen` holds the file the player picked, with its name
and text, or says they cancelled, or why it was refused: a file that isn't text, or is
over 256 MB.

A desktop has no picker: there, the player drops the file on the window, and the hook
hears `SystemEvent::FilesDropped` ([chapter 9](@/learn/book/targets-and-export.md)).
`ctx.platform` says which to offer: `can_pick_files()` for "choose a file",
`can_drop_files()` for "drop a file here". It also says where the saves are, to show
the player: `ctx.platform.storage_place` is a folder on a desktop
(`StoragePlace::Folder`), or the browser's storage under the game's prefix on the web
(`StoragePlace::Browser`). `ctx.open_storage_folder(Folder::Exports)` opens the exports
folder in the system's file manager, on a desktop.

A browser opens its picker only just after the player pressed something, so ask in the
frame of the press, as a menu does when its row is chosen.

A test reads what was offered with `Harness::offered`, and picks a file for the next ask
with `Harness::answer_next_ask`:

{{ snippet(file="src/saves.rs", anchor="test_files") }}

## Bundles

{% newcomer(to="games", topic="why a file of saves needs a verdict per save") %}
A player's file may come from an older build of the game, or a newer one, or a friend's
copy of another game. Before it replaces their saves, they should hear which of its saves
this build can read, and which it would write over.
{% end %}

A **save bundle** packs several entries of the storage into one text file, stamped with
what made it (`Made`), and reads one back with a verdict per entry before anything is
written. It is in `shuyi::ui::storage::bundle`:

- `write_bundle(storage, made, keep)` writes the keys `keep` keeps, in key order, so the
  same saves always give the same text;
- `plan_import(text, storage, check, now)` reads it back into an `ImportPlan`: each entry
  with its `Verdict` (`Ready`, `Incompatible { found, wants }`, `Unreadable`, or
  `Unknown` for a key the game doesn't know), whether it `replaces` a key the storage
  holds, and the stamp's `mismatch` with this build. It writes nothing;
- `apply(plan, storage, take)` writes the `Ready` entries `take` takes, all or nothing.

The game's knowledge of its own keys is the check: `FormatCheck::<Save>::new(|key| …)`
reads the keys it picks as saves of one `SaveFormat`, with `SaveHeader`, and `Checks`
puts several together (the first that knows the key decides).

{{ snippet(file="src/saves.rs", anchor="bundle") }}

The text goes to the player and back as any file does, with `ctx.offer_file` and
`ctx.ask_for_file` ([above](#files-the-player-keeps)). A file another build made still
plans: its `mismatch` says what differs, and the verdicts decide what can be written.
What the game tells the player, and whether it asks before replacing a save, are the
game's.

{{ snippet(file="src/saves.rs", anchor="test_bundle") }}

`apply` keeps a journal while it writes, so a power cut in the middle doesn't leave half
the old saves and half the new. Call `bundle::recover(storage)` at start, before reading
saves, and it puts the old ones back after such a cut; `apply` calls it first too. Keys
starting with `_bundle-` are the bundle's own: never use them for a save.

`shuyi import backup.ron` reads a bundle file and says what it holds, or where a broken
one is broken.

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
`Ok(None)`. A browser can also refuse storage: turned off, in some private windows, or
full. Then each call answers `StorageError::Backend` saying which, so warn the player and
play on, as on a full disk.

## Check yourself

{% check() %}
Why does a game save through `ctx.storage` and never open a file itself?

- [ ] Writing a file is slower than writing to storage. — Speed isn't the reason. Not every target has files a game can write ([storage](#storage)).
- [x] The platform hands it storage that fits the target, so the same code saves everywhere and a test needs no disk. — Files on desktop, `localStorage` on the web, a console's own saves and memory in tests, all behind one trait ([storage](#storage)).
- [ ] Storage keeps one save for every player who shares the computer. — `ctx.storage` is the storage of the profile in use, so each player keeps their own ([profiles](#profiles)).
{% end %}

{% check() %}
A field is added to `Garden`, and `VERSION` stays as it was. What does the new build do
with a player's old save?

- [ ] It refuses it as a save of another version. — The old save names the same version, so the refusal can't tell ([a save](#a-save)).
- [x] The version matches but the save doesn't parse, so `load` says `Damaged` to a player whose save was fine. — No tool catches this, so raise `VERSION` whenever a saved type changes ([when it goes wrong](#when-it-goes-wrong)).
- [ ] It reads the save and fills the new field with a default. — The old text lacks the new field, so it doesn't parse at all ([when it goes wrong](#when-it-goes-wrong)).
{% end %}

{% check() %}
What is the difference between `ctx.change_settings` and `ctx.preview_settings`?

- [x] `change_settings` changes the settings and saves them in the profile, and `preview_settings` changes them without saving. — An options screen previews a change before the player keeps it ([settings](#settings)).
- [ ] `preview_settings` only reads the settings, and `change_settings` writes them. — `ctx.settings()` reads them. `preview_settings` changes them too, but doesn't save ([settings](#settings)).
- [ ] `change_settings` saves for every profile, and `preview_settings` for the one in use. — Settings are kept in the profile in use, and only `change_settings` saves them ([settings](#settings)).
{% end %}
