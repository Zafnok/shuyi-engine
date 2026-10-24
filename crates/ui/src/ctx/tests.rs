//! The context's services on the toy game: text, keys and layouts,
//! voices, music picks, settings and profiles; and what a game gets when
//! it leaves [`GameData`]'s defaults alone.

use super::*;

use shuyi_assets::keymap::{Button, Chord, Key, RepeatDef};

use crate::audio::AudioRequest;
use crate::input::help_keys::key_name;
use crate::storage::MemoryStorage;
use crate::toy::{Action, DEBUG_LANG, LEFT_HANDED, Prefs, RIGHT_HANDED, Theme, Toy, ToyCtx, Voice};
use crate::toy::{ctx, first_launch};

/// The toy's settings.
type Settings = crate::settings::Settings<Prefs>;

fn code(code: &str) -> LangCode {
    LangCode::new(code).unwrap()
}

/// A storage whose every operation fails.
#[derive(Debug)]
struct Failing;

impl Storage for Failing {
    fn read(&self, _: &str) -> Result<Option<String>, StorageError> {
        Err(StorageError::Backend("down".into()))
    }
    fn write(&mut self, _: &str, _: &str) -> Result<(), StorageError> {
        Err(StorageError::Backend("down".into()))
    }
    fn delete(&mut self, _: &str) -> Result<(), StorageError> {
        Err(StorageError::Backend("down".into()))
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        Err(StorageError::Backend("down".into()))
    }
}

/// A first launch on `storage`.
fn launch(storage: impl Storage + 'static) -> ToyCtx {
    first_launch().with_storage(Box::new(storage))
}

#[test]
fn a_new_ctx_starts_on_the_games_palette_console_and_picker_keys() {
    let c = first_launch();
    let toy = Toy::new();
    let day = Palette::new::<EngineColor>(toy.palette_def()).unwrap();
    assert_eq!(c.palette, day);
    assert_eq!(c.console, ConsoleSpec::text(40, 20));
    assert_eq!(c.keymap, Keymap::layout_picker(toy.keymap_def()));
    assert_eq!(c.layout(), None);
    assert_eq!(c.saved_layout(), None);
    assert_eq!(c.device, Device::Keyboard);
    assert_eq!(*c.settings(), Settings::default());
    assert_eq!(c.profiles().all().len(), 1);
    assert_eq!(c.profiles().all()[0].name, "Player 1");
    assert_eq!(c.debug_tools, crate::DEBUG_TOOLS);
    assert_eq!(c.idle_motion, IdleMotion::Off);
    assert_eq!(c.keyboard, Keyboard::Present);
    assert_eq!(c.key_prompt, KeyPrompt::Off);
    assert_eq!(c.music_seed, DEFAULT_MUSIC_SEED);
    assert_eq!(c.music_clock, None);
    let found = c.audio.pending();
    assert!(found.is_empty(), "{found:?}");
    assert!(c.clock_s.abs() < f64::EPSILON);
    assert!(c.voices_on);
    assert_eq!(c.voice_volume, DEFAULT_VOICE_VOLUME);
    assert_eq!(c.lang, LangCode::english());
    assert!(c.game.applied.is_empty(), "nothing loaded yet");
}

#[test]
fn a_palette_lacking_colors_is_refused_with_their_names() {
    #[derive(Debug)]
    struct Pale(Toy, PaletteDef);
    impl GameData for Pale {
        type Action = Action;
        type Colors = EngineColor;
        type Voice = Voice;
        type Settings = Prefs;
        fn lang(&self) -> &Lang {
            self.0.lang()
        }
        fn keymap_def(&self) -> &KeymapDef<Action> {
            self.0.keymap_def()
        }
        fn audio_manifest(&self) -> &AudioManifest {
            self.0.audio_manifest()
        }
        fn palette_def(&self) -> &PaletteDef {
            &self.1
        }
        fn console(&self) -> ConsoleSpec {
            self.0.console()
        }
        fn theme_def(&self, _: Theme) -> Option<&PaletteDef> {
            None
        }
        fn voice_variant(&self) -> Voice {
            Voice::Only
        }
    }
    let toy = Toy::new();
    let mut def = toy.palette_def().clone();
    def.colors.remove("text");
    def.colors.remove("border");
    let err = Ctx::new(Pale(toy, def)).unwrap_err();
    assert_eq!(err, ["text", "border"]);
}

