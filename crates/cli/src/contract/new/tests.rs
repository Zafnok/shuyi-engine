//! What `new` makes of the engine's template, and its arguments.

use super::*;

/// A fresh empty folder under the system temp folder.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-new-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The engine's template.
fn built_in() -> Template {
    template::built_in().unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_crate_name_is_lowercase_words_joined_by_dashes() {
    assert_eq!(crate_name("Moon Garden").unwrap(), "moon-garden");
    assert_eq!(crate_name("  Moon -- Garden 2 ").unwrap(), "moon-garden-2");
    assert_eq!(crate_name("demo").unwrap(), "demo");
    assert!(crate_name("2048").is_err());
    assert!(crate_name("é").is_err());
    assert!(crate_name("").is_err());
}

#[test]
fn a_name_must_not_need_escaping() {
    assert!(check_name("Moon Garden").is_ok());
    for bad in ["", "  ", "a\"b", r"a\b", "a{b", "a}b", "a\nb"] {
        assert!(check_name(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn every_short_name_fills_every_placeholder_on_every_engine() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for short in ["grid", "blank"] {
        for engine in [
            Engine::Main,
            Engine::Rev {
                rev: "abc1234".to_owned(),
            },
            Engine::Path {
                path: ".".to_owned(),
            },
        ] {
            let mut input = NewInput::new("g").template(short);
            input.engine = engine.clone();
            let (plan, _) = plan(&cwd, &template, &input, "Moon Garden", "moon-garden")
                .unwrap_or_else(|(_, e)| panic!("{short}: {e}"));
            let paths = plan.paths();
            for file in [
                "Cargo.toml",
                "AGENTS.md",
                "shuyi.ron",
                ".github/workflows/ci.yml",
            ] {
                assert!(paths.iter().any(|p| p == file), "{short}: {file}");
            }
            let manifest = String::from_utf8_lossy(&plan.files[Path::new("Cargo.toml")]);
            let expected = match &engine {
                Engine::Main => format!("{{ git = \"{ENGINE_GIT}\", branch = \"main\" }}"),
                Engine::Rev { rev } => format!("{{ git = \"{ENGINE_GIT}\", rev = \"{rev}\" }}"),
                _ => "/crates/shuyi' }".to_owned(),
            };
            assert!(manifest.contains(&expected), "{manifest}");
            // The skill pack is copied, not filled: its Rust has `format!`'s `{{`.
            let filled = plan
                .files
                .iter()
                .filter(|(path, _)| !path.starts_with(crate::catalog::SKILLS));
            for (path, bytes) in filled {
                // Only text is filled: a picture's bytes may hold anything.
                let Ok(text) = std::str::from_utf8(bytes) else {
                    continue;
                };
                for (at, _) in text.match_indices("{{") {
                    assert!(
                        text[..at].ends_with('$'),
                        "{short}: {}: {}",
                        path.display(),
                        &text[at..]
                    );
                }
            }
        }
    }
}

#[test]
fn the_engines_docs_are_those_of_the_engine_the_game_builds_with() {
    let at = |rev: &str| Engine::Rev {
        rev: rev.to_owned(),
    };
    let checkout = Engine::Path {
        path: "e".to_owned(),
    };
    assert_eq!(
        engine_docs(&Engine::Main, None),
        "- its rules: [`AGENTS.md`](https://github.com/Zafnok/shuyi-engine/blob/main/AGENTS.md)\n\
         - its docs, page by page: [`llms.txt`](https://zafnok.github.io/shuyi-engine/llms.txt)\n\
         - its docs, in one file: [`llms-full.txt`](https://zafnok.github.io/shuyi-engine/llms-full.txt)"
    );
    assert_eq!(
        engine_docs(&at("abc1234"), None),
        "- its rules: [`AGENTS.md`](https://github.com/Zafnok/shuyi-engine/blob/abc1234/AGENTS.md)\n\
         - its book, in Markdown: [`site/content/learn`](https://github.com/Zafnok/shuyi-engine/tree/abc1234/site/content/learn), \
         whose code is in `site/snippets/` at the same commit\n\
         - its guides, which kits and modules go with each kind of game: \
         [`docs/guides`](https://github.com/Zafnok/shuyi-engine/tree/abc1234/docs/guides)"
    );
    assert_eq!(
        engine_docs(&checkout, Some("D:/e")),
        "- its rules: `D:/e/AGENTS.md`\n\
         - its book, in Markdown: `D:/e/site/content/learn/`, whose code is in \
         `D:/e/site/snippets/`\n\
         - its guides, which kits and modules go with each kind of game: \
         `D:/e/docs/guides/`"
    );
}

#[test]
fn a_new_games_agents_file_links_the_docs_of_its_engine() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (engine, holds, lacks) in [
        (Engine::Main, "/blob/main/AGENTS.md", "site/content/learn"),
        (
            Engine::Rev {
                rev: "abc1234".to_owned(),
            },
            "/tree/abc1234/site/content/learn",
            "blob/main",
        ),
        (
            Engine::Path {
                path: ".".to_owned(),
            },
            "/site/content/learn/`",
            "https://zafnok.github.io",
        ),
    ] {
        let mut input = NewInput::new("g");
        input.engine = engine;
        let (plan, _) = plan(&cwd, &template, &input, "Moon", "moon").unwrap();
        let agents = String::from_utf8_lossy(&plan.files[Path::new("AGENTS.md")]);
        assert!(agents.contains(holds), "{agents}");
        assert!(!agents.contains(lacks), "{agents}");
    }
}

#[test]
fn the_grid_game_calls_the_engines_playtest_workflow_at_its_engine_commit() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let full = "0123456789abcdef0123456789abcdef01234567";
    let call = "uses: Zafnok/shuyi-engine/.github/workflows/playtest.yml@";
    let pinned = |rev: &str| Engine::Rev {
        rev: rev.to_owned(),
    };
    for (engine, at, limited) in [
        (pinned(full), full, false),
        (pinned("abc1234"), "main", true),
        (Engine::Main, "main", false),
    ] {
        let mut input = NewInput::new("g").template("grid");
        input.engine = engine;
        let (plan, limits) = plan(cwd, &template, &input, "Moon", "moon").unwrap();
        let workflow = String::from_utf8_lossy(&plan.files[Path::new(PLAYTEST_WORKFLOW)]);
        assert!(workflow.contains(&format!("{call}{at} ")), "{workflow}");
        assert!(workflow.contains("cancel-in-progress: ${{ github.event_name != 'push' }}"));
        let says = limits.iter().any(|l| l.contains("40 digits"));
        assert_eq!(says, limited, "{limits:?}");
    }
    // The blank game has no stage to play.
    let (plan, _) = plan(
        cwd,
        &template,
        &NewInput::new("g").template("blank"),
        "Moon",
        "moon",
    )
    .unwrap();
    assert!(!plan.files.contains_key(Path::new(PLAYTEST_WORKFLOW)));
}

#[test]
fn the_grid_game_with_private_files_passes_their_key_to_the_playtest() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut input = private_input(Some("yes"), Some("https://github.com/o/g-assets.git"));
    input.template = Some("grid".to_owned());
    let (plan, _) = plan(cwd, &template, &input, "G", "g").unwrap_or_else(|(_, e)| panic!("{e}"));
    assert!(
        plan.parts.iter().any(|p| p == "private-files"),
        "{:?}",
        plan.parts
    );
    assert!(
        plan.files
            .contains_key(Path::new(".github/actions/private-files/action.yml"))
    );
    let workflow = String::from_utf8_lossy(&plan.files[Path::new(PLAYTEST_WORKFLOW)]);
    // By name, never every secret the game has.
    assert!(
        workflow.contains(
            "    secrets:\n      PRIVATE_ASSETS_KEY: ${{ secrets.PRIVATE_ASSETS_KEY }}\n"
        ),
        "{workflow}"
    );
    assert!(!workflow.contains("secrets: inherit"), "{workflow}");
}

#[test]
fn every_short_names_game_ships_on_the_desktop_and_the_web_and_fits_them() {
    let template = built_in();
    for short in ["grid", "blank"] {
        for (name, krate) in [("Moon", "moon"), ("Lune Café", "lune-caf")] {
            let input = NewInput::new("g").template(short);
            let (plan, _) = plan(Path::new("."), &template, &input, name, krate).unwrap();
            let dir = scratch(&format!("fits-{short}-{krate}"));
            template::apply(&plan, &dir, false).unwrap();
            let project = crate::project::Project::load(&dir).unwrap();
            let config = &project.config;
            let ids: Vec<&str> = config.targets.keys().map(String::as_str).collect();
            assert_eq!(ids, ["desktop", "web"], "{short}");
            // The game's `app_info.rs` is the one `app-info` writes.
            let app = config.app.as_ref().unwrap();
            assert_eq!(app.name, name);
            let command = config.command();
            assert!(
                crate::app_info::stale(app, &dir, command).is_empty(),
                "{short} {name}"
            );
            assert_eq!(
                crate::app_info::run(app, &dir, command, true),
                Ok("app-info: up to date".to_owned()),
                "{short} {name}"
            );
            let input = crate::contract::check::CheckInput {
                target: Some("all".to_owned()),
                ..crate::contract::check::CheckInput::default()
            };
            let answer = crate::contract::check::targets(config, &dir, &input, None);
            assert!(answer.ok, "{short}: {answer:?}");
            // The one problem: the placeholder id, on each target.
            for report in &answer.result.as_ref().unwrap().targets {
                let found: Vec<(&str, &str)> = report
                    .problems
                    .iter()
                    .map(|p| {
                        (
                            p.setting.as_str(),
                            &p.message[..p.message.find(' ').unwrap()],
                        )
                    })
                    .collect();
                let id = format!("\"com.example.{krate}\"");
                assert_eq!(found, [("app.id", id.as_str())], "{short} {name}");
            }
            // With an icon, `app-info` writes the window's icons where the
            // template says, beside the code and outside `files/`.
            assert_eq!(app.window_icons.as_deref(), Some("crates/game/icon"));
            let mut iconed = app.clone();
            iconed.icon = Some("art/icon.png".to_owned());
            fs::create_dir_all(dir.join("art")).unwrap();
            let white = vec![255; 4 * 512 * 512];
            let png = crate::codec::encode_png(512, 512, &white).unwrap();
            fs::write(dir.join("art/icon.png"), png).unwrap();
            crate::app_info::run(&iconed, &dir, command, false).unwrap();
            assert!(dir.join("crates/game/icon/64.png").is_file());
            assert_eq!(
                crate::app_info::run(&iconed, &dir, command, true),
                Ok("app-info: up to date".to_owned())
            );
            fs::remove_dir_all(&dir).unwrap();
        }
    }
}

#[test]
fn an_answers_file_makes_the_same_bytes_as_the_set_flags_and_the_short_name() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut from_file = NewInput::new("g");
    from_file.answers = Some("tests/fixtures/answers.ron".to_owned());
    let from_set = parse_args(&[
        "g".into(),
        "--set".into(),
        "perspective=top-down-grid".into(),
    ])
    .unwrap();
    let from_name = NewInput::new("g").template("grid");
    let made = |input: &NewInput| plan(cwd, &template, input, "Moon", "moon").unwrap().0;
    let file_plan = made(&from_file);
    assert_eq!(file_plan, made(&from_set));
    assert_eq!(file_plan, made(&from_name));
    assert_eq!(file_plan.stages, ["top-down-grid"]);
    // A later source answers in place of an earlier one.
    let mut blank_again = from_file.clone();
    blank_again.set = Answers::parse("set", "{ \"perspective\": [] }").unwrap();
    let stages = made(&blank_again).stages;
    assert!(stages.is_empty(), "{stages:?}");
}

