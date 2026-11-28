---
id: "3232"
title: "Text fields share one editing core: a caret, a selection, undo, and the platform's copy and paste"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2346", "3230"]
nick_input: none
completed:
---

# 3232 — Text fields share one editing core

## Context

Found by the engine blind-spot audit of 2026-11-23 (in-game UI: "no
shared text-edit core").

**What the benchmarks have.** Godot's `LineEdit`, Unity's `TextField`
and `TMP_InputField`, and egui's `TextEdit` are each one control with a
caret, a selection, undo, and the system's copy and paste. Every text
box in a game built on them behaves the same way.

**What Shuyi has today.** Three boxes, each with its own few lines of
editing, and none with a caret or a clipboard:
- `NameBox` (`crates/ui/src/widgets/name_entry.rs`, line 92) appends
  each typed character at the end and deletes with `String::pop`
  (line 136). The name grid does the same (lines 274 and 296).
- `NumberBox` (`crates/ui/src/screens/options.rs`, line 536) is the
  options screen's volume box. The struct is public, but it has no
  public constructor: only `OptionsScreen` builds it, and its editing
  is inline in the screen (`type_number`, line 875: `push` and `pop`, at most
  `MAX_DIGITS` = 3).
- The dialogue preview's go-to-line box (`crates/ui/src/dialogue/preview.rs`,
  `type_line`, line 644; from 2586, done) is a third copy: digits only,
  at most 6, `push` and `pop`.
- 2356 (open) plans a fourth: the debug console's input line, "as in
  `NameBox::handle`" (its step on `ConsoleScreen`).
- Each reads `FrameInput::text` and the text box's fixed keys from
  `input::fixed::text_key` (`crates/ui/src/input/fixed.rs`): `Enter`,
  `Backspace` and `Escape` only.
- Nothing in `crates/` calls a clipboard. miniquad 0.4.11 has
  `window::clipboard_get` and `clipboard_set` (Windows, macOS, X11,
  Wayland), and on the web its loader keeps the text of the browser's
  last `paste` event and writes `clipboard_set`'s text on the next
  `copy` or `cut` (`crates/cli/web/mq_js_bundle.js`).
- `Chord` (`crates/assets/src/keymap.rs`, line 181) holds a key and
  Shift only: no Ctrl, Alt or Cmd. So no chord can say "Ctrl+V".

**Who needs it.** A shop's search box, a seed to type, a save's name,
a level editor's sign text, the debug console (2356), chat (1149,
parked) and the editor's inspector (2507, parked). 1149 and 2507 both
plan their own paste today. A farming game's "name your farm" box and
a puzzle game's "enter a level code" box are the same widget.

**What this builds on.** 2346 (open, a blocker) makes the name box
take any letter the font draws, adds `Ctx::takes_text` (the IME on only
while a box takes text), `Frame::typing`, and the buffer's text caret
(`GlyphBuffer::set_text_caret`). This ticket puts the caret where the
field's caret is, not always at the end. 3230 (open, the other
blocker) makes `shuyi_ui::widgets::Widget` the one contract every
widget implements (`kind`, `measure`, `handle`, `keeps`, `paint`), and
implements it for `NameBox`; this ticket's field joins it. Rule 5 (`AGENTS.md`): only
`shuyi-app` reads the system's clipboard; a field never does I/O.
Rule 8: no game action is bound to these keys; they are a text box's
own fixed keys, like `Enter` and `Backspace` today, and the platform's
own shortcuts, like the fullscreen shortcut in `crates/app/src/keys.rs`
(`fullscreen_shortcut`).

## Nick input

None.

## Scope

**In:**

