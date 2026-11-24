---
id: "3416"
title: "Frame items record the file and pointer they came from, and the live link picks them and drags their handles"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3417"]
nick_input: none
completed:
---

# 3416 — Frame items record the file and pointer they came from, and the live link picks them and drags their handles

## Context

An editor's live view lets you click a thing in the running game to
select it, and drag a handle to move it (Godot's 2D viewport, Unity's
scene view, LDtk). Shuyi has neither half, and nothing records which
file a drawn thing came from:

- A frame item (`Item` in `crates/ui/src/frame/glyph_buffer.rs`:
  `Rect`, `Sprite`, `Painted`, `Gizmo`) holds pixels, an image id and a
  layer. Nothing says which asset file, or which value in it, the item
  was drawn from.
- Gizmos (2315, done; [ADR-0085](../../docs/adr/0085-the-debug-view-gizmos-as-data-from-screens-in-channels-turned-on-from-the-debug-menu.md))
  are shapes in channels (`GizmoItem` in `crates/ui/src/frame/gizmo.rs`,
  handed in through `Gizmos::push` in `crates/ui/src/debug/gizmo.rs`).
  None is a handle, and none names a file.
- The live link (2521, done; [ADR-0047](../../docs/adr/0047-the-live-link-json-lines-over-loopback-tcp-to-a-running-debug-build-read-only-but-a-players-input.md))
  answers `Status`, `Frame`, `Screenshot`, `Play`, `Log` and `Perf`
  (`Request` in `crates/platform/src/link.rs`, `VERSION = 3`). Nothing
  asks "what is under this pixel".
- Scene values are read by JSON Pointer (2581, done;
  [ADR-0084](../../docs/adr/0084-one-description-of-a-games-types.md) §5:
  `scene --path /hero/x`), but a value has no way back to its file.
- 2507 (parked) plans the editor's live view with frames, channels,
  pause and step. Selection there flows one way only: a tree node turns
  a channel on. It has no click-to-select and no handles.

The pattern to copy is already in the frame. Hit areas (2405, done;
`crates/ui/src/frame/hit.rs`) are recorded by a painter as it paints
(`GlyphBuffer::hit`), looked up by a point (`GlyphBuffer::hit_at`), kept
in a `Hits` list whose `PartialEq` is always true, and never shown in a
snapshot. A frame's sources go the same way. The frame hash a recording
keeps (`App::end_session_frame` in `crates/platform/src/app.rs`) is
built from `FrameFacts`, not from items, so a side list never reaches
it.

Moving a thing must stay "text is the source of truth" (ADR-0005 §4)
and leave the link read-only but for a player's input (ADR-0047). So a
drag never changes running state. The client turns the drag into an
edit of the file through 3417's JSON-Pointer `patch` operation (with its
check of the file's hash), and hot reload (2220, done) brings the new
value into the game. That is why 3417 comes first.

