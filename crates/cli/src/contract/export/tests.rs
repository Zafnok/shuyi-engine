use std::path::PathBuf;

use super::*;
use crate::contract::target::TargetCheck;
use crate::export::Host;
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
fn with_takes_a_feature_each_time() {
    let input = parse_args(&args(&["playdate", "--with", "bench", "--with", "demo"])).unwrap();
    assert_eq!(input.with, ["bench", "demo"]);
    assert_eq!(input.without, Vec::<String>::new());
    assert_eq!(
        parse_args(&args(&["playdate", "--with"])).unwrap_err(),
        "--with needs a feature"
    );
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
        edition: Some("public-demo".to_owned()),
        pack: Some(FileSize {
            file: "g-0.1.0-windows/g.pack".to_owned(),
            bytes: 7,
        }),
    };
    let said = text(&output).replace('\\', "/");
    for line in [
        "export windows: dist/export/windows/g-0.1.0-windows.zip (0.1.0, check --target desktop)\n",
        "  g-0.1.0-windows.zip: 12 bytes\n",
        "  pack: g-0.1.0-windows/g.pack: 7 bytes\n",
        "  debug file: dist/export/windows/g-0.1.0-windows.debug\n",
        "  signing: none\n",
        "  edition: public-demo\n",
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

#[test]
fn plan_and_tag_take_no_target_and_nothing_else() {
    let input = parse_args(&args(&["--plan"])).unwrap();
    assert!(input.plan && input.tag.is_none() && input.target.is_empty());
    let input = parse_args(&args(&["--tag", "v0.1.0", "--plan"])).unwrap();
    assert_eq!(input.tag.as_deref(), Some("v0.1.0"));
    for (wrong, why) in [
        (&["--plan", "web"][..], "--plan takes no target"),
        (&["--plan", "--release"], "--plan builds nothing"),
        (&["--plan", "--edition", "demo"], "--plan builds nothing"),
        (&["web", "--tag", "v1.0.0"], "--tag goes with --plan"),
        (&["--plan", "--tag"], "--tag needs a tag"),
        (&["--plan", "--plan"], "--plan twice"),
    ] {
        let error = parse_args(&args(wrong)).unwrap_err();
        assert!(error.starts_with(why), "{wrong:?}: {error}");
    }
    // `export` itself never answers a plan.
    let p = project(r#"(targets: {"web": (frame: (384, 384), scale: integer, text_px: 16)})"#);
    let input = ExportInput {
        plan: true,
        ..ExportInput::default()
    };
    assert_eq!(export(&p, &input, None).code.as_deref(), Some("SH0001"));
}

/// An exporter that only says what it is, for the plan.
struct Fake {
    id: &'static str,
    profile: &'static str,
    host: Host,
    container: Option<&'static str>,
    ci: InCi,
}

impl Exporter for Fake {
    fn id(&self) -> &'static str {
        self.id
    }

    fn profile(&self) -> &'static str {
        self.profile
    }

    fn host(&self) -> Host {
        self.host
    }

    fn container(&self) -> Option<&'static str> {
        self.container
    }

    fn ci(&self) -> InCi {
        self.ci
    }

    fn triples(&self) -> &'static [&'static str] {
        &[]
    }

    fn cargo_args(&self, _cx: &ExportCx) -> Vec<Vec<String>> {
        unreachable!("the plan builds nothing")
    }

    fn export(&self, _cx: &ExportCx) -> Result<ExportOutput, ExportError> {
        unreachable!("the plan builds nothing")
    }
}

const fn fake(id: &'static str, profile: &'static str, host: Host, ci: InCi) -> Fake {
    Fake {
        id,
        profile,
        host,
        container: None,
        ci,
    }
}

static FAKES: [Fake; 4] = [
    Fake {
        container: Some("image@sha256:0"),
        ..fake("linux", "desktop", Host::Linux, InCi::Build)
    },
    fake("windows", "desktop", Host::Windows, InCi::Build),
    fake("web", "web", Host::Linux, InCi::Build),
    fake(
        "pd",
        "playdate",
        Host::Linux,
        InCi::CheckOnly("no SDK in CI"),
    ),
];

fn fakes() -> Vec<&'static dyn Exporter> {
    FAKES.iter().map(|fake| fake as &dyn Exporter).collect()
}

/// The `targets` table of `ids`, each with the same frame.
fn targets(ids: &[&str]) -> Config {
    let rows: Vec<String> = ids
        .iter()
        .map(|id| format!("\"{id}\": (frame: (384, 384), scale: integer, text_px: 16)"))
        .collect();
    Config::parse(&format!("(targets: {{{}}})", rows.join(", "))).unwrap()
}

