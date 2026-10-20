//! A toy game for the engine's tests of the screens, the context, the
//! frame loop and the test `Harness`: the sample
//! game's actions and keymap file ([`crate::input::sample`]), a palette of
//! the engine's colors (and a grid map's) in two themes, a few lines of
//! text in English and in a debug language, one sound, three tracks and a
//! pool, voiced lines with two variants, one setting of its own, two
//! images, two lights, and its screens: [`Home`], the [`Menu`] over it,
//! the layout [`Picker`] the first key opens, the [`Gallery`] of pictures
//! the Debug key opens ([`hooks`]), and a grid map's stage
//! ([`grid::Field`]).
//!
//! Built for this crate's tests and, with the `toy` feature, for its
//! integration tests.

pub mod grid;

use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::any::Any;

use serde::{Deserialize, Serialize};
use shuyi_assets::audio::{AudioManifest, CreditRef, MusicCue, SoundCue};
use shuyi_assets::dialogue::{
    DialogueLimits, DialogueTable, LineId, NoDirective, NoTokens, parse_dlg,
};
use shuyi_assets::image::{ImageId, ImageInfo, ImageTable};
use shuyi_assets::keymap::{KeymapDef, Layout};
use shuyi_assets::lang::{Lang, LangCode, PackRules, pack_from_sources};
use shuyi_assets::lighting::Lighting;
use shuyi_assets::palette::PaletteDef;
use shuyi_assets::voice::{self, ClipVariant, Playable};

use crate::ctx::{Ctx, GameData};
use crate::dialogue::text_box::{self, glyph::BoxPlace};
use crate::dialogue::{DialoguePlayer, Plain, TextBox, TextView, WaitView};
use crate::frame::color::EngineColor;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, Rect, Sprite};
use crate::game::{Game, GameHooks, ScreenHook};
use crate::screen::{FrameInput, Screen, Transition};
use crate::screens::key_bindings::KeyBindingsScreen;
use crate::screens::key_bindings::KeyRow;
use crate::screens::key_bindings::RowGroup::{self, Optional, Required};
use crate::screens::layout_picker::{self, LayoutPickerScreen, LegendKeys};
use crate::screens::options::{self, OptionRow, RowAction, RowValue};
use crate::settings::{GameSettings, Settings, TextSpeed};
use crate::storage::Storage;
use shuyi_assets::dialogue::{Present, Side};

pub use crate::input::sample::{Action, LEFT_HANDED, RIGHT_HANDED};

/// The toy game's context.
pub type ToyCtx = Ctx<Toy>;

/// The debug language's code.
pub const DEBUG_LANG: &str = "dbg";

/// Which clip of a voiced line plays: its only one, or the low or the high
/// voice's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum Voice {
    /// The line's only clip.
    #[default]
    Only,
    /// Said in the low voice.
    Low,
    /// Said in the high voice.
    High,
}

impl ClipVariant for Voice {
    fn suffix(self) -> &'static str {
        match self {
            Self::Only => "",
            Self::Low => ".low",
            Self::High => ".high",
        }
    }

    fn split() -> &'static [Self] {
        &[Self::Low, Self::High]
    }
}

/// The toy's two color themes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    /// Light.
    #[default]
    Day,
    /// Dark.
    Night,
}

/// The toy's own setting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prefs {
    /// How fast something of the toy's goes.
    pub speed: u8,
}

/// The saved form of the toy's settings.
#[derive(Serialize, Deserialize)]
#[serde(default, rename = "Toy")]
struct File {
    speed: u8,
    text_speed: TextSpeed,
    fullscreen: bool,
    theme: Theme,
    music_volume: u8,
    sound_volume: u8,
    layout: Option<String>,
}

impl Default for File {
    fn default() -> Self {
        let s = Settings::<Prefs>::default();
        Self {
            speed: s.game.speed,
            text_speed: s.text_speed,
            fullscreen: s.fullscreen,
            theme: s.theme,
            music_volume: s.music_volume,
            sound_volume: s.sound_volume,
            layout: None,
        }
    }
}

