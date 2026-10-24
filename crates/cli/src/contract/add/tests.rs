//! `add`: its arguments, what it says for people, and what stops it
//! (the integration tests run it on a game made by `new`).

use super::*;
use crate::catalog::{Choice, Concern, Concerns, ModuleCrate, ModuleInfo, Status};
use crate::project::Config;

/// A catalog that plans `shuyi-gamma`, in two answers, and builds
/// `shuyi-alpha`, whose package folder is `dir`.
fn catalog(dir: &Path) -> Catalog {
    let choice = |id: &str, tickets: &[&str]| Choice {
        id: id.into(),
        name: id.into(),
        by: "shuyi-gamma".into(),
        status: Status::Parked,
        tickets: tickets.iter().map(|t| (*t).to_owned()).collect(),
    };
    Catalog {
        root: dir.to_path_buf(),
        concerns: Concerns {
            concerns: vec![Concern {
                id: "bodies".into(),
                name: "Bodies".into(),
                alternatives: true,
                answers: vec![choice("soft", &["1"]), choice("cloth", &["2", "3"])],
            }],
        },
        modules: vec![ModuleCrate {
            name: "shuyi-alpha".into(),
            dir: dir.to_path_buf(),
            info: ModuleInfo::default(),
        }],
        skills: crate::skills::Pack::default(),
    }
}

#[test]
fn its_argument_is_one_module() {
    assert_eq!(
        parse_args(&["alpha".to_owned()]),
        Ok(AddInput::new("alpha"))
    );
    for wrong in [
        &[][..],
        &["a".to_owned(), "b".to_owned()],
        &["--x".to_owned()],
    ] {
        assert_eq!(
            parse_args(wrong),
            Err("one <module>, nothing else".to_owned())
        );
    }
    assert!(usage("shuyi").starts_with("usage: shuyi add <module> [--json]"));
}

#[test]
fn a_name_it_doesnt_build_is_planned_or_unknown() {
    let catalog = catalog(Path::new("/e"));
    assert_eq!(
        unknown(&catalog, "gamma"),
        "shuyi-gamma is in the catalog but not built yet: soft is parked (1); cloth is parked \
         (2, 3)"
    );
    assert_eq!(
        unknown(&catalog, "shuyi-delta"),
        "no module shuyi-delta: the game's engine builds shuyi-alpha"
    );
    let mut empty = catalog.clone();
    empty.modules.clear();
    assert_eq!(
        unknown(&empty, "delta"),
        "no module delta: the game's engine builds no module yet"
    );
}

#[test]
fn what_stops_it_has_its_code() {
    let dir = std::env::temp_dir().join(format!("shuyi-add-stops-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let catalog = catalog(&dir);
    // Its README isn't there.
    let answer = add_in(&dir, &catalog, &AddInput::new("alpha"));
    assert_eq!(answer.code.as_deref(), Some("SH3004"));
    fs::write(dir.join("README.md"), "# shuyi-alpha\n\nA module.\n").unwrap();
    // The game's Cargo.toml isn't there, then names no `shuyi`.
    let answer = add_in(&dir, &catalog, &AddInput::new("alpha"));
    assert_eq!(answer.code.as_deref(), Some("SH3004"));
    fs::write(dir.join("Cargo.toml"), "[dependencies]\n").unwrap();
    let answer = add_in(&dir, &catalog, &AddInput::new("alpha"));
    assert_eq!(answer.code.as_deref(), Some("SH1004"));
    assert_eq!(answer.op, Op::Add);
    // An engine that can't be found.
    let project = Project {
        root: dir.clone(),
        config: Config::default(),
    };
    let answer = add(&project, &AddInput::new("alpha"));
    assert_eq!(answer.code.as_deref(), Some("SH1004"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn what_it_did_is_said_for_people() {
    let mut output = AddOutput {
        module: "shuyi-alpha".into(),
        feature: "alpha".into(),
        changed: true,
        skill: Some(".claude/skills/shuyi-alpha".into()),
        about: "Rigid bodies,\nfor side view.".into(),
        alternatives_on: vec![],
        beside: None,
        install: Some("add the line".into()),
        formats: Some("call it".into()),
    };
    assert_eq!(
        text(&output),
        "add: shuyi-alpha on (the feature `alpha` of shuyi in Cargo.toml)\n  Rigid bodies,\n  \
         for side view.\nskill: copied to .claude/skills/shuyi-alpha\nto do: add the line\nto \
         do: call it\n"
    );
    output.changed = false;
    output.skill = None;
    output.install = None;
    output.formats = None;
    output.alternatives_on = vec!["shuyi-beta".into()];
    output.beside = Some(format!("/e/README.md#{BESIDE}"));
    assert_eq!(
        text(&output),
        "add: shuyi-alpha was already on: nothing changed\n  Rigid bodies,\n  for side view.\n\
         warning: the game already has shuyi-beta on, an alternative: two alternatives never \
         drive the same thing in one stage; see /e/README.md#beside-its-alternatives\n"
    );
}