#[test]
fn text_comes_from_the_language_in_use() {
    let mut c = ctx();
    assert_eq!(c.text("home.greeting"), "Hello, {name}");
    c.lang = code(DEBUG_LANG);
    assert_eq!(c.effective_lang(), &code(DEBUG_LANG));
    assert_eq!(c.text("home.greeting"), "HELLO, {name}");
    // Stale and missing entries are English.
    assert_eq!(c.text("menu.title"), "Menu");
    assert_eq!(c.text("profile.numbered"), "Player {n}");
    // The debug language is only for builds with debug tools.
    c.debug_tools = false;
    assert_eq!(c.effective_lang(), &LangCode::english());
    assert_eq!(c.text("home.greeting"), "Hello, {name}");
    // Another language with no pack is English, debug tools or not.
    c.lang = code("zz");
    assert_eq!(c.effective_lang(), &code("zz"));
    assert_eq!(c.text("home.greeting"), "Hello, {name}");
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "text key not in the English text: \"home.nope\"")]
fn an_unknown_text_key_panics_in_debug_builds() {
    let c = ctx();
    let _ = c.text("home.nope");
}

#[test]
fn text_with_fills_values_then_the_keys_actions_are_on() {
    let mut c = ctx();
    assert_eq!(
        c.text_with("home.help", &[]),
        "f opens · d quits · arrows {Nope}"
    );
    assert_eq!(
        c.text_with("home.greeting", &[("name", &"Mei")]),
        "Hello, Mei"
    );
    // A value named like an action fills it instead.
    assert_eq!(
        c.text_with("home.help", &[("Confirm", &7)]),
        "7 opens · d quits · arrows {Nope}"
    );
    c.use_layout(LEFT_HANDED);
    assert_eq!(
        c.text_with("home.help", &[]),
        "j opens · k quits · wasd {Nope}"
    );
    c.lang = code(DEBUG_LANG);
    assert_eq!(c.text_with("home.greeting", &[("name", &1)]), "HELLO, 1");
}

#[test]
fn a_label_is_a_keys_text_in_the_players_language_or_text_as_it_is() {
    use crate::text::Label;
    let mut c = ctx();
    let greeting = Label::key("home.greeting").with_arg("name", "Mei");
    assert_eq!(c.label(&greeting), "Hello, Mei");
    assert_eq!(
        c.label(&Label::key("home.help")),
        "f opens · d quits · arrows {Nope}"
    );
    assert_eq!(c.label(&Label::from("home.greeting")), "home.greeting");
    // Text takes no values.
    assert_eq!(
        c.label(&Label::from("{name}").with_arg("name", 1)),
        "{name}"
    );
    c.lang = code(DEBUG_LANG);
    assert_eq!(c.label(&greeting), "HELLO, Mei");
}

#[test]
fn help_keys_name_keys_on_the_device_with_the_languages_words() {
    let mut c = ctx();
    assert_eq!(c.help_keys().not_mapped(), "! not mapped");
    assert_eq!(c.help_keys().device(), Device::Keyboard);
    let cursor_keys = crate::input::help_keys::cursor_keys_name;
    assert_eq!(cursor_keys(c.help_keys()), "arrows");
    c.lang = code(DEBUG_LANG);
    assert_eq!(cursor_keys(c.help_keys()), "ARROWS");
    c.device = Device::Pad(crate::input::pad::PadKind::Xbox);
    assert_eq!(c.help_keys().device(), c.device);
    c.keymap = Keymap::new([], RepeatDef::default());
    assert_eq!(key_name(c.help_keys(), Action::Confirm), "! not mapped");
}

#[test]
fn music_picks_roll_afresh_each_time_from_the_seed() {
    let mut c = ctx();
    c.music_seed = 0x29;
    let manifest = c.game.audio.clone();
    let expected: Vec<_> = (0..32)
        .map(|n| pick_from_pool(&manifest, "fights", 0x29 ^ n).map(str::to_owned))
        .collect();
    let picked: Vec<_> = (0..32).map(|_| c.pick_music("fights")).collect();
    assert_eq!(picked, expected);
    assert!(picked.iter().all(Option::is_some));
    // An unknown pool picks nothing, but counts as a pick.
    assert_eq!(c.pick_music("nope"), None);
    let next = pick_from_pool(&manifest, "fights", 0x29 ^ 0x21).map(str::to_owned);
    assert_eq!(c.pick_music("fights"), next);
}

fn play(line: &LineId, variant: Voice) -> AudioRequest<Voice> {
    AudioRequest::PlayVoice {
        line: line.clone(),
        variant,
    }
}

