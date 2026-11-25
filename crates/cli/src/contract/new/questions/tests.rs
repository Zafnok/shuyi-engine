//! The questions answered by a script ([`prompts::scripted`]), the flags,
//! and whole runs of `new` with `repos` against the scripted machine and
//! GitHub of [`crate::repos::fake`]. None reaches the network.

use std::fs;
use std::path::PathBuf;

use super::prompts::scripted::{Scripted, picked, typed};
use super::*;
use crate::repos::fake::Fake;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn flags(items: &[&str]) -> Flags {
    parse(&args(items)).unwrap().1
}

#[test]
fn each_flag_parses_and_the_rest_go_to_new() {
    let (input, flags) = parse(&args(&[
        "g",
        "--github",
        "--owner",
        "studio",
        "--visibility",
        "private",
        "--assets-visibility",
        "public",
        "--private",
        "sprites,1",
        "--private-folder",
        "files/levels",
        "--private-folder",
        "maps/",
        "--name",
        "--github",
    ]))
    .unwrap();
    assert_eq!(input.dir, "g");
    // A value of new's own is never a flag.
    assert_eq!(input.name.as_deref(), Some("--github"));
    assert_eq!(
        flags,
        Flags {
            examples: None,
            github: Some(true),
            owner: Some("studio".to_owned()),
            visibility: Some(Visibility::Private),
            assets_visibility: Some(Visibility::Public),
            private: Some(vec!["text".to_owned(), "sprites".to_owned()]),
            folders: Some(vec!["levels".to_owned(), "maps".to_owned()]),
            no_input: false,
            questions: false,
        }
    );
    assert!(self::flags(&["g", "--no-input"]).no_input);
    assert!(self::flags(&["g", "--questions"]).questions);
    assert_eq!(flags_of_none(), Flags::default());
    assert_eq!(self::flags(&["g", "--no-github"]).github, Some(false));
    assert_eq!(self::flags(&["g", "--examples"]).examples, Some(true));
    assert_eq!(self::flags(&["g", "--no-examples"]).examples, Some(false));
    assert_eq!(
        self::flags(&["g", "--private", "none"]).private,
        Some(vec![])
    );
}

fn flags_of_none() -> Flags {
    flags(&["g"])
}

#[test]
fn each_flag_refuses_what_it_cant_take() {
    let refused = |items: &[&str]| parse(&args(items)).unwrap_err();
    assert_eq!(refused(&["g", "--owner"]), "--owner needs a value");
    assert!(refused(&["g", "--owner", "-bad-"]).contains("isn't a GitHub login"));
    assert_eq!(
        refused(&["g", "--visibility", "secret"]),
        "--visibility is public or private, not `secret`"
    );
    assert!(refused(&["g", "--assets-visibility", "x"]).starts_with("--assets-visibility"));
    assert!(
        refused(&["g", "--private", "art"]).starts_with("--private: `art` is no kind"),
        "{}",
        refused(&["g", "--private", "art"])
    );
    assert_eq!(
        refused(&["g", "--private", "none,text"]),
        "--private: `none` goes alone"
    );
    assert!(refused(&["g", "--private", "6"]).contains("is no kind"));
    assert!(refused(&["g", "--private-folder", "a b"]).starts_with("--private-folder:"));
    assert!(refused(&["g", "--private-folder", "files"]).contains("is always there"));
    assert_eq!(
        refused(&["g", "--private-folder", "x,x"]),
        "--private-folder: `x` twice"
    );
    assert_eq!(refused(&["g", "--bogus"]), "unknown option: --bogus");
}

#[test]
fn kinds_are_names_or_numbers_in_their_order_and_folders_drop_files() {
    assert_eq!(kinds("music text").unwrap(), ["text", "music"]);
    assert_eq!(kinds("5, files/sounds/").unwrap(), ["sounds", "data"]);
    assert_eq!(kinds("none").unwrap(), Vec::<String>::new());
    assert!(kinds("").is_err());
    assert_eq!(
        own_folders("files/levels, docs, library").unwrap(),
        ["levels"]
    );
    assert_eq!(own_folders("").unwrap(), Vec::<String>::new());
}

