---
id: "2220"
title: "Hot reload of any asset kind: a file on disk changes and the running game sees it, through the registry, with errors shown instead of a crash"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2207"]
nick_input: none
completed:
---

# 2220 — Hot reload of any asset kind

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage, "like hot reload". Bevy's homepage: "asset changes
appear in running apps without restarts or recompiles", for scenes,
textures and meshes, and "any asset type can be integrated".

ADR-0004 §1 planned hot reload of disk layers in debug builds, but the
only ticket for it was 1124, which is parked and is mostly about Tiled
and LDtk maps. This ticket takes hot reload out of 1124 and makes it
work for every asset kind the registry knows (ADR-0007, ADR-0019), so
1124's maps get it by being formats.

What is there today:

- `shuyi_assets::bundle` (`crates/assets/src/bundle.rs`) is one bundle
  installed for the process, and every read is `&'static`;
  `bundle::watch` already tells a front end of every file read;
- `shuyi_assets::format` has `AssetKind`, `Format`, `Formats` and
  `decode_with`;
- 2207 adds layers of owned files handed in at run time, and
  `shuyi-app` reading a folder on disk into one;
- a game keeps what it loaded in its own data (`GameData`), not in a
  store the engine owns.

So reload needs: a way to see that a file changed (the platform's job),
a new copy of it in the bundle, and each loaded value told to decode
again.

## Nick input

None.

## Scope

**In:**
- **The platform sees changes** (`AGENTS.md` rule 5: only backends
  touch files and the clock):
  - `Platform::changed_files(&mut self, out: &mut Vec<String>)`
    (`crates/platform/src/platform.rs`), with a default that adds
    nothing;
  - `shuyi-app`, in debug builds with debug tools on, checks the
    modification times of its disk layers' files once a second, with no
    new dependency (a `notify`-style crate is a follow-up only if
    polling costs too much on a large folder: measure 5 000 files);
  - the headless backend takes a script of changes, for tests.
- **The bundle takes the new bytes:** with `std`, a disk layer's file can
  be replaced (`Bundle::replace(path, bytes)`). The old bytes are
  leaked, as 2207's layers are, so every `&'static` read stays valid;
  this happens only in debug builds, and the docs say so.
- **Values that reload:** `shuyi_assets::hot::Hot<T: AssetKind>`:
  - made by decoding a path through the registry; it records every file
    the decode read (through `DecodeCx` or the bundle's watcher), so a
    change to any of them (an atlas's image, a level's tileset) reloads
    it;
  - `Hot::refresh(&mut self, changed: &[String]) -> Reload` where
    `Reload` is `Unchanged`, `Reloaded`, or `Failed(Vec<ContentError>)`;
    on failure the old value stays;
  - `Deref<Target = T>`, so code reads it like the value.
- **The game hears of it:** `GameHooks` gains a `reloaded` hook that
  `App` calls with the changed paths before the frame's update. The
  hook refreshes the game's `Hot` values. A screen can rebuild its
  view after it.
- **Errors show, never crash:** a `Failed` reload's errors (file, line
  and message, as every loader gives them) go to the debug view's text
  (2315 if done; else a line at the top of the frame drawn by
  `shuyi-app`'s debug overlay) until the next good reload.
- **Determinism:** a reload is not an input. A record made while
  reloading is marked so (`Record::reloaded = true`), and replaying it
  warns that it may not match.
- **`shuyi run --watch`** (`crates/cli`): runs the game with its
  `assets/` as a disk layer over the embedded files, and debug tools on.
- **1124** changes: its hot reload part is this ticket's; its maps are
  formats, so they reload with no extra code.
- The book's chapter on assets (2802, if done) gets a section.

**Out (do not do):**
- reloading code (2905);
- hot reload on the web or a console (no files to watch);
- scene files (2221 uses this).

## Implementation steps

1. Read 2207's layers, `bundle.rs`, `format.rs`, and the `Platform`
   trait.
2. Add `changed_files` to the trait and both backends, with the poller in
   `shuyi-app`.
3. Add `Bundle::replace` and `Hot<T>` with its read tracking.
4. Add the hook, the error display and the record flag.
5. Add `shuyi run --watch`, change 1124, and run the gates.

## Acceptance criteria

- [ ] With the headless backend's script, a changed palette file is seen
      by the game on the next frame (a harness test).
- [ ] A changed image that a sprite sheet reads reloads the sheet.
- [ ] A broken file keeps the old value, and its error names the file
      and line (a test).
- [ ] A shipped build (debug tools off) never polls (a test with a
      counting fake clock).
- [ ] `shuyi run --watch` on Hello, Grid shows a changed palette within
      two seconds (checked by hand; say so in the completion notes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Hot::refresh` for each outcome; read tracking across two files;
  the poller with a fake clock and file times.
- Integration: the harness test through the headless backend's script.

## Completion notes

*(Filled in by the session that completes the ticket.)*