impl GameSettings for Prefs {
    type Theme = Theme;

    fn to_ron(s: &Settings<Self>) -> String {
        let file = File {
            speed: s.game.speed,
            text_speed: s.text_speed,
            fullscreen: s.fullscreen,
            theme: s.theme,
            music_volume: s.music_volume,
            sound_volume: s.sound_volume,
            layout: s.layout().map(|l| l.name().to_owned()),
        };
        ron::to_string(&file).unwrap_or_default()
    }

    fn from_ron(text: &str) -> Result<Settings<Self>, String> {
        let file: File = ron::from_str(text).map_err(|e| format!("unreadable: {e}"))?;
        let mut s = Settings::<Self>::default();
        s.game.speed = file.speed;
        s.text_speed = file.text_speed;
        s.fullscreen = file.fullscreen;
        s.theme = file.theme;
        s.music_volume = file.music_volume;
        s.sound_volume = file.sound_volume;
        Ok(match file.layout.as_deref().and_then(Layout::from_name) {
            Some(layout) => s.with_layout(layout),
            None => s,
        })
    }
}

/// The Key bindings screen's rows: every action a player can rebind, with
/// the text key of its label and its group: the required actions, the
/// optional ones, then the dialogue log's own key group.
pub const KEY_ROWS: [KeyRow<Action>; 18] = [
    (Action::CursorUp, "key_bindings.action.cursor_up", Required),
    (
        Action::CursorDown,
        "key_bindings.action.cursor_down",
        Required,
    ),
    (
        Action::CursorLeft,
        "key_bindings.action.cursor_left",
        Required,
    ),
    (
        Action::CursorRight,
        "key_bindings.action.cursor_right",
        Required,
    ),
    (Action::Confirm, "key_bindings.action.confirm", Required),
    (Action::Cancel, "key_bindings.action.cancel", Required),
    (Action::EndTurn, "key_bindings.action.end_turn", Required),
    (Action::Select, "key_bindings.action.select", Optional),
    (
        Action::ConfirmEndTurn,
        "key_bindings.action.confirm_end_turn",
        Optional,
    ),
    (Action::PrevUnit, "key_bindings.action.prev_unit", Optional),
    (Action::NextUnit, "key_bindings.action.next_unit", Optional),
    (Action::Info, "key_bindings.action.info", Optional),
    (
        Action::DangerZone,
        "key_bindings.action.danger_zone",
        Optional,
    ),
    (
        Action::ToggleAutoEnd,
        "key_bindings.action.toggle_auto_end",
        Optional,
    ),
    (Action::Rewind, "key_bindings.action.rewind", Optional),
    (Action::ZoomMap, "key_bindings.action.zoom_map", Optional),
    (Action::Menu, "key_bindings.action.menu", Optional),
    (
        Action::DialogueLog,
        "key_bindings.action.dialogue_log",
        RowGroup::Shared("key_bindings.dialogue_log"),
    ),
];

/// The text keys of the toy's speeds ([`Prefs::speed`]), slowest first.
pub const SPEEDS: [&str; 3] = ["toy.speed.calm", "toy.speed.brisk", "toy.speed.wild"];

