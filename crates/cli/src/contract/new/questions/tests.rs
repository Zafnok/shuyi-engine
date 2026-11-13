//! The questions with scripted lines, the flags, and whole runs of `new`
//! with `repos` against the scripted machine and GitHub of
//! [`crate::repos::fake`]. None reaches the network.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::repos::fake::Fake;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn flags(items: &[&str]) -> Flags {
    parse(&args(items)).unwrap().1
}

/// Runs `question` on the lines `typed`: its answer and what it wrote.
fn answered<T>(
    typed: &str,
    question: impl FnOnce(&mut Terminal<'_>) -> io::Result<T>,
) -> (io::Result<T>, String) {
    let mut input = typed.as_bytes();
    let mut out = Vec::new();
    let answer = question(&mut Terminal {
        input: &mut input,
        out: &mut out,
    });
    (answer, String::from_utf8(out).unwrap())
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
            github: Some(true),
            owner: Some("studio".to_owned()),
            visibility: Some(Visibility::Private),
            assets_visibility: Some(Visibility::Public),
            private: Some(vec!["text".to_owned(), "sprites".to_owned()]),
            folders: Some(vec!["levels".to_owned(), "maps".to_owned()]),
        }
    );
    assert_eq!(flags_of_none(), Flags::default());
    assert_eq!(self::flags(&["g", "--no-github"]).github, Some(false));
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
fn the_yes_or_no_questions_take_an_empty_line_as_their_default_and_ask_again() {
    let (answer, out) = answered("\n", |t| github(t, "Moon Garden", true));
    assert!(answer.unwrap());
    assert_eq!(
        out,
        "Put Moon Garden on GitHub? It needs the GitHub CLI (gh), signed in.\n  yes or no, or \
         press Enter for yes: "
    );
    // Without gh the default is no, and the question says why.
    let (answer, out) = answered("\n", |t| github(t, "Moon Garden", false));
    assert!(!answer.unwrap());
    assert!(out.contains("gh isn't installed"), "{out}");
    assert!(out.ends_with("press Enter for no: "), "{out}");
    let (answer, out) = answered("maybe\nNo\n", |t| github(t, "G", true));
    assert!(!answer.unwrap());
    // The question once, its choices again after a wrong answer.
    assert_eq!(out.matches("Put G on GitHub?").count(), 1);
    assert_eq!(out.matches("press Enter for yes").count(), 2);
    assert!(out.contains("  `maybe`: yes or no\n"), "{out}");
    let (answer, out) = answered("\n", |t| private(t, "you/g-assets"));
    assert!(answer.unwrap());
    assert_eq!(
        out,
        "Keep some files in a second, private repository, you/g-assets, so they aren't in the \
         public code?\n  yes or no, or press Enter for yes: "
    );
    assert!(!answered("n\n", |t| private(t, "r")).0.unwrap());
}

#[test]
fn the_visibility_questions_default_to_public_then_private() {
    let (answer, out) = answered("\n", |t| game_visibility(t, "you/g"));
    assert_eq!(answer.unwrap(), Visibility::Public);
    assert_eq!(
        out,
        "Should you/g be public or private? GitHub's CI is free on a public repository.\n  \
         public or private, or press Enter for public: "
    );
    let (answer, out) = answered("p\nPRIV\n", |t| game_visibility(t, "you/g"));
    assert_eq!(answer.unwrap(), Visibility::Private);
    assert!(out.contains("  `p`: public or private\n"), "{out}");
    let (answer, out) = answered("\n", |t| assets_visibility(t, "you/g-assets"));
    assert_eq!(answer.unwrap(), Visibility::Private);
    assert!(out.ends_with("press Enter for private: "), "{out}");
    assert_eq!(
        answered("public\n", |t| assets_visibility(t, "r"))
            .0
            .unwrap(),
        Visibility::Public
    );
}

#[test]
fn the_kinds_question_shows_each_folder_and_takes_numbers_or_names() {
    let (answer, out) = answered("\n", which_kinds);
    assert_eq!(answer.unwrap(), DEFAULT_FOLDERS);
    assert!(
        out.contains("  1. files/text/      writing: dialogue, scripts…\n"),
        "{out}"
    );
    assert!(out.ends_with("press Enter for all five: "), "{out}");
    let (answer, out) = answered("art\nnone\n2, music\n", which_kinds);
    assert_eq!(answer.unwrap(), ["sprites", "music"]);
    assert!(out.contains("  `art` is no kind"), "{out}");
    assert!(out.contains("  pick one kind at least"), "{out}");
}

