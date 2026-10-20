//! `shuyi check --target` on fixture games: the binary itself, for people
//! and through the JSON contract, with no game tool program to build.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::scratch;

/// A fixture game in a fresh folder whose `shuyi.ron` lists `targets`.
fn game(name: &str, targets: &str) -> std::path::PathBuf {
    let dir = scratch(name);
    let text = format!("(\n    targets: {{\n{targets}\n    }},\n)\n");
    fs::write(dir.join("shuyi.ron"), text).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

/// `shuyi <args>` run in `dir`.
fn shuyi(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_shuyi"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn shuyi: {e}"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn an_800_by_512_frame_doesnt_fit_the_playdate() {
    let dir = game(
        "target-playdate",
        r#"        "playdate": (frame: (800, 512), scale: integer, text_px: 16),"#,
    );
    let output = shuyi(&dir, &["check", "--target", "playdate"]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(
        text.contains(
            "  error: playdate: targets.\"playdate\".frame: frame 800 × 512 at scale 1 is larger \
             than the display 400 × 240\n"
        ),
        "{text}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn text_on_the_steam_deck_warns_from_9_to_11_px_and_fails_below_9() {
    for (px, code, words) in [
        (12, 0, "check --target steam-deck: OK"),
        (11, 0, "below the 12 px the target recommends"),
        (9, 0, "below the 12 px the target recommends"),
        (8, 1, "below the target's minimum of 9 px"),
    ] {
        let dir = game(
            &format!("target-deck-{px}"),
            &format!(
                r#"        "steam-deck": (frame: (800, 512), scale: integer, text_px: {px}),"#
            ),
        );
        let output = shuyi(&dir, &["check", "--target", "steam-deck"]);
        assert_eq!(output.status.code(), Some(code), "{px} px");
        assert!(
            stdout(&output).contains(words),
            "{px} px: {}",
            stdout(&output)
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn the_same_check_answers_through_the_json_contract() {
    let dir = game(
        "target-json",
        r#"        "desktop": (frame: (800, 512), scale: integer, text_px: 16),
        "playdate": (frame: (800, 512), scale: fit, text_px: 16),"#,
    );
    let output = shuyi(&dir, &["check", "--target", "all", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(json["contract"], shuyi_cli::contract::VERSION);
    assert_eq!(json["op"], "check");
    assert_eq!(json["ok"], false);
    let targets = &json["result"]["targets"];
    assert_eq!(targets[0]["target"], "desktop");
    assert_eq!(targets[0]["problems"], serde_json::json!([]));
    assert_eq!(targets[1]["target"], "playdate");
    assert_eq!(targets[1]["profile"], "targets/playdate.ron");
    assert_eq!(
        targets[1]["problems"],
        serde_json::json!([{
            "level": "error",
            "check": "text",
            "setting": "targets.\"playdate\".text_px",
            "message": "text 16 px at scale 0.46 is 7 px on 400 × 240, below the target's minimum of 8 px",
        }])
    );
    assert_eq!(json["result"]["errors"], serde_json::json!([]));
    // With no game tool program, the controls aren't checked, and it says so.
    assert!(
        json["limits"][0]
            .as_str()
            .unwrap()
            .starts_with("the controls aren't checked: they need the game's tools")
    );
    assert!(
        json["limits"][1]
            .as_str()
            .unwrap()
            .starts_with("desktop's display takes any size")
    );
    // The answer reads back as the contract's type.
    let back: shuyi_cli::contract::Answer<shuyi_cli::contract::check::CheckOutput> =
        serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(back.result.unwrap().targets.len(), 2);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_game_that_fits_passes_and_a_target_it_doesnt_list_is_refused() {
    let dir = game(
        "target-pass",
        r#"        "desktop": (frame: (400, 240), scale: integer, text_px: 16),
        "playdate": (frame: (400, 240), scale: integer, text_px: 16),
        "steam-deck": (frame: (400, 240), scale: integer, text_px: 16),"#,
    );
    let output = shuyi(&dir, &["check", "--target", "all"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(stdout(&output).contains("check --target steam-deck: OK"));
    let output = shuyi(&dir, &["check", "--target", "web"]);
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("has no \"web\""), "{error}");
    let output = shuyi(&dir, &["check", "--target"]);
    assert_eq!(output.status.code(), Some(2));
    fs::remove_dir_all(&dir).unwrap();
}