/// The options screen's rows: the engine's text speed, the toy's speed,
/// fullscreen, the theme and the volumes; the layout and the Key bindings
/// screen; the toy's level while it has one (not a setting), a row that
/// refuses, and Restore defaults.
pub fn option_rows(ctx: &ToyCtx) -> Vec<OptionRow<Toy>> {
    let mut rows: Vec<OptionRow<Toy>> = vec![
        OptionRow::text_speed(),
        OptionRow::new(
            "toy.speed",
            RowValue::Choice {
                values: |_| SPEEDS.to_vec(),
                get: |ctx| usize::from(ctx.settings().game.speed),
                set: |ctx, i| {
                    let speed = u8::try_from(i).unwrap_or(0);
                    ctx.preview_settings(|s| s.game.speed = speed)
                },
            },
        ),
        OptionRow::fullscreen(),
        OptionRow::new(
            "options.row.theme",
            RowValue::Choice {
                values: |_| vec!["toy.theme.day", "toy.theme.night"],
                get: |ctx| usize::from(ctx.settings().theme == Theme::Night),
                set: |ctx, i| {
                    let theme = if i == 1 { Theme::Night } else { Theme::Day };
                    ctx.preview_settings(|s| s.theme = theme)
                },
            },
        ),
        OptionRow::music_volume(),
        OptionRow::sound_volume(),
        OptionRow::new(
            "options.row.layout",
            RowValue::Action {
                value: |ctx| {
                    let layout = ctx.layout().map(|l| layout_picker::label(ctx, l));
                    Some(layout.unwrap_or_default().to_owned())
                },
                confirm: |ctx| RowAction::Open(Box::new(LayoutPickerScreen::change(ctx, &LEGEND))),
                help: |_| options::HELP_OPEN,
            },
        )
        .starting_group(),
        OptionRow::new(
            "options.row.key_bindings",
            RowValue::Action {
                value: |_| None,
                confirm: |ctx| RowAction::Open(Box::new(KeyBindingsScreen::new(ctx, &KEY_ROWS))),
                help: |_| options::HELP_OPEN,
            },
        ),
    ];
    if ctx.game.level.is_some() {
        let level = OptionRow::<Toy>::new(
            "toy.level",
            RowValue::Choice {
                values: |_| SPEEDS.to_vec(),
                get: |ctx| ctx.game.level.unwrap_or(0),
                set: |ctx, i| {
                    let changed = ctx.game.level != Some(i);
                    ctx.game.level = Some(i);
                    changed
                },
            },
        );
        rows.push(level.unsaved());
    }
    rows.push(OptionRow::new(
        "toy.locked",
        RowValue::Action {
            value: |_| Some("-".to_owned()),
            confirm: |_| RowAction::Refuse(Some("toy.locked.message")),
            help: |_| "options.help.none",
        },
    ));
    rows.push(OptionRow::restore_defaults().starting_group());
    rows
}

/// The layout picker's legend: what the toy's keys do, top to bottom.
pub const LEGEND: [(LegendKeys<Action>, &str); 10] = [
    (LegendKeys::Cursor, "layout_picker.legend.move"),
    (
        LegendKeys::All(Action::Confirm),
        "layout_picker.legend.select",
    ),
    (LegendKeys::All(Action::Cancel), "layout_picker.legend.back"),
    (
        LegendKeys::First(Action::PrevUnit),
        "layout_picker.legend.prev_unit",
    ),
    (
        LegendKeys::First(Action::NextUnit),
        "layout_picker.legend.next_unit",
    ),
    (
        LegendKeys::First(Action::Info),
        "layout_picker.legend.unit_info",
    ),
    (
        LegendKeys::First(Action::DangerZone),
        "layout_picker.legend.danger_zone",
    ),
    (
        LegendKeys::First(Action::EndTurn),
        "layout_picker.legend.end_turn",
    ),
    (
        LegendKeys::First(Action::ToggleAutoEnd),
        "layout_picker.legend.auto_end",
    ),
    (
        LegendKeys::First(Action::Rewind),
        "layout_picker.legend.rewind",
    ),
];

/// The toy's one image: a 16×16 card.
pub const CARD: &str = "card.png";

/// The toy's English text.
const ENGLISH: [(&str, &str); 20] = [
    ("talk.next", "{Confirm}"),
    ("toy.speed", "Speed"),
    ("toy.speed.calm", "Calm"),
    ("toy.speed.brisk", "Brisk"),
    ("toy.speed.wild", "Wild"),
    ("toy.theme.day", "Day"),
    ("toy.theme.night", "Night"),
    ("toy.level", "Level"),
    ("toy.locked", "Locked"),
    ("toy.locked.message", "Not here"),
    ("keys.not_mapped", "! not mapped"),
    ("keys.arrows", "arrows"),
    ("keys.chord_join", "+"),
    ("profile.numbered", "Player {n}"),
    (
        "home.help",
        "{Confirm} opens · {Cancel} quits · {Cursor} {Nope}",
    ),
    ("home.greeting", "Hello, {name}"),
    ("layout_picker.right_handed", "Right-handed"),
    ("layout_picker.left_handed", "Left-handed"),
    ("menu.title", "Menu"),
    ("gallery.help", "{Cancel} back"),
];

