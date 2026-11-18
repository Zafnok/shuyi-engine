---
id: "3317"
title: "shuyi-app makes a picture's texture when it is first drawn, not every picture's at start, so a picture that arrives after the first frame is drawn"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2262"]
nick_input: none
completed: 2026-11-17
---

# 3317 — A texture made when its picture is first drawn

## Context

[ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
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
  native and 5–8 ms on the web. So textures aren't freed yet (ADR-0128
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
  2265). One whose bytes don't decode is logged once and drawn magenta
  for the run, as today.
- **The font atlas** stays uploaded at start: the first frame needs it.
- **Recoloured copies** and **`remake`** (a picture saved under `--watch`)
  work on the textures made so far, as today.
- Textures are never freed (ADR-0128 §5 and §7).
- **The raster** (`shuyi_ui::frame::raster`, `frame-png`) is unchanged:
  it decodes from the image table as before.

**Out (do not do):**

- freeing a texture (ADR-0128 §7: not crossed);
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

- [x] A picture first drawn in frame N has its texture from frame N, and
      none before (a test of `Renderer` with a counting upload).
- [x] A picture whose bytes appear in the bundle at frame N is drawn from
      frame N (a test).
- [x] A picture that doesn't decode is logged once and drawn magenta,
      as before.
- [x] Hello, Grid looks the same in the window (`frame-png` and the MCP
      session's `frame_png` unchanged).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the renderer's textures made on first draw; "nothing read yet"
  against "doesn't decode".

## Completion notes

What was done:

- `crates/app/src/image_textures.rs`: `ImageTextures<T>`, the images'
  textures without a window, as `PaintedTextures` is for painted
  pictures. None at start; `texture(id, make)` reads the image's bytes
  and makes its texture the first time it is asked, and keeps it and
  its source. A read that gives nothing is `Missing::NotRead`, read
  again next time; bytes that don't decode are
  `Missing::Undecodable(Some(why))` once, then `Undecodable(None)`
  without reading again, until `replace` (a save under `--watch`).
  Test builds count the uploads.
- `Renderer` holds one, with the game's file reader. `Renderer::new`
  uploads only the font atlas; `draw_sprite` makes the image's texture
  when it has none, logs "nothing read yet" once per image (cleared by a
  remake, as before) and a decode error once, and draws magenta for
  both. A recoloured copy is made only once its image has a texture, so
  an image not read yet isn't cached as having no copy. `remake` puts a
  saved picture's texture in, whether or not it was drawn yet.
- The raster is unchanged. The window matches it: the render card
  (sprites, recolours, a missing picture) and Hello, Grid's three
  capture frames passed with 0 pixels over, on this branch over `main`
  at 2b6fdf63; the MCP session's `frame_png` of Hello, Grid's
  title is as before.

Deviations:

- The acceptance criteria's "test of `Renderer` with a counting
  upload": `Renderer` needs a GL context, so the tests are of
  `ImageTextures`, the part of it that makes textures (as
  `PaintedTextures`' are for painted pictures): made on first draw and
  kept, bytes arriving at frame N drawn from frame N, and undecodable
  given up after one read and reported once.
- `Renderer::new` lost its `&ImageTable` argument: it no longer reads
  the table at start. The game doesn't call it (`gh search code --repo
  Zafnok/visions-of-shuyi 'Renderer::new'` finds only a done ticket).

Breaking: shuyi-app: `render::Renderer::new`: takes no image table; drop the `&ImageTable` argument

```migration
since: 0.1.0
what: Renderer::new makes an image's texture when a sprite first draws it, so it takes no image table.
old: "Renderer::new(atlas, png, clear, &images, bytes)"
new: "Renderer::new(atlas, png, clear, bytes)"
before: |
  let renderer = Renderer::new(atlas, &png, clear, &images, file)?;
after: |
  let renderer = Renderer::new(atlas, &png, clear, file)?;
find: ["&images, file"]
```

Behaviour: shuyi-app: an image whose files give nothing logs "image <path>: nothing read yet" once instead of a decode error of no bytes, and is read again each frame that draws it until it has bytes; textures of images no sprite has drawn are never made

Book: none, the book never builds a `Renderer` (a game's window makes it), and a game sees no change in what it draws. `docs/platform-backends.md` (`Show::Game`) says when a backend makes an image's texture.

Game: nothing to adopt, the game doesn't call `Renderer::new` and draws the same.

Nick input: none.
