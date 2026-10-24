use super::*;

/// A `SKILL.md` named `name`, with `description`, metadata lines (each
/// `key: value`, indented here) and `body`.
fn skill_md(name: &str, description: &str, metadata: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: {description}\nlicense: MIT OR Apache-2.0\nmetadata:\n{metadata}---\n\n{body}"
    )
}

const GOOD: &str = "Start a game. Use when making one. Not for an engine change.";

/// A skill named `name` with `files` (path, text).
fn skill(name: &str, files: &[(&str, &str)]) -> Skill {
    Skill {
        name: name.to_owned(),
        files: files
            .iter()
            .map(|(p, t)| ((*p).to_owned(), t.as_bytes().to_vec()))
            .collect(),
    }
}

/// A skill named `name` that passes, with `body`, and its example.
fn good(name: &str, body: &str) -> Skill {
    let md = skill_md(name, GOOD, "  version: \"1.0\"\n", body);
    skill(
        name,
        &[
            (FILE, &md),
            (EXAMPLE, "Example(template: \"grid\", steps: [])"),
        ],
    )
}

fn pack(skills: Vec<Skill>) -> Pack {
    Pack {
        skills,
        not_skills: Vec::new(),
    }
}

/// What `check` finds in `pack`, as lines, against this `shuyi`'s
/// commands and the open tickets `open`.
fn found(pack: &Pack, open: Option<&[&str]>) -> Vec<String> {
    let mut against = Against::this_shuyi(None).unwrap();
    against.open_tickets = open.map(|ids| ids.iter().map(|id| (*id).to_owned()).collect());
    check(pack, &against)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn frontmatter_reads_the_open_formats_keys() {
    let text = skill_md(
        "shuyi-gates",
        GOOD,
        "  version: \"1.2\"\n  waits_for: \"1119, 1120 1122\"\n",
        "# Body\n",
    );
    let (frontmatter, body) = Frontmatter::parse(&text).unwrap();
    assert_eq!(frontmatter.name, "shuyi-gates");
    assert_eq!(frontmatter.license.as_deref(), Some("MIT OR Apache-2.0"));
    assert_eq!(frontmatter.metadata["version"], "1.2");
    assert_eq!(frontmatter.waits_for(), ["1119", "1120", "1122"]);
    assert_eq!(body, 9);
    assert_eq!(text.lines().nth(body - 1), Some(""));
    assert_eq!(Frontmatter::default().waits_for(), Vec::<String>::new());
}

#[test]
fn frontmatter_that_doesnt_read_says_why() {
    let no_start = Frontmatter::parse("name: x\n---\n").unwrap_err();
    assert!(no_start.contains("starts with no `---`"), "{no_start}");
    let no_end = Frontmatter::parse("---\nname: x\ndescription: y\n").unwrap_err();
    assert!(no_end.contains("no closing `---`"), "{no_end}");
    let unknown =
        Frontmatter::parse("---\nname: x\ndescription: y\nversion: 1\n---\n").unwrap_err();
    assert!(unknown.contains("unknown field `version`"), "{unknown}");
    let nested = Frontmatter::parse(
        "---\nname: x\ndescription: y\nmetadata:\n  version:\n    major: 1\n---\n",
    )
    .unwrap_err();
    assert!(nested.contains("each metadata value a string"), "{nested}");
    // A file that ends on its closing line reads too.
    assert!(Frontmatter::parse("---\nname: x\ndescription: y\n---").is_ok());
}

#[test]
fn a_good_pack_has_no_problem() {
    let body = "# Gates\n\nRun `cargo shuyi lint` and `shuyi check --json`, then the MCP \
                `start` tool. See [the other](../shuyi-test/SKILL.md#steps), \
                [a page](references/more.md) and [the web](https://example.com). Fix \
                `SH2103` and SH0001.\n\n```bash\ncargo shuyi run --script \"Enter\" --frames 2\n\
                cargo run -p shuyi-cli -- new demo\n```\n";
    let mut gates = good("shuyi-gates", body);
    gates.files.insert(
        "references/more.md".to_owned(),
        b"More: `shuyi explain SH1002`.\n".to_vec(),
    );
    let pack = pack(vec![gates, good("shuyi-test", "# Test\n")]);
    assert_eq!(found(&pack, Some(&[])), Vec::<String>::new());
}

#[test]
fn the_frontmatter_needs_its_name_when_and_when_not_and_a_version() {
    let md = skill_md("other", "Starts a game.", "  author: me\n", "");
    let pack = pack(vec![skill("shuyi-new", &[(FILE, &md), (EXAMPLE, "")])]);
    assert_eq!(
        found(&pack, None),
        [
            "shuyi-new/SKILL.md:1: name: `other` is not its folder's name, `shuyi-new`",
            "shuyi-new/SKILL.md:1: description: say when to use it, in a sentence starting `Use …`",
            "shuyi-new/SKILL.md:1: description: say when not to use it, in a sentence starting \
             `Not …`",
            "shuyi-new/SKILL.md:1: metadata.version: give the skill a version, a string of \
             numbers and dots (\"1.0\")",
        ]
    );
    let md = skill_md("Bad--Name", GOOD, "  version: \"v1\"\n", "");
    let pack = self::pack(vec![skill("Bad--Name", &[(FILE, &md), (EXAMPLE, "")])]);
    let problems = found(&pack, None);
    assert!(
        problems[0].ends_with(
            "name: `Bad--Name`: lowercase letters and digits in words joined by single `-`"
        ),
        "{problems:?}"
    );
    assert!(problems[1].contains("metadata.version"), "{problems:?}");
    let broken = skill("shuyi-x", &[(FILE, "no frontmatter"), (EXAMPLE, "")]);
    assert_eq!(
        found(&self::pack(vec![broken]), None),
        ["shuyi-x/SKILL.md:1: the file starts with no `---` line before its frontmatter"]
    );
}

#[test]
fn a_command_that_shuyi_lacks_is_named_with_its_line() {
    let body = "Read a kind's shape with `shuyi kinds palette --json`.\n\n```bash\ncargo shuyi soak\n\
                cargo shuyi frame-png a.png\n```\n\nThe shuyi way, in prose: shuyi soak.\n\
                `shuyi::prelude` and `shuyi-net` and `cargo shuyi <op>` are no commands.\n";
    let pack = pack(vec![good("shuyi-test", body)]);
    assert_eq!(
        found(&pack, None),
        [
            "shuyi-test/SKILL.md:9: `shuyi kinds`: no such command (`shuyi` alone lists them)",
            "shuyi-test/SKILL.md:12: `shuyi soak`: no such command (`shuyi` alone lists them)",
        ]
    );
}

#[test]
fn a_link_must_name_a_file_of_the_pack_and_never_main() {
    let body = "[gone](references/gone.md), [out](../../docs/contract.md), \
                [root](/AGENTS.md), [skill](../shuyi-gone/SKILL.md), [folder](references/), \
                [main](https://github.com/Zafnok/shuyi-engine/blob/main/AGENTS.md) and \
                <https://github.com/Zafnok/shuyi-engine/tree/main/docs>; the repository, \
                https://github.com/Zafnok/shuyi-engine, is fine. `[code](nope.md)` too.\n";
    let mut skill = good("shuyi-test", body);
    skill.files.insert(
        "references/a.md".to_owned(),
        b"[up](../SKILL.md) [me](a.md)\n".to_vec(),
    );
    let problems = found(&pack(vec![skill]), None);
    let main = "a fixed page on the engine's `main`: send the agent to the project's \
                `AGENTS.md`, which names the engine's docs at the version the game builds with";
    assert_eq!(
        problems,
        [
            "shuyi-test/SKILL.md:9: references/gone.md: no such file in the skill pack".to_owned(),
            "shuyi-test/SKILL.md:9: ../../docs/contract.md: the link leaves the skill pack, which \
             is all a game gets: link a file of the pack, or name the file in words"
                .to_owned(),
            "shuyi-test/SKILL.md:9: /AGENTS.md: a link from the root: give it from the file's \
             folder"
                .to_owned(),
            "shuyi-test/SKILL.md:9: ../shuyi-gone/SKILL.md: no such file in the skill pack"
                .to_owned(),
            format!(
                "shuyi-test/SKILL.md:9: https://github.com/Zafnok/shuyi-engine/blob/main/AGENTS.md: {main}"
            ),
            format!(
                "shuyi-test/SKILL.md:9: https://github.com/Zafnok/shuyi-engine/tree/main/docs: {main}"
            ),
        ]
    );
}

#[test]
fn a_skill_too_long_says_so() {
    let body = "line\n".repeat(MAX_LINES);
    let long = "x".repeat(MAX_DESCRIPTION);
    let md = skill_md(
        "shuyi-long",
        &format!("{GOOD} {long}"),
        "  version: \"1\"\n",
        &body,
    );
    let pack = pack(vec![skill("shuyi-long", &[(FILE, &md), (EXAMPLE, "")])]);
    let problems = found(&pack, None);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems[0].contains("description: 1085 characters, over 1024"),
        "{problems:?}"
    );
    assert!(
        problems[1].starts_with("shuyi-long/SKILL.md: 508 lines, over 500: move the detail"),
        "{problems:?}"
    );
}