/// A line with a clip is asked for; one without, or with voices off, is
/// not.
#[test]
fn only_lines_with_a_playable_clip_are_said() {
    let mut c = ctx();
    let lines = c.game.lines();
    // No manifest yet: nothing has a voice.
    c.play_voice(&lines[0]);
    assert!(!c.has_voice(&lines[0]));
    let found = c.audio.pending();
    assert!(found.is_empty(), "{found:?}");
    let manifest = c.game.manifest(&[(0, "Only")]);
    c.set_voice_manifest(&manifest);
    let found = c.take_warnings();
    assert!(found.is_empty(), "{found:?}");
    assert!(c.has_voice(&lines[0]));
    assert!(!c.has_voice(&lines[1]));
    c.play_voice(&lines[0]);
    c.play_voice(&lines[1]);
    c.stop_voice();
    assert_eq!(
        c.audio.take(),
        [play(&lines[0], Voice::Only), AudioRequest::StopVoice]
    );
    c.voices_on = false;
    assert!(!c.has_voice(&lines[0]));
    c.play_voice(&lines[0]);
    c.preload_voices(&lines);
    let found = c.audio.pending();
    assert!(found.is_empty(), "{found:?}");
    // Stopping is always passed on: a voice may still be playing.
    c.stop_voice();
    assert_eq!(c.audio.take(), [AudioRequest::StopVoice]);
}

/// A line whose words change plays the clip of the game's variant.
#[test]
fn a_split_line_plays_the_games_variant() {
    let mut c = ctx();
    let lines = c.game.lines();
    let manifest = c.game.manifest(&[(1, "Low"), (1, "High")]);
    c.set_voice_manifest(&manifest);
    let found = c.take_warnings();
    assert!(found.is_empty(), "{found:?}");
    c.game.voice = Voice::Low;
    c.play_voice(&lines[1]);
    c.game.voice = Voice::High;
    c.play_voice(&lines[1]);
    assert_eq!(
        c.audio.take(),
        [play(&lines[1], Voice::Low), play(&lines[1], Voice::High)]
    );
}

#[test]
fn preload_lists_the_playable_clips_in_script_order() {
    let mut c = ctx();
    let lines = c.game.lines();
    let manifest = c.game.manifest(&[(1, "Only"), (0, "Only")]);
    c.set_voice_manifest(&manifest);
    c.preload_voices(&lines);
    let expected = vec![
        (lines[0].clone(), Voice::Only),
        (lines[1].clone(), Voice::Only),
    ];
    assert_eq!(
        c.audio.take(),
        [AudioRequest::PreloadVoices { lines: expected }]
    );
    let none = c.game.manifest(&[]);
    c.set_voice_manifest(&none);
    c.preload_voices(&lines);
    c.preload_voices(&[]);
    assert!(c.audio.pending().is_empty(), "nothing playable: no request");
}

#[test]
fn a_manifest_the_game_refuses_leaves_no_voices_and_a_warning() {
    let mut c = ctx();
    let lines = c.game.lines();
    c.set_voice_manifest(&c.game.manifest(&[(0, "Only")]));
    assert!(c.has_voice(&lines[0]));
    c.set_voice_manifest("not ron");
    let warnings = c.take_warnings();
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].starts_with("no voices: voice.ron:1"),
        "{warnings:?}"
    );
    assert!(!c.has_voice(&lines[0]), "the earlier manifest is gone");
    let found = c.take_warnings();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_voice_volume_is_a_factor_of_ten_steps() {
    let mut c = ctx();
    assert!((c.voice_gain() - 0.8).abs() < f32::EPSILON);
    for (volume, gain) in [(0, 0.0), (5, 0.5), (10, 1.0), (200, 1.0)] {
        c.voice_volume = volume;
        assert!((c.voice_gain() - gain).abs() < f32::EPSILON, "{volume}");
    }
}

#[test]
fn use_layout_switches_without_saving() {
    let mut c = first_launch();
    c.use_layout(LEFT_HANDED);
    assert_eq!(c.layout(), Some(LEFT_HANDED));
    let def = c.game.keymap_def().clone();
    assert_eq!(c.keymap, Keymap::for_layout(&def, LEFT_HANDED));
    assert_eq!(
        c.keymap_for(RIGHT_HANDED),
        Keymap::for_layout(&def, RIGHT_HANDED)
    );
    assert_eq!(c.storage.read(SETTINGS_KEY), Ok(None));
    assert_eq!(c.saved_layout(), None);
    assert_eq!(c.with_layout(RIGHT_HANDED).layout(), Some(RIGHT_HANDED));
}

#[test]
fn choose_layout_saves_it_in_the_settings_and_saved_layout_reads_it_back() {
    for layout in [RIGHT_HANDED, LEFT_HANDED] {
        let mut c = first_launch();
        assert_eq!(c.choose_layout(layout), Ok(()));
        assert_eq!(c.layout(), Some(layout));
        assert_eq!(c.settings().layout(), Some(layout));
        let saved = c.storage.read(SETTINGS_KEY).unwrap().unwrap();
        assert_eq!(saved, c.settings().to_ron());
        assert_eq!(c.saved_layout(), Some(layout));
        assert_eq!(c.storage.read("layout"), Ok(None), "never the old key");
    }
}

