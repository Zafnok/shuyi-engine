# ADR-0122: Styled text: a line of spans, each with an optional role that the skin paints from the palette

- **Status:** Accepted
- **Date:** 2026-11-16
- **Related tickets:** 2347; 3215 (key caps, which will amend §4), 2766 (the message log's lines), 2330 (a `Text` node), 2322 (screen readers)

## Context

A hint that names a key ("f yes / d no", "Enter next", "b log") prints
the key's name in the same colour as the words around it, so a one-letter
key reads like a word (game#0250, from the playtest game#0804). Every
engine compared on 2026-10-20 lets one line hold parts in different
styles: Godot's `RichTextLabel` reads BBCode, Unity's TextMeshPro has
rich-text tags, Bevy builds a text from `TextSpan` children each with its
own colour, and Unreal's Common UI draws the key bound to an action in a
style of its own.

Shuyi had none of it: every function that builds hint text returned a
`String` (`text::fill`, `help_keys::help_line`, `Ctx::text_with`), the
views held `String`s, and `GlyphBuffer::print` prints in one colour. By
the time the text reached the skin, a key's name was no longer told apart
from the words.

The rules this must keep:

- graphics are a skin (ADR-0002 §4): logic says what a hint is, the skin
  decides how it looks;
- views hold text the skin shows, already words or a key the skin
  words (ADR-0020);
- code names colours by palette name, never by value (game ADR-0012);
- nothing changes how a game looks until the game asks: the game's
  snapshots and every engine snapshot stay as they are.

## Decision

1. **`shuyi_ui::text::StyledText`** is a line of spans. A
   `TextSpan { text, role: Option<TextRole> }` is a run of text in one
   role, or in none (plain). `StyledText` keeps its spans in a normal
   form: a push next to a span of the same role joins it and an empty
   push adds nothing, so two texts that read the same in the same roles
   are equal (`==`). Its `plain()` (and `Display`) is the text with no
   roles; `width` and `cut` measure and cut it as `text_width` and
   `frame::draw::cut` do the plain text, the `…` of a cut taking the
   role of the first character it stands for.
2. **`TextRole`** (`#[non_exhaustive]`, ADR-0039 kind 6) says what a span
   is, never its colour:
   - `Key`: a key's or a button's name;
   - `Engine(EngineColor)`: one of the engine's colours (`Warning`, ...);
   - `Palette(&'static str)`: a colour of the game's palette by name.
3. **Who gives a key the `Key` role.** The text says it and the engine
   does the rest: `fill_styled` puts in a `Key` span each value the
   lookup gives (a `{Confirm}` placeholder, `GameData::key_text`) and
   each value handed in as `key_args` (a key's name a screen passes as
   a value, `{done}`), with its words joined by no-break spaces as
   ticket 2302 joins the lookup's; `args` stay plain.
   `help_keys::help_line_styled` puts each hint's key in a `Key` span.
   `Ctx::text_styled`, `text_styled_with_keys` and `label_styled` are
   `text_with`, `text_with_keys` and `label` styled. Each `String`
   function is its styled twin's `plain()` (`fill` is
   `fill_styled(…).plain()`), so the two never drift, and a game that
   keeps the `String` ones keeps its text.
4. **A role never changes how many cells a span takes,** nor its text.
   Anything a key shows beyond its name, such as brackets, comes from
   `GameData::key_text` as text and is measured as text. (3215's key
   caps are to amend this rule, for a cap's box.)
5. **The skin paints a role from the palette.**
   `Palette::text_colors(role, fg) -> (Rgb, Option<Rgb>)` gives a span's
   glyph colour and, if the role sets one, its background: `Key` is the
   palette's `key` and `key_bg` (`frame::color::KEY`, `KEY_BG`), each
   optional and falling back to `fg` and no background; `Engine(e)` is
   `engine(e)`; `Palette(name)` is `lookup(name)`, else `fg`.
   `Palette::text_colors_on(fg, bg)` is the same with `bg` for a role
   that gives none, what a painter that printed with `print(fg, bg)`
   passes so each cell ends as `print` left it. `key` and `key_bg` are
   not in `required_names`: no palette breaks, and a palette without
   them paints keys as before. A theme has them when the default
   palette does (`theme_problems` asks for the same names).
6. **`GlyphBuffer` stays free of `Palette`.**
   `GlyphBuffer::print_styled(x, y, &StyledText, colors)` takes the
   colours as a function of the role and prints each span through the
   same `print_with` as `print_fg`: a span keeps each cell's background
   unless its colours give one, and giving one makes a see-through cell
   solid. `screens::print_centred_styled` centres and cuts as
   `screens::print_centred` does. A role ends as a cell's `fg` and `bg`,
   which every renderer already draws, the one-bit renderer by the
   palette's roles (ADR-0026: a game's pattern map gives `key` and
   `key_bg` patterns).
7. **Views hold text that names keys as `StyledText`:** the text box's
   `WaitView::key`, `log_hint` and `SkipView::answers`, the question
   widget's `AskView::keys`, the `help` of the options, key bindings,
   profiles and layout picker screens, and `screens::text_keys_help`.
   Text that names no key stays `String`. Their constructors take
   `impl Into<StyledText>`, and `StyledText` is `From<String>` and
   `From<&str>` (one plain span).
8. **Markup in text files** (`[warning]…[/]` in a language's `ui.ron` or
   a `.dlg`) is a later decision, with its own ADR for its syntax and its
   escape, when a game asks. Wrapping a `StyledText` waits for a box that
   needs it.

## Consequences

- A game's keys stand out everywhere the engine names one as soon as
  its palette has `key` (and `key_bg` for a box behind the glyphs), with
  no code; until then nothing changes.
- A skin that paints a hint calls `print_styled` with
  `palette.text_colors_on(fg, bg)` instead of `print(…, fg, bg)`: one
  more argument, the same look.
- The view fields above changed type, a break for a game that builds or
  reads them; `From<String>` keeps a game that builds them from a
  `String` compiling, without the key role.
- A span's role is chosen by the logic, its colour by the skin: a
  screen reader (2322) can read `Key` spans as keys, and key caps (3215)
  draw them as caps, from the same views.
- Two neighbouring keys with nothing between them (`{Up}{Down}`) are one
  `Key` span: a skin can't tell them apart. Text that names two keys
  puts a word or a space between them.

## Alternatives considered

- **Markup in the strings** (BBCode, Godot's and Unity's way) — every one
  of about 40 `Ctx::text` prints would need to read it, translators
  would need its escape for `[`, and the pseudo-language (2353) would
  have to copy it. The keys need no markup: the engine knows which part
  of a hint is a key. Kept for when a game asks (§8).
- **A colour per span** (Bevy's `TextSpan` with its own colour) — the
  logic would pick colours, which is the skin's (ADR-0002 §4), and a
  theme or the one-bit pattern map couldn't change them.
- **Brackets or a mark added by the engine** around a key's name — a
  look decided for every game, and widths that change under every
  snapshot; a game that wants brackets gets them from
  `GameData::key_text`, as text.
- **A role in the `Cell`** — every renderer and snapshot would have to
  learn roles, and `GlyphBuffer` would carry the palette's meaning; a
  role resolved to `fg` and `bg` at print time changes no renderer.
- **Marking keys with private-use characters inside a `String`** — no
  type change, but every width, cut and comparison would have to skip
  the marks, and a forgotten one prints a box.