Found by the engine blind-spot audit of 2026-11-23 ("No picking or
dragging in the viewport, and no record of which file an object came
from"). A grep of `tickets/open` for `pick`, `provenance`, `source`
handles and `hit.test` found no ticket that plans this; the `pick` hits
are RNG and bot picks (2113, 2119).

Users beyond the editor: an agent asks "what is the thing at (120, 40)
and where is it defined?" over MCP, and a farming game's developer drags
a shop sign's position in its RON file while the game runs.

## Nick input

None.

## Scope

**In:**
- **`Source`** in `shuyi_ui::frame` (new `crates/ui/src/frame/source.rs`):
  the file a thing came from (`shuyi_assets::format::AssetPath`) and a
  JSON Pointer (ADR-0084 §5) into the serde tree of the value that file
  decodes to. Kind 3 of ADR-0039: `#[non_exhaustive]`, built with
  `Source::new(path, pointer)`; the pointer is checked to be empty or to
  start with `/`.
- **The frame's source list.** `GlyphBuffer::source(rect, Source)`
  records the pixels a thing was drawn in, in paint order, as `hit`
  does. `GlyphBuffer::sources_at(x, y)` answers every source holding the
  point, the last recorded first. The list, like `Hits`, never counts in
  `PartialEq`, a snapshot, a `state_hash` or a golden value.
- **Off unless asked.** `GlyphBuffer::record_sources(bool)`, off by
  default. While off, `source` returns at once and keeps nothing, so a
  release build pays one branch per call. The backend turns it on only
  when a live-link client has said hello (the `link` feature, debug
  builds only, ADR-0047). No `cfg!` in a screen or a rule.
- **Handles.** `Handle` in `shuyi_ui::debug::gizmo`: a grab rectangle
  in console pixels, a `Source` whose pointer names the value it moves,
  which axes it moves (`HandleAxes::X`, `Y` or `XY`), and how many
  console pixels one unit of the value is (`px_per_unit`, at least 1).
  For `XY` the value is a map with integer `x` and `y`; for one axis an
  integer. `Gizmos::handle(channel, Handle)` hands one in from
  `debug_draw`, only while the channel is on. The debug skin draws it as
  a small filled square over the glyphs. Kind 3 of ADR-0039, with a
  constructor.
- **The grid kit records its tiles.** `MapScene::with_source(AssetPath)`
  names the `.map` the scene was built from. When it is set and sources
  are on, the grid skins record each visible tile's cell as
  `Source::new(map, "/tiles/cells/<y * width + x>")`, the tile's place in
  `GridMap`'s serde tree (`crates/assets/src/grid_map.rs`, `Grid<T>` in
  `crates/core/src/grid.rs`). A unit's source is the game's to record.
- **Two link requests** in `crates/platform/src/link.rs`, `VERSION`
  to 4:
  - `Request::Pick { x, y }`: the sources under that console pixel in
    the last frame shown, last recorded first, and the hit there
    (`hit_at`) if any.
  - `Request::Handles`: the handles of the last frame, each with an id
    (its index), its rectangle, axes, units and source.
  Both only read.
- **`shuyi attach`**: `--pick X,Y` answers the sources under the point;
  `--drag X,Y --to X,Y` finds the handle under the first point, reads
  the value at its pointer from the file, adds the drag's whole units
  (the pixel distance divided by `px_per_unit`, rounded toward zero, per
  axis it moves), and writes it with 3417's `patch` operation, checked
  against the file's hash. A drag of less than one unit writes nothing
  and says so. The same fields in `AttachInput` (`pick`, `drag`), so the
  MCP server's attach tool (`crates/cli/src/mcp/attached.rs`) has them.
- **A drag is a patch.** A drag in the live view becomes a `patch`
  (3417) on the file and JSON Pointer its item came from, sent with the
  file's hash as `base`; its undo is the answer's `inverse`. `--drag`
  answers that `inverse`, so a client undoes the drag by sending it back
  as a patch with `base` set to the answer's `hash`.
- The file a source names is found on disk the way hot reload finds it
  (2220): step 1 reads how. A source whose file isn't in the project (a
  baked or embedded one) is answered with a problem code, never guessed.
- **A new ADR** (the `write-adr` skill): sources as a side list of the
  frame, off unless a link client is attached; handles as gizmos; drags
  as file patches from the client, so the link stays read-only.
- Docs: the debug view's and the live link's pages, as the `document`
  skill says; the `shuyi-test` skill's section on attaching (2521) gets
  `--pick` and `--drag`.
- Everything is additive: new types, new methods, new variants of the
  `#[non_exhaustive]` `Request`. No API break. A client of link version 3
  is refused at hello as today; `shuyi attach` and the build share the
  types, so they move together.

**Out (do not do):**
- The editor's live view clicking and dragging (2507, parked: it uses
  these requests when it is built).
- A player's drag in a screen, and drop targets (3233). This is a tool's
  drag over the link.