/// The toy's English text of the engine's screens (the Key bindings
/// screen, the layout picker, the options, the dialogue box).
const SCREENS_ENGLISH: &str = include_str!("toy/en.ron");

/// The debug language's text: two keys of today's English, one of
/// yesterday's.
const DEBUG_UI: &str = r#"[
    (key: "home.greeting", source: "Hello, {name}", text: "HELLO, {name}"),
    (key: "keys.arrows", source: "arrows", text: "ARROWS"),
    (key: "menu.title", source: "Old menu", text: "OLD MENU"),
]"#;

/// The toy's one scene: two lines.
const SCENE: &str = "\
@scene hall
> The hall is quiet.
guide: Welcome in.
@end
";

/// A palette of the engine's colors, each a shade of `base`, and the grid
/// map's ([`grid::MAP_PALETTE`]).
fn palette(base: u8) -> PaletteDef {
    let engine = EngineColor::ALL
        .iter()
        .zip(0u8..)
        .map(|(c, i)| (c.name().to_owned(), [base, i, i]));
    let map = grid::MAP_PALETTE
        .iter()
        .map(|&(name, rgb)| (name.to_owned(), rgb));
    PaletteDef {
        colors: engine.chain(map).collect(),
    }
}

/// The size of [`CARD`].
const CARD_SIZE: ImageInfo = ImageInfo {
    width: 16,
    height: 16,
};

/// A track of `cue`, looped, 10 s long.
fn track(cue: &str) -> MusicCue {
    MusicCue {
        file: format!("{cue}.ogg"),
        volume: 100,
        looped: true,
        length_ms: 10_000,
        credit: CreditRef::Own,
    }
}

/// The toy's game data and state.
#[derive(Debug)]
pub struct Toy {
    lang: Lang,
    keymap: KeymapDef<Action>,
    /// The cues: the sound `click` and the engine's menu sounds, the track
    /// `theme`, four more in the pool `fights`, and a 0.5 s fade.
    pub audio: AudioManifest,
    day: PaletteDef,
    night: PaletteDef,
    /// The scene `hall`.
    pub dialogue: DialogueTable,
    /// Which clip a voiced line plays.
    pub voice: Voice,
    /// What [`GameData::apply_settings`] was told, oldest first: the speed
    /// now, and before (`None` when they were loaded).
    pub applied: Vec<(u8, Option<u8>)>,
    /// The keys the toy kept a player's data under before profiles.
    pub old_keys: Vec<String>,
    /// The images: [`CARD`] and the grid map's tiles ([`grid::TILES`]).
    pub images: ImageTable,
    /// The lights a grid map may be in ([`grid::lights`]).
    pub lights: Lighting,
    /// The toy's own level, one of [`SPEEDS`]' worth: an options row that
    /// isn't a setting, shown only while it is `Some`.
    pub level: Option<usize>,
}

