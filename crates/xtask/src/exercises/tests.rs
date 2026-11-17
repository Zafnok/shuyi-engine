use super::*;

/// A step's page with a playground of `files` and `body`.
fn page(files: &str, body: &str) -> String {
    format!("+++\ntitle = \"A step\"\n\n[extra]\nstep = 2\nfiles = [{files}]\n+++\n\n{body}")
}

/// An exercise asking for the check `name`, in `levels/1.map`.
fn exercise(name: &str) -> String {
    format!(
        "{{% exercise(check=\"{name}\", file=\"levels/1.map\") %}}\nDo it.\n\nHint: like so.\n{{% end %}}\n\n"
    )
}

const CHECKS: &str = r#"
pub const CHECKS: &[PageCheck<UiAction, Own>] =
    &[("second-box", second_box), ("blue-walls", blue_walls)];

fn second_box(data: &Data) -> Result<String, String> { Ok("(\"not a name\",".into()) }
"#;

#[test]
fn the_options_are_copy_and_steps() {
    let args = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        Options::parse(&args(&["--copy", "02-board"])),
        Some(Options {
            copy: true,
            steps: vec!["02-board".to_owned()]
        })
    );
    assert_eq!(Options::parse(&args(&[])), Some(Options::default()));
    assert_eq!(Options::parse(&args(&["--check"])), None);
}

#[test]
fn the_names_are_read_from_the_checks_table_only() {
    assert_eq!(names_in(CHECKS), ["second-box", "blue-walls"]);
    assert_eq!(names_in("fn main() {}"), Vec::<String>::new());
}

#[test]
fn a_page_and_its_checks_that_agree_pass() {
    let body = exercise("second-box") + &exercise("blue-walls");
    let text = page(r#""levels/1.map""#, &body);
    assert_eq!(
        page_problems("02-board", &text, Some(CHECKS)),
        Vec::<String>::new()
    );
}

#[test]
fn a_check_on_one_side_only_fails_naming_it() {
    let text = page(
        r#""levels/1.map""#,
        &(exercise("second-box") + &exercise("red-walls")),
    );
    let errors = page_problems("02-board", &text, Some(CHECKS));
    assert_eq!(errors.len(), 2, "{errors:#?}");
    assert!(
        errors[0].contains("`red-walls`, which site/exercises/02-board.rs"),
        "{errors:#?}"
    );
    assert!(
        errors[1].contains("the check `blue-walls` is asked for by no exercise"),
        "{errors:#?}"
    );
}

#[test]
fn a_step_with_a_playground_needs_its_checks_file() {
    let text = page(r#""levels/1.map""#, &exercise("second-box"));
    let errors = page_problems("02-board", &text, None);
    assert!(
        errors[0].ends_with("in site/exercises/02-board.rs, which isn't there"),
        "{errors:#?}"
    );
    let no_playground = "+++\ntitle = \"A step\"\n[extra]\nstep = 2\n+++\n\nText.\n";
    assert_eq!(
        page_problems("02-board", no_playground, None),
        Vec::<String>::new()
    );
}

/// Makes at `dir` a stand-in step: a workspace with the game's package at
/// `crates/game`, a build script, and a binary that prints `name`.
fn stand_in(dir: &Path, name: &str) {
    let game = dir.join("crates/game");
    fs::create_dir_all(game.join("src")).unwrap();
    let write = |file: &str, text: &str| fs::write(dir.join(file), text).unwrap();
    write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/game\"]\nresolver = \"3\"\n",
    );
    write(
        "crates/game/Cargo.toml",
        &format!("[package]\nname = \"{BIN}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    );
    write("crates/game/build.rs", "fn main() {}\n");
    write(
        "crates/game/src/main.rs",
        &format!("fn main() {{\n    print!(\"{name}\");\n}}\n"),
    );
}

/// Sets the time of every file under `dir` to `time`.
fn set_times(dir: &Path, time: std::time::SystemTime) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            set_times(&path, time);
        } else {
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(time)
                .unwrap();
        }
    }
}

/// What the game of the copy at `dir` prints, built in `build` with `config`.
fn printed(dir: &Path, build: &Path, config: &str) -> String {
    let output = Command::new("cargo")
        .args(["run", "-q", "--config", config])
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", build)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn two_steps_whose_files_share_their_times_each_build_their_own_game() {
    let root = std::env::temp_dir().join(format!("xtask-exercises-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let (first, second) = (root.join("copies/08-a"), root.join("copies/09-b"));
    stand_in(&first, "08");
    stand_in(&second, "09");
    // Both steps written by one script, within one tick of the clock: the
    // same time on every file, the newest too.
    let time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    set_times(&first, time);
    set_times(&second, time);
    let build = root.join("build");
    assert_eq!(printed(&first, &build, &own_build(7)), "08");
    assert_eq!(printed(&second, &build, &own_build(8)), "09");
    // And back, as the next run checks them again.
    assert_eq!(printed(&first, &build, &own_build(7)), "08");
    let _ = fs::remove_dir_all(&root);
}
