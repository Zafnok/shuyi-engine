//! The engine's skill pack (`skills/`, ticket 1134): it passes
//! `check-skills` against this `shuyi` and the open tickets, the binary
//! embeds it as it is, and `check-skills` runs as a command with no
//! project.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use shuyi_cli::contract::Op;
use shuyi_cli::skills::{self, Against, Pack};
use shuyi_cli::{COMMANDS, usage};

use crate::scratch;

/// The repository's root: `crates/cli/../..`.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_skill_pack_passes_check_skills() {
    let pack = Pack::read(&root().join(skills::PACK)).unwrap();
    assert!(!pack.skills.is_empty(), "no skill in skills/");
    let against = Against::this_shuyi(Some(&root().join("tickets/open"))).unwrap();
    let problems: Vec<String> = skills::check(&pack, &against)
        .iter()
        .map(|p| format!("skills/{p}"))
        .collect();
    assert!(
        problems.is_empty(),
        "fix each skill (`cargo run -p shuyi-cli -- check-skills skills --tickets \
         tickets/open` lists them):\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_binary_embeds_the_pack_as_it_is() {
    // Each file of a pack by `<skill>/<path>`, its text with LF line
    // endings (a checkout on Windows may have CRLF).
    let files = |pack: &Pack| -> BTreeMap<String, String> {
        pack.skills
            .iter()
            .flat_map(|s| {
                s.files.iter().map(|(path, bytes)| {
                    let text = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
                    (format!("{}/{path}", s.name), text)
                })
            })
            .collect()
    };
    let checkout = Pack::read(&root().join(skills::PACK)).unwrap();
    assert_eq!(files(&Pack::embedded()), files(&checkout));
    assert_eq!(checkout.not_skills, Vec::<String>::new());
}

#[test]
fn every_command_a_skill_may_name_is_in_the_usage() {
    let text = usage("shuyi");
    for name in Op::ALL.iter().map(|op| op.name()).chain(COMMANDS) {
        let listed = text.lines().any(|l| {
            l.strip_prefix("  ")
                .is_some_and(|l| l.split_whitespace().next() == Some(name))
        });
        assert!(listed, "`{name}` isn't in the usage:\n{text}");
    }
}

/// `shuyi check-skills <args>` run in `dir`: its exit code, stdout and
/// stderr.
fn check_skills(dir: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_shuyi"))
        .arg("check-skills")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn shuyi: {e}"));
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn check_skills_names_each_problem_and_needs_no_project() {
    let dir = scratch("check-skills");
    let skill = dir.join("skills/shuyi-x");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: shuyi-x\ndescription: Use when testing. Not for games.\nmetadata:\n  \
         version: \"1.0\"\n  waits_for: \"1140\"\n---\n\nRun `shuyi soak`.\n",
    )
    .unwrap();
    let (code, out, err) = check_skills(&dir, &[]);
    assert_eq!(code, Some(1), "{out}{err}");
    assert_eq!(
        err.lines().next(),
        Some("skills/shuyi-x/SKILL.md:9: `shuyi soak`: no such command (`shuyi` alone lists them)")
    );
    assert!(
        err.ends_with("check-skills: 1 problem(s) in skills\n"),
        "{err}"
    );
    assert!(
        out.contains("the ids in `waits_for` weren't checked"),
        "{out}"
    );
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: shuyi-x\ndescription: Use when testing. Not for games.\nmetadata:\n  \
         version: \"1.0\"\n  waits_for: \"1140\"\n---\n\nRun `shuyi run`.\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("open")).unwrap();
    fs::write(dir.join("open/1140-net.md"), "").unwrap();
    let (code, out, err) = check_skills(&dir, &["skills/", "--tickets", "open"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(out, "check-skills: 1 skill(s) in skills: OK\n");
    let (code, _, err) = check_skills(&dir, &["nope"]);
    assert_eq!(code, Some(2));
    assert!(
        err.starts_with("check-skills: no folder nope here\nusage: shuyi check-skills"),
        "{err}"
    );
    let (code, _, err) = check_skills(&dir, &["--fast"]);
    assert_eq!(
        (code, err.lines().next()),
        (Some(2), Some("check-skills: unknown option --fast"))
    );
    fs::remove_dir_all(&dir).unwrap();
}
