---
id: "2813"
title: "Pictures in the book: shots of Hello, Grid made by frame-png from its shuyi.ron, checked in and tested equal, shown by a shot shortcode and in the Markdown twins"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2808"]
nick_input: none
completed:
---

# 2813 — Pictures in the book

## Context

Nick, 2026-10-28 ([`docs/decisions.md`](../../docs/decisions.md),
*pictures in the book, a playground, and a site that keeps up with the
tickets*): the book should show "how things play together". Today the
book (2802, `site/content/learn/book/`) is text and code. A reader
learns what a screen, a skin or a target is without ever seeing one.

What there is:

- `frame-png` (`crates/cli/src/frame_png.rs`) starts a game in the test
  Harness, plays `--keys`, `--pad`, `--wait` and `--crank` steps, and
  writes the frame as a PNG. `--target ID` paints it as that target's
  display shows it (the Playdate's one-bit patterns). The Harness's
  frames are exact: the same steps give the same pixels on every OS.
- Hello, Grid (`examples/hello-grid/`) has a title, a stage on a grid,
  menus, a dialogue (`files/intro.dlg`), an options screen with
  rebinding, two map looks (`MapLook::Glyphs` and `MapLook::Tiles`,
  `src/settings.rs`) and a one-bit palette (`files/palette@onebit.ron`).
  Every concept the book teaches shows on one of its screens.
- Files the tools make are checked in, with a test that fails when one
  is stale: `files/tiles.png` from `glyph_tiles`, checked by
  `the_tileset_is_what_glyph_tiles_makes_of_the_codes_looks`
  (`examples/hello-grid/tests/it/files.rs`). 2811 does the same for an
  animated `preview.png` on the Examples cards, and says why a checked-in
  file beats one made at deploy: the reviewer sees the new picture in
  the PR, and a stale one fails its own PR, not `main`. The same holds
  here.
- The book's code comes from `site/snippets/` through the `snippet`
  shortcode (`site/templates/shortcodes/snippet.md`), and a missing file
  or anchor fails the build (`site/README.md`, *The rules*).
- 2808 makes a Markdown twin of every Learn page (`cargo xtask llms`,
  `crates/xtask/src/llms.rs`) and fails on a shortcode it doesn't know.
  A new shortcode needs its expansion there, or the deploy fails. That
  is why 2808 comes first.

**Either order with 2811:** if 2811 is done, `frame_png/preview.rs`
already compares a kept file with what its args make; move the parts
both checks share (decoding, the pixel comparison, the message that
ends with the command remaking the file) into `frame_png/kept.rs`, and
have both call it. If not, write `frame_png/kept.rs` with the still
check alone.

**From 2818 (done):** the tutorial's pictures are `cargo xtask
tutorial-tries`'s and stay so (their edits have no home in `shuyi.ron`). Share
their CSS (`.prose figure`, `.try-pair`) with `shot` if that's simple.

**From 2819 (done):** the book's chapters have their own pictures of Moon Garden
(`step = "book"` in `site/tutorial/tries.toml`). Add these shots beside them.
Chapter 9 has no picture yet: a target's own display is this ticket's.

## Nick input

None.

## Scope

**In:**
- **A `shots` part in `shuyi.ron`** (`crates/cli/src/project.rs`):
  - `Config::shots: BTreeMap<String, Shot>`, empty by default. A name
    is lowercase letters, digits and `-`; any other is an error naming
    it. `Shot { args: Vec<String> }` holds `frame-png`'s arguments after
    the output path, with `deny_unknown_fields` like the other parts.
    It may grow (ADR-0039): `#[non_exhaustive]`, built with
    `Shot::new(args)`.
  - Shot `<name>` is always `shots/<name>.png` in the project's root.
  - `Config` isn't `#[non_exhaustive]`, so the new field is an API
    change: CI's `api` job reports it, and the completion notes carry
    its `Breaking:` line (ADR-0038).
  - If `crates/cli/tests/formats/shuyi.ron` exists (a fixture that sets
    every part of `Config`), it gets the part too.
- **`frame_png::kept`** (`crates/cli/src/frame_png/kept.rs`):
  - `MAX_SHOT_BYTES` (256 KiB).
  - `shot_options(project, name, game_options) -> Result<Options,
    String>`: parses the shot's args with `parse_args`, with
    `shots/<name>.png` as the output. Errors name `shuyi.ron`'s
    `shots`, and the shot: no such shot (with the names there are),
    args that don't parse, an output path in the args. `--target` is
    allowed: chapter 9 shows a target's screen.
  - `check_shot(project, game: &dyn GameTools, name) -> Result<(),
    String>`: renders the shot, reads the file, decodes it with the
    function that decodes a PNG to RGBA in `shuyi-cli` on the day
    (`codec::decode_png` today), and fails on the first difference,
    which it names: the file is missing, over `MAX_SHOT_BYTES` (its
    size, and "try `--scale 1`"), another size, or the first pixel that
    differs (its x and y). Each message ends with the command that
    remakes it: `<command> frame-png --shot <name>`, with
    `project.config.command()`.
  - `check_shots(project, game)`: every shot, and a file in `shots/`
    that no shot names is an error too (a shot removed from
    `shuyi.ron` leaves no orphan).
- **`frame-png --shot NAME` and `frame-png --shots`** (the `frame_png`
  wrapper in `crates/cli/src/lib.rs`): the first writes one shot, the
  second every shot, each with `frame-png`'s usual summary line. Either
  with any other argument is an error with the usage. `frame_png::usage`
  lists both.
- **Hello, Grid's shots**, in its `shuyi.ron` with a comment like the
  other parts' ("`cargo shuyi frame-png --shots`: `shots/`, the book's
  pictures; a test fails when one is stale"), each at `--scale 1`.
  Find each script with the MCP session tools (`start`, `step`,
  `frame_png` on `examples/hello-grid`). At least:
  - `title`: the title screen (the quick start, chapter 2);
  - `stage`: the stage after a few moves, your piece and the foe on the
    grid (chapters 2 and 6);
  - `menu`: a menu open over the stage, the stage still drawn below it
    (chapter 2, screens on a stack);
  - `dialogue`: the intro's text box (chapter 2 or 4);
  - `options`: the options screen, a key being rebound (chapter 3);
  - `tiles`: the stage of `stage`, in the tile look (chapter 4, one
    view and two skins);
  - `playdate`: the stage of `stage` with `--target playdate`
    (chapter 9).
  - `shots/*.png` made with `cargo shuyi frame-png --shots` in that
    folder, and a test in `tests/it/files.rs` that calls
    `frame_png::kept::check_shots(&project(), &HelloTools)`.
  - Its README lists `shots/` under *Files made by the tools*, with the
    command that remakes them.
- **The shots on the site:**
  - `site/copy-shots.sh` copies each `examples/<name>/shots/*.png` to
    `site/static/shots/<name>/`. `pages.yml` runs it before
    `zola build`, and so does the CI job that builds the site on pull
    requests. `site/static/shots/` goes in `.gitignore`.
  - **A `shot` shortcode** (`site/templates/shortcodes/shot.html`):
    `{{ shot(example="hello-grid", name="menu", alt="…") }}`, with an
    optional `caption`. It is a `<figure>` holding an `<img>` with the
    file's `width` and `height` from `get_image_metadata` (a missing
    file fails the build), `loading="lazy"`, the `alt`, and the CSS
    `max-width: 100%; height: auto; image-rendering: pixelated`, and a
    `<figcaption>` with the caption when there is one. `alt` is
    required: a shortcode without it fails the build.
  - **The chapters show them**, each where its text speaks of what the
    picture shows, with alt text that says what is on screen: the list
    above says which chapter takes which shot. Two shots side by side
    (`stage` and `tiles`, `stage` and `playdate`) are two shortcodes in
    a row, in a `figure`-pair class the stylesheet lays out side by
    side, one above the other under 600 px.
  - `site/README.md`: the shot shortcode under *The rules*, how an
    example gets shots, and that `zola serve` shows them after
    `bash copy-shots.sh`.
