//! The engine's profiles, the fit and text maths, and the game's own
//! profiles beside the engine's.

use shuyi_assets::keymap::Button;
use shuyi_assets::target::{Color, Insets, Status};

use super::*;

fn setting(frame: (u16, u16), scale: Scale, text_px: u16) -> TargetSetting {
    TargetSetting {
        frame,
        scale,
        text_px,
    }
}

fn engine(id: &str) -> TargetProfile {
    engine_profiles().unwrap().remove(id).unwrap().profile
}

/// The messages of `problems`, each after its level.
fn shown(problems: &[TargetProblem]) -> Vec<String> {
    problems
        .iter()
        .map(|p| format!("{:?} {}: {}", p.level, p.setting, p.message))
        .collect()
}

#[test]
fn every_engine_profile_decodes_under_its_files_name() {
    let profiles = engine_profiles().unwrap();
    let ids: Vec<&str> = profiles.keys().map(String::as_str).collect();
    assert_eq!(
        ids,
        [
            "android",
            "desktop",
            "ios",
            "playdate",
            "steam-deck",
            "switch",
            "web"
        ]
    );
    for (id, profile) in &profiles {
        assert_eq!(profile.file, format!("targets/{id}.ron"));
    }
    let planned: Vec<&str> = profiles
        .iter()
        .filter(|(_, p)| p.profile.status == Status::Planned)
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(planned, ["android", "ios", "switch"]);
}

#[test]
fn the_playdate_profile_is_adr_0008s() {
    use Button::*;
    let p = engine("playdate");
    assert_eq!(p.display.sizes, [Size::new(400, 240)]);
    assert_eq!(p.display.color, Color::OneBit);
    assert_eq!((p.display.min_text_px, p.display.good_text_px), (8, 10));
    assert_eq!((p.frame_rate.target, p.frame_rate.floor), (30, 30));
    assert_eq!(
        p.input.buttons,
        [
            DpadUp,
            DpadDown,
            DpadLeft,
            DpadRight,
            East,
            West,
            CrankForward,
            CrankBack
        ]
    );
    assert_eq!(p.input.chords, [[East, West]]);
    assert_eq!(p.input.axes, ["Crank"]);
    assert_eq!(p.input.system_menu_slots, 3);
    assert!(!p.input.touch && !p.input.pointer && !p.input.keyboard);
    assert_eq!(p.budgets.frame_instructions, Some(2_000_000));
    assert_eq!(p.budgets.peak_heap_bytes, Some(10 << 20));
    assert_eq!(p.budgets.package_bytes, Some(40 << 20));
    assert_eq!(p.budgets.startup_ms, Some(5_000));
    assert_eq!(p.backend, "shuyi-playdate");
}

#[test]
fn the_steam_deck_profile_is_adr_0008s() {
    use Button::*;
    let p = engine("steam-deck");
    assert_eq!(p.display.sizes, [Size::new(1280, 800)]);
    assert_eq!(p.display.color, Color::Rgba8);
    assert_eq!((p.display.min_text_px, p.display.good_text_px), (9, 12));
    assert_eq!((p.frame_rate.target, p.frame_rate.floor), (60, 60));
    for button in [
        South,
        East,
        West,
        North,
        DpadUp,
        DpadRight,
        LeftShoulder,
        RightShoulder,
        LeftTrigger,
        RightTrigger,
        Start,
        Select,
    ] {
        assert!(p.input.buttons.contains(&button), "{button}");
    }
    assert!(!p.input.buttons.contains(&CrankForward));
    assert_eq!(p.input.axes, ["LeftStick", "RightStick"]);
    assert!(p.input.touch && !p.input.pointer && !p.input.keyboard);
    assert_eq!(p.budgets.frame_instructions, Some(20_000_000));
    assert_eq!(p.budgets.peak_heap_bytes, Some(1 << 30));
    assert_eq!(p.budgets.package_bytes, None);
    assert_eq!(p.budgets.startup_ms, Some(3_000));
    assert_eq!(p.backend, "shuyi-app");
}

#[test]
fn an_integer_scale_is_the_largest_whole_one_that_fits() {
    let deck = Size::new(1280, 800);
    assert_eq!(
        scale_on(Scale::Integer, (640, 400), &deck),
        Factor::whole(2)
    );
    // 800 × 512 fits once: 1280 / 800 is 1.6.
    assert_eq!(
        scale_on(Scale::Integer, (800, 512), &deck),
        Factor::whole(1)
    );
    // 1280 / 400 and 800 / 240 both hold 3.
    assert_eq!(
        scale_on(Scale::Integer, (400, 240), &deck),
        Factor::whole(3)
    );
    assert_eq!(
        scale_on(Scale::Integer, (800, 512), &Size::new(400, 240)),
        Factor::whole(0)
    );
}