#[test]
fn the_shown_layout_is_the_one_in_use_else_the_files_first() {
    let c = first_launch();
    assert_eq!(c.shown_layout(), Some(RIGHT_HANDED));
    assert_eq!(c.with_layout(LEFT_HANDED).shown_layout(), Some(LEFT_HANDED));
}

#[test]
fn a_saved_layout_the_keymap_doesnt_list_is_ignored() {
    let mut storage = MemoryStorage::new();
    let first = ProfileId::FIRST;
    storage
        .write(&first.key(SETTINGS_KEY), "Toy(layout: Some(\"Vim\"))")
        .unwrap();
    storage.write(&first.key("layout"), "Dvorak").unwrap();
    let c = launch(storage);
    assert_eq!(c.saved_layout(), None);
    assert_eq!(c.settings().layout(), None);
    // The old key still counts when the settings name an unknown one, and
    // spaces around it don't matter.
    let mut storage = MemoryStorage::new();
    storage
        .write(&first.key(SETTINGS_KEY), "Toy(layout: Some(\"Vim\"))")
        .unwrap();
    storage
        .write(&first.key("layout"), " LeftHanded\n")
        .unwrap();
    let c = launch(storage);
    assert_eq!(c.saved_layout(), Some(LEFT_HANDED));
    assert_eq!(c.settings().layout(), Some(LEFT_HANDED));
    // A layout of the settings beats the old key's.
    let mut storage = MemoryStorage::new();
    storage
        .write(
            &first.key(SETTINGS_KEY),
            "Toy(layout: Some(\"RightHanded\"))",
        )
        .unwrap();
    storage.write(&first.key("layout"), "LeftHanded").unwrap();
    assert_eq!(launch(storage).saved_layout(), Some(RIGHT_HANDED));
}

#[test]
fn settings_are_saved_on_change_and_loaded_with_the_storage() {
    let mut c = ctx();
    assert_eq!(*c.settings(), Settings::default());
    // Nothing changed: nothing written.
    assert_eq!(c.change_settings(|_| {}), Ok(()));
    assert_eq!(c.storage.read(SETTINGS_KEY), Ok(None));
    c.change_settings(|s| {
        s.music_volume = 3;
        s.game.speed = 5;
    })
    .unwrap();
    assert_eq!(c.settings().music_volume, 3);
    let saved = c.storage.read(SETTINGS_KEY).unwrap().unwrap();
    assert_eq!(saved, c.settings().to_ron());
    let storage = core::mem::take(&mut c.storage).into_computer();
    let mut again = first_launch().with_storage(storage);
    assert_eq!(again.settings(), c.settings());
    let found = again.take_warnings();
    assert!(found.is_empty(), "{found:?}");
    // The game was told what it loaded.
    assert_eq!(again.game.applied, [(5, None)]);
}

/// The game is told of every change, with what came before; the theme
/// changes the palette.
#[test]
fn a_change_is_applied_to_the_game_and_the_theme_to_the_palette() {
    let mut c = ctx();
    let day = c.palette.clone();
    c.change_settings(|s| s.game.speed = 2).unwrap();
    assert_eq!(c.game.applied, [(2, Some(0))]);
    assert_eq!(c.palette, day);
    c.change_settings(|s| s.theme = Theme::Night).unwrap();
    assert_eq!(c.game.applied[1], (2, Some(2)));
    let night = Palette::new::<EngineColor>(c.game.theme_def(Theme::Night).unwrap()).unwrap();
    assert_eq!(c.palette, night);
    c.change_settings(|s| s.theme = Theme::Day).unwrap();
    assert_eq!(c.palette, day);
    // A theme loaded with the settings is applied too.
    c.change_settings(|s| s.theme = Theme::Night).unwrap();
    let storage = core::mem::take(&mut c.storage).into_computer();
    assert_eq!(first_launch().with_storage(storage).palette, night);
}

