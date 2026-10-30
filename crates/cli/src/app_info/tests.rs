//! The `app` part's rules, the Rust `app-info` writes, and `--check`.

use std::path::PathBuf;

use super::*;

/// The part a new game starts with.
fn moon() -> AppPart {
    AppPart::new("Moon Garden", "io.github.someone.moon-garden")
}

/// The settings of `part`'s problems, each with its level.
fn found(part: &AppPart) -> Vec<(Level, String)> {
    fields(part)
        .into_iter()
        .map(|p| (p.level, p.setting))
        .collect()
}

fn error(setting: &str) -> Vec<(Level, String)> {
    vec![(Level::Error, setting.to_owned())]
}

/// A folder of its own under the system's temporary folder, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-app-info-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_good_part_has_no_problem() {
    assert_eq!(found(&moon()), []);
    let mut full = moon();
    full.version = Some("1.2.0-beta.1+42".to_owned());
    full.build = 3;
    full.storage = Some("Moon Garden_2.x".to_owned());
    assert_eq!(found(&full), []);
}

#[test]
fn the_name_is_not_empty_and_holds_no_control_character() {
    for name in ["", "  ", "Moon\nGarden", "Moon\tGarden"] {
        let mut part = moon();
        part.name = name.to_owned();
        assert_eq!(found(&part), error("app.name"), "{name:?}");
    }
    let mut part = moon();
    part.name = "Lune Café \"2\"".to_owned();
    assert_eq!(found(&part), []);
}

#[test]
fn the_id_is_reverse_dns_of_letters_digits_and_dashes() {
    for good in [
        "a.b",
        "com.example.moon-garden",
        "io.github.a-b.c2",
        "A.B-9",
    ] {
        assert_eq!(check_id(good), Ok(()), "{good}");
    }
    for bad in [
        "",
        "moon",
        "com.",
        ".com.a",
        "com..a",
        "com.2moon",
        "com.-moon",
        "com.moon_garden",
        "com.moon garden",
        "com.café",
    ] {
        assert!(check_id(bad).is_err(), "{bad}");
        let mut part = moon();
        part.id = bad.to_owned();
        let settings: Vec<String> = fields(&part).into_iter().map(|p| p.setting).collect();
        assert!(
            settings.contains(&"app.id".to_owned()),
            "{bad}: {settings:?}"
        );
    }
}

#[test]
fn every_crate_name_new_makes_passes_under_the_placeholder() {
    for name in ["Snake", "Moon Garden", "Lune Café", "a 2 b"] {
        let krate = crate::contract::new::crate_name(name).unwrap();
        assert_eq!(
            check_id(&format!("{PLACEHOLDER}{krate}")),
            Ok(()),
            "{krate}"
        );
    }
}

#[test]
fn a_placeholder_id_is_a_warning() {
    let part = AppPart::new("Snake", "com.example.snake");
    let problems = fields(&part);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(
        (problems[0].level, problems[0].setting.as_str()),
        (Level::Warning, "app.id")
    );
    assert!(
        problems[0].message.contains("share their saves"),
        "{problems:?}"
    );
}

#[test]
fn the_version_is_semver() {
    for good in [
        "0.1.0",
        "1.2.3",
        "1.0.0-alpha",
        "1.0.0-rc.1+build.5",
        "10.20.30",
    ] {
        assert_eq!(check_version(good), Ok(()), "{good}");
    }
    for bad in [
        "", "1", "1.2", "1.2.3.4", "v1.2.3", "01.2.3", "1.2.x", "1.2.3-",
    ] {
        assert!(check_version(bad).is_err(), "{bad}");
        let mut part = moon();
        part.version = Some(bad.to_owned());
        assert_eq!(found(&part), error("app.version"), "{bad}");
    }
}

#[test]
fn the_build_starts_at_one() {
    let mut part = moon();
    part.build = 0;
    assert_eq!(found(&part), error("app.build"));
}

#[test]
fn the_storage_names_one_folder() {
    for bad in [
        "", "a/b", "a\\b", "a:b", ".hidden", "trail.", " lead", "trail ", "café",
    ] {
        assert!(check_storage(bad).is_err(), "{bad:?}");
        let mut part = moon();
        part.storage = Some(bad.to_owned());
        assert_eq!(found(&part), error("app.storage"), "{bad:?}");
    }
    for good in [
        "moon_garden",
        "Moon Garden",
        "shuyi-hello-grid",
        "com.example.a",
    ] {
        assert_eq!(check_storage(good), Ok(()), "{good}");
    }
}