#[test]
fn a_skill_that_waits_names_open_tickets_and_one_that_doesnt_has_an_example() {
    let waits = skill_md(
        "shuyi-net",
        GOOD,
        "  version: \"1.0\"\n  waits_for: \"1140 2529\"\n",
        "",
    );
    let none = skill_md("shuyi-test", GOOD, "  version: \"1.0\"\n", "");
    let pack = pack(vec![
        skill("shuyi-net", &[(FILE, &waits)]),
        skill("shuyi-test", &[(FILE, &none)]),
    ]);
    assert_eq!(
        found(&pack, Some(&["1140"])),
        [
            "shuyi-net/SKILL.md:1: waits_for: 2529 is no open ticket: if it is done, take its id \
             out, and write the skill's worked example once nothing is left",
            "shuyi-test/SKILL.md: no worked example (example.ron): a skill that waits for nothing \
             has one",
        ]
    );
    // Without the tickets, only the example is checked.
    assert_eq!(found(&pack, None).len(), 1);
}

#[test]
fn a_problem_code_must_be_in_the_table() {
    let mut skill = good(
        "shuyi-test",
        "Branch on `SH2103`, never on SH9999 or SH12345.\n",
    );
    skill.files.insert(
        EXAMPLE.to_owned(),
        b"Example(steps: [Shuyi(args: [\"lint\"], expect: [Is(\"/code\", \"SH0999\")])])".to_vec(),
    );
    assert_eq!(
        found(&pack(vec![skill]), None),
        [
            "shuyi-test/SKILL.md:9: SH9999 is no problem code: `shuyi explain` lists them",
            "shuyi-test/example.ron:1: SH0999 is no problem code: `shuyi explain` lists them",
        ]
    );
}