#[test]
fn the_folders_question_takes_a_comma_list_or_nothing() {
    let (answer, out) = answered("\n", other_folders);
    assert_eq!(answer.unwrap(), Vec::<String>::new());
    assert_eq!(
        out,
        "Any other folders under files/ to keep there?\n  folders, several with commas (levels, \
         maps), or press Enter for none: "
    );
    let (answer, out) = answered("a b\nfiles/levels, maps\n", other_folders);
    assert_eq!(answer.unwrap(), ["levels", "maps"]);
    assert!(
        out.contains("  --private-folder: `a b` isn't a folder"),
        "{out}"
    );
}

#[test]
fn no_line_left_is_an_error_not_a_default() {
    let (answer, _) = answered("", |t| github(t, "G", true));
    assert_eq!(answer.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
}

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

    /// `new moon-garden` with `items` and the lines `typed` on a
    /// terminal, if any.
    fn run(&self, items: &[&str], typed: Option<&str>, fake: &mut Fake) -> (Made, String) {
        let mut all = vec!["moon-garden", "--name", "Moon Garden"];
        all.extend_from_slice(items);
        let (input, flags) = parse(&args(&all)).unwrap();
        let mut lines = typed.unwrap_or("").as_bytes();
        let mut out = Vec::new();
        let terminal = typed.map(|_| Terminal {
            input: &mut lines,
            out: &mut out,
        });
        let made = new_with(&self.dir, input, &flags, false, terminal, fake);
        (made, String::from_utf8(out).unwrap())
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
    let (made, out) = scratch.run(&[], Some("\n\n\n\n\n\n"), &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    let report = made.repos.as_ref().unwrap();
    assert!(report.ok, "{report:?}");
    assert_eq!(made.exit_code(), 0);
    assert!(made.finish.is_none());
    // The six questions, in order.
    // A line typed isn't echoed, so each question follows the prompt
    // before it, "…press Enter for <default>: ".
    let asked: Vec<&str> = out
        .split("press Enter for ")
        .enumerate()
        .filter_map(|(at, piece)| match at {
            0 => Some(piece),
            _ => piece.split_once(": ").map(|(_, rest)| rest),
        })
        .filter_map(|q| q.lines().next())
        .filter(|q| !q.is_empty())
        .collect();
    // Each question says what Enter picks.
    let defaults: Vec<&str> = out
        .split("press Enter for ")
        .skip(1)
        .filter_map(|q| q.split(':').next())
        .collect();
    assert_eq!(
        defaults,
        ["yes", "public", "yes", "all five", "none", "private"],
        "{out}"
    );
    assert_eq!(asked.len(), 6, "{out}");
    assert!(asked[0].starts_with("Put Moon Garden on GitHub?"));
    assert!(asked[1].starts_with("Should you/moon-garden be public or private?"));
    assert!(
        asked[2]
            .starts_with("Keep some files in a second, private repository, you/moon-garden-assets")
    );
    assert!(asked[3].starts_with("Which kinds of files go there?"));
    assert!(asked[4].starts_with("Any other folders"));
    assert!(asked[5].starts_with("Should you/moon-garden-assets be private or public?"));
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
    let (made, out) = scratch.run(&["--no-github"], Some(""), &mut fake);
    assert!(made.answer.ok, "{:?}", made.answer);
    assert!(made.repos.is_none());
    assert_eq!(out, "");
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
    let mut lines = "\n".as_bytes();
    let mut out = Vec::new();
    let terminal = Terminal {
        input: &mut lines,
        out: &mut out,
    };
    let made = new_with(&scratch.dir, input, &flags, true, Some(terminal), &mut fake);
    assert!(made.answer.ok);
    assert!(made.repos.is_none());
    assert_eq!(String::from_utf8_lossy(&out), "");
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
    let (made, out) = scratch.run(
        &[
            "--github",
            "--private",
            "sprites",
            "--private-folder",
            "files/levels",
        ],
        Some("\n\n"),
        &mut fake,
    );
    assert!(made.repos.as_ref().unwrap().ok, "{:?}", made.repos);
    // Only what no flag answered was asked: the two visibilities.
    assert!(out.contains("Should you/moon-garden be public"), "{out}");
    assert!(
        out.contains("Should you/moon-garden-assets be private or public?"),
        "{out}"
    );
    assert!(!out.contains("Which kinds"), "{out}");
    assert!(!out.contains("Put Moon Garden"), "{out}");
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
