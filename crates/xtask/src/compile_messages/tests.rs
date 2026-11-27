use super::*;

/// cargo's stderr for two fixtures: one error with a note in `screen`
/// (a Windows path), one warning and one error in `game_data` (a `/`
/// path), and the lines of cargo's own with no file.
const STDERR: &str = "\
    Checking compile-messages v0.0.0 (D:\\shuyi\\target\\compile-messages)
error[E0277]: `Title` is not a screen of `Data`
  --> src\\bin\\screen.rs:12:44
   |
12 |     let screens: Vec<Box<dyn Screen<Data>>> = vec![Box::new(Title)];
   |                                               ^^^^^^^^^^^^^^^ write `impl Screen<Data> for Title`
   |
   = note: a screen writes `name`, `update` and `draw`
error: could not compile `compile-messages` (bin \"screen\") due to 1 previous error
warning: unused variable: `x`
 --> src/bin/game_data.rs:3:9
  |
3 |     let x = 1;
  |         ^ help: if this is intentional, prefix it with an underscore: `_x`
error[E0277]: `Mine` is not the data of a game
 --> src/bin/game_data.rs:9:5
  |
note: required by a bound in `Ctx`
 --> D:\\shuyi\\crates\\ui\\src\\ctx.rs:400:16
warning: build failed, waiting for other jobs to finish...
";

fn texts(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|&t| t.to_owned()).collect()
}

#[test]
fn a_fixtures_texts_are_its_first_marked_lines() {
    let source = "//~ `Title` is not a screen\n//~   write `impl Screen`  \n\
                  use shuyi::prelude::*;\n//~ not this one, past the top\n";
    assert_eq!(
        expectations(source),
        texts(&["`Title` is not a screen", "write `impl Screen`"])
    );
    assert!(expectations("fn main() {}\n//~ late\n").is_empty());
}

#[test]
fn diagnostics_go_to_the_fixture_of_their_first_file() {
    let by_fixture = split(STDERR);
    assert_eq!(
        by_fixture.keys().collect::<Vec<_>>(),
        ["game_data", "screen"]
    );
    let screen = &by_fixture["screen"];
    assert_eq!(screen.len(), 1, "{screen:?}");
    assert!(screen[0].starts_with("error[E0277]: `Title`"), "{screen:?}");
    assert!(screen[0].contains("= note: a screen writes"), "{screen:?}");
    // The `/` path, its warning first, and its error's later ` --> ` line
    // into the engine doesn't move it.
    let data = &by_fixture["game_data"];
    assert_eq!(data.len(), 2, "{data:?}");
    assert!(data[0].starts_with("warning: unused variable"), "{data:?}");
    assert!(data[1].contains("required by a bound in `Ctx`"), "{data:?}");
}

#[test]
fn a_diagnostic_with_no_file_goes_to_no_fixture() {
    let by_fixture = split(STDERR);
    let all = by_fixture.values().flatten().cloned().collect::<String>();
    assert!(!all.contains("could not compile"), "{all}");
    assert!(!all.contains("build failed"), "{all}");
    // One in another file goes to none either.
    let engine = "warning: unused import\n --> D:\\shuyi\\crates\\ui\\src\\lib.rs:1:5\n";
    assert!(split(engine).is_empty());
}

#[test]
fn a_fixture_with_no_diagnostic_compiled() {
    let found = verdict("screen", &texts(&["is not a screen"]), &[]);
    assert!(found.is_some_and(|e| e.contains("compiled without an error")));
}

#[test]
fn a_fixture_with_only_a_warning_compiled() {
    let warning = "warning: unused variable: `x`\n --> src/bin/screen.rs:3:9\n".to_owned();
    let found = verdict("screen", &texts(&["unused variable"]), &[warning]);
    assert!(found.is_some_and(|e| e.contains("compiled without an error")));
}

#[test]
fn a_fixture_whose_output_lacks_a_text_fails_naming_it() {
    let by_fixture = split(STDERR);
    let found = verdict(
        "screen",
        &texts(&["is not a screen of `Data`", "the book's chapter"]),
        &by_fixture["screen"],
    )
    .unwrap_or_default();
    assert!(found.contains("lacks\n  \"the book's chapter\""), "{found}");
    assert!(!found.contains("  \"is not a screen"), "{found}");
    assert!(found.contains("its output:\nerror[E0277]"), "{found}");
}

#[test]
fn a_fixture_with_all_its_texts_passes() {
    let by_fixture = split(STDERR);
    // The texts may come from any of its diagnostics, the warning too.
    let expected = texts(&[
        "is not the data of a game",
        "unused variable",
        "required by a bound in `Ctx`",
    ]);
    assert_eq!(
        verdict("game_data", &expected, &by_fixture["game_data"]),
        None
    );
}

#[test]
fn a_fixture_with_no_text_fails() {
    let by_fixture = split(STDERR);
    let found = verdict("screen", &[], &by_fixture["screen"]);
    assert!(found.is_some_and(|e| e.contains("no `//~ <text>` line")));
}

#[test]
fn the_package_is_its_own_workspace_on_the_engines_crates() {
    let text = manifest(Path::new("D:\\shuyi"));
    assert!(text.contains("\n[workspace]\n"), "{text}");
    assert!(text.contains("edition = \"2024\""), "{text}");
    assert!(text.contains("serde = { version = \"1\", features = [\"derive\"] }"));
    for line in [
        "shuyi = { path = \"D:/shuyi/crates/shuyi\" }",
        "shuyi-core = { path = \"D:/shuyi/crates/core\" }",
        "shuyi-cli = { path = \"D:/shuyi/crates/cli\" }",
    ] {
        assert!(text.contains(line), "{line}\n{text}");
    }
    let parsed: toml::Table = toml::from_str(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert_eq!(
        parsed["dependencies"].as_table().map(toml::Table::len),
        Some(7)
    );
}