- Dragging a tile, feature or unit of a `.map`: it needs a `.map`
  written back keeping comments (2273's writer), and then a ticket of
  its own, written when a game asks. Handles here move RON values that
  3417 can patch.
- Changing running state: a drag edits a file, and the game sees it by
  hot reload. A live edit through recorded debug commands is 2356's.
- Showing sources in the in-game debug view's readout: no ticket plans
  it; write one when a game asks.
- Rotation, scale or free-form path handles: no ticket plans them;
  write one when a game asks.
- Sources on the engine's widgets (menus, `.ui.ron` nodes): no ticket
  plans them; write one when a game asks. This ticket does the grid kit
  and handles only.

**Estimated size:** about 1,600 lines (code 800, tests 550, docs 250).

**Split:** past 900 lines, split off steps 5–7 (handles, `Request::Handles`
and `--drag`) into a new ticket blocked by this one and by 3417, and drop
3417 from this ticket's `blocked_by`.

## Implementation steps

1. Read ADR-0047, ADR-0084 §5, ADR-0085 and ADR-0134; the code of
   `crates/ui/src/frame/hit.rs`, `glyph_buffer.rs` (`Hits`, `hit`,
   `hit_at`), `crates/ui/src/debug/gizmo.rs`,
   `crates/platform/src/link.rs`, `crates/cli/src/contract/attach.rs`
   and `crates/cli/src/mcp/attached.rs`; 3417's `patch` operation; how
   2220's hot reload maps an `AssetPath` to a file on disk.
2. Add `Source` and the frame's source list beside `Hits`: `source`,
   `sources_at`, `record_sources`, equality and snapshots ignoring it.
3. Give `MapScene` its `with_source`, and record tile sources in the grid
   skins' paint when it is set and sources are on.
4. Add `Request::Pick` to the link and its answer, the server reading
   the last frame shown; have the backend turn sources on at a client's
   hello. Bump `VERSION`.
5. Add `Handle`, `HandleAxes` and `Gizmos::handle`, and the debug skin's
   drawing of a handle.
6. Add `Request::Handles` and its answer.
7. Add `--pick` and `--drag` to `shuyi attach` and `AttachInput`, the
   drag turning into 3417's `patch`, with its problem codes (no handle
   there, file not in the project, hash changed, value not integers).
8. Write the ADR, the docs and the skill section.
9. Run the gates.

## Acceptance criteria

- [ ] A frame with sources on and the same frame with them off are
      equal, and their snapshots are the same bytes.
- [ ] With sources off, `GlyphBuffer::source` keeps nothing:
      `sources_at` answers none after any number of calls.
- [ ] Two sources recorded over the same pixel are answered last first;
      a point outside both answers none.
- [ ] A Hello, Grid map scene given its `.map` path, painted with sources
      on: `sources_at` on the pixel of tile (3, 2) answers that file and
      `/tiles/cells/<2 * width + 3>`, and that pointer read from the
      map's serde tree is the tile's terrain.
- [ ] `Source::new` with a pointer that is neither empty nor starts
      with `/` is refused.
- [ ] Over the link (the `Server` driven in a test, no device), `Pick`
      answers the sources and the hit under a point of the last frame,
      and changes no frame hash of a recorded session.
- [ ] A handle of `px_per_unit` 16 on `(x: 3, y: 2)` in a RON file,
      dragged 40 pixels right and 15 down with `XY`: the file reads
      `(x: 5, y: 2)` (40 / 16 = 2 units; 15 / 16 = 0), its comments
      kept, and one line differs in `git diff`.
- [ ] That drag's answer carries 3417's `inverse`; sent back as a
      patch with `base` set to the answer's `hash`, it gives the file's
      bytes from before the drag.
- [ ] A drag of 15 pixels on that handle writes nothing and says less
      than one unit moved.
- [ ] A drag of 40 pixels left from `x: 1` gives `x: -1`, whole units
      rounded toward zero both ways.
- [ ] A drag on an `X` handle moves only `x`, whatever the vertical
      distance.
- [ ] A drag after the file changed on disk since it was read is refused
      by 3417's hash check, and the file is untouched.
- [ ] A drag whose source names a file not in the project answers its
      problem code and writes nothing.
- [ ] The MCP attach tool answers `pick` and `drag` as the CLI does.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the source list (order, off, equality); `Source::new`'s check;
  a handle's units from a drag on each axis, both signs, and under one
  unit.
- Property: for any frame and any sources recorded, the snapshot and
  `PartialEq` are those of the frame with none; for any drag, the units
  are the pixel distance divided by `px_per_unit`, rounded toward zero.
- Integration: the link's `Pick` and `Handles` through `Server` on a test
  game; `shuyi attach --drag` on a test game with a RON file, checking
  the file and that hot reload shows the new value in a later frame;
  the MCP attach tool's `pick` and `drag`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