#[test]
fn the_private_url_is_the_games_name_with_assets() {
    assert_eq!(
        private_url("you", "moon-garden"),
        "https://github.com/you/moon-garden-assets.git"
    );
}

#[test]
fn the_yes_or_no_questions_are_lists_at_their_default() {
    let question = github_question("Moon Garden", true, true);
    assert_eq!(
        question.text,
        "Put Moon Garden on GitHub? It needs the GitHub CLI (gh), signed in."
    );
    assert_eq!(question.takes, Takes::One);
    assert_eq!(question.flag.as_deref(), Some("--github"));
    assert_eq!(question.agent.as_deref(), Some(GITHUB_AGENT));
    let mut script = Scripted::enters(1);
    assert!(yes(&mut script, &question).unwrap());
    assert_eq!(
        script.lists[0].choices,
        [
            ("yes".to_owned(), "yes".to_owned()),
            ("no".to_owned(), "no".to_owned())
        ]
    );
    // Without gh the default is no, and the question says why.
    let question = github_question("Moon Garden", false, false);
    assert!(
        question.text.contains("gh isn't installed"),
        "{}",
        question.text
    );
    assert!(!yes(&mut Scripted::enters(1), &question).unwrap());
    let mut script = Scripted::new([picked(&["no"])]);
    assert!(!yes(&mut script, &github_question("G", true, true)).unwrap());
    let question = keep_question("you/g-assets", true);
    assert_eq!(
        question.text,
        "Keep some files in a second, private repository, you/g-assets, so they aren't in the \
         public code?"
    );
    assert!(yes(&mut Scripted::enters(1), &question).unwrap());
    assert!(!yes(&mut Scripted::new([picked(&["no"])]), &question).unwrap());
}

#[test]
fn the_visibility_questions_default_to_public_then_private() {
    let question = visibility_question("you/g", Visibility::Public);
    assert_eq!(
        question.text,
        "Should you/g be public or private? GitHub's CI is free on a public repository."
    );
    assert_eq!(
        visibility(&mut Scripted::enters(1), &question).unwrap(),
        Visibility::Public
    );
    assert_eq!(
        visibility(&mut Scripted::new([picked(&["private"])]), &question).unwrap(),
        Visibility::Private
    );
    let question = assets_visibility_question("you/g-assets", Visibility::Private);
    assert!(
        question
            .text
            .starts_with("Should you/g-assets be private or public?")
    );
    assert_eq!(
        visibility(&mut Scripted::enters(1), &question).unwrap(),
        Visibility::Private
    );
    assert_eq!(
        visibility(&mut Scripted::new([picked(&["public"])]), &question).unwrap(),
        Visibility::Public
    );
}

#[test]
fn the_kinds_question_ticks_every_kind_and_asks_again_for_none() {
    let mut script = Scripted::enters(1);
    assert_eq!(which_kinds(&mut script).unwrap(), DEFAULT_FOLDERS);
    assert_eq!(
        script.lists[0].choices[0],
        (
            "text".to_owned(),
            "files/text/: writing: dialogue, scripts…".to_owned()
        )
    );
    let mut script = Scripted::new([picked(&[]), picked(&["music", "sprites"])]);
    assert_eq!(which_kinds(&mut script).unwrap(), ["music", "sprites"]);
    assert!(
        script
            .shown
            .iter()
            .any(|line| line.starts_with("Tick one kind at least")),
        "{:?}",
        script.shown
    );
    // A kind that isn't one is no choice.
    let mut script = Scripted::new([picked(&["art"])]);
    assert!(
        which_kinds(&mut script)
            .unwrap_err()
            .to_string()
            .contains("`art` is no choice")
    );
}

