//! The options screen on the toy game's rows ([`toy::option_rows`]): text
//! speed, the toy's speed, fullscreen, colors, the two volumes, then the
//! layout and the Key bindings, a locked row and Restore defaults; the
//! toy's level (not a setting) only while it has one.

use shuyi_assets::keymap::{Chord, Key};

use super::*;
use crate::frame::color::Rgb;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::Cell;
use crate::settings::{SETTINGS_KEY, Settings};
use crate::storage::{Storage, StorageError};
use crate::toy::{self, Action, LEFT_HANDED, Prefs, Theme, Toy, ToyCtx, ctx};

type Ctx = ToyCtx;
type OptionsScreen = super::OptionsScreen<Toy>;
type FrameInput = crate::screen::FrameInput<Action>;

/// The rows' keys, top to bottom (without the toy's level).
const ROWS: [&str; 10] = [
    "options.row.text_speed",
    "toy.speed",
    "options.row.fullscreen",
    "options.row.theme",
    "options.row.music_volume",
    "options.row.sound_volume",
    "options.row.layout",
    "options.row.key_bindings",
    "toy.locked",
    "options.row.restore_defaults",
];

fn press(s: &mut OptionsScreen, c: &mut Ctx, actions: &[Action]) -> String {
    let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
    format!("{:?}", s.update(c, &input))
}

/// The screen of the toy's rows.
fn screen() -> OptionsScreen {
    OptionsScreen::new(toy::option_rows)
}

/// The screen focused on the row `key`.
fn on(key: &'static str) -> OptionsScreen {
    screen().focused_on(key)
}

/// The sounds asked for since the last call.
fn sounds(c: &mut Ctx) -> Vec<String> {
    let cue = |r: crate::audio::AudioRequest<toy::Voice>| r.cue().map(str::to_owned);
    c.audio.take().into_iter().filter_map(cue).collect()
}

/// What the focused row shows beside its name.
fn focused_value(s: &OptionsScreen, c: &Ctx) -> ValueView {
    let view = s.view(c);
    view.focused().map(|r| r.value.clone()).unwrap()
}

/// The volume box as the view shows it: its digits, and whether it is
/// typed in.
fn boxed(digits: &str, typing: bool) -> ValueView {
    ValueView::NumberBox {
        digits: digits.to_owned(),
        typing,
    }
}

fn saved(c: &Ctx) -> Settings<Prefs> {
    let text = c.storage.read(SETTINGS_KEY).unwrap().unwrap();
    Settings::from_ron(&text).unwrap()
}

/// Whether the settings in use aren't the ones saved: a change on the
/// screen is saved only when kept on leaving.
fn unsaved(c: &Ctx) -> bool {
    let text = c.storage.read(SETTINGS_KEY).unwrap();
    text.map(|t| Settings::from_ron(&t).unwrap()).as_ref() != Some(c.settings())
}

#[test]
fn up_and_down_move_the_focus_and_wrap() {
    use Action::{CursorDown, CursorUp};
    let mut c = ctx();
    let mut s = screen();
    assert_eq!(s.name(), "options");
    assert!(!s.is_overlay());
    assert_eq!(s.focus(&c), ROWS[0]);
    press(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.focus(&c), ROWS[1]);
    assert_eq!(sounds(&mut c), ["ui_move"]);
    press(&mut s, &mut c, &[CursorUp, CursorUp]);
    assert_eq!(s.focus(&c), "options.row.restore_defaults");
    press(&mut s, &mut c, &[CursorUp, CursorUp]);
    // No level: it isn't a row.
    assert_eq!(s.focus(&c), "options.row.key_bindings");
    c.game.level = Some(0);
    press(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.focus(&c), "toy.level");
    press(&mut s, &mut c, &[CursorDown, CursorDown, CursorDown]);
    assert_eq!(s.focus(&c), ROWS[0]);
    // A focus on a row that isn't shown is on the first.
    c.game.level = None;
    assert_eq!(on("toy.level").focus(&c), ROWS[0]);
    let keys: Vec<_> = s.rows(&c).iter().map(|r| r.key).collect();
    assert_eq!(keys, ROWS);
}

#[test]
fn left_and_right_step_a_setting_and_stop_at_the_ends() {
    use Action::{CursorLeft, CursorRight};
    let mut c = ctx();
    let mut s = screen();
    press(&mut s, &mut c, &[CursorRight]);
    assert_eq!(c.settings().text_speed, TextSpeed::Fast);
    assert!(unsaved(&c));
    assert_eq!(sounds(&mut c), ["ui_move"]);
    press(&mut s, &mut c, &[CursorRight, CursorRight, CursorRight]);
    assert_eq!(c.settings().text_speed, TextSpeed::Instant);
    // Only the step that changed something sounds.
    assert_eq!(sounds(&mut c), ["ui_move"]);
    press(&mut s, &mut c, &[CursorLeft; 5]);
    assert_eq!(c.settings().text_speed, TextSpeed::Slow);
    assert_eq!(sounds(&mut c).len(), 3);
    // On/off reads `Off  On` left to right.
    let mut s = on("options.row.fullscreen");
    press(&mut s, &mut c, &[CursorLeft]);
    assert!(!c.settings().fullscreen);
    assert!(sounds(&mut c).is_empty());
    press(&mut s, &mut c, &[CursorRight, CursorRight]);
    assert!(c.settings().fullscreen);
    assert_eq!(sounds(&mut c), ["ui_move"]);
    press(&mut s, &mut c, &[CursorLeft]);
    assert!(!c.settings().fullscreen);
}

