---
id: "2242"
title: "Hot reload of the pictures a backend uploaded: a changed PNG redrawn in the running game"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["2220"]
nick_input: none
completed: 2026-11-01
---

# 2242 — Hot reload of the pictures a backend uploaded

## Context

**From 2220 (done):** a `Hot<T>` reloads any value read through the registry,
and `shuyi run --watch` lays a game's files on disk over its embedded ones. But
`shuyi-app`'s `Renderer` is made once, from the first game frame's `Look`, and
uploads its font atlas and pictures through `AppConfig::files`
(`crates/app/src/backend.rs`, `draw_game`). So saving a `tiles.png` changes
nothing on the screen. A game's own `main` also passes a function over its
embedded files (`hello_grid::file`), which never sees the disk layer.

Bevy's homepage names textures among what reloads (`docs/ROADMAP.md`, *Bevy's
homepage*).

## Nick input

None.

## Scope

**In:**
- `shuyi-app` makes the renderer's textures again when a changed path
  (`Macroquad::changed_files`) is the font's picture or one of the look's images,
  keeping the old ones if the new picture doesn't decode, with the error shown as
  ADR-0071 §5 shows reload errors.
- `AppConfig::files` read through the disk layer under `run --watch`.
- A change of a picture's size: what its sprites then show, stated and tested.

**Out:** 3D meshes and textures (3104); sounds.

## Implementation steps

1. Read ADR-0071, `crates/app/src/render.rs` and `backend.rs`.
2. Split what decides which textures to make again from the upload, so a test
   reaches it without a window.
3. Add the reload, and run the gates.

## Acceptance criteria

- [x] `shuyi run --watch` on Hello, Grid shows a changed `tiles.png` within two
      seconds (checked by hand; say so in the completion notes).
- [x] A broken picture keeps the old texture, and its error shows (a test of the
      decision).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: which changed paths remake which textures; a picture that doesn't decode.

## Completion notes

**Done.** `crates/app/src/pictures.rs` decides, without a window, which changed
paths remake which textures and what decoded. `Renderer::remake` swaps the
textures, and `backend.rs` runs it from the paths `changed_files` told.

- `AppConfig::files` stays a plain `fn`: `watched_layer` keeps its layer in a
  static, and the backend reads through it (`pictures::read`).
- The renderer keeps the bytes of each texture's last good picture, so a
  recoloured copy first needed while the file on disk is broken still draws.
- A font picture that was broken at the start, whose reason showed instead of
  the game, now makes the renderer when it changes.

**A picture's size.** A sprite keeps its source, in pixels from the top-left. A
picture that grows shows the same pixels. A sprite whose source is past the
edge of one that shrank draws in magenta, logged once. The check runs only for
images remade since the start, so a shipped game draws as it did. A font
picture has no such check: its glyph rectangles stay what the atlas says.

**Checked by hand**, in the window, reading its picture over the live link: a
recoloured `tiles.png` showed within two seconds on the stage in the Tiles look,
junk kept the old texture with its error over the frame, a 48-pixel-wide picture
showed its tiles and magenta for the three past the edge, and the good file
mended all of it. The run was `cargo run -p hello-grid --features link` with
`SHUYI_WATCH` naming a copy of `files/`, not `cargo shuyi run --watch` itself,
whose build has no link: that command only starts the same program with that
variable (ADR-0071 §6). The font's picture wasn't saved by hand; its path is
covered by the unit tests.

Book: changed, *Reloading while the game runs*. No public item changed: no
break.