- **The Markdown twins** (`crates/xtask/src/llms.rs`, 2808): the `shot`
  shortcode becomes `![<alt>](<absolute URL of the PNG>)`, and its
  caption, when it has one, a line in italics under it. A shot whose
  file isn't in the built site is an error with the page and line, as
  2808's other link errors are. A fixture test covers it beside 2808's.
- **The document skill** for the new part, the new flags and the
  shortcode.

**Out (do not do):**
- animated pictures in the book. 2811 makes the APNG previews of the
  Examples cards; the book's pictures are stills;
- shots of any other example or template. One that wants shots adds the
  part, the files and the test line;
- making shots in `pages.yml` (Context says why);
- pictures drawn by hand, or a diagram tool;
- any change to `frame-png`'s other flags, the contract's schemas or the
  MCP tool `frame_png`.

## Implementation steps

1. Read `crates/cli/src/frame_png.rs` (`parse_args`, `Options`,
   `GameTools`), `crates/cli/src/project.rs`,
   `examples/hello-grid/tests/it/files.rs`, the `snippet` shortcode,
   `crates/xtask/src/llms.rs` and its tests, `site/README.md`,
   `pages.yml` and the CI job that builds the site. If 2811 is done,
   read `frame_png/preview.rs` too.
2. Add `Shot` and `Config::shots`, with parse tests (a good part, an
   unknown field, a bad name).