#[test]
fn answers_the_template_doesnt_take_are_refused() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut wrong = NewInput::new("g");
    wrong.set = Answers::parse("set", "{ \"perspective\": [\"sideways\"] }").unwrap();
    let (code, error) = plan(cwd, &template, &wrong, "Moon", "moon").unwrap_err();
    assert_eq!(code, codes::WRONG_ARGUMENTS);
    assert!(
        error.contains("`sideways` is no answer of `perspective`"),
        "{error}"
    );
    // A parked answer is the catalog's, shown and never picked (ADR-0013
    // §6).
    wrong.set = Answers::parse("set", "{ \"perspective\": [\"side-view\"] }").unwrap();
    let (code, error) = plan(cwd, &template, &wrong, "Moon", "moon").unwrap_err();
    assert_eq!(code, codes::WRONG_ARGUMENTS);
    assert_eq!(
        error,
        "`side-view` (side view), an answer of `perspective`, is parked: ticket 1127 builds it, \
         so no game picks it yet"
    );
    let (code, error) = plan(
        cwd,
        &template,
        &NewInput::new("g").template("platformer"),
        "Moon",
        "moon",
    )
    .unwrap_err();
    assert_eq!(code, codes::WRONG_ARGUMENTS);
    assert_eq!(
        error,
        "no template platformer: the templates are blank, grid"
    );
    let mut missing = NewInput::new("g");
    missing.answers = Some("tests/fixtures/nowhere.ron".to_owned());
    assert!(plan(cwd, &template, &missing, "Moon", "moon").is_err());
}

/// A blank game, with private files if `private` and `url` as its
/// `--private-url`.
fn private_input(private: Option<&str>, url: Option<&str>) -> NewInput {
    let mut input = NewInput::new("g").template("blank");
    if let Some(answer) = private {
        input.set = Answers::parse("set", &format!("{{ \"private-files\": [\"{answer}\"] }}"))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    input.private_url = url.map(str::to_owned);
    input
}

#[test]
fn private_files_need_the_address_of_a_github_repository_and_only_they_take_one() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let refused = |input: &NewInput| {
        let (code, error) = plan(cwd, &template, input, "G", "g").unwrap_err();
        assert_eq!(code, codes::WRONG_ARGUMENTS, "{error}");
        error
    };
    let url = "https://github.com/o/g-assets.git";
    assert_eq!(
        refused(&private_input(Some("yes"), None)),
        "private-files=yes needs --private-url https://github.com/<owner>/<repo>.git: the \
         private repository the game's private files are kept in"
    );
    for answer in [None, Some("no")] {
        assert_eq!(
            refused(&private_input(answer, Some(url))),
            "--private-url is for a game with private files: add --set private-files=yes"
        );
    }
    for bad in [
        "http://github.com/o/g.git",
        "https://github.com/o/g",
        "https://gitlab.com/o/g.git",
        "git@github.com:o/g.git",
        "https://github.com/o.git",
        "https://github.com/o/g/h.git",
        "https://github.com//g.git",
        "https://github.com/-o/g.git",
        "https://github.com/o--p/g.git",
        "https://github.com/o_p/g.git",
        "https://github.com/o/.git",
        "https://github.com/o/...git",
        "https://github.com/o/g.git.git",
        "https://github.com/o/g.GIT.git",
        "https://github.com/o/g\".git",
        "https://github.com/o/g h.git",
    ] {
        assert_eq!(
            refused(&private_input(Some("yes"), Some(bad))),
            format!(
                "--private-url: not a GitHub repository's address: {bad}; write it \
                 https://github.com/<owner>/<repo>.git"
            ),
        );
    }
    for good in [
        url,
        "https://github.com/Zafnok/visions-of-shuyi-assets.git",
        "https://github.com/o-p/g_h.1.git",
    ] {
        let (plan, _) = plan(
            cwd,
            &template,
            &private_input(Some("yes"), Some(good)),
            "G",
            "g",
        )
        .unwrap_or_else(|(_, e)| panic!("{good}: {e}"));
        assert_eq!(plan.parts, ["base", "private-files"]);
    }
}

#[test]
fn the_private_part_writes_the_repository_its_ignore_line_and_its_ci_step() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let url = "https://github.com/o/g-assets.git";
    let text = |plan: &Plan, path: &str| {
        String::from_utf8_lossy(&plan.files[Path::new(path)]).into_owned()
    };
    let (private, _) = plan(
        cwd,
        &template,
        &private_input(Some("yes"), Some(url)),
        "G",
        "g",
    )
    .unwrap_or_else(|(_, e)| panic!("{e}"));
    let (blank, _) = plan(cwd, &template, &private_input(None, None), "G", "g")
        .unwrap_or_else(|(_, e)| panic!("{e}"));
    assert_eq!(blank.parts, ["base"]);

    let config = crate::project::Config::parse(&text(&private, "shuyi.ron"))
        .unwrap_or_else(|e| panic!("{e}"));
    let assets = config
        .private_assets
        .unwrap_or_else(|| panic!("no private_assets"));
    assert_eq!(assets.url, url);
    assert_eq!(
        (assets.checkout.as_str(), assets.pin_file.as_str()),
        ("private", "private.rev")
    );
    assert_eq!(assets.folders, ["files", "docs"]);
    assert_eq!(assets.library.as_deref(), Some("library"));
    assert!(
        assets.no_access.contains("public files"),
        "{}",
        assets.no_access
    );
    assert_eq!(
        config.app,
        crate::project::Config::parse(&text(&blank, "shuyi.ron"))
            .unwrap()
            .app
    );
    assert!(
        crate::project::Config::parse(&text(&blank, "shuyi.ron"))
            .unwrap()
            .private_assets
            .is_none()
    );

    let ignored = |plan: &Plan| text(plan, ".gitignore").lines().any(|l| l == "/private/");
    assert!(ignored(&private));
    assert!(!ignored(&blank));
    // No pin yet: an empty pin file, which `shuyi repos` fills.
    assert_eq!(text(&private, "private.rev"), "");

    let action = ".github/actions/private-files/action.yml";
    assert_the_ci_runs_the_part_and_keeps_its_files(&private, &blank, action, url);

    // Its words follow the base's in AGENTS.md and README.md.
    for doc in ["AGENTS.md", "README.md"] {
        let (with, without) = (text(&private, doc), text(&blank, doc));
        assert!(with.starts_with(without.trim_end()), "{doc}");
        assert!(with.contains("## The private files"), "{doc}");
        assert!(with.contains(url), "{doc}");
        assert!(!without.contains("## The private files"), "{doc}");
    }

    // Every other file is the blank game's.
    let only_private = [
        "shuyi.ron",
        ".gitignore",
        "private.rev",
        action,
        "AGENTS.md",
        "README.md",
    ];
    for (path, bytes) in &blank.files {
        let shown = path.to_string_lossy().replace('\\', "/");
        if !only_private.contains(&shown.as_str()) {
            assert_eq!(private.files.get(path), Some(bytes), "{shown}");
        }
    }
    assert_eq!(private.files.len(), blank.files.len() + 2);
}