/// A previewed change applies at once and is saved only when kept;
/// restoring puts it back and saves nothing unless the storage holds
/// something else.
#[test]
fn previewed_settings_are_saved_only_when_kept() {
    let mut c = ctx();
    assert!(!c.preview_settings(|_| {}));
    let before = c.settings().clone();
    assert!(c.preview_settings(|s| {
        s.theme = Theme::Night;
        s.fullscreen = true;
    }));
    assert!(c.settings().fullscreen);
    assert_ne!(
        c.palette.engine(EngineColor::Text),
        c.palette_for(Theme::Day)
    );
    assert_eq!(c.storage.read(SETTINGS_KEY), Ok(None));
    // Put back: as before, nothing written.
    c.restore_settings(&before).unwrap();
    assert_eq!(*c.settings(), before);
    assert_eq!(
        c.palette.engine(EngineColor::Text),
        c.palette_for(Theme::Day)
    );
    assert_eq!(c.storage.read(SETTINGS_KEY), Ok(None));
    // Kept: written; keeping again writes nothing new.
    c.preview_settings(|s| s.music_volume = 3);
    c.keep_settings().unwrap();
    let saved = c.storage.read(SETTINGS_KEY).unwrap().unwrap();
    assert_eq!(Settings::from_ron(&saved).unwrap().music_volume, 3);
    c.storage.write(SETTINGS_KEY, "marker").unwrap();
    c.keep_settings().unwrap();
    assert_eq!(c.storage.read(SETTINGS_KEY).unwrap().unwrap(), "marker");
    // Restoring what storage holds writes it.
    c.restore_settings(&before).unwrap();
    let saved = c.storage.read(SETTINGS_KEY).unwrap().unwrap();
    assert_eq!(Settings::from_ron(&saved), Ok(before));
    // A layout picked meanwhile saves only the layout.
    c.preview_settings(|s| s.music_volume = 9);
    c.choose_layout(LEFT_HANDED).unwrap();
    let saved = c.storage.read(SETTINGS_KEY).unwrap().unwrap();
    let saved = Settings::from_ron(&saved).unwrap();
    assert_eq!(
        (saved.music_volume, saved.layout()),
        (80, Some(LEFT_HANDED))
    );
    assert_eq!(c.settings().music_volume, 9);
}

impl ToyCtx {
    /// The text color of `theme`'s palette.
    fn palette_for(&self, theme: Theme) -> crate::frame::color::Rgb {
        let def = self.game.theme_def(theme).unwrap();
        Palette::new::<EngineColor>(def)
            .unwrap()
            .engine(EngineColor::Text)
    }
}

#[test]
fn change_settings_cant_change_the_layout() {
    let mut c = ctx();
    c.choose_layout(LEFT_HANDED).unwrap();
    c.change_settings(|s| *s = Settings::default()).unwrap();
    assert_eq!(c.settings().layout(), Some(LEFT_HANDED));
    assert_eq!(c.layout(), Some(LEFT_HANDED));
    assert!(!c.preview_settings(|s| *s = Settings::default().with_layout(RIGHT_HANDED)));
}

#[test]
fn unreadable_settings_give_the_defaults_and_a_warning() {
    let mut storage = MemoryStorage::new();
    storage
        .write(&ProfileId::FIRST.key(SETTINGS_KEY), "nonsense")
        .unwrap();
    let mut c = launch(storage);
    assert_eq!(*c.settings(), Settings::default());
    let warnings = c.take_warnings();
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].starts_with("settings: unreadable: ")
            && warnings[0].ends_with(", using the defaults"),
        "{warnings:?}"
    );
}

#[test]
fn a_failing_storage_leaves_the_defaults_and_a_warning_for_each() {
    let mut c = launch(Failing);
    assert_eq!(c.player_keys(), &PlayerKeys::default());
    assert_eq!(
        c.take_warnings(),
        [
            "profiles: can't be read: storage error: down",
            "keybindings: can't be read, using the default keys: storage error: down",
            "settings: can't be read: storage error: down, using the defaults"
        ]
    );
    assert_eq!(*c.settings(), Settings::default());
    let found = c.take_warnings();
    assert!(found.is_empty(), "{found:?}");
    // A failed save still changes the settings, and switches the layout.
    assert!(c.change_settings(|s| s.sound_volume = 1).is_err());
    assert_eq!(c.settings().sound_volume, 1);
    assert!(c.choose_layout(LEFT_HANDED).is_err());
    assert_eq!(c.layout(), Some(LEFT_HANDED));
}

/// `Info` on `g` instead of its default key.
fn info_on_g(c: &ToyCtx, layout: Layout) -> LayoutBindings<Action> {
    let mut b = c.layout_bindings(layout);
    assert_eq!(b.bind(Action::Info, 0, Chord::plain(Key::G)), Ok(None));
    b
}

