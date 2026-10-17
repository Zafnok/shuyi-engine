//! A toy game for the engine's tests of the screens, the context, the
//! frame loop and the test `Harness`: the sample
//! game's actions and keymap file ([`crate::input::sample`]), a palette of
//! the engine's colors in two themes, a few lines of text in English and
//! in a debug language, one sound, three tracks and a pool, voiced lines
//! with two variants, one setting of its own, one image, and its screens:
//! [`Home`], the [`Menu`] over it, the layout [`Picker`] the first key
//! opens and the [`Gallery`] of pictures the Debug key opens ([`hooks`]).
//!
//! Built for this crate's tests and, with the `toy` feature, for its
//! integration tests.

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
use shuyi_assets::palette::PaletteDef;
use shuyi_assets::voice::{self, ClipVariant, Playable};

use crate::ctx::{Ctx, GameData};
use crate::frame::color::EngineColor;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, Rect, Sprite};
use crate::game::{Game, GameHooks, ScreenHook};
use crate::screen::{FrameInput, Screen, Transition};
use crate::settings::{GameSettings, Settings, TextSpeed};
use crate::storage::Storage;

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

/// The toy's one image: a 16×16 card.
pub const CARD: &str = "card.png";

/// The toy's English text.
const ENGLISH: [(&str, &str); 10] = [
    ("keys.not_mapped", "! not mapped"),
    ("keys.arrows", "arrows"),
    ("keys.chord_join", "+"),
    ("profile.numbered", "Player {n}"),
    (
        "home.help",
        "{Confirm} opens · {Cancel} quits · {Cursor} {Nope}",
    ),
    ("home.greeting", "Hello, {name}"),
    ("layout_picker.right_handed", "Right hand"),
    ("layout_picker.left_handed", "Left hand"),
    ("menu.title", "Menu"),
    ("gallery.help", "{Cancel} back"),
];

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

/// A palette of the engine's colors, each a shade of `base`.
fn palette(base: u8) -> PaletteDef {
    let colors = EngineColor::ALL
        .iter()
        .zip(0u8..)
        .map(|(c, i)| (c.name().to_owned(), [base, i, i]))
        .collect();
    PaletteDef { colors }
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
    /// The cues: the sound `click`, the track `theme`, four more in the
    /// pool `fights`, and a 0.5 s fade.
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
    /// The image [`CARD`].
    pub images: ImageTable,
}

impl Toy {
    /// The toy game.
    ///
    /// # Panics
    ///
    /// If its own data doesn't load.
    pub fn new() -> Self {
        let english: BTreeMap<String, String> = ENGLISH
            .iter()
            .map(|&(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
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
        audio.sounds.insert("click".to_owned(), click);
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
                images: [(CARD, CARD_SIZE)].into(),
            },
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