/// The CI step is the base's, run when the part's action is there, and
/// only export.yml uploads an artifact. What a job with the private files
/// may cache and upload is
/// `every_job_with_the_private_files_caches_no_build_and_uploads_none`'s.
fn assert_the_ci_runs_the_part_and_keeps_its_files(
    private: &Plan,
    blank: &Plan,
    action: &str,
    url: &str,
) {
    let text = |plan: &Plan, path: &str| {
        String::from_utf8_lossy(&plan.files[Path::new(path)]).into_owned()
    };
    assert!(text(private, action).contains(&format!("URL: \"{url}\"")));
    assert!(!blank.files.contains_key(Path::new(action)));
    let ci = text(private, ".github/workflows/ci.yml");
    assert_eq!(ci, text(blank, ".github/workflows/ci.yml"));
    let step = ci.find("uses: ./.github/actions/private-files").unwrap();
    let cargo = ci.find("run: cargo").unwrap();
    assert!(step < cargo, "the step runs before the first cargo command");
    assert!(ci.contains(&format!("if: hashFiles('{action}') != ''")));
    // A release's packages hold the private files: its caller asks for
    // them on a tag, which a public repository's export.yml seals first
    // (ADR-0133), or in a private repository.
    let release = text(private, RELEASE_WORKFLOW);
    assert!(release.contains(
        "upload-private: ${{ github.event_name == 'push' || github.event.repository.private }}"
    ));
    for plan in [private, blank] {
        for (path, bytes) in &plan.files {
            let shown = path.to_string_lossy().replace('\\', "/");
            if shown.starts_with(".github/") && shown != EXPORT_WORKFLOW {
                let text = String::from_utf8_lossy(bytes);
                assert!(
                    !text.contains("upload-artifact"),
                    "{shown} uploads an artifact"
                );
            }
        }
    }
}

/// The base's reusable workflow that builds one target's package.
const EXPORT_WORKFLOW: &str = ".github/workflows/export.yml";

/// The base's workflow that releases a game from a tag.
const RELEASE_WORKFLOW: &str = ".github/workflows/release.yml";

/// The jobs of the workflow `text`, each with its text: a line indented two
/// spaces under `jobs:` starts one.
fn jobs_of(text: &str) -> Vec<(String, String)> {
    let body = &text[text.find("\njobs:\n").unwrap()..];
    let mut jobs: Vec<(String, String)> = Vec::new();
    for line in body.lines().skip(2) {
        let starts = line.len() > 2 && line.starts_with("  ") && !line.starts_with("   ");
        match (starts, jobs.last_mut()) {
            (true, _) => jobs.push((line.trim().trim_end_matches(':').to_owned(), String::new())),
            (false, Some((_, job))) => {
                job.push_str(line);
                job.push('\n');
            }
            (false, None) => {}
        }
    }
    jobs
}

/// The workflows of `plan` but the grid stage's playtest (its own, ticket
/// 2021), by path.
fn workflows(plan: &Plan) -> BTreeMap<String, Vec<u8>> {
    plan.files
        .iter()
        .map(|(path, bytes)| (path.to_string_lossy().replace('\\', "/"), bytes))
        .filter(|(shown, _)| {
            shown.starts_with(".github/workflows/") && !shown.ends_with("playtest.yml")
        })
        .map(|(shown, bytes)| (shown, bytes.clone()))
        .collect()
}

#[test]
fn every_games_workflows_are_the_same_and_the_engines_export_is_theirs() {
    let games = every_short_names_game();
    let first = workflows(&games[0].1);
    let names: Vec<&str> = first.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            ".github/workflows/ci.yml",
            EXPORT_WORKFLOW,
            RELEASE_WORKFLOW
        ]
    );
    for (short, plan) in &games {
        assert_eq!(workflows(plan), first, "{short}");
    }
    // The engine's CI runs the file a game's release runs.
    let engine = fs::read(engine_checkout().join(EXPORT_WORKFLOW)).unwrap();
    let base = engine_checkout()
        .join("templates/shuyi-default/base")
        .join(EXPORT_WORKFLOW);
    let base = fs::read(base).unwrap();
    assert!(engine == base, "the engine's export.yml isn't the base's");
    assert!(
        first[EXPORT_WORKFLOW] == base,
        "export.yml has no placeholder"
    );
    // Its Linux leg builds in the image the plan names.
    let ci = fs::read_to_string(engine_checkout().join(".github/workflows/ci.yml")).unwrap();
    assert!(ci.contains(&format!("container: {}\n", crate::export::SNIPER)));
    assert!(crate::export::SNIPER.contains("@sha256:"));
}

#[test]
fn no_job_of_a_tag_reads_a_cache_and_only_the_release_writes() {
    let (_, plan) = every_short_names_game().remove(0);
    let text = |path: &str| String::from_utf8_lossy(&plan.files[Path::new(path)]).into_owned();
    let tagless = "if: ${{ !startsWith(github.ref, 'refs/tags/') }}";
    let ci_path = ".github/workflows/ci.yml";
    for path in [ci_path, EXPORT_WORKFLOW, RELEASE_WORKFLOW] {
        let text = text(path);
        assert!(
            text.contains("\npermissions:\n  contents: read\n"),
            "{path}"
        );
        for (at, _) in text.match_indices("uses: Swatinem/rust-cache@") {
            let next = text[at..].lines().nth(1).unwrap_or_default().trim();
            assert_eq!(next, tagless, "{path}: a cache read on a tag");
        }
        for (job, body) in jobs_of(&text) {
            let writes = body.contains(": write");
            assert_eq!(writes, job == "release", "{path}: the job {job}");
        }
    }
    let release = text(RELEASE_WORKFLOW);
    let jobs: Vec<String> = jobs_of(&release).into_iter().map(|(job, _)| job).collect();
    assert_eq!(jobs, ["gates", "plan", "export", "release", "itch"]);
    // The values of the event reach the scripts through `env:`.
    for line in release.lines().filter(|line| line.contains("${{")) {
        assert!(!line.trim_start().starts_with("run:"), "{line}");
    }
    // ci.yml checks the lock file first, and keeps to it.
    let ci = text(ci_path);
    let lock = ci.find("name: Cargo.lock is committed").unwrap();
    assert!(lock < ci.find("cargo fmt --all --check").unwrap());
    assert!(ci.contains(
        "commit Cargo.lock: run \\`cargo generate-lockfile\\` (or any build) and commit it"
    ));
    assert!(ci.contains("run: cargo clippy --workspace --all-targets --locked -- -D warnings\n"));
    assert!(ci.contains("run: cargo test --workspace --locked\n"));
    assert!(ci.contains("\n  workflow_call:\n"));
}