#[test]
fn a_folder_without_a_skill_is_named() {
    let pack = Pack {
        skills: Vec::new(),
        not_skills: vec!["notes".to_owned()],
    };
    assert_eq!(
        found(&pack, None),
        ["notes/: a folder of the pack with no SKILL.md: every skill has one"]
    );
}

#[test]
fn a_pack_reads_its_folders_and_leaves_files_beside_them() {
    let dir = std::env::temp_dir().join(format!("shuyi-skills-read-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("shuyi-b/references")).unwrap();
    fs::create_dir_all(dir.join("shuyi-a")).unwrap();
    fs::create_dir_all(dir.join("notes")).unwrap();
    fs::write(dir.join("README.md"), "# The pack\n").unwrap();
    fs::write(dir.join("shuyi-b/SKILL.md"), "b").unwrap();
    fs::write(dir.join("shuyi-b/references/more.md"), "more").unwrap();
    fs::write(dir.join("shuyi-a/SKILL.md"), "a").unwrap();
    fs::write(dir.join("notes/todo.txt"), "x").unwrap();
    let pack = Pack::read(&dir).unwrap();
    let names: Vec<&str> = pack.skills.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["shuyi-a", "shuyi-b"]);
    assert_eq!(pack.not_skills, ["notes"]);
    let b = pack.skill("shuyi-b").unwrap();
    assert_eq!(
        b.files.keys().collect::<Vec<_>>(),
        ["SKILL.md", "references/more.md"]
    );
    assert!(!b.has_example());
    assert_eq!(Pack::read(&dir.join("nope")), Ok(Pack::default()));
    let _ = fs::remove_dir_all(&dir);
}
