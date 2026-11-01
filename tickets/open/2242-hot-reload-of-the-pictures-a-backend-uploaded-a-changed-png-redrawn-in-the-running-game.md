---
id: "2242"
title: "Hot reload of the pictures a backend uploaded: a changed PNG redrawn in the running game"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["2220"]
nick_input: none
completed:
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

- [ ] `shuyi run --watch` on Hello, Grid shows a changed `tiles.png` within two
      seconds (checked by hand; say so in the completion notes).
- [ ] A broken picture keeps the old texture, and its error shows (a test of the
      decision).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: which changed paths remake which textures; a picture that doesn't decode.

## Completion notes

*(Filled in by the session that completes the ticket.)*
