---
id: "2811"
title: "Animated previews on the website's Examples cards: a preview.png per example, made by frame-png --preview from its shuyi.ron, checked in and tested equal"
type: feature
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: ["2514", "2801"]
nick_input: none
completed:
---

# 2811 — Animated previews on the website's Examples cards

## Context

2801 builds the website. Its Examples page has one card per example,
each playing in the browser at `/examples/<name>/`. 2514 lets
`frame-png` capture frames over time and write them as an APNG. Both
leave the same piece out, and both name this ticket for it:

- 2514's **Out**: previews on the website's Examples cards, after it
  and 2801;
- 2801's **Out**: 2514 makes an APNG of a game's frames, and this
  ticket puts them on the cards, after both.

What there is once both are done:

- `frame-png` (`crates/cli/src/frame_png.rs`) plays a `Script` of keys,
  pad buttons and waits in the game's Harness. With 2514,
  `--every <seconds> --count <n> --apng` captures `n` frames, `every`
  apart, the first right after the script, through
  `GameTools::frames` (`crates/cli/src/game.rs`). `encode_apng`
  (`crates/cli/src/codec.rs`) writes them, every frame whole, each with
  a delay of `every/60` s, looping forever.
- The Harness's frames are exact. The same script gives the same
  pixels on every run and every OS: 2514's `walk_sheet.png` snapshot of
  Hello, Grid passes on Windows, Linux and macOS in CI's `test` job.
- 2801's `pages.yml` builds each example for the web, runs
  `zola build` in `site/`, and copies each example's `dist/web` into
  `public/examples/<name>/`. A CI job builds and checks the site on
  every pull request.
- The examples already keep files the tools make, checked in, with a
  test that fails when one is stale. Hello, Grid's `files/tiles.png` is
  made by `glyph-tiles` from `shuyi.ron`'s `glyph_tiles` part, and
  `the_tileset_is_what_glyph_tiles_makes_of_the_codes_looks`
  (`examples/hello-grid/tests/it/files.rs`) checks it. Its README lists
  such files under *Files made by the tools*.

**Built at deploy, or checked in?** `pages.yml` could make each preview
on every deploy. Or the preview is checked in, and a test checks that
it is still what its script makes. This ticket checks it in:

- one file serves the README on GitHub and the card on the site. The
  parked brick-breaker and snake examples (2738, 2739) already plan an
  animated picture in their READMEs;
- a change that alters what an example shows fails its own pull
  request until the preview is remade, and the reviewer sees the new
  picture in the diff, as with `files/tiles.png`. Made at deploy, a
  broken preview would fail only on `main`, after the merge;
- `pages.yml` stays a copy. It doesn't build each example's tool
  program a second time.

The cost is a binary file in git that changes when the example's look
does. A cap keeps it small (512 KiB). The test compares decoded frames
and delays, not bytes, so a new `png` version that compresses
differently doesn't make a preview stale.

Why it matters:

- **Other 2D games:** a visitor sees an example move before its web
  build loads, and before they click.
- **Any game made with the engine** gets the same for its own README:
  a `preview` part in its `shuyi.ron` and one test line.
- **The game** needs nothing here. The part is optional, so nothing
  changes for it.

The ADRs: ADR-0039 (public types that may grow), ADR-0038 (the API gate
and `Breaking:` lines), and the site's ADR, which 2801 writes. As 2514
says, an APNG is a file a tool writes, not an asset format the engine
loads, so ADR-0007's registry doesn't apply. No ADR here: the part is
like `glyph_tiles`, and the file is checked like `files/tiles.png`.

## Nick input

None.

## Scope

**In:**
- **A `preview` part in `shuyi.ron`** (`crates/cli/src/project.rs`):
  - `Config::preview: Option<Preview>`. `Preview { args: Vec<String> }`
    holds `frame-png`'s arguments as typed after the output path, with
    `deny_unknown_fields` like the other parts. It may grow (ADR-0039):
    `#[non_exhaustive]`, built with `Preview::new(args)`.
  - The file is always `preview.png` in the project's root
    (`frame_png::preview::FILE`). A README links it and the site finds
    it with no setting.
  - Hello, Grid's, for example:
    ```ron
    preview: (
        args: ["--stage", "--keys", "Enter Up Right Right Enter",
               "--every", "0.1", "--count", "20", "--scale", "1",
               "--apng"],
    ),
    ```
  - If `Config` isn't `#[non_exhaustive]` yet, the new field is an API
    change. CI's `api` job reports it, and the completion notes carry
    its `Breaking:` line (ADR-0038).
  - If `crates/cli/tests/formats/shuyi.ron` exists (a fixture that sets
    every part of `Config`), it gets the part too.
