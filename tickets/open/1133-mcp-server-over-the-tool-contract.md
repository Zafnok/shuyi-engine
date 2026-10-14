---
id: "1133"
title: "An MCP server over the tool contract, with a session that keeps one headless game running"
type: feature
milestone: "Engine: public"
model: opus-5.5
effort: high
status: todo
blocked_by: ["1129"]
nick_input: none
completed:
---

# 1133 — MCP server over the tool contract

## Context

ADR-0005 §2. Every engine with agent support found in the research gives
agents the same handful of abilities:

- start the game headless;
- send it input;
- read the scene and the logs;
- take screenshots.

Godot's are community MCP servers. Unity and Unreal have official MCP
plugins. Bevy has `bevy_brp_mcp`.

Our contract (1129) already does all of that without a window. This
ticket exposes it over MCP (stdio). It keeps a session so an agent can
step a game, look at it, and step it again.

## Nick input

None.

## Scope

**In:**
- `shuyi mcp` is an MCP server over stdio. Each contract operation is a
  tool, with the JSON schemas from 1129.
- Session tools: `start` (build and load a game headless), `step`
  (frames or a key script), `scene`, `frame_png` (returns the image),
  `events`, `reset`, and `stop`.
- `shuyi mcp install` registers the server with the agents it finds
  (Claude Code first; others whose config format is documented). It shows
  the diff and asks before writing, and keeps a backup, as REA does.
- An MCP library under an allowed license (check with `cargo deny`), or
  a small hand-written JSON-RPC layer if none fits.
- A test drives the server with a scripted client over stdio against
  `hello-grid`: start, step to the battle, read the scene, render a frame.

**Out (do not do):**
- a network transport (stdio only);
- editing files through MCP (agents already do that).

## Implementation steps

1. Write the server and the session state.
2. Write the install command with a dry run.
3. Write the scripted client test.

## Acceptance criteria

- [ ] The scripted client test passes in CI.
- [ ] `shuyi mcp install --dry-run` prints what it would write.
- [ ] `cargo deny check` passes.
- [ ] All gates pass.

## Tests required

- Integration: the stdio client test.

## Completion notes
