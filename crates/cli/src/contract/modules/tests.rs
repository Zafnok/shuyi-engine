//! `modules`: what it lists for people, the features no module has, and
//! a game it can't read.

use std::path::PathBuf;

use super::*;
use crate::catalog::{Answered, Choice, Concerns, ModuleCrate, Status};
use crate::project::Config;

/// An empty folder of its own for test `name`.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-modules-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A catalog of one concern, answered by the default engine and by
/// `shuyi-alpha`, which it builds.
fn catalog() -> Catalog {
    let choice = |id: &str, by: &str, status| Choice {
        id: id.into(),
        name: id.into(),
        by: by.into(),
        status,
        tickets: vec![],
    };
    Catalog {
        root: PathBuf::from("/e"),
        concerns: Concerns {
            concerns: vec![Concern {
                id: "bodies".into(),
                name: "Bodies".into(),
                alternatives: true,
                answers: vec![
                    choice("movers", "default", Status::Parked),
                    choice("rigid", "shuyi-alpha", Status::Ready),
                ],
            }],
        },
        modules: vec![ModuleCrate {
            name: "shuyi-alpha".into(),
            dir: PathBuf::from("/e/crates/alpha"),
            info: ModuleInfo {
                answers: vec![Answered {
                    concern: "bodies".into(),
                    answer: "rigid".into(),
                }],
                serves: vec!["side-view".into()],
                ..ModuleInfo::default()
            },
        }],
        skills: crate::skills::Pack::default(),
    }
}

#[test]
fn it_lists_each_answer_and_each_module_on_or_off() {
    let dir = folder("list");
    fs::write(
        dir.join("Cargo.toml"),
        "[dependencies]\nshuyi = { path = \"/e/crates/shuyi\", features = [\"alpha\", \"ghost\"] }\n",
    )
    .unwrap();
    let answer = modules_in(&dir, &catalog(), &ModulesInput::default());
    assert!(answer.ok, "{answer:?}");
    let output = answer.result.as_ref().unwrap();
    assert_eq!(output.on, ["shuyi-alpha", "shuyi-ghost"]);
    assert!(output.modules[0].on);
    assert_eq!(output.modules[0].feature, "alpha");
    assert!(
        answer.limits.contains(
            &"the game turns on the feature `ghost` of `shuyi`, which no module of its engine \
              has"
            .to_owned()
        ),
        "{:?}",
        answer.limits
    );
    assert_eq!(
        text(output),
        "modules: the catalog of /e\nBodies\n  movers: default, parked\n  rigid: shuyi-alpha, \
         ready (on)\nshuyi-alpha (on): feature `alpha`\n"
    );
    let mut none = output.clone();
    none.modules.clear();
    assert!(text(&none).ends_with("the engine builds no module yet\n"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_game_it_cant_read_fails_with_its_code() {
    let dir = folder("unread");
    let answer = modules_in(&dir, &catalog(), &ModulesInput::default());
    assert_eq!(answer.code.as_deref(), Some("SH3004"));
    fs::write(dir.join("Cargo.toml"), "[dependencies]\nserde = \"1\"\n").unwrap();
    let answer = modules_in(&dir, &catalog(), &ModulesInput::default());
    assert_eq!(answer.code.as_deref(), Some("SH1004"));
    let project = Project {
        root: dir.clone(),
        config: Config::default(),
    };
    let answer = modules(&project, &ModulesInput::default());
    assert_eq!(answer.code.as_deref(), Some("SH1004"));
    assert_eq!(answer.op, Op::Modules);
    assert!(usage("shuyi").starts_with("usage: shuyi modules [--json]"));
    fs::remove_dir_all(&dir).unwrap();
}