#[test]
fn a_fit_scale_is_the_smaller_of_across_and_down() {
    let deck = Size::new(1280, 800);
    let fit = scale_on(Scale::Fit, (800, 512), &deck);
    // 1280 / 800 = 1.6 across, 800 / 512 = 1.5625 down.
    assert_eq!(fit, Factor { num: 800, den: 512 });
    assert_eq!(fit.shown(), "1.56");
    assert_eq!(fit.of(16), 25);
    let small = scale_on(Scale::Fit, (800, 512), &Size::new(400, 240));
    // 400 / 800 = 0.5 across, 240 / 512 = 0.46875 down.
    assert_eq!(small, Factor { num: 240, den: 512 });
    assert_eq!(small.shown(), "0.46");
    assert_eq!(Factor::whole(3).shown(), "3");
    assert_eq!(Factor { num: 4, den: 2 }.shown(), "2");
    // A frame of nothing doesn't divide by zero.
    assert_eq!(scale_on(Scale::Fit, (0, 0), &deck), Factor::whole(800));
}

#[test]
fn a_frame_too_big_for_the_display_is_an_error_that_names_both() {
    let problems = check_target(
        "playdate",
        &setting((800, 512), Scale::Integer, 16),
        &engine("playdate"),
    );
    assert_eq!(
        shown(&problems),
        [
            "Error targets.\"playdate\".frame: frame 800 × 512 at scale 1 is larger than the \
             display 400 × 240"
        ]
    );
    // At fit it shrinks to fit, and its text with it.
    let problems = check_target(
        "playdate",
        &setting((800, 512), Scale::Fit, 16),
        &engine("playdate"),
    );
    assert_eq!(
        shown(&problems),
        [
            "Error targets.\"playdate\".text_px: text 16 px at scale 0.46 is 7 px on 400 × 240, \
             below the target's minimum of 8 px"
        ]
    );
    let problems = check_target(
        "playdate",
        &setting((800, 512), Scale::Fit, 20),
        &engine("playdate"),
    );
    assert_eq!(
        shown(&problems),
        [
            "Warning targets.\"playdate\".text_px: text 20 px at scale 0.46 is 9 px on 400 × 240, \
             below the 10 px the target recommends"
        ]
    );
}

#[test]
fn the_safe_area_is_what_the_frame_must_fit() {
    let mut profile = engine("steam-deck");
    profile.display.safe_area = Insets {
        left: 300,
        right: 300,
        ..Insets::default()
    };
    let problems = check_target("deck", &setting((800, 512), Scale::Integer, 16), &profile);
    assert_eq!(
        shown(&problems),
        [
            "Error targets.\"deck\".frame: frame 800 × 512 at scale 1 is larger than the safe \
             area 680 × 800 of the display 1280 × 800"
        ]
    );
    // At fit, 680 / 800 across decides: 0.85, and 16 px of text is 13.
    let fit = setting((800, 512), Scale::Fit, 16);
    assert!(check_target("deck", &fit, &profile).is_empty());
    let small = check_target("deck", &setting((800, 512), Scale::Fit, 12), &profile);
    assert_eq!(
        shown(&small),
        [
            "Warning targets.\"deck\".text_px: text 12 px at scale 0.85 is 10 px on 1280 × 800, \
             below the 12 px the target recommends"
        ]
    );
}

#[test]
fn text_on_the_deck_warns_from_9_to_11_px_and_fails_below_9() {
    let deck = engine("steam-deck");
    // 800 × 512 is at scale 1 on the Deck: the text is its own height.
    let at = |px| {
        check_target(
            "steam-deck",
            &setting((800, 512), Scale::Integer, px),
            &deck,
        )
    };
    for px in [12, 16] {
        assert!(at(px).is_empty(), "{px}");
    }
    for px in 9..=11 {
        let problems = at(px);
        assert_eq!(problems.len(), 1, "{px}");
        assert_eq!(problems[0].level, Level::Warning);
        assert_eq!(problems[0].check, TargetCheck::Text);
    }
    assert_eq!(
        shown(&at(8)),
        [
            "Error targets.\"steam-deck\".text_px: text 8 px at scale 1 is 8 px on 1280 × 800, \
             below the target's minimum of 9 px"
        ]
    );
    // A frame half the size is shown at scale 2, and so is its text.
    let half = check_target("steam-deck", &setting((640, 400), Scale::Integer, 6), &deck);
    assert_eq!(half.len(), 0);
}

