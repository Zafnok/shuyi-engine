---
id: "2347"
title: "Styled text: key names that stand out in every hint, and colour roles inside a line"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2347 — Styled text: key names that stand out in every hint, and colour roles inside a line

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each lets one line of text hold parts in different
styles:

- Godot's `RichTextLabel` reads BBCode such as `[color=red]…[/color]`
  (https://docs.godotengine.org/en/stable/tutorials/ui/bbcode_in_richtextlabel.html);
- Unity's TextMeshPro has rich-text tags and style sheets;
- Bevy builds a text from `TextSpan` children, each with its own colour
  (https://docs.rs/bevy/latest/bevy/text/struct.TextSpan.html);
- Unreal's `RichTextBlock` has decorators, and Common UI shows the key
  or button bound to an input action in a style of its own.

**A request from the game:**

- **The game ticket:** game#0250
  ([`0250-key-names-stand-out-in-hints.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0250-key-names-stand-out-in-hints.md)),
  milestone M1 Engine. Nick's playtest (game#0804) found that a
  dialogue box's "b log" hint reads like a word: a key's name is drawn
  in the same style as the words around it. game#0250 asks for "a text span
  with a 'key' style that skins paint", used everywhere a key is named,
  and says to do it in the engine if the engine owns hints. Since the
  carve it does. It is not a blocker of this ticket; it waits for this
  one.
- **The job:** every hint that names a key shows the key's name apart
  from the words around it, on every screen and every target, the
  Playdate's black and white included.
- **Friction:** no friction review: a missing feature, found in a
  playtest.
- **The generic shape:** a line of text made of spans, each with an
  optional role: a key's name, or a palette colour. Text filled from
  `{Action}` placeholders, and help lines, give each key's name the key
  role on their own. A skin paints a role with the palette.
- **Outside tactics:** a platformer's sign post ("{Jump} jump"), a
  farming game's tool hint ("{Use} water"), a puzzle game's bar
  ("{Undo} undo · {Restart} restart"): the same hints, the same keys.

What Shuyi has today (read 2026-10-20):

- `crates/ui/src/text.rs`: `key_text` (line 18) and `fill` (line 38)
  return a plain `String`. A key's name filled in for `{Confirm}` is not
  marked in any way.
- `crates/ui/src/input/help_keys.rs`: `key_name` (134), `all_key_names`
  (149), `cursor_keys_name` (178) and `help_line` (188) return
  `String`. `help_line` joins `format!("{k} {label}")`.
- `crates/ui/src/ctx.rs`: `GameData::key_text` (114) can be overridden,
  so a game can already put brackets around a key's name. `Ctx::text_with`
  (379), `Ctx::label` (386) and `Ctx::text_with_keys` (402) return
  `String`.
- `GlyphBuffer::print` and `print_fg`
  (`crates/ui/src/frame/glyph_buffer.rs:631`, `:636`) print in one
  colour, through the private `print_with`. So do `print_centred` and
  `print_cut` (`crates/ui/src/frame/draw.rs:185`, `:209`) and
  `screens::print_centred` (`crates/ui/src/screens.rs:43`).
- The views hold hints as `String`: `SkipView::answers` and
  `WaitView { key, log_hint }` (`crates/ui/src/dialogue/text_box.rs:220`,
  `:229`), `AskView::keys` (`crates/ui/src/widgets/ask/view.rs:17`), and
  `help` in the views of the options, key bindings, profiles and layout
  picker screens (`crates/ui/src/screens/*/view.rs`). Each is painted in
  `EngineColor::TextDim`, for example by `paint_waiting`
  (`crates/ui/src/dialogue/text_box/glyph.rs:153`). The engine builds
  one `WaitView` itself, in the toy (`crates/ui/src/toy.rs:839`);
  `examples/hello-grid` builds another (`src/screens.rs:234`), and its
  own `QuestionView::answers` (`src/skin.rs:42`) names keys too.
- `EngineColor::Accent` (`crates/ui/src/frame/color.rs:118`) is
  described as "the keys that move the cursor", and the layout picker
  colours its key caps by `KeyRole`
  (`crates/ui/src/screens/layout_picker/view.rs:74`). Nothing else gives
  a key a look.
- There is no markup, rich text or styled span anywhere in the engine.

Planned: 2334 draws proportional text runs in one colour each (its Out
names this ticket); 3006 maps palette roles to 1-bit patterns; 2302
keeps a multi-word key name from being split; 2322 (parked) reads
screens to screen readers.

Why it matters:

- **The game:** game#0250 is in the game's M1 Engine milestone. The
  engine owns the hint text (the help-line builders, `fill`, the text
  box's corner), so the game can't style keys without copying them.
  Nick's choice of look (brackets, a colour, a key-cap box) stays the
  game's: this ticket gives the role and the palette names that paint
  it, and changes no game's look until its palette names them.
- **Other 2D games:** every game with hints wants its keys to stand
  out, and a word in a warning colour inside a line ("HP 3, poisoned").
- **The end goal** (2026-10-19): an agent writes `"{Confirm} start"`
  and the key stands out, with no extra code, on the web and on the
  Playdate.

Two things the comparison found are left out, on purpose:

- **Markup in text files** (`[warning]…[/]` in a language's `ui.ron`
  or in `.dlg`). The engine prints `Ctx::text(key)` raw in about 40
  places, so markup needs a pass over all of them, an ADR on its syntax
  and its escape for `[`, a check that packs keep English's roles, and
  2353's `pseudo_text` copying the markup as it is (2353's Out). That
  is a ticket of its own, for when a game asks.
- **Pauses and speed changes inside a dialogue line.** No game asks for
  them, and the parked visual novel tickets own text reveal (2722,
  2723).

## Nick input

None. How the game's keys look is game#0250's decision (its own
`nick_input`). The engine leaves every game's look as it is until the
game's palette names `key`.

## Scope

**In:**

- **An ADR** (the `write-adr` skill, the next free number): styled
  text.
  - Views hold text that names keys as `StyledText`.
  - A span has an optional role, and a skin paints a role from the
    palette.
  - A role never changes how many cells a span takes. Anything extra a
    key shows, such as brackets, comes from `GameData::key_text` as text
    and is measured as text.
  - Markup in text files is a later decision.
  - It cites ADR-0002 §4 (skins) and ADR-0020 (views hold text the skin
    shows). `docs/adr/README.md` gets the row.
- **`shuyi_ui::text`** (`crates/ui/src/text.rs`), `no_std` with
  `alloc`:
  - `TextRole`, `Copy + Eq + Hash`:
    - `Key`: a key's or a button's name;
    - `Engine(EngineColor)`: an engine colour (`Warning`, `Highlight`,
      and so on);
    - `Palette(&'static str)`: a colour of the game's palette, by its
      `ColorSet::name`;
  - `TextSpan { text: String, role: Option<TextRole> }`;
  - `StyledText`, a list of spans:
    - `new`, `push(text, role)`, `push_plain`, `push_key`, `append`.
      A push next to a span of the same role joins it, and an empty
      push adds nothing, so two texts that read the same with the same
      roles are equal;
    - `spans()`, `plain() -> String`, `is_empty()`, and `Display` (the
      plain text);
    - `width()`: `shuyi_assets::lang::text_width` of the plain text;
    - `cut(w)`: as `frame::draw::cut`, with its `…` in the role of the
      span it ends;
    - `From<String>` and `From<&str>`: one plain span.
  - `fill_styled(text, args, key_args, lookup) -> StyledText`: what
    `fill` does, with each value the lookup gives
    (`GameData::key_text`) in a `Key` span, and each value of `key_args`
    (a key's name handed in as a value) in a `Key` span too. Values in
    `args` are plain. `fill` becomes `fill_styled(…).plain()`: one
    implementation, so the two never drift. The shape of `key_args` is
    the implementer's; say it in the completion notes.
  - **Either order with 2302 and 2348,** which both change `fill`.
    Whichever lands second keeps `fill` as `fill_styled(…).plain()`
    and makes its change in `fill_styled`. If 2302 is done,
    `fill_styled` keeps its no-break join of a key name's words, inside
    the `Key` span. If 2348 is done, `fill_styled` takes its `lang` and
    typed arguments and formats a message as its `fill` does, each key
    name in it still a `Key` span, and `Ctx`'s styled functions take
    the same arguments as `text_with`.
  - **Either order with 2390:** if 2390 is done, `fill` walks
    `shuyi_core::text::split_tokens`: `fill_styled` keeps that walk (a
    `Text` part plain, a `Token` filled, an `Unclosed` part copied as it
    is) and `fill` becomes its `plain()`; if not, build `fill_styled` on
    `split_tokens` all the same, never on a `split_once('{')` loop of its
    own, and 2390 then has nothing to rewrite in `fill`.
- **`shuyi_ui::input::help_keys`:** `help_line_styled(hints) ->
  StyledText`, each key in a `Key` span, the labels and `SEPARATOR`
  plain. `help_line` becomes its `plain()`.
- **`Ctx`** (`crates/ui/src/ctx.rs`): `text_styled(key, args)`,
  `text_styled_with_keys(key, keys, args, key_args)` and
  `label_styled(&Label)`. `text_with`, `text_with_keys` and `label` keep
  returning `String`, and equal the styled ones' `plain()`.
- **The palette** (`crates/ui/src/frame/color.rs`):
  - two optional palette names, as constants: `key` (a key's glyphs)
    and `key_bg` (the cells behind a key). `required_names` doesn't
    change, so no palette breaks. A theme has them when its default
    palette does (`theme_problems`, `crates/assets/src/palette.rs:118`,
    already asks a theme for the same names);
  - `Palette::text_colors(role: Option<TextRole>, fg: Rgb) -> (Rgb,
    Option<Rgb>)`, the glyph colour and the background if the role
    sets one:
    - `None`: `(fg, None)`;
    - `Key`: `key`, else `fg`; and `key_bg`, else `None`;
    - `Engine(e)`: `engine(e)`, no background;
    - `Palette(name)`: `lookup(name)`, else `fg`; no background.
- **The buffer:**
  - `GlyphBuffer::print_styled(x, y, &StyledText, colors: impl
    Fn(Option<TextRole>) -> (Rgb, Option<Rgb>)) -> u16`, through the
    same `print_with` as `print_fg`. A span keeps each cell's
    background unless its colours give one; giving one makes a
    see-through cell solid. `GlyphBuffer` stays free of `Palette`;
  - `screens::print_centred_styled(buf, y, &StyledText, colors)`, beside
    `screens::print_centred`, cutting and centring as it does;
  - a painter that prints a hint with `print` today, which sets the
    background and makes the cell solid, passes `colors` that give its
    old background to every span whose role gives none. Each cell then
    ends as `print` left it, so no snapshot changes until a palette
    names `key`;
  - no renderer changes: a role ends up as a cell's `fg` and `bg`, which
    `shuyi-app`, `frame-png` and the one-bit renderer already draw.
- **Every engine hint that names a key** holds and paints `StyledText`:
  - the text box: `WaitView::key`, `WaitView::log_hint` and
    `SkipView::answers`, painted by `text_box/glyph.rs`
    (`paint_waiting`, and the skip question at line 63). The toy's
    `Talk::view` (`crates/ui/src/toy.rs:838`) builds its `WaitView` with
    `text_styled`;
  - the question widget: `AskView::keys`, built at
    `crates/ui/src/widgets/ask.rs:153`, measured and painted in
    `widgets/ask/glyph.rs` (`rect` at line 40, and the keys' line in
    `paint` at line 64);
  - the `help` of `OptionsView`, `KeyBindingsView`, `ProfilesView` and
    `LayoutPickerView`, the functions that build it
    (`OptionsScreen::help`, `options.rs:830`; `KeyBindingsScreen::help`,
    `key_bindings.rs:783`; `ProfilesScreen::help`, `profiles.rs:391`;
    `LayoutPickerScreen::help`, `layout_picker.rs:147`), and
    `screens::text_keys_help` (`screens.rs:27`);
  - key names handed in as values get the key role: `text_keys_help`'s
    `{done}`, `{delete}` and `{cancel}`; the layout picker's `{up}`,
    `{down}` and `{confirm}` (`layout_picker.rs:163`); the key bindings
    screen's `{key}` and `{clear}` (`key_bindings.rs:795`, `:819`);
  - the help lines of `templates/blank` (`crates/game/src/title.rs`,
    which prints its lines with `screens::print_centred`: its help line
    moves to `print_centred_styled`),
    `templates/grid` (`crates/game/src/stage.rs`) and
    `examples/hello-grid` (the views in `src/skin.rs` and its `help`
    painter at line 79);
  - `hello-grid`'s other hints: its `WaitView` (`src/screens.rs:234`,
    built with `text_styled` as the toy's is) and its
    `QuestionView::answers` (`src/skin.rs:42`, built at
    `src/screens.rs:320` and painted at `src/skin.rs:148`);
  - the palettes of both templates and of `hello-grid`
    (`files/palette.ron`) get a `key` colour, so a new game's keys
    stand out.
- **check-text** (`crates/cli/src/check_text.rs`, `bracket_of` at line
  205): `text_styled` and `text_styled_with_keys` are key calls like
  `text_with`, and `help_line_styled` and `push_plain` are sinks like
  `help_line` (`print_styled` and `print_centred_styled` already are,
  by their `print` prefix). The test at line 818 lists them.
- **The `input-actions` skill**, rule 2: help bars use
  `help_line_styled` and text uses `Ctx::text_styled`, so a key's name
  gets the key role, which a skin paints with the palette's `key` and
  `key_bg`.
- **For the game:** the view fields above change type, and so do the
  four screens' `help` functions and `text_keys_help`. The game builds
  `WaitView` and `SkipView` itself, so its PR that moves the pin builds
  them with `ctx.text_styled`. `text_with`, `fill`, `help_line`,
  `key_name` and `GameData::key_text` keep their signatures. With no
  `key` in its palette, the game's snapshots don't change; game#0250
  picks its look. Say all this in the PR.
- **Public types (ADR-0039):** `TextRole` and `TextSpan` follow it
  (`#[non_exhaustive]` and constructors).
- **Either order with 2301:** if 2301 is done, `StyledText::width` and
  `cut` take its `CellWidths`, and `print_styled` advances two cells
  for a wide glyph as `print` does. If not, 2301 does it: its step 3
  changes the printing that `print_styled` shares with `print`, and
  its step 4 changes `text_width` and the layout code that counts
  characters, `width` and `cut` among them.
- **Either order with 2330:** if 2330 is done, its `Text` node paints
  with `Ctx::label_styled` and `print_styled` in this PR; if not, 2330
  does it.
- **Either order with 2766** (parked, the message log): if 2766 is
  done, its `MessageLog` lines hold a `StyledText` in place of one role
  each (a line's role becomes the role of its one span), its widget
  wraps a line over its spans, each span keeping its role, and the
  widget and the history screen paint with `print_styled`; if not,
  nothing to do (2766 builds its lines on `StyledText`, as its clause
  says).
- **1-bit (3006 is done, ADR-0026):** `key` and `key_bg` are palette
  roles, so `hello-grid`'s pattern map (`palette@onebit.ron`) gives them
  patterns, lists them among its `text` roles, and puts `key` in a
  `distinct` group with `text`; its one-bit check test still passes.
- **Size:** if the PR would pass about 600 changed lines (snapshots not
  counted), step 7 (the templates, the rest of `hello-grid` and the
  `key` in their palettes) goes to a follow-up ticket written with the
  `write-ticket` skill (After MVP, blocked by 2347). `hello-grid`'s
  `WaitView` stays in this PR, since the workspace must build. The PR
  says so.

**Out (do not do):**

- markup in text files (`[role]…[/]` in `ui.ron`, packs or `.dlg`),
  and Yarn markup (2202): a ticket of its own, with its ADR, when a game
  asks. That ticket also makes 2353's `pseudo_text` copy the markup as
  it is;
- pauses and speed changes inside a dialogue line. The text box's pages
  stay `String` (`TextView::lines`);
- wrapping a `StyledText` (a text box or a tip that wraps): a follow-up
  with 2302's rules, when a box needs it. The message log's lines are
  the exception: whichever of this ticket and 2766 lands second wraps
  them (the 2766 clause under In);
- styled spans inside 2334's proportional runs, which draw one colour
  each;
- bold, italic, inline icons or pictures of buttons, motion per glyph,
  spans the pointer can click;
- the debug view's hint and help lines (`crates/ui/src/debug.rs:40`,
  `:165`, `crates/ui/src/debug/camera.rs:164`): debug tools are English
  only and not for players;
- the layout picker's legend and keyboard picture, and the key bindings
  screen's table: there the keys are the subject, coloured by `KeyRole`
  and their own rules;
- how a screen reader reads a `Key` span (2322, parked);
- the game's look or its palette (game#0250).

## Implementation steps

1. Read ADR-0002 §4, ADR-0020, game#0250, and the files named above.
2. Write the ADR.
3. Write `TextRole`, `TextSpan`, `StyledText`, `fill_styled` (with
   `fill` built on it), `help_line_styled` and the `Ctx` functions, with
   their unit tests.
4. Add `Palette::text_colors`, `GlyphBuffer::print_styled` and
   `screens::print_centred_styled`.
5. Switch the hints over, one view at a time. Tests that compared a
   hint with a string compare its `plain()`. `hello-grid` is in the
   workspace and builds a `WaitView`, so switch its `Talk::view` in the
   same step as `WaitView`.
6. Check that every existing engine snapshot is unchanged: the toy's
   palette (`crates/ui/src/toy.rs:387`) has no `key`. Add the snapshot
   with a `key` palette and the 1-bit test.
7. Switch the templates and the rest of `hello-grid` (its help lines
   and its `QuestionView`), add `key` to their palettes, and review
   the snapshot changes.
8. Update check-text and the `input-actions` skill.
9. Run the gates, the template job's checks included (`cargo fmt
   --all` doesn't format `templates/`).

## Acceptance criteria

- [ ] With a palette that has no `key` (the toy's), every existing
      engine snapshot is unchanged.
- [ ] With a palette that has `key` and `key_bg`, at values no other
      name in it has (the legend names a colour by `Palette::name_of`),
      a snapshot of the options screen shows its help line's key cells
      as `fg:key bg:key_bg` in the legend, and the words as before
      (reviewed).
- [ ] A test with the toy's `Ctx` builds each switched hint (the toy's
      text box wait corner, a yes/no question's keys, the four screens'
      help lines, `text_keys_help`) and finds that its `Key` spans are
      exactly the names `key_name`, `all_key_names`, `cursor_keys_name`
      or the fixed keys give, in order.
- [ ] A text box test paints a `SkipView` whose answers hold a `Key`
      span and finds the key's cells in the key colours.
- [ ] For every key of the toy's English text,
      `ctx.text_styled(key, &[]).plain() == ctx.text_with(key, &[])`.
- [ ] With a palette whose `key` is `#ffffff`, `key_bg` `#000000` and
      `text_dim` `#000000`, on a `#ffffff` panel, the one-bit reference
      renderer (`crates/ui/src/frame/onebit/reference.rs`) draws a help
      line's key glyphs white on black and its words black on white.
- [ ] `hello-grid`'s snapshots change only in key cells: its help
      lines, its text box's wait corner and its leave question
      (reviewed).
- [ ] check-text's unit test treats `text_styled` and
      `text_styled_with_keys` as key calls and `help_line_styled` and
      `push_plain` as sinks.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `StyledText` (joining on push, `append`, `cut` and the role of
  its `…`, `width`, `Display`); `fill_styled` (lookup values and
  `key_args` in `Key` spans, `args` plain, an unclosed `{` as `fill`
  treats it); `text_colors` for each role, with and without `key` and
  `key_bg`; `print_styled` keeping a background, setting `key_bg`, and
  making a see-through cell solid.
- Property: over random pushes, the spans' texts join to `plain()` and
  no two neighbouring spans share a role;
  `s.cut(w).plain() == cut(&s.plain(), w)`;
  `s.width() == text_width(&s.plain())`; `print_styled` writes the same
  glyphs as `print_fg` of the plain text.
- Kept: `fill`'s and `help_line`'s existing tests pass unchanged, which
  holds their plain output to today's now that both are built on the
  styled functions.
- Snapshot / integration: the options screen with a `key` palette; the
  1-bit render; `hello-grid`'s snapshots.

## Completion notes

*(Filled in by the session that completes the ticket.)*
