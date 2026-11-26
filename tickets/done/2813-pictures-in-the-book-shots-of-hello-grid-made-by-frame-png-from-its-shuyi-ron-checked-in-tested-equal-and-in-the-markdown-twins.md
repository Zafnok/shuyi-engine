---
id: "2813"
title: "Pictures in the book: shots of Hello, Grid made by frame-png from its shuyi.ron, checked in and tested equal, shown by a shot shortcode and in the Markdown twins"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2808"]
nick_input: none
completed: 2026-11-26
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
  (`examples/hello-grid/tests/it/files.rs`). 2811 (done) did the same for
  an animated `preview.png` on the Examples cards, and says why a checked-in
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

**From 2811 (done):** `frame_png/preview.rs` already compares a kept
file with what its args make: `check` reads the file, `compare` names
the first difference (frame count, size, the first pixel that differs,
delay), and each message of the file's ends with the command remaking
it. Move the parts both checks share (the pixel comparison, the message
that ends with the command) into `frame_png/kept.rs`, and have both call
it; `codec::decode_apng` stays in `codec.rs`.

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
  - `Config` is `#[non_exhaustive]` (it gained that after this ticket
    was written), so the new field is an addition, not a break.
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
  - `title`: the title screen (the quick start, chapter 6);
  - `stage`: the stage after a few moves, your piece and the foe on the
    grid (chapters 5 and 6);
  - `leave`: the "Leave the stage?" question pushed over the stage, the
    stage still drawn below it (chapter 6, *Asking the screen below*;
    Hello, Grid has no menu over its stage);
  - `dialogue`: the intro's text box (chapter 6 or 2);
  - `options`: the options screen, a key being rebound (chapter 4);
  - `tiles`: the stage of `stage`, in the tile look (chapter 2, one
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
   `pages.yml` and the CI job that builds the site, and
   `frame_png/preview.rs` (2811).
2. Add `Shot` and `Config::shots`, with parse tests (a good part, an
   unknown field, a bad name).
3. Write `frame_png/kept.rs` (over what it shares with
   `frame_png/preview.rs`), then
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

- [x] `examples/hello-grid/shots/` holds the seven shots above, each at
      most 256 KiB, each reviewed by eye in the PR.
- [x] In `examples/hello-grid`, `cargo shuyi frame-png --shots` remakes
      them, and `check_shots` passes on the result, on Windows, Linux
      and macOS in CI's `test` job.
- [x] On the toy, `check_shot` fails, naming x and y, when one pixel
      differs, and fails on another size, a missing file and an orphan
      file. Each failure gives the command that remakes the file.
- [x] Each error of `shot_options` names the problem: no such shot, a
      bad name, args that don't parse, an output path.
- [x] In the site built by CI on the pull request, each chapter named
      above has its `<img>`, with alt text, and its file is the same as
      the example's (`cmp`).
- [x] A `shot` with no `alt`, or naming a missing file, fails the site's
      build.
- [x] Each twin of a chapter with a shot has the Markdown image at the
      site's address, and `cargo xtask llms` fails on a shot whose file
      isn't there (fixture test).
- [x] The book's chapter is updated (2802), or the completion notes say
      why not.
- [x] All gates in the `run-gates` skill pass.

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

**What was done.**

- `shuyi.ron` has a `shots` part (`Config::shots`, `project::Shot`,
  `Shot::check_name`); a bad name fails `Config::parse`, naming it.
- `frame_png::kept` (`crates/cli/src/frame_png/kept.rs`): `MAX_SHOT_BYTES`,
  `file`, `shot_options`, `check_shot`, `check_shots`, `orphans` and
  `orphan_problem`, with the size check, the first differing pixel and the
  "remake it" line shared with `frame_png::preview`, which now calls them.
  `frame_png::frame_of` paints the one frame `run` writes, so the check and
  the command render alike, a one-bit target's patterns too.
- `frame-png --shot NAME` and `frame-png --shots`, in the usage too.
- Hello, Grid's seven shots in `examples/hello-grid/shots/` (each 3–5 KB),
  every one looked at by eye; its `shuyi.ron` part, the test
  `the_shots_are_what_frame_png_makes_of_their_part`, and the README line.
- The site: `site/copy-shots.sh` (run by `check-build.sh`, CI's `site` job,
  `pages.yml` and `cargo xtask book-pad`), the `shot` shortcode, its CSS
  (`.figure-pair` shares `.try-pair`'s rules), and `check-build.sh` checks
  each built shot against the example's with `cmp`, each page that shows
  one for its `<img>` with alt text, and that a chapter shows a Hello,
  Grid shot. The shots are in the quick start (`title`), chapter 2
  (`stage` beside `tiles`), chapter 4 (`options`), chapter 5 (`stage`),
  chapter 6 (`dialogue`, `leave`) and chapter 9 (`stage` beside
  `playdate`); chapter 8 says how a game keeps its own shots.
- `cargo xtask llms` expands `shot` to `![alt](URL)` and its caption in
  italics under it, with unit tests and a fixture test of a twin.
- Docs: `site/README.md` (*Preview it*, the table, *The rules*, *The docs
  for agents*, *The checks*), the `shuyi-ship` skill, the docs map's row
  for the book.

**Deviations.**

- The ticket said `Config` isn't `#[non_exhaustive]`; it is now, so the
  new field is no break (Scope fixed above).
- `menu` is `leave`: Hello, Grid opens no menu over its stage. Its
  overlay is the "Leave the stage?" question, which is what chapter 6's
  *Asking the screen below* teaches; its end menu replaces the stage
  rather than covering it (Scope fixed above). The title is in the quick
  start only, and `stage` in chapters 2, 5 and 9; chapter 6 has two
  shots of screens over the stage.
- A shot is one frame: `shot_options` refuses `--count`, as the book's
  pictures are stills (frames over time are the preview's).
- A file in `shots/` that no shot names fails `check_shots` and
  `--shots` with what to do, but no command removes it: it may be the
  game's own (the `shuyi-ship` skill's reveal example keeps
  `shots/preview-steam.webp`). `--shots` on a project with no `shots`
  writes nothing and exits 2.
- `cargo xtask llms` checks a shot's file in `site/static/shots/`, which
  Zola copies into the build, as it checks a try's picture: the same file,
  and `--full-only` checks it too.
- Size: 1,080 counted lines; it would have split cleanly between the
  tool (the part, `kept`, the flags, Hello, Grid's shots) and the site
  (the shortcode, the chapters, the twins).

Behaviour: shuyi-cli: `frame-png` with `--preview` among other arguments now says "--preview and --shots take no other argument, and --shot only the shot's name" (it said "--preview takes no other argument"); a `shots` key in `shuyi.ron` that isn't lowercase letters, digits and `-` fails to parse.

No break: `Config` is `#[non_exhaustive]`, and every other item is new.
`gh search code --repo Zafnok/visions-of-shuyi` for `takes no other
argument`, `frame_png::preview` and `shots` finds nothing of the game's
code that reads them (its "shots" are cinematics').

Game: nothing to adopt, the game gets the new part and flags with its next pin move and needs none of them.

Book: the chapters above show the shots, and chapter 8 (`testing.md`) teaches the `shots` part.

Skills: `skills/shuyi-ship/SKILL.md` teaches the `shots` part beside the preview.