#[test]
fn set_layout_bindings_saves_and_switches_the_layout_in_use() {
    let mut c = ctx();
    let def = c.game.keymap_def().clone();
    let b = info_on_g(&c, RIGHT_HANDED);
    assert_eq!(c.set_layout_bindings(RIGHT_HANDED, b.clone()), Ok(()));
    assert_eq!(c.keymap, b.keymap(def.repeat).with_default_pad(&def));
    assert_eq!(c.layout_bindings(RIGHT_HANDED), b);
    let saved = c.storage.read(KEYBINDINGS_KEY).unwrap().unwrap();
    assert_eq!(saved, c.player_keys().to_ron());
    // The other layout's keys change only the saved bindings.
    let before = c.keymap.clone();
    let left = info_on_g(&c, LEFT_HANDED);
    c.set_layout_bindings(LEFT_HANDED, left.clone()).unwrap();
    assert_eq!(c.keymap, before);
    c.use_layout(LEFT_HANDED);
    assert_eq!(c.keymap, left.keymap(def.repeat).with_default_pad(&def));
    assert_eq!(c.keymap_for(RIGHT_HANDED), before);
    // They load with the storage, already in use for a layout in use.
    let storage = core::mem::take(&mut c.storage).into_computer();
    let mut again = ctx().with_storage(storage);
    assert_eq!(again.layout_bindings(RIGHT_HANDED), b);
    assert_eq!(again.keymap, b.keymap(def.repeat).with_default_pad(&def));
    let found = again.take_warnings();
    assert!(found.is_empty(), "{found:?}");
    // A failed save still switches them.
    let mut failing = launch(Failing).with_layout(LEFT_HANDED);
    assert!(
        failing
            .set_layout_bindings(LEFT_HANDED, left.clone())
            .is_err()
    );
    assert_eq!(
        failing.keymap,
        left.keymap(def.repeat).with_default_pad(&def)
    );
}

#[test]
fn set_pad_bindings_applies_in_every_layout_and_before_one_is_chosen() {
    let mut c = ctx();
    let mut b = c.pad_bindings();
    assert!(b.bind(Action::Info, 1, Button::RightStickPress).is_ok());
    assert_eq!(c.set_pad_bindings(b.clone()), Ok(()));
    assert_eq!(c.pad_bindings(), b);
    assert_eq!(
        c.keymap.pad_action(Button::RightStickPress),
        Some(Action::Info)
    );
    let saved = c.storage.read(KEYBINDINGS_KEY).unwrap().unwrap();
    assert_eq!(saved, c.player_keys().to_ron());
    c.use_layout(LEFT_HANDED);
    assert_eq!(
        c.keymap.pad_action(Button::RightStickPress),
        Some(Action::Info)
    );
    // Before a layout is chosen: the layout picker's keys, these buttons.
    let mut none = first_launch();
    assert_eq!(none.set_pad_bindings(b.clone()), Ok(()));
    assert_eq!(none.layout(), None);
    let def = none.game.keymap_def().clone();
    assert_eq!(none.keymap, Keymap::layout_picker(&def).with_pad(b.pairs()));
    // They load with the storage.
    let storage = core::mem::take(&mut none.storage).into_computer();
    let again = first_launch().with_storage(storage);
    assert_eq!(again.pad_bindings(), b);
    // A failed save still applies them.
    let mut failing = launch(Failing);
    assert!(failing.set_pad_bindings(b.clone()).is_err());
    assert_eq!(failing.pad_bindings(), b);
}

#[test]
fn repaired_keys_leave_a_warning() {
    let mut storage = MemoryStorage::new();
    storage
        .write(&ProfileId::FIRST.key(KEYBINDINGS_KEY), "nonsense")
        .unwrap();
    let mut c = launch(storage);
    let warnings = c.take_warnings();
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].starts_with("keybindings: unreadable"),
        "{warnings:?}"
    );
}

/// A first launch makes `Player 1`, named by the game's text, and moves
/// the game's keys from before profiles into it.
#[test]
fn a_first_launch_makes_the_first_profile_and_moves_the_old_keys_into_it() {
    let mut storage = MemoryStorage::new();
    storage.write(SETTINGS_KEY, "Toy(speed: 6)").unwrap();
    storage.write("seen", "a").unwrap();
    let mut toy = Toy::new();
    toy.old_keys = vec![SETTINGS_KEY.to_owned(), "seen".to_owned()];
    let c = Ctx::new(toy).unwrap().with_storage(Box::new(storage));
    assert_eq!(c.profiles().all()[0].name, "Player 1");
    assert_eq!(c.settings().game.speed, 6);
    assert_eq!(c.storage.read("seen"), Ok(Some("a".to_owned())));
    assert_eq!(c.storage.computer().read("seen"), Ok(None));
}