#[test]
fn confirm_steps_a_setting_and_goes_round() {
    let mut c = ctx();
    let mut s = on("toy.speed");
    let speeds: Vec<_> = (0..3)
        .map(|_| {
            press(&mut s, &mut c, &[Action::Confirm]);
            c.settings().game.speed
        })
        .collect();
    assert_eq!(speeds, [1, 2, 0]);
    // On and off go round too.
    let mut s = on("options.row.fullscreen");
    press(&mut s, &mut c, &[Action::Confirm]);
    assert!(c.settings().fullscreen);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert!(!c.settings().fullscreen);
}

/// One frame of typing: the characters `text`, then the keys `keys`.
fn type_in(s: &mut OptionsScreen, c: &mut Ctx, text: &str, keys: &[Key]) {
    let chords = keys.iter().map(|&k| Chord::plain(k)).collect();
    let input = FrameInput::new(vec![], 0.0, vec![]).with_typing(chords, text.chars().collect());
    s.update(c, &input);
}

#[test]
fn left_and_right_move_a_volumes_slider_five_at_a_time() {
    use Action::{CursorLeft, CursorRight};
    let mut c = ctx();
    let mut s = on("options.row.music_volume");
    assert_eq!(s.help(&c), "arrows move · f type a number · d back");
    press(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(c.settings().music_volume, 75);
    assert!(unsaved(&c));
    press(&mut s, &mut c, &[CursorRight; 6]);
    assert_eq!(c.settings().music_volume, 100);
    // Five steps changed it; the sixth did nothing.
    assert_eq!(sounds(&mut c).len(), 6);
    c.change_settings(|s| s.music_volume = 3).unwrap();
    press(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(c.settings().music_volume, 0);
    press(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(c.settings().music_volume, 0);
    assert_eq!(c.settings().sound_volume, 80);
}

#[test]
fn confirm_on_a_volume_opens_a_box_to_type_the_number() {
    let mut c = ctx();
    let mut s = on("options.row.sound_volume");
    // The key that opens the box types nothing, nor do the keys after it.
    press(&mut s, &mut c, &[Action::Confirm, Action::CursorDown]);
    assert_eq!(s.typing().map(NumberBox::value), Some(None));
    assert_eq!(s.focus(&c), "options.row.sound_volume");
    assert_eq!(sounds(&mut c), ["ui_select"]);
    assert_eq!(s.help(&c), "Enter done · Backspace delete · Escape cancel");
    assert_eq!(focused_value(&s, &c), boxed("", true));
    // The game's keys do nothing while it is open.
    press(&mut s, &mut c, &[Action::CursorDown, Action::Cancel]);
    assert!(s.typing().is_some());
    // Digits only, three at most.
    type_in(&mut s, &mut c, "3x7", &[]);
    assert_eq!(s.typing().and_then(NumberBox::value), Some(37));
    assert_eq!(sounds(&mut c), ["ui_cancel"], "the x is refused");
    assert_eq!(focused_value(&s, &c), boxed("37", true));
    // The other rows show their values as ever.
    let music = s
        .view(&c)
        .row("options.row.music_volume")
        .map(|r| r.value.clone());
    assert_eq!(
        music,
        Some(ValueView::Volume {
            level: 80,
            max: 100
        })
    );
    type_in(&mut s, &mut c, "55", &[]);
    assert_eq!(s.typing().map(NumberBox::digits), Some("375"));
    // Backspace deletes; nothing changes until Enter.
    type_in(&mut s, &mut c, "", &[Key::Backspace, Key::Backspace]);
    assert_eq!(s.typing().and_then(NumberBox::value), Some(3));
    // The second 5 was refused: three digits at most.
    assert_eq!(sounds(&mut c), ["ui_cancel"; 3]);
    assert_eq!(c.settings().sound_volume, 80);
    type_in(&mut s, &mut c, "7", &[Key::Enter]);
    assert!(s.typing().is_none());
    assert_eq!(c.settings().sound_volume, 37);
    assert_eq!(sounds(&mut c), ["ui_select", "ui_move"]);
    assert!(unsaved(&c));
    assert_eq!(c.settings().music_volume, 80);
    // More than 100 is 100.
    press(&mut s, &mut c, &[Action::Confirm]);
    type_in(&mut s, &mut c, "250", &[Key::Enter]);
    assert_eq!(c.settings().sound_volume, 100);
    // Escape, or Enter on an empty box, leaves the volume alone.
    press(&mut s, &mut c, &[Action::Confirm]);
    type_in(&mut s, &mut c, "12", &[Key::Escape]);
    assert!(s.typing().is_none());
    press(&mut s, &mut c, &[Action::Confirm]);
    type_in(&mut s, &mut c, "", &[Key::Enter]);
    assert!(s.typing().is_none());
    press(&mut s, &mut c, &[Action::Confirm]);
    type_in(&mut s, &mut c, "", &[Key::Backspace, Key::Enter]);
    assert_eq!(c.settings().sound_volume, 100);
    // The same number again is no change: it doesn't sound like one.
    press(&mut s, &mut c, &[Action::Confirm]);
    sounds(&mut c);
    type_in(&mut s, &mut c, "100", &[Key::Enter]);
    assert_eq!(sounds(&mut c), ["ui_select"]);
    // The music row's box sets the music.
    let mut s = on("options.row.music_volume");
    press(&mut s, &mut c, &[Action::Confirm]);
    type_in(&mut s, &mut c, "0", &[Key::Enter]);
    assert_eq!(c.settings().music_volume, 0);
    assert_eq!(c.settings().sound_volume, 100);
}

/// On a controller there is nothing to type with: the box holds the
/// volume and the cursor steps it by one.
#[test]
fn the_volume_box_steps_by_one_on_a_controller() {
    let pad = |actions: &[Action]| FrameInput::new(actions.to_vec(), 0.0, vec![]).with_pad(true);
    let mut c = ctx();
    let mut s = on("options.row.music_volume");
    s.update(&mut c, &pad(&[Action::Confirm]));
    assert_eq!(s.typing().and_then(NumberBox::value), Some(80));
    assert_eq!(s.help(&c), "arrows change · f done · d cancel");
    assert_eq!(focused_value(&s, &c), boxed("80", false));
    sounds(&mut c);
    let steps = [Action::CursorUp, Action::CursorRight, Action::CursorUp];
    s.update(&mut c, &pad(&steps));
    assert_eq!(s.typing().and_then(NumberBox::value), Some(83));
    assert_eq!(sounds(&mut c).len(), 3);
    s.update(&mut c, &pad(&[Action::CursorDown, Action::Info]));
    assert_eq!(s.typing().and_then(NumberBox::value), Some(82));
    s.update(&mut c, &pad(&[Action::CursorLeft]));
    assert_eq!(s.typing().and_then(NumberBox::value), Some(81));
    // Not until Confirm.
    assert_eq!(c.settings().music_volume, 80);
    s.update(&mut c, &pad(&[Action::Confirm, Action::CursorDown]));
    assert!(s.typing().is_none());
    assert_eq!(c.settings().music_volume, 81);
    assert_eq!(s.focus(&c), "options.row.music_volume");
    // Cancel keeps the old volume; the ends hold.
    s.update(&mut c, &pad(&[Action::Confirm]));
    s.update(&mut c, &pad(&[Action::CursorLeft, Action::Cancel]));
    assert_eq!(c.settings().music_volume, 81);
    c.change_settings(|s| s.music_volume = 100).unwrap();
    s.update(&mut c, &pad(&[Action::Confirm]));
    sounds(&mut c);
    s.update(&mut c, &pad(&[Action::CursorUp]));
    assert_eq!(s.typing().and_then(NumberBox::value), Some(100));
    assert!(sounds(&mut c).is_empty());
    c.change_settings(|s| s.music_volume = 0).unwrap();
    s.update(&mut c, &pad(&[Action::Cancel]));
    s.update(&mut c, &pad(&[Action::Confirm]));
    s.update(&mut c, &pad(&[Action::CursorDown, Action::Confirm]));
    assert_eq!(c.settings().music_volume, 0);
}

#[test]
fn rows_show_their_values() {
    let mut c = ctx();
    // What each row shows, as the view says it: the words, or `<level>`
    // for a volume, and `*` if left and right change it.
    let shown = |c: &Ctx| -> Vec<String> {
        screen()
            .rows(c)
            .iter()
            .map(|row| match OptionsScreen::value(c, row) {
                ValueView::None => String::new(),
                ValueView::Text { text, adjustable } => {
                    format!("{text}{}", if adjustable { "*" } else { "" })
                }
                ValueView::Volume { level, max } => format!("<{level}/{max}>"),
                ValueView::NumberBox { .. } => "box".to_owned(),
            })
            .collect()
    };
    assert_eq!(
        shown(&c),
        [
            "Normal*",
            "Calm*",
            "Off*",
            "Day*",
            "<80/100>",
            "<80/100>",
            "Right-handed",
            "",
            "-",
            ""
        ]
    );
    c.change_settings(|s| {
        s.game.speed = 2;
        s.fullscreen = true;
        s.theme = Theme::Night;
        s.music_volume = 0;
        s.sound_volume = 100;
    })
    .unwrap();
    c.use_layout(LEFT_HANDED);
    c.game.level = Some(1);
    assert_eq!(
        shown(&c),
        [
            "Normal*",
            "Wild*",
            "On*",
            "Night*",
            "<0/100>",
            "<100/100>",
            "Left-handed",
            "",
            "Brisk*",
            "-",
            ""
        ]
    );
    // A choice that says it has no such value shows its first.
    c.change_settings(|s| s.game.speed = 9).unwrap();
    assert_eq!(shown(&c)[1], "Calm*");
    // A choice with nothing to choose from shows nothing.
    let empty = OptionRow::<Toy>::new(
        "toy.speed",
        RowValue::Choice {
            values: |_| Vec::new(),
            get: |_| 0,
            set: |_, _| false,
        },
    );
    assert_eq!(OptionsScreen::value(&c, &empty), ValueView::None);
    // Before a layout is chosen, the layout row shows nothing.
    let c = toy::first_launch();
    assert_eq!(shown(&c)[6], "");
}

/// The toy's level isn't a setting: it changes at once and, like any row,
/// is kept or put back on leaving.
#[test]
fn a_row_that_isnt_a_setting_is_kept_or_put_back_with_the_settings() {
    let mut c = ctx();
    c.game.level = Some(0);
    let mut s = on("toy.level");
    assert_eq!(s.help(&c), "arrows move · f change · d back");
    // Left stops at the first; right steps on, and stops at the last.
    press(&mut s, &mut c, &[Action::CursorLeft]);
    assert_eq!(c.game.level, Some(0));
    assert!(sounds(&mut c).is_empty());
    assert!(!s.changed(&c));
    press(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!(c.game.level, Some(1));
    assert_eq!(sounds(&mut c), ["ui_move"]);
    assert!(s.changed(&c));
    press(&mut s, &mut c, &[Action::CursorRight, Action::CursorRight]);
    assert_eq!(c.game.level, Some(2));
    // Confirm goes round.
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(c.game.level, Some(0));
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(c.game.level, Some(1));
    // No setting changed, but leaving still asks. Discard puts it back.
    press(&mut s, &mut c, &[Action::Cancel]);
    assert_eq!(s.leaving(), Some(LeaveChoice::Back));
    press(&mut s, &mut c, &[Action::CursorLeft]);
    assert_eq!(press(&mut s, &mut c, &[Action::Confirm]), "Pop");
    assert_eq!(c.game.level, Some(0));
    // Keep keeps it.
    let mut s = on("toy.level");
    press(&mut s, &mut c, &[Action::CursorRight, Action::CursorRight]);
    press(&mut s, &mut c, &[Action::Cancel]);
    let keep = [Action::CursorLeft, Action::CursorLeft, Action::Confirm];
    assert_eq!(press(&mut s, &mut c, &keep), "Pop");
    assert_eq!(c.game.level, Some(2));
    // Without a level when the screen opened, there is nothing to put
    // back.
    c.game.level = None;
    let mut s = screen();
    press(&mut s, &mut c, &[]);
    c.game.level = Some(1);
    assert!(!s.changed(&c));
    assert_eq!(press(&mut s, &mut c, &[Action::Cancel]), "Pop");
}

/// The view is the whole screen as data: what a skin paints.
#[test]
fn the_view_says_what_the_screen_shows() {
    let mut c = ctx();
    let s = on("options.row.layout");
    let view = s.view(&c);
    assert_eq!(view.title, "Options");
    assert_eq!(view.help, s.help(&c));
    assert_eq!((view.message.clone(), view.question.clone()), (None, None));
    let keys: Vec<&str> = view.rows.iter().map(|r| r.key).collect();
    assert_eq!(keys, ROWS);
    let labels: Vec<&str> = view.rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels[0], "Text speed");
    assert_eq!(labels[ROWS.len() - 1], "Restore defaults");
    // One row in focus; the groups start at Layout and Restore defaults.
    let focused: Vec<&str> = view
        .rows
        .iter()
        .filter(|r| r.focused)
        .map(|r| r.key)
        .collect();
    assert_eq!(focused, ["options.row.layout"]);
    assert_eq!(view.focused().map(|r| r.key), Some("options.row.layout"));
    let groups: Vec<&str> = view
        .rows
        .iter()
        .filter(|r| r.starts_group)
        .map(|r| r.key)
        .collect();
    assert_eq!(
        groups,
        ["options.row.layout", "options.row.restore_defaults"]
    );
    for (row, view) in s.rows(&c).iter().zip(&view.rows) {
        assert_eq!(view.value, OptionsScreen::value(&c, row), "{}", row.key);
    }
    assert_eq!(view.row("toy.level"), None);
    assert_eq!(
        view.row("options.row.layout").and_then(|r| r.value.text()),
        Some("Right-handed")
    );
    assert_eq!(
        view.row("options.row.music_volume")
            .and_then(|r| r.value.text()),
        None
    );
    // A question and a message, in the player's words and keys.
    let mut s = on("options.row.restore_defaults");
    press(&mut s, &mut c, &[Action::Confirm]);
    let question = s.view(&c).question;
    assert_eq!(
        question,
        Some(crate::widgets::AskView {
            text: "Restore every option to its default?".to_owned(),
            answers: Vec::new(),
            keys: Some("f yes / d no".to_owned()),
        })
    );
    press(&mut s, &mut c, &[Action::Confirm]);
    let view = s.view(&c);
    assert_eq!(view.question, None);
    assert_eq!(view.message.as_deref(), Some("Defaults restored"));
}

/// Drawing is the glyph skin painting the view, nothing more.
#[test]
fn draw_paints_the_view_with_the_glyph_skin() {
    use crate::frame::color::EngineColor;
    let mut c = ctx();
    let mut s = on("options.row.sound_volume");
    press(&mut s, &mut c, &[Action::CursorLeft]);
    let bg = c.palette.engine(EngineColor::Background);
    let blank = Cell::new(' ', bg, bg);
    let pc = ConsoleSpec::text(100, 32);
    let mut drawn = GlyphBuffer::with_spec(pc, blank);
    s.draw(&c, &mut drawn);
    let mut painted = GlyphBuffer::with_spec(pc, blank);
    glyph::paint(&c, &s.view(&c), &mut painted);
    assert_eq!(drawn, painted);
    assert!(s.as_any().is_some());
}

/// A game's own painter paints the same view instead (ticket 1045).
#[test]
fn a_game_may_paint_the_view_its_own_way() {
    /// Writes the focused row's label at the top left.
    fn focused_label(_: &Ctx, view: &OptionsView, buf: &mut GlyphBuffer) {
        let label = view.focused().map_or("", |r| r.label.as_str());
        let cell = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
        buf.fill_rect(buf.bounds(), cell);
        buf.print(0, 0, label, Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
    }
    let c = ctx();
    let s = on("options.row.sound_volume").painted_by(focused_label);
    let blank = Cell::new('x', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
    let mut drawn = GlyphBuffer::with_spec(ConsoleSpec::text(20, 2), blank);
    s.draw(&c, &mut drawn);
    let mut expected = GlyphBuffer::with_spec(ConsoleSpec::text(20, 2), blank);
    focused_label(&c, &s.view(&c), &mut expected);
    assert_eq!(drawn, expected);
    assert_eq!(drawn.get(0, 0).map(|c| c.glyph), Some('S'));
}

#[test]
fn the_layout_and_key_bindings_rows_open_their_screens() {
    let mut c = ctx();
    assert_eq!(
        press(&mut on("options.row.layout"), &mut c, &[Action::Confirm]),
        "Push(layout_picker)"
    );
    assert_eq!(
        press(
            &mut on("options.row.key_bindings"),
            &mut c,
            &[Action::Confirm]
        ),
        "Push(key_bindings)"
    );
    assert_eq!(sounds(&mut c), ["ui_select", "ui_select"]);
    // Left and right do nothing on rows that only do something.
    let before = c.settings().clone();
    for row in [
        "options.row.layout",
        "options.row.key_bindings",
        "toy.locked",
        "options.row.restore_defaults",
    ] {
        let keys = [Action::CursorLeft, Action::CursorRight];
        assert_eq!(press(&mut on(row), &mut c, &keys), "None");
    }
    assert_eq!(*c.settings(), before);
    assert!(sounds(&mut c).is_empty());
    assert_eq!(
        on("options.row.layout").help(&c),
        "arrows move · f open · d back"
    );
}

#[test]
fn a_row_that_refuses_says_why_until_the_cursor_moves() {
    let mut c = ctx();
    let mut s = on("toy.locked");
    assert_eq!(s.help(&c), "arrows move · d back");
    press(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    assert_eq!(
        (s.asking(), s.message()),
        (None, Some("toy.locked.message"))
    );
    assert_eq!(s.view(&c).message.as_deref(), Some("Not here"));
    assert_eq!(sounds(&mut c), ["ui_cancel", "ui_cancel"]);
    press(&mut s, &mut c, &[Action::CursorDown]);
    assert_eq!(s.message(), None);
}

#[test]
fn cancel_closes_the_screen() {
    let mut c = ctx();
    let mut s = screen();
    assert_eq!(press(&mut s, &mut c, &[Action::Cancel]), "Pop");
    assert_eq!(sounds(&mut c), ["ui_cancel"]);
}

#[test]
fn restore_defaults_asks_first_and_keeps_the_layout() {
    let mut c = ctx();
    c.choose_layout(LEFT_HANDED).unwrap();
    c.change_settings(|s| {
        s.text_speed = TextSpeed::Slow;
        s.fullscreen = true;
        s.music_volume = 2;
        s.game.speed = 1;
    })
    .unwrap();
    let changed = c.settings().clone();
    let mut s = on("options.row.restore_defaults");
    assert_eq!(s.help(&c), "wasd move · j restore · k back");
    press(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    // The key that asked isn't the answer.
    assert_eq!(s.asking(), Some("options.question.restore"));
    assert_eq!(s.help(&c), "j yes · k no");
    // Nothing but the answers does anything.
    press(&mut s, &mut c, &[Action::CursorDown, Action::CursorRight]);
    assert_eq!(s.focus(&c), "options.row.restore_defaults");
    assert!(s.asking().is_some());
    press(&mut s, &mut c, &[Action::Cancel]);
    assert_eq!((s.asking(), s.message()), (None, None));
    assert_eq!(*c.settings(), changed);
    press(&mut s, &mut c, &[Action::Confirm]);
    sounds(&mut c);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(sounds(&mut c), ["ui_select"]);
    let expected = Settings::default().with_layout(LEFT_HANDED);
    assert_eq!(*c.settings(), expected);
    // A change like any other: saved only if kept on leaving.
    assert_eq!(saved(&c), changed);
    assert_eq!(s.message(), Some(RESTORED_MESSAGE));
    assert_eq!(c.layout(), Some(LEFT_HANDED));
}

/// A storage that reads nothing and can't be written.
#[derive(Debug)]
struct ReadOnly;

impl Storage for ReadOnly {
    fn read(&self, _: &str) -> Result<Option<String>, StorageError> {
        Ok(None)
    }
    fn write(&mut self, _: &str, _: &str) -> Result<(), StorageError> {
        Err(StorageError::Backend("full".into()))
    }
    fn delete(&mut self, _: &str) -> Result<(), StorageError> {
        Err(StorageError::Backend("full".into()))
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        Ok(Vec::new())
    }
}

#[test]
fn a_failed_save_keeps_the_change_says_so_and_stays() {
    let mut c = ctx().with_storage(Box::new(ReadOnly));
    // A change isn't saved yet, so it says nothing.
    let mut s = screen();
    press(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!(c.settings().text_speed, TextSpeed::Fast);
    assert_eq!(s.message(), None);
    // Keep can't save: the change stays, the screen says so and stays.
    press(&mut s, &mut c, &[Action::Cancel]);
    press(&mut s, &mut c, &[Action::CursorLeft, Action::CursorLeft]);
    assert_eq!(s.leaving(), Some(LeaveChoice::Keep));
    sounds(&mut c);
    assert_eq!(press(&mut s, &mut c, &[Action::Confirm]), "None");
    assert_eq!((s.leaving(), s.message()), (None, Some(NOT_SAVED_MESSAGE)));
    assert_eq!(sounds(&mut c), ["ui_cancel"]);
    assert_eq!(c.settings().text_speed, TextSpeed::Fast);
}

/// The screen with Text speed one step on, its question on leaving open.
fn asking_to_leave(c: &mut Ctx) -> OptionsScreen {
    let mut s = screen();
    press(&mut s, c, &[Action::CursorRight]);
    sounds(c);
    assert_eq!(press(&mut s, c, &[Action::Cancel]), "None");
    s
}

#[test]
fn leaving_with_a_change_asks_with_the_cursor_on_back() {
    let mut c = ctx();
    let mut s = asking_to_leave(&mut c);
    assert!(s.changed(&c));
    assert_eq!(s.leaving(), Some(LeaveChoice::Back));
    assert_eq!(sounds(&mut c), ["ui_select"]);
    let view = s.view(&c);
    let leaving = view.leaving.as_ref().unwrap();
    assert_eq!(leaving.text, "Keep these changes?");
    let labels: Vec<&str> = leaving.answers.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["Keep", "Discard", "Back"]);
    assert_eq!(leaving.focused(), Some(LeaveChoice::Back));
    assert_eq!(view.help, "arrows choose · f pick · d back");
    // The cursor moves between the answers, going round at the ends; the
    // rows stay as they are.
    press(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!(s.leaving(), Some(LeaveChoice::Keep));
    assert_eq!(sounds(&mut c), ["ui_move"]);
    press(&mut s, &mut c, &[Action::CursorUp]);
    assert_eq!(s.leaving(), Some(LeaveChoice::Back));
    press(&mut s, &mut c, &[Action::CursorLeft, Action::CursorLeft]);
    assert_eq!(s.leaving(), Some(LeaveChoice::Keep));
    press(&mut s, &mut c, &[Action::CursorDown, Action::Info]);
    assert_eq!(s.leaving(), Some(LeaveChoice::Discard));
    assert_eq!(
        (s.focus(&c), c.settings().text_speed),
        (ROWS[0], TextSpeed::Fast)
    );
}

#[test]
fn back_or_cancel_stays_on_the_screen_with_the_change() {
    let mut c = ctx();
    let mut s = asking_to_leave(&mut c);
    assert_eq!(press(&mut s, &mut c, &[Action::Confirm]), "None");
    assert_eq!(s.leaving(), None);
    // The question opening, then Back.
    assert_eq!(sounds(&mut c), ["ui_select", "ui_cancel"]);
    assert_eq!(c.settings().text_speed, TextSpeed::Fast);
    // Cancel is Back; the key that answers doesn't go on to leave.
    press(&mut s, &mut c, &[Action::Cancel]);
    assert_eq!(
        press(&mut s, &mut c, &[Action::Cancel, Action::Cancel]),
        "None"
    );
    assert_eq!(s.leaving(), None);
    assert_eq!(c.storage.read(SETTINGS_KEY), Ok(None));
}

#[test]
fn keep_saves_the_changes_and_leaves() {
    let mut c = ctx();
    let mut s = asking_to_leave(&mut c);
    press(&mut s, &mut c, &[Action::CursorLeft, Action::CursorLeft]);
    sounds(&mut c);
    assert_eq!(press(&mut s, &mut c, &[Action::Confirm]), "Pop");
    assert_eq!(sounds(&mut c), ["ui_select"]);
    assert_eq!(saved(&c).text_speed, TextSpeed::Fast);
    assert_eq!(c.settings().text_speed, TextSpeed::Fast);
}

#[test]
fn discard_puts_every_setting_back_and_saves_nothing() {
    let mut c = ctx();
    c.choose_layout(LEFT_HANDED).unwrap();
    let before = c.settings().clone();
    let stored = c.storage.read(SETTINGS_KEY);
    let mut s = screen();
    for row in s.rows(&c) {
        if row.is_setting() {
            s = s.focused_on(row.key);
            press(&mut s, &mut c, &[Action::CursorLeft, Action::CursorRight]);
            press(&mut s, &mut c, &[Action::CursorRight]);
        }
    }
    assert!(s.changed(&c) && c.settings().fullscreen && c.settings().music_volume != 80);
    press(&mut s, &mut c, &[Action::Cancel]);
    press(&mut s, &mut c, &[Action::CursorLeft]);
    assert_eq!(s.leaving(), Some(LeaveChoice::Discard));
    assert_eq!(press(&mut s, &mut c, &[Action::Confirm]), "Pop");
    assert_eq!(*c.settings(), before);
    assert_eq!(c.storage.read(SETTINGS_KEY), stored);
    assert_eq!(sounds(&mut c).last().map(String::as_str), Some("ui_cancel"));
}

#[test]
fn restored_defaults_are_a_change_kept_or_discarded_on_leaving() {
    let mut c = ctx();
    c.change_settings(|s| s.music_volume = 2).unwrap();
    let mut s = on("options.row.restore_defaults");
    press(&mut s, &mut c, &[Action::Confirm]);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(
        c.settings().music_volume,
        Settings::<Prefs>::default().music_volume
    );
    press(&mut s, &mut c, &[Action::Cancel]);
    assert_eq!(s.leaving(), Some(LeaveChoice::Back));
    press(&mut s, &mut c, &[Action::CursorLeft, Action::Confirm]);
    assert_eq!(c.settings().music_volume, 2);
    assert_eq!(saved(&c).music_volume, 2);
}

#[test]
fn leaving_with_nothing_changed_asks_nothing() {
    let mut c = ctx();
    let mut s = screen();
    assert!(!s.changed(&c), "nothing is taken before the first frame");
    // A change taken back is no change.
    press(&mut s, &mut c, &[Action::CursorRight, Action::CursorLeft]);
    sounds(&mut c);
    assert!(!s.changed(&c));
    assert_eq!(press(&mut s, &mut c, &[Action::Cancel]), "Pop");
    assert_eq!(sounds(&mut c), ["ui_cancel"]);
    assert_eq!(c.storage.read(SETTINGS_KEY), Ok(None));
    // A layout picked meanwhile is saved at once, so it isn't a change.
    let mut s = screen();
    press(&mut s, &mut c, &[]);
    c.choose_layout(LEFT_HANDED).unwrap();
    assert!(!s.changed(&c));
    assert_eq!(press(&mut s, &mut c, &[Action::Cancel]), "Pop");
}

#[test]
fn discard_after_another_save_puts_the_saved_settings_back_too() {
    let mut c = ctx();
    let before = c.settings().clone();
    let mut s = asking_to_leave(&mut c);
    // A shortcut and a layout picked save at once, with the unkept speed
    // left out.
    c.change_settings(|s| s.fullscreen = !s.fullscreen).unwrap();
    c.choose_layout(LEFT_HANDED).unwrap();
    press(&mut s, &mut c, &[Action::CursorUp, Action::Confirm]);
    assert_eq!(*c.settings(), before.with_layout(LEFT_HANDED));
    assert_eq!(saved(&c), *c.settings());
}

#[test]
fn help_names_what_confirm_does_on_each_kind_of_row() {
    let c = ctx();
    assert_eq!(screen().help(&c), "arrows move · f change · d back");
    assert_eq!(
        on("options.row.fullscreen").help(&c),
        "arrows move · f change · d back"
    );
    assert_eq!(
        on("options.row.key_bindings").help(&c),
        "arrows move · f open · d back"
    );
    // With no rows at all there is nothing to focus.
    let none: OptionsScreen = OptionsScreen::new(|_| Vec::new());
    assert_eq!(none.focus(&c), "");
    assert_eq!(none.help(&c), "arrows move · f change · d back");
}

#[test]
fn a_screen_with_no_rows_does_nothing_but_leave() {
    let mut c = ctx();
    let mut s = OptionsScreen::new(|_| Vec::new());
    let keys = [
        Action::CursorLeft,
        Action::CursorRight,
        Action::Confirm,
        Action::CursorDown,
    ];
    assert_eq!(press(&mut s, &mut c, &keys), "None");
    assert!(s.view(&c).rows.is_empty());
    assert_eq!(press(&mut s, &mut c, &[Action::Cancel]), "Pop");
}

#[test]
fn stepping_helpers_stop_or_wrap() {
    let all = [1, 2, 3];
    assert_eq!(stepped(&all, 3, true, false), 3);
    assert_eq!(stepped(&all, 3, true, true), 1);
    assert_eq!(stepped(&all, 1, false, false), 1);
    assert_eq!(stepped(&all, 1, false, true), 3);
    assert_eq!(stepped(&all, 2, true, false), 3);
    assert_eq!(stepped(&all, 2, false, false), 1);
    assert_eq!(stepped(&all, 2, true, true), 3);
    assert_eq!(stepped(&all, 2, false, true), 1);
    // A value not in the list steps from the first.
    assert_eq!(stepped(&all, 9, true, false), 2);
    assert_eq!(stepped::<i32>(&[], 9, true, true), 9);
    assert_eq!(stepped_index(0, 4, true, true), 4);
    assert_eq!(stepped_index(3, 7, false, false), 1);
    assert_eq!(stepped_volume(100, true), 100);
    assert_eq!(stepped_volume(97, true), 100);
    assert_eq!(stepped_volume(50, true), 55);
    assert_eq!(stepped_volume(50, false), 45);
    assert_eq!(stepped_volume(3, false), 0);
    assert_eq!(stepped_volume(0, false), 0);
}

#[test]
fn leave_choices_and_questions_name_their_keys() {
    let keys: Vec<_> = LeaveChoice::ALL.iter().map(|c| c.key()).collect();
    assert_eq!(
        keys,
        [
            "options.leave.keep",
            "options.leave.discard",
            "options.leave.back"
        ]
    );
    let q: Question<Toy> = Question {
        key: "q",
        yes: |_| "m",
    };
    assert_eq!(format!("{q:?}"), "Question(\"q\")");
    let copy = q;
    assert_eq!(copy.key, q.key);
    let s = screen();
    assert!(format!("{s:?}").starts_with("OptionsScreen"));
    let row = OptionRow::<Toy>::fullscreen();
    assert!(row.is_setting() && !row.starts_group && !row.unsaved);
    assert!(!OptionRow::<Toy>::restore_defaults().is_setting());
}

/// Rows that aren't settings: a toggle (debug tools) and a volume (the
/// voices').
fn unsaved_rows(_: &Ctx) -> Vec<OptionRow<Toy>> {
    vec![
        OptionRow::new(
            "toy.speed",
            RowValue::Toggle {
                get: |ctx| ctx.debug_tools,
                set: |ctx, on| {
                    let changed = ctx.debug_tools != on;
                    ctx.debug_tools = on;
                    changed
                },
            },
        )
        .unsaved(),
        OptionRow::new(
            "toy.level",
            RowValue::Volume {
                get: |ctx| ctx.voice_volume,
                set: |ctx, v| {
                    let changed = ctx.voice_volume != v;
                    ctx.voice_volume = v;
                    changed
                },
            },
        )
        .unsaved(),
    ]
}

#[test]
fn a_toggle_and_a_volume_that_arent_settings_are_put_back_on_discard() {
    use Action::{Cancel, Confirm, CursorDown, CursorLeft};
    for on in [true, false] {
        let mut c = ctx();
        c.debug_tools = on;
        c.voice_volume = 8;
        let mut s = OptionsScreen::new(unsaved_rows);
        press(&mut s, &mut c, &[Confirm, CursorDown, CursorLeft]);
        assert_eq!((c.debug_tools, c.voice_volume), (!on, 3));
        assert!(s.changed(&c));
        // The key that asks isn't the answer: Discard comes after.
        press(&mut s, &mut c, &[Cancel]);
        press(&mut s, &mut c, &[CursorLeft, Confirm]);
        assert_eq!((c.debug_tools, c.voice_volume), (on, 8), "on: {on}");
    }
}