1. **`shuyi_ui::widgets::text_field`**, a pure model (no I/O, `alloc`
   only, as the rest of `shuyi-ui`):
   - `TextField`, `#[non_exhaustive]`, built with
     `TextField::new(text, filter)` (ADR-0039: it may grow): the text,
     the caret (a `char` index), an optional selection anchor, and the
     undo and redo stacks.
   - `Filter`: `max_chars`, and a whole-text check
     (`fn(&str) -> bool`). An edit whose result the check refuses
     changes nothing and reports `FieldEvent::Refused`, so a widget
     sounds Denied. A check of the whole text, not of the one
     character, because with a caret a character goes in the middle
     (a space at the start of a name is refused wherever it is typed).
   - `TextField::end_only()`: the caret stays at the end, and moving it
     does nothing. For the 3- and 6-digit boxes, whose views show no
     caret.
   - Edits: insert typed text at the caret (replacing the selection),
     delete back and forward, delete a word back, move by character,
     by word and to either end, each with or without extending the
     selection, select all, cut, copy, paste, undo, redo.
   - A word boundary is between a letter or digit and anything else.
   - Undo keeps the last 64 steps. A run of typed characters with no
     caret move between them is one step. A run of deletes is one
     step. A paste or a cut is one step. A new edit clears redo.
   - `TextField::handle(&mut self, input, ctx) -> FieldEvent`: the
     frame's typed text, then its edit keys, then its text keys
     (`Done`, `Cancel`), in that order, as `NameBox::handle` reads
     them now. It calls `ctx.takes_text()` (2346).
   - `FieldEvent`: `Open`, `Done`, `Cancel`, `Refused`, and `Copied`
     when it asked the platform to copy. `#[non_exhaustive]`.
   - `TextFieldView`, plain data for a skin (rule 9): the text, the
     caret and the selection as `char` ranges, and the first `char`
     shown when the text is wider than the box (the view keeps the
     caret in sight). `#[non_exhaustive]` with `TextFieldView::new`.
   - `text_field::glyph::paint(palette, buf, rect, view)`: the text,
     the selection in the focused-item colours `menu::glyph` uses, the
     caret as `_` at the end or as the caret cell drawn in those
     colours inside the text. It calls `buf.set_text_caret` (2346) on
     the caret's cell, placed by `shuyi_assets::lang::text_width` of
     the text before it, so wide glyphs count two cells.
2. **The edit keys** (`crates/ui/src/input/fixed.rs`):
   - `EditKey`, `#[non_exhaustive]`: `Left`, `Right`, `Home`, `End`,
     `WordLeft`, `WordRight` (each with `select: bool`), `DeleteForward`,
     `DeleteWordBack`, `SelectAll`, `Copy`, `Cut`, `Paste`, `Undo`,
     `Redo`.
   - `edit_key(chord) -> Option<EditKey>`: `Left`, `Right`, `Home` and
     `End` (Shift extends the selection) and `Delete`. These are
     chords, so they come through the keymap's key reader as today.
   - `TextKey` and `text_key` stay as they are.
3. **The platform's own shortcuts** (`crates/app/src/keys.rs`), read
   only while the last frame said `Frame::typing` (2346), so play never
   sees them:
   - Ctrl (Cmd on macOS) with `C`, `X`, `V`, `A`, `Z`, and Ctrl+Shift+`Z`
     and Ctrl+`Y` for redo; Ctrl (Alt on macOS) with the arrows for
     word moves and with `Backspace` for a word back.
   - They come as a new `RawInputEvent::Edit(EditKey)`. `RawInputEvent`
     is `#[non_exhaustive]`, so this is additive. The key's own `Down`
     still comes, as for `Text`; a field ignores it.
   - A `cfg!` in the backend is allowed (rule 5 names backends); no
     screen reads it.
4. **Paste:** on the paste shortcut, `shuyi-app` reads its clipboard
   (`clipboard_get`), cuts the text at its first line break and at
   4,096 characters, and sends each character as a new
   `RawInputEvent::Pasted(char)` (serialised with `one_char`, as
   `Text`). `FrameInput::pasted() -> &str` collects them. A session
   record holds them, so a replay pastes the same text, and a field
   never reads the clipboard. A paste whose result the filter refuses
   is refused whole.
5. **Copy:** `Ctx::copy_text(text)` queues the text, and `App` hands it
   after the frame to a new `Platform::set_clipboard(&mut self, text:
   &str)`, by default doing nothing (a device with no clipboard). Cut
   is a copy, then a delete. `shuyi-app` calls `clipboard_set`. The
   headless backend keeps the last text, and `Harness::clipboard() ->
   Option<&str>` reads it. `Harness::paste(text)` sends the `Pasted`
   characters as the backend would.
