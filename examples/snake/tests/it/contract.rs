//! The tool contract on Snake: `check` and `check --target all` on its
//! files, `scene` with the data an agent asks for, and `replay` on the
//! playthrough's record, as `cargo shuyi <op>` runs them in its tool
//! program.

use std::fs;

use shuyi_cli::contract::check::{CheckInput, check, targets};
use shuyi_cli::contract::replay::{ReplayInput, replay};
use shuyi_cli::contract::run::{RunInput, keys, scene};
use shuyi_cli::project::Project;
use shuyi_cli::serde_json::json;

use crate::{playthrough, root, scratch, tools};

#[test]
fn check_finds_no_error_and_checks_the_text_against_the_font() {
    let answer = check(&tools(), &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
}

#[test]
fn every_target_it_lists_fits_its_controls_and_its_one_bit_look() {
    let project = Project::load(root()).unwrap_or_else(|e| panic!("{e}"));
    let mut input = CheckInput::default();
    input.target = Some("all".to_owned());
    let answer = targets(&project.config, &project.root, &input, Some(&tools()));
    assert!(answer.ok, "{answer:?}");
    let limits = answer.limits.join("\n");
    assert!(!limits.contains("controls aren't"), "{limits}");
    assert!(!limits.contains("patterns weren't"), "{limits}");
    let ids = (project.config.targets.keys()).map(String::as_str);
    assert_eq!(
        ids.collect::<Vec<_>>(),
        ["desktop", "playdate", "steam-deck", "web"]
    );
}

#[test]
fn the_scene_has_the_length_the_score_and_whether_it_is_over() {
    let mut input = RunInput::default();
    input.script = keys("Enter");
    let answer = scene(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap_or_else(|| panic!("no answer"));
    let shown = out.scene.unwrap_or_else(|| panic!("no scene"));
    let data = &shown.data;
    assert_eq!(
        (&data["length"], &data["score"], &data["over"]),
        (&json!(3), &json!(0), &json!(false))
    );
    assert!(shown.text.contains("►► (6,7)"), "{}", shown.text);
    // The board's rows, each terrain by its name.
    assert!(
        shown.text.contains(
            "   1: wall floor*23 wall
"
        ),
        "{}",
        shown.text
    );
    // Left alone, it runs into the wall.
    input.script = keys("Enter");
    input.frames = 60 * 30;
    let answer = scene(&tools(), &input);
    let out = answer.result.unwrap_or_else(|| panic!("no answer"));
    let shown = out.scene.unwrap_or_else(|| panic!("no scene"));
    assert_eq!(shown.data["over"], json!(true));
}

#[test]
fn the_playthroughs_record_replays_a_step_per_tick_with_no_difference() {
    let (_, record) = playthrough();
    let dir = scratch("replay");
    let file = dir.join("seed-1.ron");
    let text = ron::to_string(&record).unwrap_or_else(|e| panic!("{e}"));
    fs::write(&file, text).unwrap_or_else(|e| panic!("{e}"));
    let answer = replay(&tools(), &dir, &ReplayInput::new("seed-1.ron"));
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap_or_else(|| panic!("no answer"));
    assert_eq!(out.first_difference, None);
    let run = &out.runs[0];
    // Step 0, then one per tick.
    assert_eq!(run.steps.len() as u64, record.len() + 1);
    assert_eq!(run.steps[0].command, None);
    assert!(
        run.steps
            .iter()
            .any(|s| s.command.as_deref() == Some("pressed Down")
                || s.command.as_deref() == Some("pressed Up")),
        "no turn in the replay"
    );
    assert!(
        run.outcome
            .as_deref()
            .is_some_and(|o| o.starts_with("Died"))
    );
    assert_eq!(run.made.as_ref(), Some(&snake::made()));
}