- **`frame_png::preview`** (`crates/cli/src/frame_png/preview.rs`):
  - `FILE` (`"preview.png"`) and `MAX_BYTES` (512 KiB).
  - `options(project: &Project, game: &[GameOption]) -> Result<Options,
    String>`: parses the part's `args` with `parse_args`, with `FILE`
    as the output path. Each error names `shuyi.ron`'s `preview`:
    - no part (through `Project::part`);
    - args that don't parse (with `parse_args`' message);
    - no `--count`, or an output other than `--apng`: "a preview is an
      APNG: add --count and --apng";
    - an output path in the args;
    - `--target`: a preview shows the game's own colours.
  - `check(project: &Project, game: &dyn GameTools) -> Result<(),
    String>`: captures the frames with `GameTools::frames` from
    `options`, reads `<root>/preview.png`, decodes it, and fails on the
    first difference, which it names:
    - the file is missing;
    - it is larger than `MAX_BYTES`: the message gives its size and
      suggests a longer `--every`, a smaller `--count` or `--scale 1`;
    - the frame count, a frame's size, a frame's pixels (the frame's
      index), a delay (not `every/60` s), or the plays (not 0, forever).

    Each message ends with the command that remakes the file:
    `<command> frame-png --preview`, with `project.config.command()`.
- **`codec::decode_apng(bytes: &[u8]) -> Result<Animation, String>`**
  in `crates/cli/src/codec.rs`, over `png`: each frame as RGBA, its
  delay as a fraction of a second, and the plays. `Animation` is
  `#[non_exhaustive]` (ADR-0039). A frame that isn't the whole picture
  at (0, 0) is an error: `encode_apng` writes every frame whole. It
  reads only what `encode_apng` writes, 8-bit RGBA: any other colour
  type or depth is an error too, so it never trusts `png` to have
  turned another kind of PNG into RGBA. If 2514's round-trip test
  already decodes an APNG, move that code here and have the test call
  it.
- **`frame-png --preview`** (`frame_png` in `crates/cli/src/lib.rs`):
  with `--preview` as its only argument, `frame-png` takes its options
  from `preview::options` and writes `preview.png` as `--apng` does,
  with the same summary. `--preview` with any other argument is an
  error with the usage. `frame_png::usage` lists it.
- **Hello, Grid gets a preview:**
  - the `preview` part in `examples/hello-grid/shuyi.ron`, with a
    comment like the other parts' ("`cargo shuyi frame-png --preview`:
    `preview.png`, …; a test fails when it is stale"). Its script is
    2514's walk sheet's (`--stage`, then the keys `Enter Up Right Right
    Enter`), or another that shows your piece walk and the foe's step
    after it. Pick `--every` and `--count` so the loop covers both, in
    about 2 s, at `--scale 1` (384 × 384 px);
  - `examples/hello-grid/preview.png`, made with `cargo shuyi frame-png
    --preview` in that folder;
  - a test in `tests/it/files.rs`:
    `frame_png::preview::check(&project(), &HelloTools)`;
  - its README shows the preview under its first paragraph, with alt
    text that says what moves, and lists `preview.png` under *Files
    made by the tools* with the command that remakes it.