#[test]
fn a_display_of_any_size_shows_the_frame_at_scale_1() {
    let desktop = engine("desktop");
    assert!(
        check_target(
            "desktop",
            &setting((800, 512), Scale::Integer, 16),
            &desktop
        )
        .is_empty()
    );
    assert_eq!(
        shown(&check_target(
            "desktop",
            &setting((9000, 9000), Scale::Fit, 11),
            &desktop
        )),
        [
            "Warning targets.\"desktop\".text_px: text 11 px at scale 1 is 11 px, below the 12 \
             px the target recommends"
        ]
    );
}

#[test]
fn each_mode_of_a_display_is_checked() {
    let switch = engine("switch");
    let problems = check_target("switch", &setting((1600, 900), Scale::Integer, 16), &switch);
    assert_eq!(
        shown(&problems),
        [
            "Error targets.\"switch\".frame: frame 1600 × 900 at scale 1 is larger than the \
             display 1280 × 720 handheld"
        ]
    );
}

#[test]
fn the_targets_asked_for_are_those_shuyi_ron_lists() {
    let mut config = Config::default();
    assert_eq!(
        asked(&config, ALL).unwrap_err(),
        "shuyi.ron lists no targets (`targets`)"
    );
    assert_eq!(
        asked(&config, "web").unwrap_err(),
        "shuyi.ron's `targets` has no \"web\" (it lists none): a game lists the targets it ships on"
    );
    let one = setting((8, 8), Scale::Integer, 8);
    config.targets.insert("desktop".to_owned(), one);
    config.targets.insert("web".to_owned(), one);
    let ids = |r: Vec<(&str, &TargetSetting)>| {
        r.into_iter()
            .map(|(id, _)| id.to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(asked(&config, ALL).unwrap()), ["desktop", "web"]);
    assert_eq!(ids(asked(&config, "web").unwrap()), ["web"]);
    assert!(
        asked(&config, "playdate")
            .unwrap_err()
            .contains("(it lists desktop, web)")
    );
}

#[test]
fn a_games_profiles_add_to_the_engines_or_replace_one() {
    let dir = std::env::temp_dir().join(format!("shuyi-target-profiles-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("t")).unwrap();
    let playdate = String::from_utf8(
        TARGETS
            .get_file("playdate.ron")
            .unwrap()
            .contents()
            .to_vec(),
    )
    .unwrap()
    .replace("min_text_px: 8", "min_text_px: 6");
    fs::write(dir.join("t/playdate.ron"), &playdate).unwrap();
    let kiosk = playdate
        .replace("\"playdate\"", "\"kiosk\"")
        .replace("400, height: 240", "1024, height: 768");
    fs::write(dir.join("t/kiosk.ron"), kiosk).unwrap();
    fs::write(dir.join("t/README.md"), "not a profile").unwrap();
    let config = Config {
        target_profiles: Some("t".to_owned()),
        ..Config::default()
    };
    let all = profiles(&config, &dir).unwrap();
    assert_eq!(all.len(), 8);
    assert_eq!(all["playdate"].file, "t/playdate.ron");
    assert_eq!(all["playdate"].profile.display.min_text_px, 6);
    assert_eq!(all["kiosk"].profile.display.sizes, [Size::new(1024, 768)]);
    assert_eq!(all["web"].file, "targets/web.ron");
    // A broken one names its file; a missing folder names the folder.
    fs::write(dir.join("t/bad.ron"), "(id: \"bad\"").unwrap();
    let errors = profiles(&config, &dir).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].file, "t/bad.ron");
    let missing = Config {
        target_profiles: Some("nope".to_owned()),
        ..Config::default()
    };
    assert_eq!(profiles(&missing, &dir).unwrap_err()[0].file, "nope");
    // A profile that can't be read is an error, even when the rest decode.
    fs::remove_file(dir.join("t/bad.ron")).unwrap();
    fs::create_dir_all(dir.join("t/folder.ron")).unwrap();
    let errors = profiles(&config, &dir).unwrap_err();
    let files: Vec<&str> = errors.iter().map(|e| e.file.as_str()).collect();
    assert_eq!(files, ["t/folder.ron"]);
    assert_eq!(profiles(&Config::default(), &dir).unwrap().len(), 7);
    fs::remove_dir_all(&dir).unwrap();
}
