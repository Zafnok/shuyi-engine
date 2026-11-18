# ADR-0131: A language text's form is learned when it loads and kept, a parse on byte ranges

- **Status:** Accepted
- **Date:** 2026-11-17
- **Related tickets:** 2266; 3219 (the plain check), 3218 (every fill parsed), 2268 (the walk's values lent)
- **Amends:** ADR-0127 §4 and §9 (a fill reads the form its language learned; the parse is kept apart from its text)

## Context

ADR-0127 made a text a message, and 3218 made every fill of a screen's
text run `Message::parse` on it, each frame. The game's Playdate cursor
sweep went over its budget (2,050,631 instructions against 2,000,000).
3219 took the cost off plain text with a one-pass check
(`has_message_syntax`), but a message (a plural, a select on `device`)
was still parsed in every frame that showed it, and the load already
parsed every text to check it, and kept nothing of it.

`Message<'a>` borrowed its text (`Node::Text(&'a str)`), so `Lang`, which
owns its texts as `String`s, couldn't keep a parse beside them without
borrowing from itself. A text a game ships with (`en/ui.ron`, a pack's
entry) never changes after the load: hot reload builds a new `Lang`.

## Decision

1. **A parse records where each piece is, not the piece.** The parser
   (`shuyi_assets::message`) keeps byte ranges into its text (a private
   `Span`), and `Message<'a>` is its text and its nodes, `Cow<'a,
   [Node]>`: owned from `Message::parse`, borrowed when lent from a kept
   parse. `parse`, `walk`, `format`, `args`, `names` and `forms` keep
   their signatures; `Message::text` is added. A range read against
   another text gives nothing, never a panic.
2. **`Lang` learns each text's form as it is built** (`Lang::with_source`,
   which `Lang::new` and `from_files_in` call): the source's screen text
   and data text, and each pack's screen and data entries
   (`lang/forms.rs`). A text is **plain** when it has no message syntax
   (`message::has_message_syntax`, 3219's check, moved to
   `shuyi-assets`), a **message** (its parse kept), or **not a message**
   (message syntax that doesn't parse: filled by its `{name}`s, as
   ADR-0127 §9 says). Only what isn't plain is kept, in a map beside the
   texts: a language of plain texts allocates nothing more. Packs are
   learned when the `Lang` is built from them, not when each is read: a
   `LangPack` is a public, serialized type that may come from anywhere.
   `Lang` stays `Clone`, `Eq` and `Default`; two `Lang`s of the same
   texts are equal, their forms too.
3. **`message::Form<'a>`** (`#[non_exhaustive]`, ADR-0039 kind 2):
   `Plain(&str)`, `Message(Message<'a>)`, `NotAMessage(&str)`, with
   `Form::of(text)` for text a caller holds (it parses only if the text
   has message syntax) and `Form::text`.
   **`Lang::form_in(code, key) -> (Form, &str)`** is `text_in` with the
   form, and **`Lang::data_form_in`** is `data_text_in` with it: text
   the source's data doesn't have is read by `Form::of`, as text a
   caller holds. `text_in` and `data_text_in` stay.
4. **`Ctx`'s key fills read the form:** `text_with`, `text_with_keys`,
   `text_styled`, `text_styled_with_keys`, `label` and `label_styled`
   call `form_in` and never parse. A fill of text a caller holds
   (`Ctx::fill_text`, `text::fill`, `fill_values`) reads `Form::of`, so
   3219's check keeps a plain text from being parsed.
5. **Plain is the check alone.** A text whose parse finds only text and
   simple arguments is still a message: `{ n }` trims its name and `'{'`
   quotes, which a fill by its `{name}`s alone would not, so calling such
   a text plain would change what it fills to.

## Consequences

- No frame parses a screen's text, in any language: a test counts
  `Message::parse` (the `parse-count` feature, a thread's count, only
  tests turn it on) through the toy's and Hello, Grid's screens.
- The load parses a text at most twice: the format's check (which
  `English`, a public map of strings, keeps apart) and the record, for
  a text with message syntax only. A synthetic load the size of a
  game's menus (833 texts, 33 messages, two packs) took 6.0% more
  instructions, allocated 1.7% more and peaked 2.4% higher (ticket
  2266's notes): a load's cost, paid once, for none in a frame.
- A kept parse is only correct with its own text: `Parsed::with` is
  crate-private, and `Lang`'s texts never change after it is built. A
  future method that changes a text in place must learn its form again.
- What a fill still copies (each value into an `Arg`, a select's value
  into a `String`) is 2268's: there the walk borrows its values.

## Alternatives considered

- **Keep each `Message` in `Lang` with its text** (a self-borrowing
  struct): needs `unsafe` or a dependency (`ouroboros`, `yoke`), and the
  ticket forbids both.
- **Store each text as its parse alone** (no `String`): every reader of
  `text_in`, `source_text` and the status wants the text, and a plain
  text would cost its nodes too.
- **A cache filled at the first fill:** a frame still parses, `Lang`
  would need interior mutability behind `&self`, and the first frame of
  a screen is the one a budget is measured on.
- **Change `ui: BTreeMap<String, String>` to a map of text and form:**
  `Lang::new` takes the plain map, so building the other map would
  allocate a node per text, against "only messages cost a kept parse".
- **Fold the format's check and the record into one parse:** `English`
  is a public map of strings, and changing it is a break (ticket 2266,
  Out).