- **The cards** (`site/`, as 2801 made it):
  - `site/copy-previews.sh` copies each `examples/<name>/preview.png`
    to `site/static/previews/<name>.png`. `pages.yml` runs it before
    `zola build`, and so does the CI job that builds the site on pull
    requests. `site/static/previews/` goes in `.gitignore`. An example
    that gets a `preview.png` is on the site with no workflow change.
  - The Examples card template shows the preview when there is one:
    `get_image_metadata` on `previews/<name>.png` with
    `allow_missing = true`. It is an `<img>` with the file's `width`
    and `height`, `loading="lazy"`, the alt text "<the example's
    title>, playing", and the CSS `width: 100%; height: auto;
    image-rendering: pixelated`. Under `prefers-reduced-motion:
    reduce` it is hidden. A card with no preview stays as 2801 made
    it.
  - If the pinned Zola's `get_image_metadata` has no `allow_missing`
    (check its docs on the day), each card's entry gets
    `preview = true` instead, and `copy-previews.sh` fails when an
    entry says so and its example has no `preview.png`.
  - `site/README.md` says how an example gets a preview, and that
    `zola serve` shows the previews after `sh copy-previews.sh`.
- **Either order with 2738 and 2739** (the brick-breaker and the
  snake, both parked): if one of them is done, its README shows an
  animated picture of its first seconds, made with 2514's
  `frame-png --apng`. Make that picture its preview:
  - name it `preview.png` in the example's folder if it has another
    name, and fix the README's link;
  - add the `preview` part whose args make it (the README or the PR
    that made it says how; else write args that show the same);
  - remake the file with `cargo shuyi frame-png --preview` if `check`
    fails on it, the size cap included;
  - add the test line in its `tests/it/`. Its card then shows it.

  If not, nothing to do (each makes its README picture its preview when
  it lands; its own clause with 2811 says so).
- **Either order with 2565:** if 2565 is done, `codec::decode_png` is
  gone and `codec.rs`'s docs say it only encodes: `decode_apng` still
  goes in `codec.rs`, beside `encode_apng` (`png_pixels` reads only a
  PNG's first frame, and an APNG is no asset of `shuyi-assets`), and
  the module docs say it encodes PNG and APNG files and reads back only
  its own APNGs, any other PNG being read with
  `shuyi_assets::bitmap::convert::png_pixels`; if not, write
  `decode_apng` the same way and leave the module docs, which 2565
  rewrites around it.
- **Either order with 2813:** if 2813 is done, `frame_png/kept.rs`
  already compares a kept still with what its args make; write
  `preview::check` over the parts of it both share (decoding, the pixel
  comparison, the message that ends with the command remaking the
  file), moved into `kept.rs` if they are private there. If not,
  nothing to do (2813 moves what both share when it lands).

**Out (do not do):**
- previews for any other example. An example that wants one adds the
  part, the file and the test line;
- a GIF preview: an APNG keeps the frames' exact colours, and every
  browser the site supports plays it;
- a still picture on a card with no preview, or a still frame in place
  of the preview under reduced motion;
- an example playing inside its card (an `iframe`);
- APNGs smaller than 2514's encoder writes (frame differences, merged
  frames), and video formats;
- a `preview` part in the templates' `shuyi.ron` files;
- any change to `frame-png`'s other flags, the contract's schemas or
  the MCP tool `frame_png`;
- making previews in `pages.yml` (Context says why).

## Implementation steps

1. Read what 2514 added: `Capture`, `Output`, `Options::capture`,
   `GameTools::frames` and `encode_apng`, and their tests. Read 2801's
   `site/` (the Examples template and how each card's entry is kept),
   `.github/workflows/pages.yml`, and its CI job for the site. Read
   `crates/cli/src/project.rs`, `examples/hello-grid/tests/it/files.rs`
   and ADR-0039.
2. Add `Preview` and `Config::preview`, with a parse test.
3. Write `Animation` and `codec::decode_apng`, with round-trip tests
   against `encode_apng`.
4. Write `crates/cli/src/frame_png/preview.rs`: `FILE`, `MAX_BYTES`,
   `options` and `check`. Keep the size test in a private function that
   takes the cap, so a test can pass a small one.
5. Add `--preview` to the `frame_png` wrapper in `lib.rs` and to
   `usage`. Update `frame_png.rs`'s module docs and `lib.rs`'s command
   list.
6. Write the toy's tests (below).
7. Give Hello, Grid its part, its `preview.png`, its test and its
   README lines. Look at the picture in a browser before committing.
8. Write `site/copy-previews.sh`, run it in `pages.yml` and the site's
   CI job, add `site/static/previews/` to `.gitignore`, and change the
   card template, the CSS and `site/README.md`.
9. Settle the clauses with 2738 and 2739, and with 2565.
10. Run the `document` skill for the new part and the new flag, then
    the gates. After the merge, look at the live Examples page.

## Acceptance criteria

- [ ] `examples/hello-grid/preview.png` is an APNG of at most 512 KiB
      that loops forever and shows your piece walk and the foe's step.
      It is reviewed by eye in the PR.
- [ ] In `examples/hello-grid`, `cargo shuyi frame-png --preview`
      remakes it, and `check` passes on the result.
- [ ] Hello, Grid's preview test passes on Windows, Linux and macOS in
      CI's `test` job.
- [ ] On the toy, `check` fails, naming the frame, when one pixel of
      one frame differs, and it fails when the frame count or a delay
      differs. Each failure gives the command that remakes the file.
- [ ] The same frames written with another `png` compression (other
      bytes) pass `check`.
- [ ] Each error of `preview::options` names the problem: no part, no
      `--count`, no `--apng`, an output path, `--target`.
- [ ] Hello, Grid's README shows the preview under its first paragraph
      and lists `preview.png` under *Files made by the tools*.
- [ ] In the site built by CI on the pull request,
      `public/previews/hello-grid.png` is the same file as
      `examples/hello-grid/preview.png`, and the Examples page's HTML
      names it in Hello, Grid's card.
- [ ] On the live site, Hello, Grid's card plays the preview, and with
      reduced motion turned on it shows no moving picture.
- [ ] The clauses with 2738 and 2739, and with 2565, are settled.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Preview` parses from RON, and an unknown field is an error;
  each error of `preview::options`; `decode_apng` round trip with
  `encode_apng` (frames, delays, plays); a frame that isn't whole is an
  error, and so is an APNG that isn't 8-bit RGBA; the size test over
  and under a small cap.
- Integration (the toy, `crates/cli/tests/it/toy.rs`): in a temporary
  folder, with a `Project::new` holding a `preview` part,
  `frame-png --preview` writes a file that `check` passes. Then `check`
  fails, naming what differs, on a file with one pixel of one frame
  changed, with one frame fewer, with another delay, and with no file.
  The same frames written by `png` at another `Compression` pass.
- Example: Hello, Grid's preview test in `tests/it/files.rs`.
- Site: in the site's CI job, a script step compares
  `public/previews/hello-grid.png` with
  `examples/hello-grid/preview.png` (`cmp`), and greps the Examples
  page for `previews/hello-grid.png`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