#[test]
fn the_folders_question_takes_a_comma_list_or_nothing() {
    let mut script = Scripted::enters(1);
    assert_eq!(other_folders(&mut script).unwrap(), Vec::<String>::new());
    assert!(script.shown[0].starts_with("Any other folders under files/ to keep there?"));
    let mut script = Scripted::new([typed("a b"), typed("files/levels, maps")]);
    assert_eq!(other_folders(&mut script).unwrap(), ["levels", "maps"]);
    assert!(
        script
            .shown
            .iter()
            .any(|line| line.starts_with("  --private-folder: `a b` isn't a folder")),
        "{:?}",
        script.shown
    );
}

#[test]
fn no_answer_left_is_an_error_not_a_default() {
    let answer = yes(&mut Scripted::default(), &github_question("G", true, true));
    assert_eq!(answer.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
}

#[test]
fn the_github_questions_are_listed_after_the_wizards_with_the_agents_field() {
    let scratch = Scratch::new("listed");
    let (input, flags) = parse(&args(&["moon-garden", "--name", "Moon Garden"])).unwrap();
    let answer = listed(&scratch.dir, input, &flags);
    assert!(answer.ok, "{answer:?}");
    let questions = answer.result.unwrap().questions;
    let ids: Vec<&str> = questions.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(
        ids[ids.len() - 7..],
        [
            "targets",
            GITHUB,
            VISIBILITY,
            KEEP_PRIVATE,
            PRIVATE,
            PRIVATE_FOLDER,
            ASSETS_VISIBILITY
        ]
    );
    let github = questions.iter().find(|q| q.id == GITHUB).unwrap();
    assert_eq!(github.default, ["no"]);
    assert!(github.skipped.is_none());
    assert!(
        github
            .agent
            .as_deref()
            .unwrap()
            .contains("never answer yes yourself")
    );
    // Only `github` says what an agent must do, and the rest wait for it.
    assert_eq!(questions.iter().filter(|q| q.agent.is_some()).count(), 1);
    for question in &questions[questions.len() - 5..] {
        assert_eq!(
            question.skipped.as_deref(),
            Some("asked only when the game goes on GitHub (--github)"),
            "{}",
            question.id
        );
    }
    // With --github they are asked, at the flags' answers; with --private
    // none the private repository's aren't.
    let (input, flags) = parse(&args(&[
        "moon-garden",
        "--github",
        "--owner",
        "studio",
        "--visibility",
        "private",
        "--private",
        "none",
    ]))
    .unwrap();
    let questions = listed(&scratch.dir, input, &flags)
        .result
        .unwrap()
        .questions;
    let find = |id: &str| questions.iter().find(|q| q.id == id).unwrap();
    assert_eq!(find(GITHUB).default, ["yes"]);
    assert_eq!(find(VISIBILITY).default, ["private"]);
    assert!(
        find(VISIBILITY)
            .text
            .starts_with("Should studio/moon-garden be")
    );
    assert!(find(VISIBILITY).skipped.is_none());
    assert_eq!(find(KEEP_PRIVATE).default, ["no"]);
    assert_eq!(
        find(PRIVATE).skipped.as_deref(),
        Some("no files are kept private (--private none)")
    );
    assert!(!scratch.game().exists());
}

/// The flags that answer the wizard's questions but the name's (which
/// [`Scratch::run`] gives) and the examples'.
const WIZARD: [&str; 8] = [
    "--id",
    "com.example.moon-garden",
    "--perspective",
    "top-down-grid",
    "--scripting",
    "none",
    "--targets",
    "desktop,web",
];

/// A working folder with the game's folder in it, and the fake's
/// temporary folder.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("shuyi-new-questions-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("temp")).unwrap();
        Self { dir }
    }

    fn game(&self) -> PathBuf {
        self.dir.join("moon-garden")
    }

    fn fake(&self) -> Fake {
        Fake::new(&self.game(), &self.dir.join("temp"))
    }

    /// `new moon-garden` with `items`, answered by `script` as on a
    /// terminal, if any: what it made, and what the prompts showed.
    fn run(
        &self,
        items: &[&str],
        script: Option<Scripted>,
        fake: &mut Fake,
    ) -> (Made, Vec<String>) {
        let mut all = vec!["moon-garden", "--name", "Moon Garden"];
        all.extend_from_slice(items);
        let (input, flags) = parse(&args(&all)).unwrap();
        let mut script = script;
        let prompter = script.as_mut().map(|s| s as &mut dyn Prompter);
        let made = new_with(&self.dir, input, &flags, false, prompter, fake);
        (made, script.map_or_else(Vec::new, |s| s.shown))
    }

    fn shuyi_ron(&self) -> String {
        fs::read_to_string(self.game().join("shuyi.ron")).unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// The lines of `fake`'s calls that change something on GitHub.
fn github_changes(fake: &Fake) -> Vec<String> {
    fake.changes()
        .into_iter()
        .filter(|line| line.starts_with("gh "))
        .map(|line| match line.find(" --description") {
            Some(at) => line[..at].to_owned(),
            None => line,
        })
        .filter(|line| !line.contains("deploy-key") && !line.contains("secret set"))
        .collect()
}

#[test]
fn every_default_taken_makes_the_game_and_both_repositories() {
    let scratch = Scratch::new("defaults");
    let mut fake = scratch.fake();
    let (made, shown) = scratch.run(&[], Some(Scripted::enters(11)), &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    let report = made.repos.as_ref().unwrap();
    assert!(report.ok, "{report:?}");
    assert_eq!(made.exit_code(), 0);
    assert!(made.finish.is_none());
    // The wizard's questions (the name is given), then the GitHub ones, in
    // order; the concerns with nothing to pick between say why on a line.
    let asked: Vec<&str> = shown
        .iter()
        .map(String::as_str)
        .filter(|line| !line.contains("(not asked: "))
        .collect();
    let starts = [
        "What is its reverse-DNS id",
        "Which perspectives?",
        "How are the rules written?",
        "Include the engine skills' worked examples",
        "Which targets does it ship on?",
        "Put Moon Garden on GitHub?",
        "Should you/moon-garden be public or private?",
        "Keep some files in a second, private repository, you/moon-garden-assets",
        "Which kinds of files go there?",
        "Any other folders",
        "Should you/moon-garden-assets be private or public?",
    ];
    assert_eq!(asked.len(), starts.len(), "{shown:#?}");
    for (asked, start) in asked.iter().zip(starts) {
        assert!(asked.starts_with(start), "{asked}");
    }
    assert!(
        shown.iter().any(|line| line
            == "How do bodies move? none (not asked: its answers are for the perspectives \
                top-down, isometric, side-view, first-person, third-person, top-down-3d, and the \
                game's are top-down-grid)"),
        "{shown:#?}"
    );
    // Enter took the examples: the game has them, as before.
    assert!(
        scratch
            .game()
            .join(".claude/skills/shuyi-balance-bots/example/bots.rs")
            .is_file()
    );
    assert!(
        scratch
            .shuyi_ron()
            .contains("https://github.com/you/moon-garden-assets.git"),
        "{}",
        scratch.shuyi_ron()
    );
    assert_eq!(
        github_changes(&fake),
        [
            "gh repo create you/moon-garden --public --source . --remote origin",
            "gh repo create you/moon-garden-assets --private --source . --remote origin --push",
            "gh api repos/you/moon-garden-assets/rulesets -X POST --input -",
        ]
    );
    let files: Vec<&String> = fake.repos["you/moon-garden-assets"].files.keys().collect();
    assert_eq!(
        files,
        [
            ".gitattributes",
            ".gitignore",
            "README.md",
            "docs/.gitkeep",
            "files/data/.gitkeep",
            "files/music/.gitkeep",
            "files/sounds/.gitkeep",
            "files/sprites/.gitkeep",
            "files/text/.gitkeep",
            "library/.gitkeep",
        ]
    );
    assert!(!fake.repos["you/moon-garden"].private);
    assert!(fake.repos["you/moon-garden-assets"].private);
}

#[test]
fn no_github_asks_nothing_and_makes_only_the_folder() {
    let scratch = Scratch::new("no-github");
    let mut fake = scratch.fake();
    let mut items = WIZARD.to_vec();
    items.extend(["--no-github", "--examples"]);
    let (made, shown) = scratch.run(&items, Some(Scripted::default()), &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(made.repos.is_none());
    // Every question answered by a flag: only the lines of the concerns
    // with nothing to pick between.
    assert!(
        shown.iter().all(|line| line.contains("(not asked: ")),
        "{shown:#?}"
    );
    assert_eq!(fake.calls.len(), 0);
    assert!(!scratch.shuyi_ron().contains("private_assets"));
}

#[test]
fn off_a_terminal_it_asks_nothing_and_makes_no_repository() {
    let scratch = Scratch::new("no-terminal");
    let mut fake = scratch.fake();
    let (made, _) = scratch.run(&[], None, &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(made.repos.is_none());
    assert!(fake.calls.is_empty(), "{:?}", fake.calls);
    // Nor with --json, even on a terminal; and --github is refused there.
    let (input, flags) = parse(&args(&["json-game"])).unwrap();
    let mut script = Scripted::enters(20);
    let made = new_with(
        &scratch.dir,
        input,
        &flags,
        true,
        Some(&mut script),
        &mut fake,
    );
    assert!(made.answer.ok);
    assert!(made.repos.is_none());
    assert!(script.shown.is_empty(), "{:?}", script.shown);
    assert_eq!(script.steps.len(), 20);
    assert_eq!(fake.calls.len(), 0);
    let (input, flags) = parse(&args(&["json-game-2", "--github"])).unwrap();
    let made = new_with(&scratch.dir, input, &flags, true, None, &mut fake);
    assert!(!made.answer.ok);
    assert!(
        made.answer
            .error
            .as_deref()
            .unwrap()
            .starts_with("--github isn't for --json"),
        "{:?}",
        made.answer
    );
    assert_eq!(fake.calls.len(), 0);
}

#[test]
fn private_none_makes_only_the_public_repository() {
    let scratch = Scratch::new("private-none");
    let mut fake = scratch.fake();
    let (made, _) = scratch.run(&["--github", "--private", "none"], None, &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(made.repos.as_ref().unwrap().ok, "{:?}", made.repos);
    assert!(!scratch.shuyi_ron().contains("private_assets"));
    assert_eq!(
        github_changes(&fake),
        ["gh repo create you/moon-garden --public --source . --remote origin"]
    );
}

#[test]
fn chosen_kinds_and_own_folders_are_all_the_private_repository_holds() {
    let scratch = Scratch::new("kinds");
    let mut fake = scratch.fake();
    let mut items = WIZARD.to_vec();
    items.extend([
        "--examples",
        "--github",
        "--private",
        "sprites",
        "--private-folder",
        "files/levels",
    ]);
    let (made, shown) = scratch.run(&items, Some(Scripted::enters(2)), &mut fake);
    assert!(made.repos.as_ref().unwrap().ok, "{:?}", made.repos);
    // Only what no flag answered was asked: the two visibilities.
    let asked: Vec<&String> = shown
        .iter()
        .filter(|line| !line.contains("(not asked: "))
        .collect();
    assert_eq!(asked.len(), 2, "{shown:#?}");
    assert!(asked[0].starts_with("Should you/moon-garden be public"));
    assert!(asked[1].starts_with("Should you/moon-garden-assets be private or public?"));
    let folders: Vec<&String> = fake.repos["you/moon-garden-assets"]
        .files
        .keys()
        .filter(|f| f.ends_with(".gitkeep"))
        .collect();
    assert_eq!(
        folders,
        [
            "docs/.gitkeep",
            "files/levels/.gitkeep",
            "files/sprites/.gitkeep",
            "library/.gitkeep",
        ]
    );
}

#[test]
fn owner_and_visibilities_go_to_repos() {
    let scratch = Scratch::new("owner");
    let mut fake = scratch.fake();
    fake.login = "someone".to_owned();
    let (made, _) = scratch.run(
        &[
            "--github",
            "--owner",
            "you",
            "--visibility",
            "private",
            "--assets-visibility",
            "public",
        ],
        None,
        &mut fake,
    );
    assert!(made.repos.as_ref().unwrap().ok, "{:?}", made.repos);
    assert!(fake.repos["you/moon-garden"].private);
    assert!(!fake.repos["you/moon-garden-assets"].private);
    // The owner was given: only `repos`' own check asked gh who you are.
    let logins = fake
        .calls
        .iter()
        .filter(|c| c.line() == "gh api user --jq .login")
        .count();
    assert_eq!(logins, 1);
}

#[test]
fn a_repos_failure_leaves_the_game_and_names_cargo_shuyi_repos() {
    let scratch = Scratch::new("repos-fails");
    let mut fake = scratch.fake();
    fake.fail = Some("gh repo create you/moon-garden-assets".to_owned());
    let (made, _) = scratch.run(&["--github", "--private", "text,data"], None, &mut fake);
    assert!(made.answer.ok);
    let report = made.repos.as_ref().unwrap();
    assert!(!report.ok);
    assert_ne!(made.exit_code(), 0);
    assert!(scratch.game().join("shuyi.ron").is_file());
    let finish = made.finish.as_deref().unwrap();
    assert!(
        finish.contains(
            "run `cargo shuyi repos --owner you --private-folders text,data` in moon-garden"
        ),
        "{finish}"
    );
}

#[test]
fn signed_out_gh_stops_before_the_game_is_written() {
    let scratch = Scratch::new("signed-out");
    let mut fake = scratch.fake();
    fake.signed_in = false;
    fake.fail = Some("gh api user".to_owned());
    let (made, _) = scratch.run(&["--github"], None, &mut fake);
    assert!(!made.answer.ok);
    assert_eq!(made.answer.code.as_deref(), Some(codes::GITHUB.0));
    assert!(
        made.answer
            .error
            .as_deref()
            .unwrap()
            .contains("gh auth login"),
        "{:?}",
        made.answer
    );
    assert!(!scratch.game().exists());
}

#[test]
fn repository_flags_without_github_are_refused_and_private_kinds_need_the_url() {
    let scratch = Scratch::new("refused");
    let mut fake = scratch.fake();
    let (made, _) = scratch.run(&["--owner", "you"], None, &mut fake);
    assert_eq!(
        made.answer.error.as_deref(),
        Some("--owner is for --github")
    );
    let (made, _) = scratch.run(
        &["--github", "--private", "none", "--private-folder", "x"],
        None,
        &mut fake,
    );
    assert!(
        made.answer
            .error
            .as_deref()
            .unwrap()
            .starts_with("--private none makes no private"),
        "{:?}",
        made.answer
    );
    assert_eq!(fake.calls.len(), 0);
    // Without --github, --private composes the part only with --private-url.
    let (made, _) = scratch.run(&["--private", "text"], None, &mut fake);
    assert!(made.answer.ok);
    assert!(!scratch.shuyi_ron().contains("private_assets"));
    let (made, _) = scratch.run(
        &[
            "--force",
            "--private",
            "text",
            "--private-url",
            "https://github.com/o/m-assets.git",
        ],
        None,
        &mut fake,
    );
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(
        scratch
            .shuyi_ron()
            .contains("https://github.com/o/m-assets.git")
    );
    assert_eq!(fake.calls.len(), 0);
}

#[test]
fn a_game_with_private_files_has_their_skill_and_its_agents_md_names_it() {
    let scratch = Scratch::new("private-skill");
    let mut fake = scratch.fake();
    let skill = scratch
        .game()
        .join(".claude/skills/shuyi-private-files/SKILL.md");
    let agents = || fs::read_to_string(scratch.game().join("AGENTS.md")).unwrap();
    let (made, _) = scratch.run(
        &[
            "--template",
            "blank",
            "--set",
            "private-files=yes",
            "--private-url",
            "https://github.com/o/g-assets.git",
        ],
        None,
        &mut fake,
    );
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(skill.is_file());
    assert!(
        agents().contains("`shuyi-private-files` skill"),
        "{}",
        agents()
    );
    // Every game has the skill; only the part's text names it.
    let (made, _) = scratch.run(&["--force", "--template", "blank"], None, &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(skill.is_file());
    assert!(!agents().contains("shuyi-private-files"), "{}", agents());
    assert_eq!(fake.calls.len(), 0);
}

#[test]
fn the_examples_question_is_the_templates_yes_by_default() {
    let scratch = Scratch::new("examples-question");
    let (input, flags) = parse(&args(&["moon-garden"])).unwrap();
    let questions = listed(&scratch.dir, input, &flags)
        .result
        .unwrap()
        .questions;
    let examples = questions.iter().find(|q| q.id == EXAMPLES).unwrap();
    assert_eq!(examples.asker, super::super::wizard::Asker::Template);
    assert!(
        examples
            .text
            .starts_with("Include the engine skills' worked examples"),
        "{}",
        examples.text
    );
    assert_eq!(examples.default, ["yes"]);
    assert!(yes(&mut Scripted::enters(1), examples).unwrap());
    assert!(!yes(&mut Scripted::new([picked(&["no"])]), examples).unwrap());
}

/// Whether the game in `scratch` has any skill's worked example.
fn has_examples(scratch: &Scratch) -> bool {
    let skills = scratch.game().join(".claude/skills");
    let pack = crate::skills::Pack::read(&skills).unwrap();
    assert!(pack.skills.len() > 1, "{:?}", pack.not_skills);
    pack.skills.iter().any(|skill| {
        skill
            .files
            .keys()
            .any(|path| crate::skills::is_example(path))
    })
}

#[test]
fn a_no_to_the_examples_question_leaves_them_out_and_a_flag_or_set_asks_nothing() {
    let scratch = Scratch::new("examples-no");
    let mut fake = scratch.fake();
    let mut items = WIZARD.to_vec();
    items.push("--no-github");
    let (made, shown) = scratch.run(&items, Some(Scripted::new([picked(&["no"])])), &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(
        shown
            .iter()
            .any(|line| line.starts_with("Include the engine skills' worked examples")),
        "{shown:#?}"
    );
    assert!(!has_examples(&scratch));
    let result = made.answer.result.as_ref().unwrap();
    assert_eq!(result.answers.0[EXAMPLES], ["no"]);
    drop(scratch);
    // A flag or --set answers it: nothing is asked.
    for (extra, has) in [
        (&["--no-examples"][..], false),
        (&["--examples"][..], true),
        (&["--set", "examples=no"][..], false),
        (&["--set", "examples=no", "--no-examples"][..], false),
    ] {
        let scratch = Scratch::new("examples-flag");
        let mut fake = scratch.fake();
        let mut items = WIZARD.to_vec();
        items.push("--no-github");
        items.extend_from_slice(extra);
        let (made, shown) = scratch.run(&items, Some(Scripted::default()), &mut fake);
        assert!(made.answer.ok, "{items:?}: {:?}", made.answer);
        assert!(
            shown.iter().all(|line| line.contains("(not asked: ")),
            "{items:?}: {shown:#?}"
        );
        assert_eq!(has_examples(&scratch), has, "{items:?}");
    }
    // Off a terminal (and so with --json and through the MCP tool), the
    // template's default: the examples are there.
    let scratch = Scratch::new("examples-off");
    let mut fake = scratch.fake();
    let (made, _) = scratch.run(&[], None, &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(has_examples(&scratch));
    // A flag that disagrees with --set is refused.
    let (made, _) = scratch.run(&["--examples", "--set", "examples=no"], None, &mut fake);
    assert_eq!(
        made.answer.error.as_deref(),
        Some("--examples and --set examples=… disagree")
    );
}
