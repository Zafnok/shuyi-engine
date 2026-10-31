//! ADR-0008 §3 on Hello, Grid: every screen declares the actions it
//! reacts to, and on each target its required ones bind to distinct
//! buttons of the keymap file's table for that target. All six of its
//! actions are required.

use std::path::Path;

use hello_grid::tools::{HelloTools, controls};
use shuyi_cli::contract::check::{CheckInput, targets};
use shuyi_cli::contract::target::{TargetCheck, engine_profiles};
use shuyi_cli::project::Project;

#[test]
fn every_screen_declares_its_actions_and_they_fit_every_target() {
    let game = controls().unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<&str> = game.screens().iter().map(|s| s.screen).collect();
    assert_eq!(
        names,
        [
            "title",
            "stage",
            "talk",
            "leave",
            "end",
            "options",
            "key_bindings",
            "profiles"
        ]
    );
    assert!(game.screens().iter().all(|s| s.actions.is_some()));
    let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    // The Playdate too, though its frame doesn't fit (the README says
    // why): its d-pad, A and B are enough.
    for id in ["playdate", "steam-deck", "desktop", "web"] {
        assert_eq!(game.fit(&profiles[id].profile), [], "{id}");
    }
}

#[test]
fn every_screen_a_tour_of_the_game_opens_is_one_the_check_knows() {
    let known: Vec<&str> = controls()
        .unwrap_or_else(|e| panic!("{e}"))
        .screens()
        .iter()
        .map(|s| s.screen)
        .collect();
    let mut h = crate::title(1);
    // Options, then its Key bindings, and back; the profiles (the menu
    // keeps its focus); the stage, its scene skipped, and leaving it.
    h.keys("Down Enter Down Down Down Enter Escape Escape");
    h.keys("Down Enter Escape");
    h.keys("Up Up Enter Escape Escape");
    let opened: Vec<&str> = h.screen_log().iter().flat_map(|(_, s)| s.clone()).collect();
    for screen in [
        "options",
        "key_bindings",
        "profiles",
        "stage",
        "talk",
        "leave",
    ] {
        assert!(opened.contains(&screen), "{screen}: {:?}", h.screen_log());
    }
    for screen in opened {
        assert!(known.contains(&screen), "{screen} isn't in screens::all");
    }
}

#[test]
fn check_target_checks_the_controls_with_the_games_tools() {
    let project =
        Project::load(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|e| panic!("{e}"));
    let mut input = CheckInput::default();
    input.target = Some("all".to_owned());
    let answer = targets(&project.config, &project.root, &input, Some(&HelloTools));
    assert!(answer.ok, "{answer:?}");
    assert!(
        !answer.limits.iter().any(|l| l.contains("controls aren't")),
        "{:?}",
        answer.limits
    );
    let reports = answer.result.unwrap_or_else(|| panic!("no result")).targets;
    let ids: Vec<&str> = reports.iter().map(|r| r.target.as_str()).collect();
    assert_eq!(ids, ["desktop", "playdate", "steam-deck", "web"]);
    for report in reports {
        assert!(
            !report
                .problems
                .iter()
                .any(|p| p.check == TargetCheck::Controls),
            "{report:?}"
        );
    }
}
