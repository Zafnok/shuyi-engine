+++
title = "Coming from another engine"
description = "The words of Godot, Unity, Bevy, GameMaker and PICO-8, and what each is in Shuyi Engine."
weight = 110

[extra]
chapter = 11
+++

If you know another engine, most of its ideas have a counterpart here,
under another name and often in a plainer shape: the engine has no
scene tree and no entity system by default, and a game's state is plain
structs it owns. The table gives Shuyi's word first, with the chapter
that explains it, then the words of five engines for it. A dash means that engine
has no such word; "none yet" means Shuyi has no counterpart today.

| Shuyi Engine | Godot | Unity | Bevy | GameMaker | PICO-8 |
| ------------ | ----- | ----- | ---- | --------- | ------ |
| a `Screen` on the screen stack ([2](@/learn/book/games-and-screens.md)) | a scene | a scene | a state (`States`) | a room | the game's modes |
| a `Transition`: `Push`, `Pop`, `Replace` ([2](@/learn/book/games-and-screens.md#the-stack)) | `change_scene_to_file` | `SceneManager.LoadScene` | `NextState` | `room_goto` | — |
| a plain struct the game owns, in a screen or in the state its `TurnRules` change ([6](@/learn/book/rules-as-commands.md)) | a node | a GameObject | an entity | an instance | — |
| `Screen::update`, which changes the state through commands ([2](@/learn/book/games-and-screens.md#a-screen), [6](@/learn/book/rules-as-commands.md)) | `_process` | `Update()` | a system | the Step event | `_update` |
| `Screen::draw`, a skin painting a view ([4](@/learn/book/frame-and-skins.md)) | `_draw` | — | — | the Draw event | `_draw` |
| a real-time game's fixed steps ([6](@/learn/book/rules-as-commands.md)) | `_physics_process` | `FixedUpdate()` | `FixedUpdate` | — | — |
| `input.dt()`, the frame's seconds, for what a screen shows; rules count steps or turns ([6](@/learn/book/rules-as-commands.md)) | `delta` | `Time.deltaTime` | `Time` | `delta_time` | — |
| the game's data, `ctx.game`, on the context every screen shares ([2](@/learn/book/games-and-screens.md#the-context)) | an autoload | a singleton | a resource (`Res`) | a global variable | a global |
| an `ActionSet` and the player's keymap ([3](@/learn/book/actions.md)) | the input map's actions | the Input System's actions | `ButtonInput` (keys) | `keyboard_check` (keys) | `btn`, `btnp` |
| a kind of asset and its formats, through the registry ([5](@/learn/book/assets-and-formats.md)) | a resource (`.tres`), an import | an asset, a ScriptableObject | an asset, the `AssetServer` | an included file | the cart's data |
| the bundle's layers, and a baked pack ([5](@/learn/book/assets-and-formats.md#layers)) | a `.pck` file | an AssetBundle | — | — | the cart |
| the `SimRng` kept in the rules' state ([6](@/learn/book/rules-as-commands.md#turnrules)) | `randi()` | `Random` | — | `random` | `rnd` |
| a screen's answer (`Transition::pop_with`), or the rules' events ([2](@/learn/book/games-and-screens.md#asking-the-screen-below), [6](@/learn/book/rules-as-commands.md)) | a signal | an event | an event | a user event | — |
| `Storage`, in the player's profile ([7](@/learn/book/saves-and-settings.md)) | `user://`, `ConfigFile` | `PlayerPrefs` | — | `ini_open` | `cartdata`, `dset` |
| a module, turned on with `cargo shuyi add` ([2](@/learn/book/games-and-screens.md#modules)) | a plugin, an addon | a package | a `Plugin` | an extension | — |
| none yet: a game builds its pieces in code, from its data | a packed scene's instance | a prefab | a scene | — | — |
| none yet | a tween | a coroutine | — | an animation curve | — |
| `cargo test`, with the Harness ([8](@/learn/book/testing.md)) | unit tests (GUT) | the Test Runner | tests over an `App` | — | — |
| a target's profile, and `cargo shuyi web` ([9](@/learn/book/targets-and-export.md)) | an export preset | a build profile | — | a target | export |

## What has no counterpart elsewhere

A few of Shuyi's ideas are rarer in other engines, and the chapters
that explain them are worth reading first:

- **Determinism.** The same seed and commands give the same game on
  every machine, so a record of them is the whole play
  ([6](@/learn/book/rules-as-commands.md), [8](@/learn/book/testing.md#records-and-replays)).
- **Headless first.** Every game runs, and is tested and pictured,
  without a window ([8](@/learn/book/testing.md)).
- **A view between the rules and the look.** Tests read the view, so a
  new look breaks no test ([4](@/learn/book/frame-and-skins.md)).
- **One contract for people and agents.** The tools answer the same
  shape on the command line, over MCP and in tests
  ([10](@/learn/book/tools-for-agents.md)).