#[test]
fn the_rust_file_holds_the_defaults_alone() {
    let mut part = AppPart::new("Moon Garden", "com.example.moon-garden");
    part.code = Some("crates/game/src/app_info.rs".to_owned());
    insta::assert_snapshot!("rust_defaults", rust(&part, "cargo shuyi"));
}

#[test]
fn the_rust_file_sets_each_field_off_its_default() {
    let mut part = AppPart::new("Hello, Grid", "io.github.someone.hello-grid");
    part.version = Some("1.2.0-beta.1".to_owned());
    part.storage = Some("shuyi-hello-grid".to_owned());
    part.build = 12;
    part.publisher = "Someone \"S\" \\ Co".to_owned();
    insta::assert_snapshot!("rust_every_field", rust(&part, "shuyi"));
}

#[test]
fn the_rust_file_keeps_a_name_beyond_ascii_as_it_is() {
    let part = AppPart::new("Lune Café", "com.example.lune-caf");
    let text = rust(&part, "shuyi");
    assert!(
        text.contains("AppInfo::new(\"Lune Café\", \"com.example.lune-caf\", "),
        "{text}"
    );
    assert_eq!(quoted("a\u{7}b"), "\"a\\u{7}b\"");
    assert!(
        text.starts_with("// Generated by `shuyi app-info`"),
        "{text}"
    );
    assert!(
        text.lines().next().unwrap().ends_with("Do not edit."),
        "{text}"
    );
}

#[test]
fn app_info_writes_the_file_and_check_finds_it_stale() {
    let root = scratch("run");
    let mut part = moon();
    part.code = Some("crates/game/src/app_info.rs".to_owned());
    let path = root.join("crates/game/src/app_info.rs");
    // Missing: stale, and out of date in the problems.
    assert_eq!(
        stale(&part, &root, "shuyi"),
        ["crates/game/src/app_info.rs"]
    );
    let stale_found = problems(&part, &root, "shuyi");
    assert_eq!(stale_found.len(), 1, "{stale_found:?}");
    assert_eq!(stale_found[0].setting, "app.code");
    assert_eq!(
        stale_found[0].message,
        "crates/game/src/app_info.rs is out of date: run `shuyi app-info`"
    );
    let error = run(&part, &root, "shuyi", true).unwrap_err();
    assert!(error.contains("crates/game/src/app_info.rs"), "{error}");
    // Written, then the same bytes again.
    assert_eq!(
        run(&part, &root, "shuyi", false),
        Ok("app-info: wrote crates/game/src/app_info.rs".to_owned())
    );
    let first = fs::read(&path).unwrap();
    run(&part, &root, "shuyi", false).unwrap();
    assert_eq!(fs::read(&path).unwrap(), first);
    assert_eq!(first, rust(&part, "shuyi").into_bytes());
    assert_eq!(
        run(&part, &root, "shuyi", true),
        Ok("app-info: up to date".to_owned())
    );
    let none = problems(&part, &root, "shuyi");
    assert!(none.is_empty(), "{none:?}");
    // Line endings don't count.
    let crlf = String::from_utf8(first).unwrap().replace('\n', "\r\n");
    fs::write(&path, crlf).unwrap();
    let none = stale(&part, &root, "shuyi");
    assert!(none.is_empty(), "{none:?}");
    // A change to the part makes it stale.
    part.build = 2;
    assert_eq!(
        stale(&part, &root, "shuyi"),
        ["crates/game/src/app_info.rs"]
    );
    assert!(run(&part, &root, "shuyi", true).is_err());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn app_info_writes_nothing_without_code_or_with_an_error() {
    let root = scratch("none");
    assert_eq!(
        run(&moon(), &root, "shuyi", false),
        Ok("app-info: shuyi.ron's `app` names no `code`: no file to write".to_owned())
    );
    let none = stale(&moon(), &root, "shuyi");
    assert!(none.is_empty(), "{none:?}");
    let mut bad = moon();
    bad.id = "moon".to_owned();
    bad.code = Some("app_info.rs".to_owned());
    let error = run(&bad, &root, "shuyi", false).unwrap_err();
    assert!(error.contains("app.id: "), "{error}");
    assert!(!root.join("app_info.rs").exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn app_info_takes_check_or_nothing() {
    assert_eq!(parse_args(&[]), Ok(false));
    assert_eq!(parse_args(&["--check".to_owned()]), Ok(true));
    assert!(parse_args(&["--write".to_owned()]).is_err());
}
