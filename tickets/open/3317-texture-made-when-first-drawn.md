---
id: "3317"
title: "shuyi-app makes a picture's texture when it is first drawn, not every picture's at start, so a picture that arrives after the first frame is drawn"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2262"]
nick_input: none
completed:
---

# 3317 — A texture made when its picture is first drawn

## Context

[ADR-0127](../../docs/adr/0127-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§5:

- `Renderer::new` (`crates/app/src/render.rs`) uploads the font atlas and
  a texture for every image of the game's `ImageTable` at start, reading
  each with `AppConfig::files`. An image whose bytes can't be read gets no
  texture, is logged, and is drawn as a magenta rectangle for the rest of
  the run.
- On the web, 3318 streams the pack, so a picture's bytes may arrive
  after the game's first frame: today it would never get a texture.
- 2906's numbers (`docs/research/2906-asset-memory.md`, *Textures*): the
  game's 33 pictures take 3.5 MB as textures and upload in about 10 ms
  native and 5–8 ms on the web. So textures aren't freed yet (ADR-0127
  §7: freeing waits for textures over a quarter of `peak_heap_bytes`),
  and making them on first draw costs little.

## Nick input

None.

## Scope

**In:**

- **`Renderer`:** `images` starts empty. A sprite whose image has no
  texture yet reads its bytes (`AppConfig::files`, a `Cow` since 2262),
  uploads them, and keeps the texture and its source, as `with_files` does
  today. An image whose read gives nothing is drawn magenta, as today,
  and read again the next frame that draws it: on the web it may not
  have arrived yet (a game waits for its folder with `Bundle::arrived`,
  2263). One whose bytes don't decode is logged once and drawn magenta
  for the run, as today.
- **The font atlas** stays uploaded at start: the first frame needs it.
- **Recoloured copies** and **`remake`** (a picture saved under `--watch`)
  work on the textures made so far, as today.
- Textures are never freed (ADR-0127 §5 and §7).
- **The raster** (`shuyi_ui::frame::raster`, `frame-png`) is unchanged:
  it decodes from the image table as before.

**Out (do not do):**

- freeing a texture (ADR-0127 §7: not crossed);
- the web's streamed pack (3318);
- 2222's atlas pages: **Either order with 2222:** if 2222 is done, its
  pages are made when first drawn too, and uploaded again when they
  change; if not, nothing to do (2222 makes them so).

## Implementation steps

1. Make `Renderer::new` upload only the atlas, and make the sprite's draw
   upload its image's texture when it has none.
2. Keep "nothing read yet" (tried again) apart from "doesn't decode"
   (given up).
3. Run the gates.

## Acceptance criteria

- [ ] A picture first drawn in frame N has its texture from frame N, and
      none before (a test of `Renderer` with a counting upload).
- [ ] A picture whose bytes appear in the bundle at frame N is drawn from
      frame N (a test).
- [ ] A picture that doesn't decode is logged once and drawn magenta,
      as before.
- [ ] Hello, Grid looks the same in the window (`frame-png` and the MCP
      session's `frame_png` unchanged).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the renderer's textures made on first draw; "nothing read yet"
  against "doesn't decode".

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