#[test]
fn the_plan_lists_each_targets_exporters_in_the_order_they_are_registered() {
    // The table's order is by id, whatever `shuyi.ron`'s.
    let config = targets(&["web", "desktop", "playdate"]);
    let answer = plan_of(&config, "0.1.0", &fakes(), None);
    assert!(answer.ok && answer.limits.is_empty(), "{answer:?}");
    let plan = answer.result.unwrap();
    assert_eq!((plan.version.as_str(), plan.prerelease), ("0.1.0", false));
    let listed: Vec<(&str, &str)> = plan
        .exports
        .iter()
        .map(|e| (e.exporter.as_str(), e.profile.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            ("linux", "desktop"),
            ("windows", "desktop"),
            ("pd", "playdate"),
            ("web", "web")
        ]
    );
    let json = serde_json::to_value(&plan.exports).unwrap();
    assert_eq!(
        json[0],
        serde_json::json!({
            "exporter": "linux", "profile": "desktop", "host": "linux",
            "container": "image@sha256:0", "ci": "build",
        })
    );
    assert_eq!(json[1]["host"], "windows");
    assert_eq!(json[1]["container"], serde_json::Value::Null);
    assert_eq!(json[2]["ci"], "check-only");
    assert_eq!(json[2]["why"], "no SDK in CI");
    let said = plan_text(&plan);
    assert!(said.starts_with("export --plan: version 0.1.0\n"), "{said}");
    assert!(said.contains("  linux (desktop): built on linux in image@sha256:0\n"));
    assert!(said.contains("  pd (playdate): checked only: no SDK in CI\n"));
}

#[test]
fn a_target_no_exporter_builds_is_a_limit_and_the_plan_passes() {
    for id in ["switch", "steam-deck"] {
        let answer = plan_of(&targets(&["desktop", id]), "0.1.0", &fakes(), None);
        assert!(answer.ok, "{answer:?}");
        assert_eq!(answer.result.unwrap().exports.len(), 2);
        assert_eq!(
            answer.limits,
            [format!(
                "`{id}` is checked (`check --target {id}`) but not packaged: no exporter builds \
                 it yet"
            )]
        );
    }
}

#[test]
fn the_tag_must_be_v_and_the_version() {
    let config = targets(&["web"]);
    assert!(plan_of(&config, "0.1.0", &fakes(), Some("v0.1.0")).ok);
    for (version, tag) in [
        ("0.1.0", "v0.1.1"),
        ("0.1.0", "0.1.0"),
        ("1.2.0-beta.1", "v1.2.0"),
    ] {
        let answer = plan_of(&config, version, &fakes(), Some(tag));
        assert_eq!(answer.code.as_deref(), Some("SH0001"), "{tag}");
        let error = answer.error.unwrap();
        assert!(
            error.contains(&format!("`{tag}`")) && error.contains(&format!("`{version}`")),
            "{error}"
        );
        assert!(
            answer
                .help
                .unwrap()
                .contains(&format!("git tag -a v{version}"))
        );
    }
    let answer = plan_of(&config, "1.2.0-beta.1", &fakes(), Some("v1.2.0-beta.1"));
    assert!(answer.result.unwrap().prerelease);
    let answer = plan_of(&config, "one", &fakes(), None);
    assert_eq!(answer.code.as_deref(), Some("SH1002"));
}

#[test]
fn the_plans_version_is_app_version_when_the_game_sets_one() {
    // No package to read: `app.version` is the version, as the reports'.
    let p = project(
        r#"(app: (name: "A", id: "a.b", version: "2.0.0-rc.1"),
            targets: {"desktop": (frame: (384, 384), scale: integer, text_px: 16)})"#,
    );
    let input = parse_args(&args(&["--plan", "--tag", "v2.0.0-rc.1"])).unwrap();
    let answer = plan(&p, &input);
    assert!(answer.ok, "{answer:?}");
    let planned = answer.result.unwrap();
    assert_eq!(
        (planned.version.as_str(), planned.prerelease),
        ("2.0.0-rc.1", true)
    );
    let ids: Vec<&str> = planned
        .exports
        .iter()
        .map(|e| e.exporter.as_str())
        .collect();
    assert_eq!(ids, ["windows", "linux", "macos"]);
    // Without one, and no package named, it can't say.
    let p = project(r#"(targets: {"desktop": (frame: (384, 384), scale: integer, text_px: 16)})"#);
    assert_eq!(plan(&p, &input).code.as_deref(), Some("SH1002"));
}

#[test]
fn every_registered_exporter_says_where_ci_builds_it() {
    let planned: Vec<PlannedExport> = crate::export::exporters()
        .iter()
        .map(|exporter| PlannedExport::of(*exporter))
        .collect();
    let json = serde_json::to_value(&planned).unwrap();
    let row = |i: usize| {
        (
            json[i]["exporter"].as_str().unwrap(),
            json[i]["host"].as_str().unwrap(),
            json[i]["ci"].as_str().unwrap(),
        )
    };
    let rows: Vec<_> = (0..planned.len()).map(row).collect();
    assert_eq!(
        rows,
        [
            ("windows", "windows", "build"),
            ("linux", "linux", "build"),
            ("macos", "macos", "build"),
            ("web", "linux", "build"),
            ("playdate", "linux", "check-only"),
        ]
    );
    let containers: Vec<Option<&str>> = planned.iter().map(|e| e.container.as_deref()).collect();
    assert_eq!(
        containers,
        [None, Some(crate::export::SNIPER), None, None, None]
    );
    assert!(planned[4].why.as_deref().unwrap().contains("license"));
}