impl Toy {
    /// The toy game.
    ///
    /// # Panics
    ///
    /// If its own data doesn't load.
    pub fn new() -> Self {
        let mut english: BTreeMap<String, String> =
            ron::from_str(SCREENS_ENGLISH).unwrap_or_else(|e| panic!("toy/en.ron: {e}"));
        english.extend(ENGLISH.iter().map(|&(k, v)| (k.to_owned(), v.to_owned())));
        let rules = PackRules {
            data_problem: |_, _| None,
            tokens: &NoTokens,
            limits: DialogueLimits {
                box_width: 40,
                box_lines: 3,
                max_text: 200,
                max_option: 30,
                min_options: 2,
                max_options: 4,
                max_reaction_texts: 3,
            },
            variants: &[],
        };
        let info = r#"(name: "Debug", made_by: Machine)"#;
        let pack = pack_from_sources(
            ("dbg/lang.ron", info),
            ("dbg/ui.ron", DEBUG_UI),
            &english,
            &rules,
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
        let code = LangCode::new(DEBUG_LANG).unwrap_or_default();
        let lang = Lang::new(english, [(code, pack)].into());
        let mut audio = AudioManifest {
            music_fade_ms: 500,
            ..AudioManifest::default()
        };
        let click = SoundCue {
            files: vec!["click.wav".to_owned()],
            volume: 100,
            credit: CreditRef::Own,
        };
        audio.sounds.insert("click".to_owned(), click.clone());
        // The engine's menu sounds, by their default names.
        let config = crate::audio::AudioConfig::DEFAULT;
        for cue in [
            config.move_cue,
            config.select_cue,
            config.cancel_cue,
            config.cursor_cue,
        ] {
            audio.sounds.insert(cue.to_owned(), click.clone());
        }
        let fights = ["drums", "horns", "pipes", "bells"];
        for cue in ["theme"].iter().chain(&fights) {
            audio.music.insert((*cue).to_owned(), track(cue));
        }
        let fights = fights.map(str::to_owned).to_vec();
        audio.pools.insert("fights".to_owned(), fights);
        let (parsed, errors) = parse_dlg::<NoDirective>("hall.dlg", SCENE);
        assert!(errors.is_empty(), "{errors:?}");
        let dialogue = DialogueTable {
            scenes: parsed
                .into_iter()
                .map(|p| (p.scene.id.clone(), p.scene))
                .collect(),
            files: BTreeMap::new(),
        };
        Self {
            lang,
            keymap: crate::input::sample::load_keymap().unwrap_or_else(|e| panic!("{e:?}")),
            audio,
            day: palette(200),
            night: palette(20),
            dialogue,
            voice: Voice::Only,
            applied: Vec::new(),
            old_keys: Vec::new(),
            images: ImageTable {
                images: [(CARD, CARD_SIZE), (grid::TILES, grid::TILES_SIZE)].into(),
            },
            lights: grid::lights(),
            level: None,
        }
    }

    /// The image [`CARD`]'s id.
    ///
    /// # Panics
    ///
    /// If the toy's table lacks it.
    pub fn card(&self) -> ImageId {
        self.images.id(CARD).unwrap_or_else(|| panic!("no {CARD}"))
    }

    /// The ids of the scene's lines, in order.
    pub fn lines(&self) -> Vec<LineId> {
        self.dialogue.scenes["hall"]
            .lines()
            .iter()
            .map(|line| line.id.clone())
            .collect()
    }

    /// A voice manifest with a clip of each `(line, variant)`, saying what
    /// the line says.
    pub fn manifest(&self, clips: &[(usize, &str)]) -> String {
        let lines = self.dialogue.scenes["hall"].lines();
        let clip = |&(i, variant): &(usize, &str)| {
            let line = &lines[i];
            format!(
                "(line: \"{}\", variant: {variant}, spoken: \"{}\", voice: \"v\", \
                 made_by: Recorded(actor: \"a\")),",
                line.id, line.text
            )
        };
        let clips: String = clips.iter().map(clip).collect();
        format!("(clips: [{clips}])")
    }
}

impl Default for Toy {
    fn default() -> Self {
        Self::new()
    }
}

impl GameData for Toy {
    type Action = Action;
    type Colors = EngineColor;
    type Voice = Voice;
    type Settings = Prefs;

    fn lang(&self) -> &Lang {
        &self.lang
    }

    fn keymap_def(&self) -> &KeymapDef<Action> {
        &self.keymap
    }

    fn audio_manifest(&self) -> &AudioManifest {
        &self.audio
    }

    fn palette_def(&self) -> &PaletteDef {
        &self.day
    }

    fn console(&self) -> ConsoleSpec {
        ConsoleSpec::text(40, 20)
    }

    fn theme_def(&self, theme: Theme) -> Option<&PaletteDef> {
        match theme {
            Theme::Day => Some(&self.day),
            Theme::Night => Some(&self.night),
        }
    }

    fn debug_lang(&self) -> Option<&str> {
        Some(DEBUG_LANG)
    }

    fn voice_variant(&self) -> Voice {
        self.voice
    }