/// Each profile has its own settings, keys and layout; a switch loads
/// them, and is what the next launch opens on.
#[test]
fn switching_profiles_switches_their_settings_keys_and_layout() {
    let mut c = launch(MemoryStorage::new());
    c.choose_layout(LEFT_HANDED).unwrap();
    c.change_settings(|s| s.game.speed = 3).unwrap();
    let b = info_on_g(&c, LEFT_HANDED);
    c.set_layout_bindings(LEFT_HANDED, b.clone()).unwrap();
    let mei = c.add_profile("Mei").unwrap();
    assert_eq!(
        c.profiles().current(),
        ProfileId::FIRST,
        "adding doesn't switch"
    );
    c.switch_profile(mei).unwrap();
    assert_eq!(c.storage.profile(), mei);
    assert_eq!(*c.settings(), Settings::default());
    assert_eq!(c.layout(), None);
    let def = c.game.keymap_def().clone();
    assert_eq!(c.keymap, Keymap::layout_picker(&def));
    assert_eq!(c.game.applied.last(), Some(&(0, None)));
    c.change_settings(|s| s.game.speed = 8).unwrap();
    c.switch_profile(ProfileId::FIRST).unwrap();
    assert_eq!(c.settings().game.speed, 3);
    assert_eq!(c.layout(), Some(LEFT_HANDED));
    assert_eq!(c.layout_bindings(LEFT_HANDED), b);
    assert_eq!(c.keymap, c.keymap_for(LEFT_HANDED));
    c.switch_profile(mei).unwrap();
    // The next launch opens on the profile used last.
    let storage = core::mem::take(&mut c.storage).into_computer();
    let again = first_launch().with_storage(storage);
    assert_eq!(again.profiles().current(), mei);
    assert_eq!(again.settings().game.speed, 8);
}

#[test]
fn profiles_are_renamed_and_deleted_with_their_keys() {
    let mut c = launch(MemoryStorage::new());
    let mei = c.add_profile("Mei").unwrap();
    assert_eq!(c.add_profile("mei"), Err(ProfileError::NameTaken));
    c.rename_profile(mei, "Odo").unwrap();
    assert_eq!(c.profiles().get(mei).unwrap().name, "Odo");
    assert_eq!(c.rename_profile(mei, " "), Err(ProfileError::NameEmpty));
    c.switch_profile(mei).unwrap();
    c.change_settings(|s| s.game.speed = 4).unwrap();
    assert!(
        c.storage
            .computer()
            .read(&mei.key(SETTINGS_KEY))
            .unwrap()
            .is_some()
    );
    // Deleting the one in use uses the first one left.
    c.delete_profile(mei).unwrap();
    assert_eq!(c.profiles().all().len(), 1);
    assert_eq!(c.profiles().current(), ProfileId::FIRST);
    assert_eq!(c.storage.profile(), ProfileId::FIRST);
    assert_eq!(c.settings().game.speed, 0);
    assert_eq!(c.storage.computer().read(&mei.key(SETTINGS_KEY)), Ok(None));
    assert_eq!(
        c.delete_profile(ProfileId::FIRST),
        Err(ProfileError::LastOne)
    );
    // Deleting another keeps the one in use.
    let kim = c.add_profile("Kim").unwrap();
    c.change_settings(|s| s.game.speed = 2).unwrap();
    c.delete_profile(kim).unwrap();
    assert_eq!(c.settings().game.speed, 2);
}

#[test]
fn a_list_that_cant_be_saved_changes_nothing() {
    #[derive(Debug)]
    struct NoWrites(MemoryStorage);
    impl Storage for NoWrites {
        fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
            self.0.read(key)
        }
        fn write(&mut self, _: &str, _: &str) -> Result<(), StorageError> {
            Err(StorageError::Backend("full".into()))
        }
        fn delete(&mut self, key: &str) -> Result<(), StorageError> {
            self.0.delete(key)
        }
        fn list(&self) -> Result<Vec<String>, StorageError> {
            self.0.list()
        }
    }
    let mut storage = MemoryStorage::new();
    let two = "Profiles(version: 1, profiles: [(id: 1, name: \"A\"), (id: 2, name: \"B\")], \
               current: 1, next_id: 3)";
    storage.write(profiles::PROFILES_KEY, two).unwrap();
    let mut c = launch(NoWrites(storage));
    let full = ProfileError::Storage(StorageError::Backend("full".into()));
    assert_eq!(c.add_profile("C"), Err(full.clone()));
    assert_eq!(c.profiles().all().len(), 2);
    let b = c.profiles().all()[1].id;
    assert_eq!(c.rename_profile(b, "D"), Err(full.clone()));
    assert_eq!(c.profiles().get(b).unwrap().name, "B");
    assert_eq!(c.delete_profile(b), Err(full.clone()));
    assert_eq!(c.profiles().all().len(), 2);
    // A switch happens anyway.
    assert_eq!(c.switch_profile(b), Err(full));
    assert_eq!(c.profiles().current(), b);
    assert_eq!(c.storage.profile(), b);
}

/// A game that leaves every default alone.
#[derive(Debug)]
struct Bare(Toy);