#[test]
fn new_makes_the_game_in_an_empty_folder_only_unless_forced() {
    let dir = scratch("make");
    let input = NewInput::new("Moon Garden");
    let answer = new(&dir, &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.as_ref().unwrap();
    assert_eq!(
        (out.name.as_str(), out.crate_name.as_str()),
        ("Moon Garden", "moon-garden")
    );
    // No answer: the catalog's defaults, a top-down grid (ADR-0013 §2).
    assert_eq!(
        (out.template.as_str(), out.parts.as_slice()),
        (
            "shuyi-default",
            &["base".to_owned(), "stage-top-down-grid".to_owned()][..]
        )
    );
    assert!(out.files.contains(&"Cargo.toml".to_owned()));
    assert!(out.overwritten.is_empty() && out.modules.is_empty());
    assert_eq!(out.stages, ["top-down-grid"]);
    let made = fs::read_to_string(dir.join("Moon Garden/README.md")).unwrap();
    assert!(made.starts_with("# Moon Garden\n"), "{made}");
    assert!(
        answer.limits[0].contains("main branch"),
        "{:?}",
        answer.limits
    );
    assert!(answer.limits.last().unwrap().contains("not built"));
    assert!(
        answer
            .limits
            .last()
            .unwrap()
            .ends_with("and commit the Cargo.lock it writes")
    );
    // Not again on top of it.
    let again = new(&dir, &input);
    let error = again.error.as_deref().unwrap();
    assert!(
        error.starts_with("Moon Garden is not empty (it would write over .cargo/config.toml, "),
        "{error}"
    );
    assert!(
        error.ends_with(": pass --force to write into it"),
        "{error}"
    );
    assert_eq!(again.code.as_deref(), Some("SH0001"));
    assert_eq!(again.exit_code(), 2);
    // With --force, over it, saying so.
    let mut forced = input.clone();
    forced.force = true;
    let answer = new(&dir, &forced);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    assert_eq!(out.overwritten, out.files);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_engine_is_a_commit_or_a_checkout() {
    let dir = scratch("engine");
    let input = |engine| {
        let mut input = NewInput::new("g");
        input.name = Some("Game".to_owned());
        input.engine = engine;
        input
    };
    let bad_rev = new(
        &dir,
        &input(Engine::Rev {
            rev: "main".to_owned(),
        }),
    );
    assert_eq!(
        bad_rev.error.as_deref(),
        Some("--engine-rev: not a commit: main")
    );
    let no_engine = new(
        &dir,
        &input(Engine::Path {
            path: "nowhere".to_owned(),
        }),
    );
    assert!(
        no_engine
            .error
            .unwrap()
            .contains("no checkout of the engine")
    );
    // This repository is one.
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let answer = new(
        &dir,
        &input(Engine::Path {
            path: engine.to_string_lossy().into_owned(),
        }),
    );
    assert!(answer.ok, "{answer:?}");
    let manifest = fs::read_to_string(dir.join("g/Cargo.toml")).unwrap();
    assert!(manifest.contains("/crates/shuyi' }"), "{manifest}");
    assert!(!manifest.contains('\\'), "{manifest}");
    assert!(answer.limits[0].contains("engine checkout"));
    let mut pinned = input(Engine::Rev {
        rev: "0123abcd".to_owned(),
    });
    pinned.dir = "h".to_owned();
    let pinned = new(&dir, &pinned);
    assert!(pinned.ok, "{pinned:?}");
    assert!(!pinned.limits[0].contains("main branch"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_arguments_name_a_folder_and_the_answers() {
    let args = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).into()).collect() };
    let input = parse_args(&args(&[
        "g",
        "--template",
        "grid",
        "--name",
        "G",
        "--engine-rev",
        "abc1234",
        "--private-url",
        "https://github.com/o/g-assets.git",
        "--answers",
        "a.ron",
        "--set",
        "perspective=top-down-grid",
        "--set",
        "music=none,recorded",
        "--force",
    ]))
    .unwrap();
    assert_eq!(input.dir, "g");
    assert_eq!(input.template.as_deref(), Some("grid"));
    assert_eq!(input.answers.as_deref(), Some("a.ron"));
    assert_eq!(input.name.as_deref(), Some("G"));
    assert_eq!(
        input.private_url.as_deref(),
        Some("https://github.com/o/g-assets.git")
    );
    assert!(input.force);
    assert_eq!(input.set.0["music"], ["none", "recorded"]);
    assert_eq!(input.set.0["perspective"], ["top-down-grid"]);
    assert_eq!(
        input.engine,
        Engine::Rev {
            rev: "abc1234".to_owned()
        }
    );
    // The older form: a template, then the folder.
    let older = parse_args(&args(&["blank", "g", "--engine-path", ".."])).unwrap();
    assert_eq!(
        (older.template.as_deref(), older.dir.as_str()),
        (Some("blank"), "g")
    );
    assert_eq!(
        older.engine,
        Engine::Path {
            path: "..".to_owned()
        }
    );
    assert!(parse_args(&args(&[])).is_err());
    assert!(parse_args(&args(&["a", "b", "c"])).is_err());
    assert!(parse_args(&args(&["a", "b", "--template", "grid"])).is_err());
    assert!(parse_args(&args(&["a", "--name"])).is_err());
    assert!(parse_args(&args(&["a", "--set", "nothing"])).is_err());
    assert_eq!(
        parse_args(&args(&["a", "--bogus"])).unwrap_err(),
        "unknown option: --bogus"
    );
    let usage = usage("shuyi");
    assert!(
        usage.starts_with("usage: shuyi new <dir> [--template NAME]"),
        "{usage}"
    );
    assert!(usage.contains("(blank, grid)"), "{usage}");
}

#[test]
fn a_commit_is_seven_to_forty_hex_digits() {
    let dir = scratch("revs");
    let make = |dir_name: &str, rev: &str| {
        let mut input = NewInput::new(dir_name);
        input.name = Some("Game".to_owned());
        input.engine = Engine::Rev {
            rev: rev.to_owned(),
        };
        new(&dir, &input)
    };
    assert!(make("a", "abc1234").ok);
    assert!(make("b", &"f".repeat(40)).ok);
    for (i, bad) in ["abc123", &"f".repeat(41), "zzzzzzz", "abc123g"]
        .iter()
        .enumerate()
    {
        let answer = make(&format!("bad{i}"), bad);
        assert_eq!(
            answer.error.as_deref(),
            Some(format!("--engine-rev: not a commit: {bad}").as_str())
        );
    }
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_engine_path_with_a_quote_is_refused() {
    let dir = scratch("quote");
    let engine = dir.join("it's");
    fs::create_dir_all(engine.join("crates/core")).unwrap();
    fs::write(engine.join("crates/core/Cargo.toml"), "").unwrap();
    let error = engine_root(&dir, "it's").unwrap_err();
    assert!(
        error.starts_with("the engine's path may not hold a quote: "),
        "{error}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_name_from_the_folder_must_make_a_crate_name() {
    let dir = scratch("folder");
    let answer = new(&dir, &NewInput::new("123"));
    assert!(answer.error.unwrap().contains("makes no crate name"));
    assert!(!dir.join("123").exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn what_new_made_reads_for_people() {
    let out = NewOutput {
        dir: "g".into(),
        name: "Game".into(),
        crate_name: "game".into(),
        id: "com.example.game".into(),
        template: "shuyi-default".into(),
        answers: Answers::default(),
        parts: vec!["base".into(), "stage".into()],
        files: vec!["Cargo.toml".into(), "README.md".into()],
        overwritten: vec!["README.md".into()],
        modules: vec!["shuyi-physics".into()],
        targets: vec!["desktop".into()],
        stages: Vec::new(),
        skills: vec!["shuyi-gates".into(), "shuyi-test".into()],
    };
    assert_eq!(
        text(&out),
        "new: Game (game) in g, 2 file(s), from shuyi-default: base, stage\n\
         modules on: shuyi-physics\nskills: shuyi-gates, shuyi-test\nwritten over: README.md\n  \
         Cargo.toml\n  README.md\n"
    );
}

#[test]
fn a_new_input_names_the_game_for_its_folder_on_mains_engine() {
    let input = NewInput::new("games/moon");
    assert_eq!(input.dir, "games/moon");
    assert!(input.template.is_none() && input.answers.is_none() && input.set.0.is_empty());
    assert_eq!(
        (input.name, input.engine, input.force),
        (None, Engine::Main, false)
    );
    assert_eq!(
        NewInput::new("g").template("grid").template.as_deref(),
        Some("grid")
    );
}

/// The engine's checkout, this crate's grandparent.
fn engine_checkout() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The game each of the engine's template's short names makes, by its
/// short name.
fn every_short_names_game() -> Vec<(String, Plan)> {
    let template = built_in();
    template
        .short_names
        .keys()
        .map(|short| {
            let input = NewInput::new("g").template(short);
            let (plan, _) = plan(&engine_checkout(), &template, &input, "Moon", "moon")
                .unwrap_or_else(|(_, e)| panic!("{short}: {e}"));
            (short.clone(), plan)
        })
        .collect()
}

/// The words of the script's `NAME="…"` line.
fn script_list<'a>(script: &'a str, name: &str) -> Vec<&'a str> {
    let line = script
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=\"")))
        .unwrap_or_else(|| panic!("no {name}= line"));
    line.strip_suffix('"').unwrap().split_whitespace().collect()
}

#[test]
fn every_setup_script_is_the_engines_and_installs_doctors_table() {
    use crate::contract::doctor::linux::ROWS;
    let engine = fs::read(engine_checkout().join("scripts/setup-linux.sh")).unwrap();
    assert!(!engine.contains(&b'\r'));
    let script = String::from_utf8(engine.clone()).unwrap();
    let column = |pick: fn(&crate::contract::doctor::linux::Row) -> &'static str| {
        ROWS.iter().map(pick).collect::<Vec<_>>()
    };
    assert_eq!(script_list(&script, "APT"), column(|r| r.apt));
    assert_eq!(script_list(&script, "DNF"), column(|r| r.dnf));
    assert_eq!(script_list(&script, "PACMAN"), column(|r| r.pacman));
    for (short, plan) in every_short_names_game() {
        let copy = &plan.files[Path::new("scripts/setup-linux.sh")];
        assert!(
            copy == &engine,
            "{short}: scripts/setup-linux.sh is not the engine's"
        );
    }
}

#[test]
fn every_dev_container_is_set_up_as_the_engines() {
    let read = |bytes: &[u8]| -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap_or_else(|e| panic!("{e}"))
    };
    let engine =
        read(&fs::read(engine_checkout().join(".devcontainer/devcontainer.json")).unwrap());
    assert_eq!(engine["name"], "Shuyi Engine");
    let setup = "bash scripts/setup-linux.sh && . \"$HOME/.cargo/env\" && ";
    let engine_command = engine["postCreateCommand"].as_str().unwrap();
    assert_eq!(
        engine_command,
        format!(
            "{setup}cargo run -q --bin shuyi -- mcp install --yes --command \"cargo run -q --bin shuyi --\""
        )
    );
    for (short, plan) in every_short_names_game() {
        let game = read(&plan.files[Path::new(".devcontainer/devcontainer.json")]);
        assert_eq!(game["name"], "Moon", "{short}");
        for key in ["image", "remoteEnv", "customizations"] {
            assert_eq!(game[key], engine[key], "{short}: {key}");
        }
        assert_eq!(
            game["postCreateCommand"].as_str().unwrap(),
            format!("{setup}cargo shuyi mcp install --yes"),
            "{short}"
        );
    }
}