    fn voices(&self, manifest: &str) -> Result<Playable<Voice>, String> {
        let manifest =
            voice::from_source::<Voice, NoDirective>("voice.ron", manifest, &self.dialogue)
                .map_err(|errors| {
                    let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
                    errors.join("; ")
                })?;
        // A clip says what its line says, whatever its variant.
        Ok(manifest.playable(&self.dialogue, |text, _| Some(text.to_owned())))
    }

    fn apply_settings(&mut self, now: &Settings<Prefs>, before: Option<&Settings<Prefs>>) {
        self.applied
            .push((now.game.speed, before.map(|b| b.game.speed)));
    }

    fn keys_before_profiles(&self) -> Vec<String> {
        self.old_keys.clone()
    }

    fn old_layout_key(&self) -> Option<&'static str> {
        Some("layout")
    }
}

/// The toy's context, with the right-handed layout chosen and debug tools
/// on.
pub fn ctx() -> ToyCtx {
    let mut ctx = first_launch();
    ctx.use_layout(RIGHT_HANDED);
    ctx.debug_tools = true;
    ctx
}

/// The toy's context as on a first launch: no layout chosen.
///
/// # Panics
///
/// If the toy's palette lacks a color.
pub fn first_launch() -> ToyCtx {
    Ctx::new(Toy::default()).unwrap_or_else(|missing| panic!("{missing:?}"))
}

/// The toy's context as a later launch finds it in `storage`: with the
/// layout saved there in use, if one is, and debug tools on.
pub fn launch(storage: Box<dyn Storage>) -> ToyCtx {
    let mut ctx = first_launch().with_storage(storage);
    if let Some(layout) = ctx.saved_layout() {
        ctx.use_layout(layout);
    }
    ctx.debug_tools = true;
    ctx
}

/// Where the toy game has a say in the frame loop: with no layout chosen
/// the first key opens the [`Picker`], and the Debug key opens the
/// [`Gallery`].
pub fn hooks() -> GameHooks<Toy> {
    GameHooks {
        first_key: Some(ScreenHook {
            name: "picker",
            open: |_| Box::new(Picker::default()),
        }),
        debug: Some(ScreenHook {
            name: "gallery",
            open: |_| Box::new(Gallery),
        }),
        debug_screens: &["gallery"],
    }
}

/// The toy game on `ctx`, at its [`Home`], with its [`hooks`].
pub fn game_on(ctx: ToyCtx) -> Game<Toy> {
    Game::new(ctx, Box::new(Home::default()), hooks())
}

/// The toy game with the right-handed layout chosen ([`ctx`]), at its
/// [`Home`], with its [`hooks`].
pub fn game() -> Game<Toy> {
    game_on(ctx())
}

/// The toy game with the right-handed layout chosen, at its [`Lobby`],
/// with its [`hooks`].
pub fn lobby() -> Game<Toy> {
    Game::new(ctx(), Box::new(Lobby::new()), hooks())
}

/// The toy's lobby: an engine [`Menu`](crate::widgets::Menu) of what the
/// engine's screens offer: the options, the Key bindings screen, a talk
/// ([`Talk`]) and quitting. Nothing cancels it.
#[derive(Debug)]
pub struct Lobby {
    /// The menu.
    pub menu: crate::widgets::Menu<LobbyChoice>,
}

/// What an item of the [`Lobby`] stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyChoice {
    /// Opens the options.
    Options,
    /// Opens the Key bindings screen.
    KeyBindings,
    /// Opens the [`Talk`].
    Talk,
    /// Quits.
    Quit,
}

impl Lobby {
    /// The lobby's items, top to bottom.
    pub const ITEMS: [&str; 4] = ["Options", "Key bindings", "Talk", "Quit"];

    /// What the [`ITEMS`](Self::ITEMS) stand for, in their order.
    const CHOICES: [LobbyChoice; 4] = [
        LobbyChoice::Options,
        LobbyChoice::KeyBindings,
        LobbyChoice::Talk,
        LobbyChoice::Quit,
    ];