impl GameData for Bare {
    type Action = Action;
    type Colors = EngineColor;
    type Voice = Voice;
    type Settings = Prefs;
    fn lang(&self) -> &Lang {
        self.0.lang()
    }
    fn keymap_def(&self) -> &KeymapDef<Action> {
        self.0.keymap_def()
    }
    fn audio_manifest(&self) -> &AudioManifest {
        self.0.audio_manifest()
    }
    fn palette_def(&self) -> &PaletteDef {
        self.0.palette_def()
    }
    fn console(&self) -> ConsoleSpec {
        self.0.console()
    }
    fn theme_def(&self, theme: Theme) -> Option<&PaletteDef> {
        self.0.theme_def(theme)
    }
    fn voice_variant(&self) -> Voice {
        Voice::Only
    }
}

#[test]
fn a_game_that_leaves_the_defaults_has_no_debug_language_voices_or_old_keys() {
    let bare = Bare(Toy::new());
    assert_eq!(bare.debug_lang(), None);
    assert!(bare.voices("(clips: [])").is_err());
    let found = bare.keys_before_profiles();
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(bare.old_layout_key(), None);
    let mut c = Ctx::new(bare).unwrap();
    c.lang = code(DEBUG_LANG);
    c.debug_tools = false;
    assert_eq!(
        c.text("home.greeting"),
        "HELLO, {name}",
        "no debug language"
    );
    c.set_voice_manifest("(clips: [])");
    assert_eq!(c.take_warnings(), ["no voices: this game has no voices"]);
    c.storage.write("layout", "LeftHanded").unwrap();
    assert_eq!(c.saved_layout(), None, "no old layout key");
    // Settings change with no game to tell.
    c.change_settings(|s| s.game.speed = 1).unwrap();
    assert_eq!(c.settings().game.speed, 1);
}

#[test]
fn a_line_logged_waits_with_its_level_and_category_until_taken() {
    let mut c = ctx();
    c.log(LogLevel::Info, LogCategory::Game, "hello");
    let (lines, dropped) = c.take_log();
    assert_eq!(
        lines,
        [LogLine {
            frame: 0,
            level: LogLevel::Info,
            category: LogCategory::Game,
            text: "hello".to_owned(),
        }]
    );
    assert_eq!(dropped, 0);
    assert_eq!(c.take_log(), (Vec::new(), 0));
}

#[test]
fn saved_data_that_cant_be_read_is_logged_by_what_it_is_about() {
    let mut c = launch(Failing);
    c.set_voice_manifest("not a manifest");
    let (lines, _) = c.take_log();
    let said: Vec<_> = lines.iter().map(|l| (l.level, l.category)).collect();
    assert_eq!(
        said,
        [
            (LogLevel::Warn, LogCategory::Storage),
            (LogLevel::Warn, LogCategory::Input),
            (LogLevel::Warn, LogCategory::Storage),
            (LogLevel::Warn, LogCategory::Audio),
        ]
    );
}

#[test]
fn too_many_lines_waiting_drop_the_oldest_and_count_them() {
    let mut c = ctx();
    for n in 0..=LOG_LINES {
        c.log(LogLevel::Debug, LogCategory::Game, format!("{n}"));
    }
    let (lines, dropped) = c.take_log();
    assert_eq!(lines.len(), LOG_LINES);
    assert_eq!(lines[0].text, "1");
    assert_eq!(dropped, 1);
}

#[test]
fn take_warnings_takes_every_line_and_gives_the_warnings_and_errors() {
    let mut c = ctx();
    c.log(LogLevel::Info, LogCategory::Game, "info");
    c.log(LogLevel::Warn, LogCategory::Game, "warn");
    c.log(LogLevel::Debug, LogCategory::Game, "debug");
    c.log(LogLevel::Error, LogCategory::Game, "error");
    assert_eq!(c.take_warnings(), ["warn", "error"]);
    assert_eq!(c.take_log(), (Vec::new(), 0));
}

#[test]
fn a_modules_state_is_found_by_its_type_and_none_is_there_otherwise() {
    let mut c = ctx();
    assert_eq!(c.module_state::<u32>(), None);
    let mut install = module::Install::new(&mut c, "counter");
    install.state(3_u32);
    install.state("label");
    assert!(install.finish().is_ok());
    assert_eq!(c.module_state::<u32>(), Some(&3));
    assert_eq!(c.module_state::<&str>(), Some(&"label"));
    assert_eq!(c.module_state::<u64>(), None);
    if let Some(n) = c.module_state_mut::<u32>() {
        *n += 1;
    }
    assert_eq!(c.module_state::<u32>(), Some(&4));
    assert_eq!(c.module_state_mut::<u64>(), None);
}
