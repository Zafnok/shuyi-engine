# ADR-0095: A folder watched in every build: `read_watched_folder`, and the platform saying so to `App`

- **Status:** Accepted; amends ADR-0071 §1 (a build that ships asks for
  changed files when the game named a folder to watch)
- **Date:** 2026-11-09
- **Related tickets:** 2585 (this), 2220 (hot reload), game#0764 (a
  script preview for writers)

## Context

- ADR-0071 §1: `App` asks the platform which files changed only with
  `DEBUG_TOOLS` and `Ctx::debug_tools` on, and `shuyi_app::read_folder`
  notes its folder for the backend's `Poller` only in a build with debug
  tools. "A build that ships never asks."
- A game hands a writer who has no source a dialogue preview program
  (2585, game#0764). It is an optimized build, so it has no debug tools,
  and it must play each `.dlg` file the writer saves.
- Debug tools in such a build would also bring the Debug action, the
  live link and the debug view (`DEBUG_TOOLS`, `shuyi_ui/src/lib.rs`),
  which a writer has no use for.
- A player's mod folder read with `read_folder` in a shipped game must
  still never be watched: the reason ADR-0071 gave (each version of a
  changed file is leaked) holds, and nothing reads a mod again.
- `Platform` grows by defaulted methods (ADR-0040 §7).

## Decision

1. **The game names the folder.** `shuyi_app::read_watched_folder(path,
   name, max_file_bytes)` reads it as `read_folder` does, and notes it for
   the `Poller` as watched in every build. `read_folder` is unchanged.
2. **The poller looks at it with debug tools off.** `Poller` keeps
   `on = DEBUG_TOOLS`; off, it takes and looks at only the folders read to
   be watched in every build. With none, it never reads the clock or the
   disk (2220's test with the counting fake clock stays, and gains a folder
   read for debug builds only, never looked at).
3. **The platform says so.** `Platform::watches_files(&self) -> bool`,
   defaulted to `false`. The macroquad backend answers whether a folder was
   read with `read_watched_folder`; the headless backend answers what
   `HeadlessPlatform::set_watches_files` (`Harness::watch_files`) set.
4. **`App` asks then.** It asks for `changed_files` once a frame the game
   runs while debug tools are on (ADR-0071 §1), or while the platform
   `watches_files`. The rest of ADR-0071 holds: the bytes are replaced in
   the layer, modules hear of it through `Module::reloaded`, and reload
   errors reach the frame.
5. **Not in the web build.** The web has no folders; `read_watched_folder`
   is native only, like `read_folder`.

## Consequences

- A preview program is one call away: it reads its folder with
  `read_watched_folder` and keeps a module that reloads, in an optimized
  build without debug tools.
- That build leaks each version of a file saved in the folder, as a debug
  build does: a writer saves a file a few hundred times a session.
- A game can't turn the watch on for its mods by mistake: only the folder
  it names is looked at.
- The flag is process-wide: once a folder is read to be watched, `App`
  asks every frame. Asking costs a comparison when a second hasn't passed.

## Alternatives considered

- **A `Game` setting the game turns on beside the folder.** Two calls a
  game must both make, and forgetting one gives no error, only no reload.
- **Debug tools on in the preview's build** (the `debug-tools` feature).
  It brings the Debug action, the live link and the debug view to a
  writer's program, for one folder's changes.
- **`read_folder` with a flag argument.** A break for every caller, for a
  case one program has.
- **`App` asking every platform every frame.** The headless test that a
  build without debug tools is never asked (2220) would go, and a backend
  would have to decide on its own what a shipped game watches.