    /// The lobby, on its first item.
    pub fn new() -> Self {
        let items = Self::ITEMS
            .iter()
            .zip(Self::CHOICES)
            .map(|(&i, choice)| crate::widgets::MenuItem::new(i).with_value(choice));
        Self {
            menu: crate::widgets::Menu::new(items.collect()).without_cancel(),
        }
    }
}

impl Default for Lobby {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen<Toy> for Lobby {
    fn name(&self) -> &'static str {
        "lobby"
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for &action in &input.actions {
            let Some(crate::widgets::MenuEvent::Chosen(choice)) =
                self.menu.handle_with_sound(action, &mut ctx.audio)
            else {
                continue;
            };
            return match choice {
                LobbyChoice::Options => {
                    Transition::Push(Box::new(options::OptionsScreen::new(option_rows)))
                }
                LobbyChoice::KeyBindings => {
                    Transition::Push(Box::new(KeyBindingsScreen::new(ctx, &KEY_ROWS)))
                }
                LobbyChoice::Talk => Transition::Push(Box::new(Talk::new(ctx, "hall"))),
                LobbyChoice::Quit => Transition::Quit,
            };
        }
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let bg = ctx.palette.engine(EngineColor::Background);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        crate::widgets::menu::glyph::paint(ctx, &self.menu.view(), buf, 1, 1);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

/// Where a [`Talk`]'s text box goes: the bottom seven rows of the toy's
/// console.
pub const TALK_BOX: BoxPlace = BoxPlace {
    frame: Rect::new(0, 13, 40, 7),
    text_x: 2,
    text_y: 15,
};

/// A scene of the toy's, read in the engine's text box: Confirm reveals the
/// page, then goes on; holding it types faster; Cancel leaves. Pops when
/// the scene ends.
#[derive(Debug)]
pub struct Talk {
    /// The scene playing.
    pub player: DialoguePlayer<Plain>,
    /// Its text box.
    pub text: TextBox,
}

impl Talk {
    /// Characters per line of the box.
    const WIDTH: usize = 34;
    /// Lines per page.
    const LINES: usize = 3;

    /// The toy's scene `id`, everyone there (an empty scene if it hasn't
    /// one).
    pub fn new(ctx: &ToyCtx, id: &str) -> Self {
        let scene = ctx.game.dialogue.get(id).cloned().unwrap_or_default();
        let player = DialoguePlayer::new(&scene, Plain, &Present::Everyone);
        let text = Self::text_of(&player);
        Self { player, text }
    }

    fn text_of(player: &DialoguePlayer<Plain>) -> TextBox {
        let text = player.current().text.unwrap_or_default();
        TextBox::new(&text, Self::WIDTH, Self::LINES)
    }

