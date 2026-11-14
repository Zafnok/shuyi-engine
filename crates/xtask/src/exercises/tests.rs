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