#[test]
fn no_workflow_installs_doctors_packages_by_hand() {
    use crate::contract::doctor::linux::ROWS;
    let mut workflows = Vec::new();
    for entry in fs::read_dir(engine_checkout().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        workflows.push((path.display().to_string(), text));
    }
    for (short, plan) in every_short_names_game() {
        for (path, bytes) in &plan.files {
            if path.starts_with(".github/workflows") {
                let text = String::from_utf8(bytes.clone()).unwrap();
                workflows.push((format!("{short}: {}", path.display()), text));
            }
        }
    }
    assert!(workflows.len() > 3, "{}", workflows.len());
    let packages: Vec<&str> = ROWS.iter().flat_map(|r| [r.apt, r.dnf, r.pacman]).collect();
    for (file, text) in &workflows {
        for line in text.lines().filter(|l| l.contains("apt-get install")) {
            let words = line.split(|c: char| c.is_whitespace() || c == '"' || c == '\'');
            for word in words {
                assert!(
                    !packages.contains(&word),
                    "{file} installs {word} by hand: call `bash scripts/setup-linux.sh`\n{line}"
                );
            }
        }
    }
    // The playtest workflow's default packages, for a game with no script
    // of its own, are the table's.
    let (_, playtest) = workflows
        .iter()
        .find(|(file, _)| file.ends_with("playtest.yml") && !file.contains(": "))
        .unwrap();
    let default = playtest
        .split("system-packages:")
        .nth(1)
        .and_then(|input| {
            input
                .lines()
                .find_map(|l| l.trim().strip_prefix("default: "))
        })
        .unwrap();
    let apt: Vec<&str> = ROWS.iter().map(|r| r.apt).collect();
    for package in default.split_whitespace() {
        assert!(apt.contains(&package), "playtest.yml's default: {package}");
    }
}

/// True when the game has no private files: the guard of every cache of
/// `target/` and every upload in a job that checks them out (ADR-0111 §5,
/// as ADR-0130 amends it).
const NO_PRIVATE_FILES: &str = "hashFiles('.github/actions/private-files/action.yml') == ''";

/// The private files' action, as a step's `uses:`.
const PRIVATE_FILES_ACTION: &str = "./.github/actions/private-files";

/// The conditions that may open a plain upload in a job with the private
/// files: the caller's word that a release needs the package, in a private
/// repository, whose artifacts only its own people read (ADR-0133 §3).
const UPLOAD_INPUTS: &[&str] = &["inputs.upload-private && github.event.repository.private"];

/// The step of export.yml that seals a public repository's package before
/// its upload (ADR-0133 §3).
const SEAL_STEP: &str = "Seal the package";

/// The folder below the game's that holds the sealed files, and only them.
const SEALED_DIR: &str = "/dist/sealed/";

/// A workflow the test reads: the game it is in (its short name) or
/// `engine`, its path there, and its YAML.
struct Workflow {
    origin: String,
    path: String,
    yaml: serde_norway::Value,
}

impl Workflow {
    fn parse(origin: &str, path: &str, bytes: &[u8]) -> Self {
        let yaml =
            serde_norway::from_slice(bytes).unwrap_or_else(|e| panic!("{origin}: {path}: {e}"));
        Self {
            origin: origin.to_owned(),
            path: path.to_owned(),
            yaml,
        }
    }

    /// Its jobs, by name.
    fn jobs(&self) -> Vec<(&str, &serde_norway::Value)> {
        self.yaml
            .get("jobs")
            .and_then(serde_norway::Value::as_mapping)
            .map_or_default(|jobs| {
                jobs.iter()
                    .map(|(name, job)| (name.as_str().unwrap_or("?"), job))
                    .collect()
            })
    }

    /// The events of its `on:`, whether one word, a list or a map.
    fn events(&self) -> Vec<&str> {
        use serde_norway::Value;
        match self.yaml.get("on") {
            Some(Value::String(event)) => vec![event.as_str()],
            Some(Value::Sequence(events)) => events.iter().filter_map(Value::as_str).collect(),
            Some(Value::Mapping(events)) => events.keys().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        }
    }

    /// The jobs that check the private files out, by name.
    fn jobs_with_the_private_files(&self) -> Vec<&str> {
        self.jobs()
            .into_iter()
            .filter(|(_, job)| has_the_private_files(job))
            .map(|(name, _)| name)
            .collect()
    }
}

/// A string field's text, or empty.
fn text_of(value: Option<&serde_norway::Value>) -> &str {
    value
        .and_then(serde_norway::Value::as_str)
        .unwrap_or_default()
}

/// The steps of a job, or none.
fn steps_of(job: &serde_norway::Value) -> &[serde_norway::Value] {
    job.get("steps")
        .and_then(serde_norway::Value::as_sequence)
        .map_or(&[], Vec::as_slice)
}

/// Whether a job checks the private files out.
fn has_the_private_files(job: &serde_norway::Value) -> bool {
    steps_of(job)
        .iter()
        .any(|step| text_of(step.get("uses")) == PRIVATE_FILES_ACTION)
}

/// An expression without its `${{ }}`, if any.
fn bare(expression: &str) -> &str {
    let trimmed = expression.trim();
    trimmed
        .strip_prefix("${{")
        .and_then(|inner| inner.strip_suffix("}}"))
        .map_or(trimmed, str::trim)
}

/// Whether a job that calls a workflow passes it the private files' key,
/// by name or with every secret.
fn passes_the_key(job: &serde_norway::Value) -> bool {
    match job.get("secrets") {
        Some(serde_norway::Value::String(all)) => all == "inherit",
        Some(named) => named.get("PRIVATE_ASSETS_KEY").is_some(),
        None => false,
    }
}

/// The workflow a job's `uses:` calls, among those the test reads: one of
/// its own game's (`./…`), or one of the engine's at any commit.
fn called<'a>(workflows: &'a [Workflow], from: &Workflow, uses: &str) -> Option<&'a Workflow> {
    let (origin, path) = if let Some(local) = uses.strip_prefix("./") {
        (from.origin.as_str(), local)
    } else {
        let engine = uses.strip_prefix("Zafnok/shuyi-engine/")?;
        ("engine", engine.split('@').next()?)
    };
    workflows
        .iter()
        .find(|w| w.origin == origin && w.path == path)
}

/// Each way a job of `workflows` that has the private files could let them
/// out through a cache, an artifact or its triggers, named by its file, job
/// and step. None is the rule kept.
fn private_files_leaks(workflows: &[Workflow]) -> Vec<String> {
    let mut leaks = Vec::new();
    for workflow in workflows {
        let file = format!("{}: {}", workflow.origin, workflow.path);
        let events = workflow.events();
        if events.is_empty() {
            leaks.push(format!("{file}: no `on:` read"));
        }
        let unsafe_events: Vec<&str> = events
            .iter()
            .copied()
            .filter(|e| ["pull_request_target", "workflow_run"].contains(e))
            .collect();
        for (name, job) in workflow.jobs() {
            let at = format!("{file}: the job {name}");
            if let Some(uses) = job.get("uses").and_then(serde_norway::Value::as_str) {
                if passes_the_key(job) {
                    if called(workflows, workflow, uses).is_none() {
                        leaks.push(format!(
                            "{at} passes PRIVATE_ASSETS_KEY to {uses}, a workflow this test \
                             doesn't read"
                        ));
                    }
                    for event in &unsafe_events {
                        leaks.push(format!("{at} passes PRIVATE_ASSETS_KEY on {event}"));
                    }
                }
                continue;
            }
            if !has_the_private_files(job) {
                continue;
            }
            for event in &unsafe_events {
                leaks.push(format!("{at} checks out the private files on {event}"));
            }
            let steps = steps_of(job);
            for (i, step) in steps.iter().enumerate() {
                let label = [step.get("name"), step.get("uses"), step.get("run")]
                    .into_iter()
                    .map(text_of)
                    .find(|t| !t.is_empty())
                    .unwrap_or_default();
                let at = format!("{at}: step {} ({label})", i + 1);
                leaks.extend(step_leak(step, &steps[..i]).map(|why| format!("{at} {why}")));
            }
        }
    }
    leaks
}

/// Whether an upload takes only sealed files: every path in the sealed
/// folder, after a step that seals them under the same `if:`.
fn uploads_sealed(step: &serde_norway::Value, earlier: &[serde_norway::Value]) -> bool {
    let condition = text_of(step.get("if"));
    let paths = text_of(step.get("with").and_then(|w| w.get("path")));
    let only_sealed = !paths.trim().is_empty()
        && paths
            .lines()
            .filter(|line| !line.trim().is_empty())
            .all(|line| line.contains(SEALED_DIR));
    let sealed_first = earlier.iter().any(|seal| {
        text_of(seal.get("name")) == SEAL_STEP
            && text_of(seal.get("if")) == condition
            && text_of(seal.get("run")).contains("openssl enc -aes-256-cbc")
    });
    !condition.is_empty() && only_sealed && sealed_first
}

/// How one step of a job with the private files lets them out, if it does,
/// after the job's `earlier` steps.
fn step_leak(step: &serde_norway::Value, earlier: &[serde_norway::Value]) -> Option<String> {
    let action = text_of(step.get("uses"))
        .split('@')
        .next()
        .unwrap_or_default();
    let condition = bare(text_of(step.get("if")));
    let with = step.get("with");
    if action == "Swatinem/rust-cache" {
        let targets = bare(text_of(with.and_then(|w| w.get("cache-targets"))));
        return (targets != NO_PRIVATE_FILES).then(|| {
            format!("caches target/: its cache-targets isn't ${{{{ {NO_PRIVATE_FILES} }}}}")
        });
    }
    let caches =
        action.to_lowercase().contains("cache") || with.and_then(|w| w.get("cache")).is_some();
    if caches {
        let guarded = !condition.contains("||")
            && condition.split("&&").any(|c| c.trim() == NO_PRIVATE_FILES);
        return (!guarded).then(|| format!("caches, and its if: doesn't hold {NO_PRIVATE_FILES}"));
    }
    if action.starts_with("actions/upload-artifact")
        || action.starts_with("actions/upload-pages-artifact")
    {
        let guarded = !condition.is_empty()
            && condition
                .split("||")
                .map(str::trim)
                .all(|c| c == NO_PRIVATE_FILES || UPLOAD_INPUTS.contains(&c));
        return (!guarded && !uploads_sealed(step, earlier)).then(|| {
            format!(
                "uploads an artifact, and its if: isn't {NO_PRIVATE_FILES} or one of \
                 {UPLOAD_INPUTS:?}, nor does it take only what `{SEAL_STEP}` sealed: \
                 `{condition}`"
            )
        });
    }
    None
}