    /// What its text box shows.
    pub fn view(&self, ctx: &ToyCtx) -> TextView {
        let waiting = self.text.is_revealed().then(|| WaitView {
            key: ctx.text_with("talk.next", &[]),
            log_hint: None,
            marker: self.text.marker_lit(),
        });
        TextView {
            narration: self.player.current().narration,
            lines: self.text.page_lines().to_vec(),
            shown: self.text.shown(),
            choices: None,
            skip: None,
            waiting,
        }
    }
}

impl Screen<Toy> for Talk {
    fn name(&self) -> &'static str {
        "talk"
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        if self.player.is_finished() {
            return Transition::Pop;
        }
        for &action in &input.actions {
            match action {
                Action::Cancel => return Transition::Pop,
                Action::Confirm if !self.text.is_revealed() => self.text.reveal(),
                Action::Confirm if !self.text.next_page() => {
                    self.player.advance();
                    if self.player.is_finished() {
                        return Transition::Pop;
                    }
                    self.text = Self::text_of(&self.player);
                }
                _ => {}
            }
        }
        self.text.hold(input.is_held(Action::Confirm), input.dt);
        self.text
            .type_on(input.dt, ctx.settings().text_speed.chars_per_s());
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        // Whoever speaks, by their id, if they stand on a side.
        let view = self.player.current();
        let speaking = match view.speaker {
            Some(Side::Left) => view.left,
            Some(Side::Right) => view.right,
            None => None,
        };
        let speaker = speaking.map(|p| p.character.0.as_str());
        text_box::glyph::paint(ctx, buf, TALK_BOX, &self.view(ctx), speaker);
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

/// The toy's root screen: Confirm opens the [`Menu`] over it, Cancel quits.
/// It draws `H` in its top-left cell.
#[derive(Debug, Default)]
pub struct Home {
    /// How many frames it has seen.
    pub frames: usize,
}

impl Screen<Toy> for Home {
    fn name(&self) -> &'static str {
        "home"
    }

    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        self.frames += 1;
        for action in &input.actions {
            match action {
                Action::Confirm => return Transition::Push(Box::new(Menu)),
                Action::Cancel => return Transition::Quit,
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let text = ctx.palette.engine(EngineColor::Text);
        buf.set(
            0,
            0,
            Cell::new('H', text, ctx.palette.engine(EngineColor::Panel)),
        );
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

/// An overlay: Cancel closes it, Confirm clicks. It draws `M` in the
/// second cell.
#[derive(Debug)]
pub struct Menu;

impl Screen<Toy> for Menu {
    fn name(&self) -> &'static str {
        "menu"
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for action in &input.actions {
            match action {
                Action::Cancel => return Transition::Pop,
                Action::Confirm => ctx.audio.play_sound("click"),
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let text = ctx.palette.engine(EngineColor::Highlight);
        buf.set(
            1,
            0,
            Cell::new('M', text, ctx.palette.engine(EngineColor::Panel)),
        );
    }

    fn is_overlay(&self) -> bool {
        true
    }
}

/// The layout picker: Up and Down choose a layout, Confirm picks it and
/// closes; nothing cancels it. It draws `P` in the third cell and the
/// chosen layout's name on the second row.
#[derive(Debug, Default)]
pub struct Picker {
    /// The layout the cursor is on: 0 right-handed, 1 left-handed.
    pub focus: usize,
}

impl Picker {
    /// The layouts, in the order shown.
    pub const LAYOUTS: [Layout; 2] = [RIGHT_HANDED, LEFT_HANDED];
}

impl Screen<Toy> for Picker {
    fn name(&self) -> &'static str {
        "picker"
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for action in &input.actions {
            match action {
                Action::CursorUp | Action::CursorDown => self.focus = 1 - self.focus,
                Action::Confirm => {
                    if let Err(e) = ctx.choose_layout(Self::LAYOUTS[self.focus]) {
                        panic!("saving the layout: {e}");
                    }
                    return Transition::Pop;
                }
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let text = ctx.palette.engine(EngineColor::Text);
        let panel = ctx.palette.engine(EngineColor::Panel);
        buf.set(2, 0, Cell::new('P', text, panel));
        let name = if self.focus == 0 {
            "layout_picker.right_handed"
        } else {
            "layout_picker.left_handed"
        };
        buf.print(0, 1, ctx.text(name), text, panel);
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

/// A debug tool showing pictures: the [`CARD`] whole at twice its size in
/// the top-left corner, and again half off the right edge, so a snapshot
/// lists one whole and one clipped. Its help line names the Cancel key
/// of the device used last; Cancel closes it.
#[derive(Debug)]
pub struct Gallery;

impl Screen<Toy> for Gallery {
    fn name(&self) -> &'static str {
        "gallery"
    }

    fn update(&mut self, _: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        if input.actions.contains(&Action::Cancel) {
            return Transition::Pop;
        }
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let card = ctx.game.card();
        let src = Rect::new(0, 0, 16, 16);
        let right = buf.pixel_bounds().w - 16;
        for dest in [Rect::new(0, 0, 32, 32), Rect::new(right, 0, 32, 32)] {
            buf.add_sprite(Sprite::new(card, src, dest, Layer::Over));
        }
        let text = ctx.palette.engine(EngineColor::Text);
        let panel = ctx.palette.engine(EngineColor::Panel);
        let last = i32::from(buf.height()) - 1;
        buf.print(0, last, &ctx.text_with("gallery.help", &[]), text, panel);
    }
}
