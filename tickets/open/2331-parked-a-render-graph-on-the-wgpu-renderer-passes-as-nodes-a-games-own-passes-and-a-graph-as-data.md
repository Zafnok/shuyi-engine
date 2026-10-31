---
id: "2331"
title: "Parked: A render graph on the wgpu renderer: passes as nodes with named inputs and outputs, a game's own passes, encoded in parallel on native"
type: feature
milestone: "Engine: 2D effects"
model: reasoning
effort: high
status: blocked
blocked_by: ["3034"]
nick_input: none
completed:
---

# 2331 — A render graph on the wgpu renderer

> **Parked** (Nick, 2026-10-20, chose that Bevy features already parked stay parked, `docs/decisions.md`): it needs the wgpu renderer that 3034 plans, which is parked. Nobody starts it until Nick unparks 3034's work. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage. Its Render Graph composes render pipelines as a
graph: nodes render in parallel, render logic is modular and reusable,
and it is not tied to one graphics API.

Today `shuyi-app` paints with macroquad. 3034 plans the move to winit
and our own 2D renderer over wgpu, which already isn't tied to one API
(Vulkan, Metal, Direct3D 12, OpenGL, WebGL 2, WebGPU). 2326 plans screen
effects on it, 2327 a game's own WGSL passes, and 2328 2D lights. Each
is a pass. This ticket makes the passes a graph, so they compose
without each knowing the others.

When 3034 writes the renderer's ticket, it adds that ticket's id to
this one's `blocked_by`, as it does for 2326: the graph is built in
the renderer's crate, so the research alone is not enough.

## Nick input

None.

## Scope

**In:**
- **`RenderGraph`** in the wgpu renderer's crate (named by 3034's ADR):
  - a node has named inputs and outputs (textures and buffers, with
    their formats and sizes relative to the frame), and a `run` that
    records commands;
  - the graph sorts nodes by their links, in a stable order (ties by
    the order they were added), and rejects cycles and unlinked inputs
    with the nodes' names;
  - textures are reused between nodes whose lifetimes don't overlap;
  - **on native, independent nodes record their command buffers on
    threads**, then submit in the sorted order; on the web, one thread.
    The output is the same image either way (a test reads both back).
- **The engine's own drawing as nodes:** the frame (glyphs, sprites),
  then 2326's effects, 2327's game passes and 2328's lights where they
  are done, each moved to a node. **Either order:** a ticket of those
  three still open when this lands adds its pass as a node.
- **A graph as data:** `render.ron` lists the nodes a game turns on and
  their settings, so a skin's look (ADR-0002, graphics are a skin) is a
  file, reloadable with 2220. For 2326's screen effects, `render.ron`
  turns each pass's node on or off; the passes' numbers stay in 2326's
  `effects.ron`, and their amounts in the frame's `effects`.
- **A game's own node:** a Rust type implementing the node trait,
  added to the graph by name.

**Out (do not do):**
- 3D (3104's painter is over miniquad; a 3D graph waits for 3D on wgpu);
- a visual editor for the graph.

## Implementation steps

1. Read 3034's ADR and the renderer's code, and 2326–2328.
2. Write the graph, its sort and its checks, with fake nodes.
3. Move the frame's drawing into nodes; add the threaded recording.
4. Add `render.ron` and the game node example.
5. Run the gates.

## Acceptance criteria

- [ ] A cycle or a missing input is an error naming the nodes.
- [ ] The threaded and single-thread runs give the same image (a test
      reading the output back).
- [ ] Turning an effect off in `render.ron` removes its node and the
      frame's snapshot changes only by that effect.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the sort, checks and texture reuse, with fake nodes.
- Snapshot: frames with each built-in node on and off.

## Completion notes

*(Filled in by the session that completes the ticket.)*