/// Every file of `dir`, at any depth.
fn files_under(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

#[test]
fn every_job_with_the_private_files_caches_no_build_and_uploads_none() {
    let template = built_in();
    let mut games = every_short_names_game();
    let private_name = "grid, private-files=yes";
    games.push((private_name.to_owned(), private_grid_game()));

    let mut workflows = Vec::new();
    for (short, plan) in &games {
        for (path, bytes) in &plan.files {
            let shown = path.to_string_lossy().replace('\\', "/");
            if shown.starts_with(".github/workflows/") {
                workflows.push(Workflow::parse(short, &shown, bytes));
            }
        }
    }
    for path in [".github/workflows/playtest.yml", EXPORT_WORKFLOW] {
        let bytes = fs::read(engine_checkout().join(path)).unwrap();
        workflows.push(Workflow::parse("engine", path, &bytes));
    }
    let leaks = private_files_leaks(&workflows);
    assert!(leaks.is_empty(), "{}", leaks.join("\n"));

    // The rules bite: each of these has a job that checks the files out.
    for (origin, path) in [
        (private_name, ".github/workflows/ci.yml"),
        (private_name, EXPORT_WORKFLOW),
        ("engine", ".github/workflows/playtest.yml"),
        ("engine", EXPORT_WORKFLOW),
    ] {
        let workflow = workflows
            .iter()
            .find(|w| w.origin == origin && w.path == path)
            .unwrap_or_else(|| panic!("{origin}: {path} not read"));
        assert!(
            !workflow.jobs_with_the_private_files().is_empty(),
            "{origin}: {path}: no job checks out the private files"
        );
    }

    // Nothing left out: every workflow file of the template is in a game
    // composed here, from the part whose folder holds it.
    let templates = engine_checkout().join("templates");
    for file in files_under(&templates) {
        let shown = file
            .strip_prefix(&templates)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let Some((folder, name)) = shown.split_once("/.github/workflows/") else {
            continue;
        };
        let part = folder
            .strip_prefix(&format!("{}/", template.id))
            .and_then(|folder| {
                template
                    .parts
                    .iter()
                    .find(|p| p.files.to_string_lossy().replace('\\', "/") == folder)
            })
            .unwrap_or_else(|| {
                panic!("templates/{shown}: in no part of the template composed here")
            });
        let path = format!(".github/workflows/{name}");
        let held = games.iter().any(|(_, plan)| {
            plan.parts.contains(&part.id) && plan.files.contains_key(Path::new(&path))
        });
        assert!(
            held,
            "templates/{shown}: no game composed here has the part {}: make one with it",
            part.id
        );
    }
}

/// A workflow on `on`, of one job that checks out the private files and
/// then runs `steps` (YAML list items, indented six spaces).
fn private_job(on: &str, steps: &str) -> String {
    format!(
        "on: {on}\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      \
         - uses: ./.github/actions/private-files\n{steps}"
    )
}

/// The leaks of the one workflow `text`, read with `others` (path, text)
/// of the same game.
fn leaks_of(text: &str, others: &[(&str, &str)]) -> Vec<String> {
    let mut workflows = vec![Workflow::parse(
        "case",
        ".github/workflows/case.yml",
        text.as_bytes(),
    )];
    for (path, other) in others {
        workflows.push(Workflow::parse("case", path, other.as_bytes()));
    }
    private_files_leaks(&workflows)
}

/// A rust-cache step whose `cache-targets` is `targets`.
fn rust_cache(targets: &str) -> String {
    format!(
        "      - uses: Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0\n        \
         with:\n          cache-targets: {targets}\n"
    )
}

/// An upload step under `condition`, none if empty.
fn upload(condition: &str) -> String {
    let condition = if condition.is_empty() {
        String::new()
    } else {
        format!("        if: {condition}\n")
    };
    format!("      - name: Upload\n{condition}        uses: actions/upload-artifact@v7\n")
}

/// Asserts that the workflow `text` keeps the rules.
fn assert_kept(text: &str) {
    assert_eq!(leaks_of(text, &[]), Vec::<String>::new(), "{text}");
}

/// Asserts that a leak of the workflow `text` says `why`.
fn assert_refused(text: &str, why: &str) {
    let leaks = leaks_of(text, &[]);
    assert!(
        leaks.iter().any(|l| l.contains(why)),
        "{why}: {leaks:?}\n{text}"
    );
}

#[test]
fn each_rule_on_a_job_with_the_private_files_refuses_its_leak() {
    let guard = format!("${{{{ {NO_PRIVATE_FILES} }}}}");
    // The rules kept: the base's steps.
    let good = format!(
        "{}{}{}",
        rust_cache(&guard),
        upload(&format!(
            "inputs.upload-private && github.event.repository.private || {NO_PRIVATE_FILES}"
        )),
        upload(&guard)
    );
    assert_kept(&private_job("pull_request", &good));
    assert_kept(&private_job("[push, workflow_call]", &good));

    // A second rust-cache, unguarded or guarded the wrong way.
    for targets in ["true", "false", "${{ hashFiles('x') == '' }}"] {
        assert_refused(
            &private_job("pull_request", &format!("{good}{}", rust_cache(targets))),
            "caches target/",
        );
    }
    assert_refused(
        &private_job("pull_request", "      - uses: Swatinem/rust-cache@v2\n"),
        "caches target/",
    );
    // An upload with no if:, or one opened by anything else.
    // A release's plain package of a public repository (3050's) is one too.
    for condition in [
        String::new(),
        "always()".to_owned(),
        "inputs.upload-private || always()".to_owned(),
        "inputs.other".to_owned(),
        format!("{NO_PRIVATE_FILES} && always()"),
        format!("inputs.upload-private || {NO_PRIVATE_FILES}"),
        "inputs.upload-private".to_owned(),
    ] {
        assert_refused(
            &private_job("pull_request", &upload(&condition)),
            "uploads an artifact",
        );
    }
    // A sealed upload: kept after its seal, under the same if:, of the
    // sealed folder only.
    let when = "inputs.upload-private && !github.event.repository.private";
    let seal = |condition: &str| {
        format!(
            "      - name: {SEAL_STEP}\n        if: {condition}\n        run: openssl enc \
             -aes-256-cbc -in a -out dist/sealed/a.sealed\n"
        )
    };
    let sealed_upload = |path: &str| {
        format!(
            "      - name: Upload\n        if: {when}\n        uses: \
             actions/upload-artifact@v7\n        with:\n          path: {path}\n"
        )
    };
    let sealed = "${{ inputs.dir }}/dist/sealed/${{ inputs.target }}/";
    assert_kept(&private_job(
        "workflow_call",
        &format!("{}{}", seal(when), sealed_upload(sealed)),
    ));
    for steps in [
        sealed_upload(sealed),
        format!("{}{}", sealed_upload(sealed), seal(when)),
        format!("{}{}", seal("always()"), sealed_upload(sealed)),
        format!("{}{}", seal(when), sealed_upload("dist/export/")),
        format!(
            "{}{}",
            seal(when),
            sealed_upload(&format!(
                "|\n            {sealed}\n            dist/export/*.zip"
            ))
        ),
    ] {
        assert_refused(&private_job("workflow_call", &steps), "uploads an artifact");
    }
    assert_refused(
        &private_job(
            "pull_request",
            "      - uses: actions/upload-pages-artifact@v4\n        if: success()\n",
        ),
        "uploads an artifact",
    );
    // Any other cache: kept only behind the guard.
    for step in [
        "      - uses: actions/cache@v4\n".to_owned(),
        "      - uses: actions/cache/save@v4\n        if: always()\n".to_owned(),
        format!("      - uses: actions/cache@v4\n        if: always() || {NO_PRIVATE_FILES}\n"),
        "      - uses: actions/setup-node@v4\n        with:\n          cache: npm\n".to_owned(),
    ] {
        assert_refused(&private_job("pull_request", &step), "caches, and its if:");
    }
    assert_kept(&private_job(
        "pull_request",
        &format!(
            "      - uses: actions/cache@v4\n        if: github.ref == 'x' && {NO_PRIVATE_FILES}\n"
        ),
    ));
    // A trigger that runs a fork's changes with the secrets.
    for on in [
        "pull_request_target",
        "[push, workflow_run]",
        "{ workflow_run: { workflows: [CI] } }",
    ] {
        assert_refused(&private_job(on, ""), "checks out the private files on");
    }
    // A job without the private files' step isn't held to them.
    assert_kept(
        "on: pull_request_target\njobs:\n  a:\n    steps:\n      - uses: actions/cache@v4\n",
    );
}

/// The grid game with private files.
fn private_grid_game() -> Plan {
    let mut input = private_input(Some("yes"), Some("https://github.com/o/g-assets.git"));
    input.template = Some("grid".to_owned());
    plan(&engine_checkout(), &built_in(), &input, "Moon", "moon")
        .unwrap_or_else(|(_, e)| panic!("{e}"))
        .0
}

/// The grid game with private files' workflow at `path`.
fn private_grid_workflow(path: &str) -> Workflow {
    let game = private_grid_game();
    Workflow::parse(
        "grid, private-files=yes",
        path,
        &game.files[Path::new(path)],
    )
}

/// The private files' action, from the game's root.
const PRIVATE_FILES_ACTION_FILE: &str = ".github/actions/private-files/action.yml";

/// Whether a step installs `ssh` when, and only when, it is missing.
fn makes_ssh_available(step: &serde_norway::Value) -> bool {
    let run = text_of(step.get("run"));
    run.contains("command -v ssh")
        && run.contains("apt-get install")
        && run.contains("openssh-client")
}

/// Whether the private files' action (its YAML) makes `ssh` available
/// before each step that checks out with a key, under the same `if:`.
fn the_action_makes_ssh_available(action: &serde_norway::Value) -> bool {
    let steps = action
        .get("runs")
        .and_then(|runs| runs.get("steps"))
        .and_then(serde_norway::Value::as_sequence)
        .map_or(&[][..], Vec::as_slice);
    let keyed: Vec<usize> = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| {
            text_of(step.get("uses")).starts_with("actions/checkout@")
                && step.get("with").and_then(|w| w.get("ssh-key")).is_some()
        })
        .map(|(i, _)| i)
        .collect();
    !keyed.is_empty()
        && keyed.iter().all(|&i| {
            steps[..i].iter().any(|step| {
                makes_ssh_available(step) && text_of(step.get("if")) == text_of(steps[i].get("if"))
            })
        })
}