6. **The three boxes move onto `TextField`:**
   - `NameBox` holds a `TextField` whose check is `NameRules`' whole
     text. **Either order with 2380:** if 2380 is done, the check is the
     every-character-in-turn rule inside `NameRules::accepts`, factored
     out as `NameRules::allows_text(ctx, text) -> bool` (no trim: a
     name being typed may end in a space), which `accepts` calls; if
     not, add `NameRules::allows_text` here, from `allows`, and 2380's
     `accepts` calls it.
   - `TypingView` gains the caret and the selection. It is not
     `#[non_exhaustive]` and the game builds one in a test, so it
     becomes `#[non_exhaustive]` with `TypingView::new(title, typed,
     hint)` in the same change: a `Breaking:` line.
   - `paint_typing` draws the caret and selection as `text_field::glyph`
     does.
   - The name grid (`NameEntry`) keeps appending and deleting at the
     end: it has no caret.
   - The volume box (`NumberBox`) and the go-to-line box hold an
     `end_only` `TextField` with a digits check and their limits (3 and
     6). Their pad stepping stays. Their views (`ValueView::NumberBox`
     and the preview's) don't change.
7. **Either order with 2356:** if 2356 is done, its console's input
   line becomes a `TextField` (printable ASCII, at most 120 characters,
   not `end_only`), and its own `push` and `pop` go; if not, nothing to
   do (2356 builds the line on `TextField`).
8. **`TextField` implements 3230's `Widget` contract** (3230's Out
   leaves text editing to this ticket): `kind` is `"text_field"`;
   `measure` is the box's width in cells for `max_chars` and one row;
   `handle` calls `TextField::handle` and maps `FieldEvent::Done` and
   `Cancel` to `WidgetEvent::Done` and `Cancelled`, an edit to
   `Changed`, and `Refused` to `Used` with the Denied sound; `keeps` is
   Left and Right (none for `end_only`), so focus never takes the
   caret's arrows; `paint` is `view()` then `text_field::glyph::paint`
   in `paint.rect`. `NameBox`'s impl from 3230 stays and reads its
   field, so its tests through the contract pass unchanged. If 2322 has
   added `describe` to `Widget` by then, `TextField` describes itself
   as an editable text with its value.
9. **An ADR** (the `write-adr` skill): a text box's editing keys are
   fixed keys and the platform's own shortcuts, never actions, because
   while a box takes text every key is a letter, a pad player spells on
   the grid and never types, and players expect their system's
   shortcuts. It amends the line in 1149 and 2507 that paste is "on an
   action, never a key". Pasted text comes as input, so records replay
   it; copied text goes out after the frame, as `Ctx::offer_file` does.

**For the game** (pins `50472bbc`): `gh search code --repo
Zafnok/visions-of-shuyi NameBox` finds `crates/ui/src/screens/lead_select.rs`;
`TypingView` finds its lead-select view and a test in
`crates/ui/src/widgets/name_entry/playdate.rs` that builds
`TypingView { title, typed, hint }` (line 131). The game re-exports the
engine's `name_entry` (`crates/ui/src/widgets/mod.rs`). So:
- `TypingView` breaks (step 6): the game's test uses
  `TypingView::new`, and its Playdate `paint_typing` still prints
  `typed` with `_` (the caret at the end looks as before). The PR has
  a `Breaking:` line and a `migration` block, and the game's PR that
  moves its pin follows it.
- `Behaviour:` lines for the name box (the arrow keys now move a caret
  while typing; a paste and the shortcuts work), and for the volume box
  (it takes a paste).
- `ValueView::NumberBox`, which the game's Playdate options painter
  reads, doesn't change.

**Out (do not do):**
- Multi-line fields (a sign's text, chat history): 3268.
- The IME's composing text drawn in the field: miniquad 0.4.11 has
  none, and it comes with the winit backend's IME work.
- The pointer placing the caret or dragging a selection: keys answer a
  text box, as 2409's Out says of the name box.
- Grapheme clusters: the caret moves by `char`. 2346's letters are
  single precomposed characters.
- Help text for the new keys: the box's help line keeps naming `Enter`,
  `Backspace` and `Escape` (`text_key_names`).
- Copy and paste on a pad, or a system keyboard's paste (2380's
  `TextAsk`): there is no field typing there.
- Building chat's box (1149, parked) or the editor's inspector boxes
  (2507, parked): neither exists yet.
- A `TextField` widget in `.ui.ron`: 2330's widget list is its own.

**Estimated size:** about 1,400 lines (code 650, tests 550, docs 200).

**Split:** past 900 lines, split off steps 3 to 5 (the platform's
shortcuts, paste and copy) into a new ticket blocked by this one; the
field then takes the chord edit keys only.

## Implementation steps

