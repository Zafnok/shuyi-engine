use std::path::PathBuf;

use super::*;
use crate::contract::target::TargetCheck;
use crate::project::Config;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// A project at the working folder whose `shuyi.ron` is `text`.
fn project(text: &str) -> Project {
    Project {
        root: PathBuf::from("."),
        config: Config::parse(text).unwrap(),
    }
}

#[test]
fn the_arguments_are_a_target_then_out_release_and_run_once_each() {
    let input = parse_args(&args(&["windows"])).unwrap();
    assert_eq!(input.target, "windows");
    assert_eq!((input.out, input.release, input.run), (None, false, false));
    let input = parse_args(&args(&["--run", "linux", "--out", "pkg", "--release"])).unwrap();
    assert_eq!(input.target, "linux");
    assert_eq!(input.out.as_deref(), Some("pkg"));
    assert!(input.release && input.run);
    for wrong in [
        &[][..],
        &["--release"],
        &["web", "linux"],
        &["web", "--out"],
        &["web", "--release", "--release"],
        &["web", "--out", "a", "--out", "b"],
        &["web", "--bogus"],
    ] {
        assert!(parse_args(&args(wrong)).is_err(), "{wrong:?}");
    }
    assert!(usage("cargo shuyi").starts_with("usage: cargo shuyi export <windows | linux"));
}

#[test]
fn without_takes_a_feature_each_time() {
    let input = parse_args(&args(&["web", "--without", "a", "--without", "b"])).unwrap();
    assert_eq!(input.without, ["a", "b"]);
    assert_eq!(
        parse_args(&args(&["web", "--without"])).unwrap_err(),
        "--without needs a feature"
    );
}

#[test]
fn a_game_that_fails_the_targets_check_is_refused_with_the_checks_errors() {
    // The Playdate's screen is 400 × 240: an 800 × 512 frame doesn't fit.
    let p = project(r#"(targets: {"playdate": (frame: (800, 512), scale: integer, text_px: 16)})"#);
    let input = ExportInput {
        target: "playdate".to_owned(),
        ..ExportInput::default()
    };
    let answer = export(&p, &input, None);
    assert!(!answer.ok);
    assert_eq!(answer.code.as_deref(), Some("SH3008"));
    assert_eq!(answer.exit_code(), 1);
    let error = answer.error.unwrap();
    assert!(
        error.starts_with("check --target playdate found errors; nothing was built:\n  "),
        "{error}"
    );
    assert!(error.contains("targets.\"playdate\".frame"), "{error}");
}

#[test]
fn a_target_that_passes_with_no_exporter_says_so_and_names_the_exporters() {
    let p =
        project(r#"(targets: {"steam-deck": (frame: (384, 384), scale: integer, text_px: 16)})"#);
    let input = ExportInput {
        target: "steam-deck".to_owned(),
        ..ExportInput::default()
    };
    let answer = export(&p, &input, None);
    assert_eq!(answer.code.as_deref(), Some("SH3010"));
    let error = answer.error.unwrap();
    assert!(error.contains("no exporter for it yet"), "{error}");
    assert!(
        error.ends_with("(the exporters: windows, linux, macos, web, playdate)"),
        "{error}"
    );
}

#[test]
fn a_target_the_game_doesnt_list_is_the_checks_error() {
    let p = project(r#"(targets: {"web": (frame: (384, 384), scale: integer, text_px: 16)})"#);
    let input = ExportInput {
        target: "windows".to_owned(),
        ..ExportInput::default()
    };
    // `windows` checks the desktop profile, which this game doesn't list.
    let answer = export(&p, &input, None);
    assert_eq!(answer.code.as_deref(), Some("SH3003"));
    assert!(answer.error.unwrap().contains("has no \"desktop\""));
}

#[test]
fn the_report_for_people_names_the_package_its_sizes_and_warnings() {
    let output = ExportOutput {
        target: "windows".to_owned(),
        profile: "desktop".to_owned(),
        version: "0.1.0".to_owned(),
        out: "dist/export/windows".to_owned(),
        package: "g-0.1.0-windows.zip".to_owned(),
        folder: "g-0.1.0-windows".to_owned(),
        beside: vec![FileSize {
            file: "g-0.1.0-windows/music".to_owned(),
            bytes: 9,
        }],
        sizes: vec![FileSize {
            file: "g-0.1.0-windows.zip".to_owned(),
            bytes: 12,
        }],
        warnings: vec![TargetProblem {
            level: Level::Warning,
            check: TargetCheck::Text,
            setting: "targets.\"desktop\".text_px".to_owned(),
            message: "small".to_owned(),
            screen: None,
            actions: Vec::new(),
            inputs: Vec::new(),
            roles: Vec::new(),
        }],
        signing: Signing::None,
        debug: Some("g-0.1.0-windows.debug".to_owned()),
    };
    let said = text(&output).replace('\\', "/");
    for line in [
        "export windows: dist/export/windows/g-0.1.0-windows.zip (0.1.0, check --target desktop)\n",
        "  g-0.1.0-windows.zip: 12 bytes\n",
        "  debug file: dist/export/windows/g-0.1.0-windows.debug\n",
        "  signing: none\n",
        "  report: dist/export/windows/export-report.json\n",
        "  warning: targets.\"desktop\".text_px: small\n",
    ] {
        assert!(said.contains(line), "{line} in {said}");
    }
    // The report's JSON names the signing as ADR-0008's words.
    let json = serde_json::to_value(&output).unwrap();
    assert_eq!(json["signing"], "none");
    let signed = serde_json::to_value(Signing::AdHoc).unwrap();
    assert_eq!(signed, "ad-hoc");
}