/// Each job of `workflows` that runs in a container and checks out the
/// private files without `ssh` made available first, by a step of its own
/// before the action's or by the action itself (`action_has_it`).
fn container_jobs_without_ssh(workflows: &[Workflow], action_has_it: bool) -> Vec<String> {
    let mut missing = Vec::new();
    for workflow in workflows {
        for (name, job) in workflow.jobs() {
            if text_of(job.get("container")).trim().is_empty() || !has_the_private_files(job) {
                continue;
            }
            let steps = steps_of(job);
            let action = steps
                .iter()
                .position(|step| text_of(step.get("uses")) == PRIVATE_FILES_ACTION)
                .unwrap_or_default();
            if !action_has_it && !steps[..action].iter().any(makes_ssh_available) {
                missing.push(format!(
                    "{}: {}: the job {name}",
                    workflow.origin, workflow.path
                ));
            }
        }
    }
    missing
}

#[test]
fn every_container_job_with_the_private_files_has_ssh_for_their_checkout() {
    let game = private_grid_game();
    let origin = "grid, private-files=yes";
    let mut workflows: Vec<Workflow> = game
        .files
        .iter()
        .map(|(path, bytes)| (path.to_string_lossy().replace('\\', "/"), bytes))
        .filter(|(shown, _)| shown.starts_with(".github/workflows/"))
        .map(|(shown, bytes)| Workflow::parse(origin, &shown, bytes))
        .collect();
    for path in [".github/workflows/playtest.yml", EXPORT_WORKFLOW] {
        let bytes = fs::read(engine_checkout().join(path)).unwrap();
        workflows.push(Workflow::parse("engine", path, &bytes));
    }
    let action: serde_norway::Value =
        serde_norway::from_slice(&game.files[Path::new(PRIVATE_FILES_ACTION_FILE)]).unwrap();
    let has_it = the_action_makes_ssh_available(&action);
    let missing = container_jobs_without_ssh(&workflows, has_it);
    assert!(
        missing.is_empty(),
        "no ssh before the private files' checkout:\n{}",
        missing.join("\n")
    );

    // The rule bites: export.yml's job runs in the plan's image (the sniper
    // one for Linux) and checks the files out, and without the action's
    // step it has no ssh.
    assert_eq!(
        container_jobs_without_ssh(&workflows, false),
        [
            format!("{origin}: {EXPORT_WORKFLOW}: the job export"),
            format!("engine: {EXPORT_WORKFLOW}: the job export"),
        ]
    );
    let mut without = action.clone();
    if let Some(steps) = without
        .get_mut("runs")
        .and_then(|runs| runs.get_mut("steps"))
        .and_then(serde_norway::Value::as_sequence_mut)
    {
        steps.retain(|step| !makes_ssh_available(step));
    }
    assert!(!the_action_makes_ssh_available(&without));
}

/// The job `name` of `workflow`.
fn job_named<'a>(workflow: &'a Workflow, name: &str) -> &'a serde_norway::Value {
    workflow
        .jobs()
        .into_iter()
        .find(|(job, _)| *job == name)
        .unwrap_or_else(|| panic!("{}: no job {name}", workflow.path))
        .1
}

/// The index and the step named `name` of `job`.
fn step_named<'a>(job: &'a serde_norway::Value, name: &str) -> (usize, &'a serde_norway::Value) {
    steps_of(job)
        .iter()
        .enumerate()
        .find(|(_, step)| text_of(step.get("name")) == name)
        .unwrap_or_else(|| panic!("no step {name}"))
}

/// The `openssl enc` options of a script, one per word, without `-d`
/// (opening) and `-salt` (sealing's default, said aloud), and without the
/// files it reads and writes.
fn openssl_options(script: &str) -> Vec<&str> {
    let at = script
        .find("openssl enc")
        .unwrap_or_else(|| panic!("no openssl enc: {script}"));
    script[at + "openssl enc".len()..]
        .split_whitespace()
        .take_while(|word| *word != "-in")
        .filter(|word| !["-d", "-salt", "\\"].contains(word))
        .collect()
}

#[test]
fn a_release_publishes_nowhere_by_default_and_never_a_debug_file() {
    let workflow = private_grid_workflow(RELEASE_WORKFLOW);
    let release = job_named(&workflow, "release");

    // The debug files are moved to debug/ first; everything left goes to
    // release/, the folder the release publishes.
    let (files_at, files) = step_named(release, "The files and their checksums");
    let run = text_of(files.get("run"));
    let to_debug = run
        .find("find artifacts -type f -name '*.debug' -exec mv -n {} debug/ \\;")
        .unwrap_or_else(|| panic!("the debug files aren't moved to debug/: {run}"));
    let to_release = run
        .find("find artifacts -type f ! -name '*.json' -exec mv -n {} release/ \\;")
        .unwrap_or_else(|| panic!("the packages aren't moved to release/: {run}"));
    assert!(to_debug < to_release, "{run}");
    for (i, step) in steps_of(release).iter().enumerate() {
        let run = text_of(step.get("run"));
        if i != files_at {
            assert!(
                !run.contains("mv ") && !run.contains("cp "),
                "only the files' step moves files: {run}"
            );
        }
        assert!(!run.contains("gh release edit"), "{run}");
        assert!(!run.contains("--draft=false"), "{run}");
    }

    // The tag's release: a draft unless the developer asks for a public one
    // (`PUBLISH_ON_GITHUB`), or the repository is private.
    assert_eq!(
        text_of(release.get("env").and_then(|env| env.get("PUBLISH"))),
        "${{ vars.PUBLISH_ON_GITHUB == 'true' || github.event.repository.private }}"
    );
    let (_, publish) = step_named(release, "Release");
    let run = text_of(publish.get("run"));
    assert!(
        run.contains("if [ \"$PUBLISH\" != true ]; then\n  flags+=(--draft)\nfi"),
        "{run}"
    );
    assert!(
        run.contains(
            "gh release create \"$TAG\" --verify-tag --notes-file notes.md \"${flags[@]}\" \
             release/*\n"
        ),
        "{run}"
    );
    assert!(!run.contains("debug/"), "{run}");

    // The debug files: a draft of their own, which nothing publishes.
    let (debug_at, debug) = step_named(release, "The debug files' draft");
    let run = text_of(debug.get("run"));
    assert!(run.contains("files=(debug/*)"), "{run}");
    assert!(run.contains("draft=\"$TAG-debug\""), "{run}");
    assert!(
        run.contains("gh release create \"$draft\" --draft --target \"$GITHUB_SHA\""),
        "{run}"
    );
    assert!(debug_at > files_at);

    // Both jobs that read the packages open the sealed ones first, with
    // the key that sealed them and the same cipher.
    let export = private_grid_workflow(EXPORT_WORKFLOW);
    let (_, seal) = step_named(job_named(&export, "export"), SEAL_STEP);
    let sealing = openssl_options(text_of(seal.get("run")));
    assert_eq!(
        sealing,
        [
            "-aes-256-cbc",
            "-pbkdf2",
            "-iter",
            "100000",
            "-md",
            "sha256",
            "-pass",
            "env:PRIVATE_ASSETS_KEY"
        ]
    );
    for (job, reads) in [
        ("release", "The files and their checksums"),
        ("itch", "Push to itch.io"),
    ] {
        let job = job_named(&workflow, job);
        let (open_at, open) = step_named(job, "Open the sealed files");
        assert!(open_at < step_named(job, reads).0);
        let run = text_of(open.get("run"));
        assert!(run.contains("openssl enc -d"), "{run}");
        assert_eq!(openssl_options(run), sealing);
        assert_eq!(
            text_of(
                open.get("env")
                    .and_then(|env| env.get("PRIVATE_ASSETS_KEY"))
            ),
            "${{ secrets.PRIVATE_ASSETS_KEY }}"
        );
    }
}