3. Write `frame_png/kept.rs` (settle the clause with 2811), then
   `--shot` and `--shots` in `lib.rs` and the usage.
4. Write the toy's tests (below).
5. Find Hello, Grid's shot scripts with the MCP tools, add the part,
   make the files, add the test and the README lines. Look at every
   picture before committing.
6. Write `site/copy-shots.sh`, the shortcode and its CSS, run the script
   in `pages.yml` and the CI site job, and put the shots in the
   chapters.
7. Add the shortcode's expansion to `cargo xtask llms`, with its test.
8. Run the `document` skill, then the gates. After the merge, look at
   the live book, and at one chapter's twin.

## Acceptance criteria

- [ ] `examples/hello-grid/shots/` holds the seven shots above, each at
      most 256 KiB, each reviewed by eye in the PR.
- [ ] In `examples/hello-grid`, `cargo shuyi frame-png --shots` remakes
      them, and `check_shots` passes on the result, on Windows, Linux
      and macOS in CI's `test` job.
- [ ] On the toy, `check_shot` fails, naming x and y, when one pixel
      differs, and fails on another size, a missing file and an orphan
      file. Each failure gives the command that remakes the file.
- [ ] Each error of `shot_options` names the problem: no such shot, a
      bad name, args that don't parse, an output path.
- [ ] In the site built by CI on the pull request, each chapter named
      above has its `<img>`, with alt text, and its file is the same as
      the example's (`cmp`).
- [ ] A `shot` with no `alt`, or naming a missing file, fails the site's
      build.
- [ ] Each twin of a chapter with a shot has the Markdown image at the
      site's address, and `cargo xtask llms` fails on a shot whose file
      isn't there (fixture test).
- [ ] The book's chapter is updated (2802), or the completion notes say
      why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Shot` parses from RON, an unknown field and a bad name are
  errors; each error of `shot_options`; the size test over and under a
  small cap.
- Integration (the toy, `crates/cli/tests/it/toy.rs`): in a temporary
  folder, with a `Project::new` holding two shots, `frame-png --shots`
  writes files that `check_shots` passes; then it fails on one changed
  pixel, another size, a missing file and an orphan, each naming what
  differs.
- Example: Hello, Grid's shots test in `tests/it/files.rs`.
- `llms.rs`: a fixture page with a shot, its twin's image line, and the
  error on a missing file.
- Site: in the site's CI job, a step compares each copied shot with the
  example's (`cmp`) and greps a chapter for `shots/hello-grid/`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