1. Read `name_entry.rs` and its glyph skin and tests, the volume box in
   `options.rs` (`NumberBox`, `open_box`, `type_number`, `close_box`), `preview.rs`'s
   `type_line`, `input/fixed.rs`, `input/frame.rs` (`FrameInput`),
   `game.rs` (`RawInputEvent`, the raw-event loop near line 636),
   `crates/platform/src/platform.rs` (`Platform`, `offer_file`),
   `app.rs`, `headless.rs`, `harness.rs` (`type_text`), `session.rs`
   (the record's `input`), and `crates/app/src/keys.rs` and
   `backend.rs`, as 2346 left them.
2. Check whether `FrameInput::pressed_chords` repeats a held key. If it
   doesn't, a held arrow or `Backspace` repeats through the keymap's
   repeat timing (the `InputState` repeat), so holding Left walks the
   caret.
3. `text_field.rs`: `TextField`, `Filter`, the edits, undo, the view,
   and unit and property tests on the model alone.
4. `text_field/glyph.rs` and its snapshot tests.
5. `EditKey`, `edit_key`, `RawInputEvent::Edit` and `Pasted`, and
   `FrameInput::edits()` and `pasted()`, filled in `Game::frame`.
6. `Ctx::copy_text`, `Platform::set_clipboard`, `App` calling it, the
   headless backend, `Harness::clipboard` and `Harness::paste`.
7. `shuyi-app`: the shortcuts, paste and copy. On the web, check how
   miniquad's loader orders the browser's `paste` and `copy` events
   against the key, try it in the browser build, and say in the PR what
   works. If copy on the web needs the selection handed over before the
   key (the browser's `copy` event fires first), write that follow-up
   with the `write-ticket` skill and say so in the PR and the docs.
8. Move `NameBox`, the volume box and the go-to-line box onto
   `TextField` (step 6 of the Scope), keeping their tests passing as
   they are, then add the new cases.
9. The ADR (step 9 of the Scope).
10. Docs: `docs/platform-backends.md` (`set_clipboard`, `Edit` and
    `Pasted`, and when a backend sends them); the book's
    `site/content/learn/book/games-and-screens.md` (a text box); the
    `input-actions` skill (`shuyi-app`'s shortcut reader is one of the
    places that knows keys) and the shipped `skills/shuyi-input` skill
    (a text box's fixed keys).
11. Run `cargo run -q -p shuyi-cli -- lint` (a lone letter in a Rust
    string reads as a key: write `Ctrl+V` only in docs and tests), then
    the gates.

## Acceptance criteria

- [ ] In a name box, typing "Jon", then Left, then "h" gives "John"
      with the caret after the `h` (index 3).
- [ ] Shift+Home in "John" with the caret at the end selects all four
      letters; typing "K" then gives "K".
- [ ] `DeleteWordBack` at the end of "Ann Lee" gives "Ann "; at the end
      of "Ann " it gives "".
- [ ] Home, then a space, in "Ann" is refused (`FieldEvent::Refused`,
      the Denied sound) and leaves "Ann". A space after the space in
      "Ann Lee" (caret at index 4) is refused too.
- [ ] With `max_chars` 12, pasting 12 letters into an empty box keeps
      them, and pasting 13 is refused whole and leaves it empty.
- [ ] Pasting "Mira\nextra" gives "Mira". Pasting "Kai!" is refused
      whole.
- [ ] Typing "Jon" then Undo gives "" and Redo gives "Jon". Typing
      "Jon", Left, "h", then Undo gives "Jon" with the caret at index 2.
      A new edit after an Undo makes Redo do nothing.
- [ ] After 65 separate steps (a caret move between each typed
      letter), 64 Undos reach the text after the first step, and a 65th
      does nothing.
- [ ] With "Lee" selected in "Ann Lee", Copy gives
      `Harness::clipboard() == Some("Lee")` and leaves the text; Cut
      gives the same and leaves "Ann ". Copy with no selection leaves
      the clipboard as it was.
- [ ] A session record of a paste plays back to the same text and
      state hash, with no clipboard read.
- [ ] While no box takes text, `shuyi-app` sends no `Edit` and no
      `Pasted` event (a unit test on the shortcut function, given the
      last frame's `typing`).
- [ ] In the volume box, Left does nothing, pasting "37" gives 37,
      pasting "3a" is refused, and pasting "1000" (4 digits for 3) is
      refused. The go-to-line box takes 6 digits and refuses a 7th.
- [ ] The name box's painted caret is on the cell after the `h` in
      "John" (`buf.text_caret()`), and on the cell after a wide glyph
      it counts two cells.
- [ ] The name grid, the volume box's pad stepping and the go-to-line
      box's stepping behave as before (their existing tests pass
      unchanged).
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: every edit of `TextField` on its own, from the cases above;
  word boundaries at digits, `-` and `'`; `end_only`; the filter on
  insert, paste and delete (a delete that would make a refused text,
  such as the letter between two spaces, is refused).
- Property: any sequence of edits keeps the caret and the selection
  inside the text, keeps the text passing the filter, and Undo of every
  step returns the text it started from (within 64 steps).
- Snapshot / integration: the field's glyph look (caret at the end, in
  the middle, a selection, a scrolled long text); a Harness script
  that types, moves, selects, copies and pastes in Hello, Grid's
  profile name box; a session record with a paste played back.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