#[test]
fn a_public_games_package_leaves_its_runner_sealed() {
    let workflow = private_grid_workflow(EXPORT_WORKFLOW);
    let export = job_named(&workflow, "export");
    let public = "inputs.upload-private && !github.event.repository.private && \
                  hashFiles('.github/actions/private-files/action.yml') != ''";

    // The plain upload: a game without the private files, or a private
    // repository's release.
    let (_, plain) = step_named(export, "Upload the package");
    assert_eq!(
        text_of(plain.get("if")),
        format!("{} || {NO_PRIVATE_FILES}", UPLOAD_INPUTS[0])
    );

    // A public repository's: sealed first, under the same condition, and
    // only the sealed folder uploaded.
    let (seal_at, seal) = step_named(export, SEAL_STEP);
    let (upload_at, upload) = step_named(export, "Upload the sealed package");
    assert!(seal_at < upload_at);
    assert_eq!(text_of(seal.get("if")), public);
    assert_eq!(text_of(upload.get("if")), public);
    assert!(uploads_sealed(upload, &steps_of(export)[..upload_at]));
    assert_eq!(
        text_of(upload.get("with").and_then(|w| w.get("path"))),
        "${{ inputs.dir }}/dist/sealed/${{ inputs.target }}/"
    );
    assert_eq!(
        text_of(upload.get("with").and_then(|w| w.get("name"))),
        text_of(plain.get("with").and_then(|w| w.get("name")))
    );
    // It seals each package and debug file into the folder, and copies
    // only the two reports there as they are.
    let run = text_of(seal.get("run"));
    assert!(
        run.contains("for file in \"$out\"/*.zip \"$out\"/*.tar.gz \"$out\"/*.debug; do"),
        "{run}"
    );
    assert!(
        run.contains("-out \"$sealed/$(basename \"$file\").sealed\""),
        "{run}"
    );
    assert!(
        run.contains("for report in export-report.json exposure-report.json; do"),
        "{run}"
    );
    assert_eq!(run.matches("cp ").count(), 1, "{run}");
    assert_eq!(
        text_of(
            seal.get("env")
                .and_then(|env| env.get("PRIVATE_ASSETS_KEY"))
        ),
        "${{ secrets.PRIVATE_ASSETS_KEY }}"
    );

    // A dry run of a public repository uploads none: the release passes
    // `upload-private` on a tag only, or in a private repository.
    let release = private_grid_workflow(RELEASE_WORKFLOW);
    let with = job_named(&release, "export").get("with");
    assert_eq!(
        text_of(with.and_then(|w| w.get("upload-private"))),
        "${{ github.event_name == 'push' || github.event.repository.private }}"
    );
}

#[test]
fn a_workflow_given_the_private_files_key_is_one_the_test_reads() {
    let caller = |handover: &str| {
        format!("on: push\njobs:\n  call:\n    uses: ./.github/workflows/other.yml\n{handover}")
    };
    let by_name = "    secrets:\n      PRIVATE_ASSETS_KEY: ${{ secrets.PRIVATE_ASSETS_KEY }}\n";
    let guard = format!("${{{{ {NO_PRIVATE_FILES} }}}}");
    let other = private_job("workflow_call", &rust_cache(&guard));
    let read = [(".github/workflows/other.yml", other.as_str())];
    for handover in [by_name, "    secrets: inherit\n"] {
        assert_refused(&caller(handover), "a workflow this test doesn't read");
        assert_eq!(leaks_of(&caller(handover), &read), Vec::<String>::new());
    }
    // No key, nothing to read.
    assert_kept(&caller(""));
    assert_refused(
        "on: push\njobs:\n  call:\n    \
         uses: Zafnok/shuyi-engine/.github/workflows/playtest.yml@main\n    secrets: inherit\n",
        "a workflow this test doesn't read",
    );
    // A caller on a fork's trigger hands the key over too.
    let leaks = leaks_of(
        &caller(by_name).replace("on: push", "on: pull_request_target"),
        &read,
    );
    assert!(
        leaks
            .iter()
            .any(|l| l.contains("passes PRIVATE_ASSETS_KEY on pull_request_target")),
        "{leaks:?}"
    );
    // The called workflow's own leak is found in it.
    let unguarded = private_job("workflow_call", &rust_cache("true"));
    let leaks = leaks_of(
        &caller(by_name),
        &[(".github/workflows/other.yml", &unguarded)],
    );
    assert!(
        leaks
            .iter()
            .any(|l| l.starts_with("case: .github/workflows/other.yml: the job build")),
        "{leaks:?}"
    );
}

/// Every file under `dir`, by its path from it, with its bytes.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(at) = pending.pop() {
        for entry in fs::read_dir(&at).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let shown = path
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(shown, fs::read(&path).unwrap());
            }
        }
    }
    files
}

#[test]
fn the_wizards_flags_answer_its_questions_as_set_does() {
    let args: Vec<String> = [
        "g",
        "--perspective",
        "top-down-grid",
        "--bodies",
        "movers, rigid-body",
        "--online",
        "none",
        "--targets",
        "desktop,web",
        "--id",
        "org.example.g",
        "--set",
        "examples=no",
    ]
    .iter()
    .map(|a| (*a).to_owned())
    .collect();
    let input = parse_args(&args).unwrap();
    assert_eq!(
        input.set,
        Answers::parse(
            "set",
            "{ \"perspective\": [\"top-down-grid\"], \"bodies\": [\"movers\", \"rigid-body\"], \
             \"online\": [\"none\"], \"targets\": [\"desktop\", \"web\"], \"examples\": [\"no\"] }"
        )
        .unwrap()
    );
    assert_eq!(input.id.as_deref(), Some("org.example.g"));
    // `time` has no flag: it follows from the perspectives.
    let time: Vec<String> = ["g", "--time", "turns"]
        .iter()
        .map(|a| (*a).to_owned())
        .collect();
    assert_eq!(parse_args(&time).unwrap_err(), "unknown option: --time");
    let bare: Vec<String> = ["g", "--kind"].iter().map(|a| (*a).to_owned()).collect();
    assert_eq!(parse_args(&bare).unwrap_err(), "--kind needs a value");
}

#[test]
fn no_answer_makes_the_grid_game() {
    let dir = scratch("defaults");
    let mut first = NewInput::new("first");
    first.name = Some("Moon".to_owned());
    let made = new(&dir, &first);
    assert!(made.ok, "{made:?}");
    let answers = made.result.unwrap().answers;
    assert_eq!(answers.0["perspective"], ["top-down-grid"]);
    assert_eq!(answers.0["targets"], ["desktop", "web"]);
    // The same as the grid's short name.
    let mut grid = NewInput::new("grid").template("grid");
    grid.name = Some("Moon".to_owned());
    assert!(new(&dir, &grid).ok);
    assert_eq!(tree(&dir.join("first")), tree(&dir.join("grid")));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_id_goes_into_the_project_and_app_info_and_a_bad_one_is_refused() {
    let dir = scratch("id");
    let mut input = NewInput::new("g").template("blank");
    input.id = Some("org.example.demo".to_owned());
    let made = new(&dir, &input);
    assert!(made.ok, "{made:?}");
    assert_eq!(made.result.unwrap().id, "org.example.demo");
    let project = crate::project::Project::load(&dir.join("g")).unwrap();
    let app = project.config.app.as_ref().unwrap();
    assert_eq!(app.id, "org.example.demo");
    // The game's app_info.rs is the one app-info writes.
    assert_eq!(
        crate::app_info::stale(app, &dir.join("g"), project.config.command()),
        Vec::<String>::new()
    );
    input.id = Some("9.example".to_owned());
    input.force = true;
    let refused = new(&dir, &input);
    assert_eq!(refused.code.as_deref(), Some(codes::WRONG_ARGUMENTS.0));
    assert_eq!(
        refused.error.unwrap(),
        "--id: \"9.example\": each part between dots starts with a letter, and \"9\" doesn't"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_parked_answer_or_a_planned_target_by_flag_is_refused_with_its_status() {
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let template = built_in();
    let mut parked = NewInput::new("g");
    parked.set = Answers::parse("set", "{ \"bodies\": [\"procedural\"] }").unwrap();
    let (code, error) = plan(cwd, &template, &parked, "G", "g").unwrap_err();
    assert_eq!(code, codes::WRONG_ARGUMENTS);
    assert_eq!(
        error,
        "`procedural` (procedural animation), an answer of `bodies`, is parked: tickets 2701, \
         2702, 2703 and 2704 build it, so no game picks it yet"
    );
    let mut planned = NewInput::new("g");
    planned.set = Answers::parse("set", "{ \"targets\": [\"switch\"] }").unwrap();
    let (_, error) = plan(cwd, &template, &planned, "G", "g").unwrap_err();
    assert!(
        error.starts_with("the target `switch` is planned"),
        "{error}"
    );
    planned.set = Answers::parse("set", "{ \"targets\": [\"moon\"] }").unwrap();
    let (_, error) = plan(cwd, &template, &planned, "G", "g").unwrap_err();
    assert!(
        error.starts_with("no target `moon`: the engine's are "),
        "{error}"
    );
    // A ready target the manifest doesn't list yet is a limit (2504).
    let mut more = NewInput::new("g");
    more.set = Answers::parse("set", "{ \"targets\": [\"desktop\", \"playdate\"] }").unwrap();
    let (_, limits) = plan(cwd, &template, &more, "G", "g").unwrap();
    assert!(
        limits.contains(
            &"shuyi.ron lists the template's targets (desktop, web), not the ones picked \
              (desktop, playdate): 2504 makes it list the picked ones"
                .to_owned()
        ),
        "{limits:?}"
    );
}
