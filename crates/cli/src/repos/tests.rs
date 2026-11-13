//! `repos`' tests: the plan for each state, and whole runs against the
//! scripted machine and GitHub of [`super::fake`]. None reaches the
//! network.

use std::fs;
use std::path::PathBuf;

use super::fake::{Fake, is_read};
use super::*;
use crate::project::Config;

const GAME: &str = "you/moon-garden";
const PRIVATE: &str = "you/moon-garden-assets";

/// A game made with the private part, and the folders the fake writes
/// in.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("shuyi-repos-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("game")).unwrap();
        fs::create_dir_all(dir.join("temp")).unwrap();
        fs::write(dir.join("game/private.rev"), "").unwrap();
        Self { dir }
    }

    fn root(&self) -> PathBuf {
        self.dir.join("game")
    }

    fn temp(&self) -> PathBuf {
        self.dir.join("temp")
    }

    fn project(&self, private: bool) -> Project {
        let private = if private {
            r#"private_assets: (
                url: "https://github.com/you/moon-garden-assets.git",
                checkout: "private",
                pin_file: "private.rev",
                folders: ["files", "docs"],
                library: "library",
                no_access: "ask its owner",
            ),"#
        } else {
            ""
        };
        let text =
            format!(r#"(app: (name: "Moon Garden", id: "com.example.moon-garden"), {private})"#);
        Project {
            root: self.root(),
            config: Config::parse(&text).unwrap(),
        }
    }

    fn fake(&self) -> Fake {
        Fake::new(&self.root(), &self.temp())
    }

    /// Whether the temporary folder is empty: every key and first commit
    /// removed.
    fn temp_is_empty(&self) -> bool {
        fs::read_dir(self.temp()).unwrap().next().is_none()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn parsed(items: &[&str]) -> Args {
    match parse_args(&args(items)) {
        Ok(Parsed::Run(args)) => args,
        other => panic!("{other:?}"),
    }
}

fn statuses(report: &Report) -> Vec<(&str, Status)> {
    report.steps.iter().map(|l| (l.step, l.status)).collect()
}

/// Every file under `dir`, at any depth: its path and text.
fn files_under(dir: &std::path::Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push((path.clone(), fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    out
}

// The plan.

fn slug(path: &str) -> Slug {
    Slug::of_url(&format!("https://github.com/{path}")).unwrap()
}

fn inputs(private: bool) -> Inputs {
    Inputs {
        game: slug(GAME),
        visibility: Visibility::Public,
        private: private.then(|| Private {
            repo: slug(PRIVATE),
            visibility: Visibility::Private,
            folders: vec!["text".to_owned()],
        }),
        new_keys: false,
    }
}

fn everything() -> State {
    State {
        git: Git::Committed,
        branch: "main".to_owned(),
        origin: Some("https://github.com/you/moon-garden.git".to_owned()),
        game_repo: Some(GameRepo {
            pushed: true,
            secrets: vec![Use::Builds, Use::Dependabot],
        }),
        private_repo: Some(PrivateRepo {
            keys: vec![
                DeployKey {
                    id: 1,
                    title: "moon-garden builds (read-only)".to_owned(),
                },
                DeployKey {
                    id: 2,
                    title: "moon-garden dependabot (read-only)".to_owned(),
                },
            ],
            ruleset: true,
        }),
        pin: Pin::Committed,
        checkout: true,
    }
}

/// The ids of the steps `plan` makes.
fn made(inputs: &Inputs, state: &State) -> Vec<&'static str> {
    plan(inputs, state)
        .iter()
        .filter(|s| s.make)
        .map(|s| s.thing.id())
        .collect()
}

#[test]
fn with_nothing_there_the_plan_makes_every_step_in_order() {
    let steps = plan(&inputs(true), &State::default());
    let ids: Vec<&str> = steps.iter().map(|s| s.thing.id()).collect();
    assert_eq!(
        ids,
        [
            "git",
            "game-repository",
            "private-repository",
            "pin",
            "checkout",
            "builds-key",
            "dependabot-key",
            "ruleset",
            "pin-commit",
            "push"
        ]
    );
    assert!(steps.iter().all(|s| s.make && s.replaces.is_empty()));
}

#[test]
fn with_everything_there_the_plan_makes_nothing() {
    assert_eq!(made(&inputs(true), &everything()), Vec::<&str>::new());
    assert_eq!(made(&inputs(false), &everything()), Vec::<&str>::new());
}

#[test]
fn each_piece_missing_alone_is_all_the_plan_makes() {
    fn game(state: &mut State) -> &mut GameRepo {
        state.game_repo.as_mut().unwrap()
    }
    fn private(state: &mut State) -> &mut PrivateRepo {
        state.private_repo.as_mut().unwrap()
    }
    type Case = (&'static str, fn(&mut State), &'static [&'static str]);
    let cases: [Case; 12] = [
        ("no git", |s| s.git = Git::None, &["git"]),
        ("no commit", |s| s.git = Git::NoCommit, &["git"]),
        ("no origin", |s| s.origin = None, &["game-repository"]),
        // A private repository that isn't there has no key or ruleset.
        (
            "no private repository",
            |s| s.private_repo = None,
            &[
                "private-repository",
                "builds-key",
                "dependabot-key",
                "ruleset",
            ],
        ),
        ("no pin", |s| s.pin = Pin::Missing, &["pin", "pin-commit"]),
        (
            "a pin not committed",
            |s| s.pin = Pin::Changed,
            &["pin-commit"],
        ),
        ("no checkout", |s| s.checkout = false, &["checkout"]),
        (
            "no Actions secret",
            |s| game(s).secrets.retain(|u| *u != Use::Builds),
            &["builds-key"],
        ),
        (
            "no Dependabot secret",
            |s| game(s).secrets.retain(|u| *u != Use::Dependabot),
            &["dependabot-key"],
        ),
        ("no ruleset", |s| private(s).ruleset = false, &["ruleset"]),
        ("not pushed", |s| game(s).pushed = false, &["push"]),
        // A game's repository that isn't there has no secret, and nothing
        // pushed; its origin was added by hand.
        (
            "no game's repository",
            |s| s.game_repo = None,
            &["builds-key", "dependabot-key", "push"],
        ),
    ];
    for (name, change, expected) in cases {
        let mut state = everything();
        change(&mut state);
        assert_eq!(made(&inputs(true), &state), expected, "{name}");
    }
    // A key whose deploy key is gone, its secret there: made again.
    let mut state = everything();
    private(&mut state).keys.retain(|k| k.id != 2);
    assert_eq!(made(&inputs(true), &state), ["dependabot-key"]);
    // A secret gone, its deploy key there: the key is replaced, so no key
    // whose private half is lost stays.
    let mut state = everything();
    game(&mut state).secrets.retain(|u| *u != Use::Dependabot);
    let step = plan(&inputs(true), &state)
        .into_iter()
        .find(|s| s.thing == Thing::Key(Use::Dependabot))
        .unwrap();
    assert_eq!(step.replaces, [2]);
}

#[test]
fn new_keys_replaces_both_keys() {
    let asked = Inputs {
        new_keys: true,
        ..inputs(true)
    };
    let steps = plan(&asked, &everything());
    let keys: Vec<(&str, Vec<u64>)> = steps
        .iter()
        .filter(|s| s.make)
        .map(|s| (s.thing.id(), s.replaces.clone()))
        .collect();
    assert_eq!(keys, [("builds-key", vec![1]), ("dependabot-key", vec![2])]);
}

#[test]
fn without_private_assets_the_plan_is_the_games_repository_alone() {
    let steps = plan(&inputs(false), &State::default());
    let ids: Vec<&str> = steps.iter().map(|s| s.thing.id()).collect();
    assert_eq!(ids, ["git", "game-repository", "push"]);
}

// The inputs.

#[test]
fn the_name_comes_from_the_apps_name() {
    let scratch = Scratch::new("name");
    let mut project = scratch.project(false);
    let name = |project: &Project, args: &Args| game_slug(project, args, "you", None);
    let found = name(&project, &Args::default()).unwrap();
    assert_eq!(
        (found.owner.as_str(), found.name.as_str()),
        ("you", "moon-garden")
    );
    if let Some(app) = &mut project.config.app {
        app.name = "Visions of Shuyi".to_owned();
    }
    assert_eq!(
        name(&project, &Args::default()).unwrap().path(),
        "you/visions-of-shuyi"
    );
    let asked = parsed(&["--owner", "studio", "--name", "vos"]);
    assert_eq!(name(&project, &asked).unwrap().path(), "studio/vos");
    // `origin` wins, and must be on GitHub.
    let origin = Some("git@github.com:else/where.git");
    assert_eq!(
        game_slug(&project, &asked, "you", origin).unwrap().path(),
        "else/where"
    );
    let gitlab = game_slug(&project, &asked, "you", Some("https://gitlab.com/a/b.git"));
    assert_eq!(gitlab.unwrap_err().code, codes::GITHUB);
    // A name that makes no repository name, and no app at all.
    if let Some(app) = &mut project.config.app {
        app.name = "1999".to_owned();
    }
    assert_eq!(
        name(&project, &Args::default()).unwrap_err().code,
        codes::REPO_NAME
    );
    project.config.app = None;
    assert_eq!(
        name(&project, &Args::default()).unwrap_err().code,
        codes::PROJECT_FILE
    );
}

#[test]
fn a_github_address_names_its_repository() {
    for url in [
        "https://github.com/you/moon-garden-assets.git",
        "https://github.com/you/moon-garden-assets",
        "https://github.com/you/moon-garden-assets/",
        "git@github.com:you/moon-garden-assets.git",
        "ssh://git@github.com/you/moon-garden-assets.git",
    ] {
        assert_eq!(Slug::of_url(url), Some(slug(PRIVATE)), "{url}");
    }
    for url in [
        "https://gitlab.com/you/a.git",
        "https://github.com/you",
        "https://github.com/you/a/b",
        "https://github.com/-you/a",
    ] {
        assert_eq!(Slug::of_url(url), None, "{url}");
    }
}

#[test]
fn the_folder_list_refuses_what_isnt_a_folder_under_files() {
    assert_eq!(
        folders("text, sprites/units ,music,docs,library").unwrap(),
        ["text", "sprites/units", "music"]
    );
    assert_eq!(folders("").unwrap(), Vec::<String>::new());
    for bad in [
        "../up",
        "/root",
        "a//b",
        "sprites/",
        "has space",
        "back\\slash",
        "dot.ted",
        "files",
        "text,text",
    ] {
        assert!(folders(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_arguments_parse_and_a_public_private_repository_needs_yes() {
    assert_eq!(parsed(&[]), Args::default());
    let all = parsed(&[
        "--owner",
        "studio",
        "--name",
        "moon",
        "--visibility",
        "private",
        "--private-folders",
        "text,data",
        "--new-keys",
        "--dry-run",
        "--json",
    ]);
    assert_eq!(all.owner.as_deref(), Some("studio"));
    assert_eq!(all.visibility, Visibility::Private);
    assert_eq!(all.folders, ["text", "data"]);
    assert!(all.new_keys && all.dry_run && all.json);
    let public = parse_args(&args(&["--assets-visibility", "public"])).unwrap_err();
    assert!(
        public.contains("--yes") && public.contains("anyone could copy"),
        "{public}"
    );
    let yes = parsed(&["--assets-visibility", "public", "--yes"]);
    assert_eq!(yes.assets_visibility, Visibility::Public);
    assert_eq!(parse_args(&args(&["--help"])), Ok(Parsed::Help));
    for bad in [
        &["--name"][..],
        &["--name", "has space"],
        &["--owner", "a_b"],
        &["--visibility", "secret"],
        &["--bogus"],
    ] {
        assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
    }
    // The help says what it does to the account.
    assert!(help("cargo shuyi").contains("CREATES\nREPOSITORIES, DEPLOY KEYS AND SECRETS"));
}

#[test]
fn the_ruleset_blocks_deletion_and_force_pushes_and_wants_pull_requests() {
    let json = ruleset_json();
    assert_eq!(json["name"], RULESET);
    assert_eq!(json["target"], "branch");
    assert_eq!(json["enforcement"], "active");
    assert_eq!(
        json["conditions"]["ref_name"]["include"][0],
        "~DEFAULT_BRANCH"
    );
    let rules: Vec<&str> = json["rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["type"].as_str().unwrap())
        .collect();
    assert_eq!(rules, ["deletion", "non_fast_forward", "pull_request"]);
    assert_eq!(
        json["bypass_actors"][0],
        serde_json::json!({ "actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always" })
    );
}

#[test]
fn gh_auth_status_names_the_active_accounts_scopes() {
    let two = "github.com\n  ✓ Logged in to github.com account old (keyring)\n  - Active account: \
               false\n  - Token scopes: 'gist'\n\n  ✓ Logged in to github.com account you \
               (keyring)\n  - Active account: true\n  - Token scopes: 'repo', 'workflow'\n";
    assert_eq!(
        scopes(two),
        Some(vec!["repo".to_owned(), "workflow".to_owned()])
    );
    let none = "  ✓ Logged in to github.com account you (GH_TOKEN)\n  - Token scopes: none\n";
    assert_eq!(scopes(none), Some(Vec::new()));
    let unlisted = "  ✓ Logged in to github.com account you (GH_TOKEN)\n";
    assert_eq!(scopes(unlisted), None);
}

// Whole runs.

#[test]
fn a_full_run_makes_everything_in_order_then_a_second_makes_nothing() {
    let scratch = Scratch::new("full");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    let report = repos(&project, &Args::default(), &mut fake);
    assert!(report.ok, "{report:?}");
    assert!(statuses(&report).iter().all(|(_, s)| *s == Status::Made));
    let changes: Vec<String> = fake
        .changes()
        .into_iter()
        // The temporary folders' names change from run to run.
        .map(|line| {
            let temp = scratch.temp().to_string_lossy().into_owned();
            match line.find(&temp) {
                Some(at) => {
                    let end = line[at..].find(' ').map_or(line.len(), |e| at + e);
                    let file = line[at..end].rsplit(['/', '\\']).next().unwrap().to_owned();
                    format!("{}<temp>/{file}{}", &line[..at], &line[end..])
                }
                None => line,
            }
        })
        .collect();
    assert_eq!(
        changes,
        [
            "git init --quiet -b main",
            "git add --all",
            "git commit --quiet -m First commit",
            "gh repo create you/moon-garden --public --source . --remote origin",
            "git init --quiet -b main",
            "git add --all",
            "git commit --quiet -m First commit",
            "gh repo create you/moon-garden-assets --private --source . --remote origin --push \
             --description Private files of Moon Garden. Must stay private.",
            "shuyi private-assets",
            "ssh-keygen -q -t ed25519 -N  -C moon-garden builds (read-only) -f <temp>/key",
            "gh repo deploy-key add <temp>/key.pub --repo you/moon-garden-assets --title \
             moon-garden builds (read-only)",
            "gh secret set PRIVATE_ASSETS_KEY --repo you/moon-garden",
            "ssh-keygen -q -t ed25519 -N  -C moon-garden dependabot (read-only) -f <temp>/key",
            "gh repo deploy-key add <temp>/key.pub --repo you/moon-garden-assets --title \
             moon-garden dependabot (read-only)",
            "gh secret set PRIVATE_ASSETS_KEY --repo you/moon-garden --app dependabot",
            "gh api repos/you/moon-garden-assets/rulesets -X POST --input -",
            "git add -- private.rev",
            "git commit --quiet -m Pin the private files -- private.rev",
            "git push --quiet --set-upstream origin main",
        ]
    );
    // The game's commits, the pin at the private repository's main, and
    // what GitHub holds.
    assert_eq!(fake.commits, ["First commit", "Pin the private files"]);
    let main = fake.repos[PRIVATE].main.clone().unwrap();
    assert_eq!(
        fs::read_to_string(scratch.root().join("private.rev")).unwrap(),
        format!("{main}\n")
    );
    let game = &fake.repos[GAME];
    assert!(!game.private);
    assert_eq!(game.branches, ["main"]);
    assert_eq!(game.actions.keys().collect::<Vec<_>>(), [SECRET]);
    assert_eq!(game.dependabot.keys().collect::<Vec<_>>(), [SECRET]);
    let private = &fake.repos[PRIVATE];
    assert!(private.private);
    assert_eq!(
        private.description,
        "Private files of Moon Garden. Must stay private."
    );
    let titles: Vec<&str> = private.keys.iter().map(|k| k.1.as_str()).collect();
    assert_eq!(
        titles,
        [
            "moon-garden builds (read-only)",
            "moon-garden dependabot (read-only)"
        ]
    );
    assert_eq!(private.rulesets, [RULESET]);
    assert!(scratch.root().join("private/.git").is_dir());
    assert!(scratch.temp_is_empty());
    assert!(report.text().contains("next: `shuyi run` plays the game"));

    // A second run makes nothing and says everything is there.
    fake.calls.clear();
    let again = repos(&project, &Args::default(), &mut fake);
    assert!(again.ok, "{again:?}");
    assert!(
        statuses(&again).iter().all(|(_, s)| *s == Status::There),
        "{again:?}"
    );
    assert_eq!(fake.changes(), Vec::<String>::new());
    assert!(
        again
            .text()
            .contains("everything is there: nothing was made\n"),
        "{}",
        again.text()
    );
    assert_eq!(again.exit_code(), 0);
}

#[test]
fn the_private_half_goes_to_gh_secret_set_on_stdin_alone() {
    let scratch = Scratch::new("secret");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    let report = repos(&project, &Args::default(), &mut fake);
    assert!(report.ok, "{report:?}");
    let game = &fake.repos[GAME];
    let halves = [&game.actions[SECRET], &game.dependabot[SECRET]];
    assert_ne!(halves[0], halves[1]);
    let json = serde_json::to_string(&report).unwrap();
    for half in halves {
        assert!(half.starts_with("PRIVATE-HALF-"), "{half}");
        let half = half.trim();
        for call in &fake.calls {
            assert!(call.args.iter().all(|a| !a.contains(half)), "{call:?}");
            if call.stdin.as_deref().is_some_and(|s| s.contains(half)) {
                assert_eq!(call.args[..2], ["secret", "set"], "{call:?}");
            }
        }
        assert!(!json.contains(half) && !report.text().contains(half));
    }
    // Each secret was given on stdin, and its public half to GitHub.
    let set: Vec<&str> = fake
        .calls
        .iter()
        .filter(|c| c.args.starts_with(&["secret".to_owned(), "set".to_owned()]))
        .map(|c| c.stdin.as_deref().unwrap())
        .collect();
    assert_eq!(set, halves.map(String::as_str));
    // The keys' folders are gone, and no key is in the game's folder.
    assert!(scratch.temp_is_empty());
    for (path, text) in files_under(&scratch.root()) {
        assert!(!text.contains("PRIVATE-HALF"), "{}", path.display());
    }
}

#[test]
fn a_missing_dependabot_secret_makes_only_that_key_and_secret() {
    let scratch = Scratch::new("dependabot");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    assert!(repos(&project, &Args::default(), &mut fake).ok);
    fake.repo(GAME).dependabot.clear();
    fake.calls.clear();
    let report = repos(&project, &Args::default(), &mut fake);
    assert!(report.ok, "{report:?}");
    let made: Vec<&str> = statuses(&report)
        .into_iter()
        .filter(|(_, s)| *s == Status::Made)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(made, ["dependabot-key"]);
    let changes = fake.changes();
    assert_eq!(changes.len(), 4, "{changes:#?}");
    assert!(changes[0].starts_with("gh api -X DELETE repos/you/moon-garden-assets/keys/"));
    assert!(changes[3].ends_with("--app dependabot"));
    // The old deploy key is gone: one key of each title.
    assert_eq!(fake.repos[PRIVATE].keys.len(), 2);
    assert!(scratch.temp_is_empty());
}

#[test]
fn new_keys_replaces_both_deploy_keys_and_both_secrets() {
    let scratch = Scratch::new("new-keys");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    assert!(repos(&project, &Args::default(), &mut fake).ok);
    let before = fake.repos[PRIVATE].keys.clone();
    let secrets = |fake: &Fake| {
        let game = &fake.repos[GAME];
        (
            game.actions[SECRET].clone(),
            game.dependabot[SECRET].clone(),
        )
    };
    let old = secrets(&fake);
    let report = repos(&project, &parsed(&["--new-keys"]), &mut fake);
    assert!(report.ok, "{report:?}");
    let after = &fake.repos[PRIVATE].keys;
    assert_eq!(after.len(), 2);
    for (id, title, public) in after {
        assert!(
            before
                .iter()
                .all(|(old, _, key)| old != id && key != public),
            "{title}"
        );
    }
    let new = secrets(&fake);
    assert!(old.0 != new.0 && old.1 != new.1);
    assert!(scratch.temp_is_empty());
}

#[test]
fn a_dry_run_only_reads_and_prints_the_plan() {
    let scratch = Scratch::new("dry");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    let report = repos(&project, &parsed(&["--dry-run"]), &mut fake);
    assert!(report.ok && report.dry_run, "{report:?}");
    assert!(statuses(&report).iter().all(|(_, s)| *s == Status::Planned));
    assert_ne!(fake.calls.len(), 0);
    for call in &fake.calls {
        assert!(
            is_read(&call.line()),
            "a dry run changed something: {}",
            call.line()
        );
    }
    assert!(fake.repos.is_empty() && fake.git == Git::None);
    assert_eq!(
        fs::read_to_string(scratch.root().join("private.rev")).unwrap(),
        ""
    );
    let text = report.text();
    assert!(
        text.starts_with("repos --dry-run: the plan; nothing was changed\n"),
        "{text}"
    );
    assert!(
        text.contains("  planned  https://github.com/you/moon-garden (public), as origin\n"),
        "{text}"
    );
    assert_eq!(report.next, None);
    let json: Value = serde_json::to_value(&report).unwrap();
    assert_eq!(
        json["steps"][0],
        serde_json::json!({
            "step": "git",
            "status": "planned",
            "about": "the game's folder as a git repository, with a first commit"
        })
    );
}

/// Each call that makes something, failing alone: the run stops there
/// with its code, leaves no temporary folder, and a second run finishes.
#[test]
fn a_failure_at_any_step_stops_and_a_second_run_finishes() {
    for fail in [
        "git init",
        "git commit --quiet -m First",
        "gh repo create you/moon-garden ",
        "gh repo create you/moon-garden-assets",
        "commits/main",
        "ssh-keygen",
        "deploy-key add",
        "secret set PRIVATE_ASSETS_KEY --repo you/moon-garden --app",
        "rulesets -X POST",
        "Pin the private",
        "git push",
    ] {
        let scratch = Scratch::new("fails");
        let project = scratch.project(true);
        let mut fake = scratch.fake();
        fake.fail = Some(fail.to_owned());
        let report = repos(&project, &Args::default(), &mut fake);
        assert!(!report.ok, "{fail}: {report:?}");
        assert_eq!(report.code.as_deref(), Some("SH3014"), "{fail}");
        assert_eq!(report.exit_code(), 1);
        assert!(
            report
                .error
                .as_deref()
                .unwrap()
                .contains("injected failure"),
            "{fail}"
        );
        let failed = statuses(&report)
            .iter()
            .position(|(_, s)| *s == Status::Failed)
            .unwrap();
        assert!(
            statuses(&report)[failed + 1..]
                .iter()
                .all(|(_, s)| *s == Status::NotRun),
            "{fail}"
        );
        assert!(
            scratch.temp_is_empty(),
            "{fail}: a temporary folder is left"
        );
        fake.fail = None;
        let second = repos(&project, &Args::default(), &mut fake);
        assert!(second.ok, "{fail}: {second:?}");
        assert!(
            statuses(&second)[..failed]
                .iter()
                .all(|(_, s)| *s == Status::There),
            "{fail}: {second:?}"
        );
        assert!(scratch.temp_is_empty());
        // Finished: a third run makes nothing.
        fake.calls.clear();
        let third = repos(&project, &Args::default(), &mut fake);
        assert!(third.ok);
        assert_eq!(fake.changes(), Vec::<String>::new(), "{fail}");
        assert_eq!(fake.repos[PRIVATE].keys.len(), 2, "{fail}");
        assert_eq!(fake.repos[GAME].branches, ["main"], "{fail}");
    }
}

#[test]
fn a_refused_ruleset_is_a_warning_and_the_run_succeeds() {
    let scratch = Scratch::new("ruleset");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    // GitHub refuses rulesets on a private repository of a free account,
    // even to list them.
    fake.repo(PRIVATE).no_rulesets = true;
    fake.repo(PRIVATE).admin = true;
    fake.repo(PRIVATE).main = Some("1".repeat(40));
    let report = repos(&project, &Args::default(), &mut fake);
    assert!(report.ok, "{report:?}");
    assert_eq!(report.exit_code(), 0);
    assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    assert!(report.warnings[0].contains("pushing to its main is not blocked"));
    assert!(
        report
            .text()
            .contains("warning: GitHub refused the ruleset")
    );
    assert_eq!(fake.repos[GAME].branches, ["main"]);
}

#[test]
fn a_checkout_that_fails_is_a_warning() {
    let scratch = Scratch::new("checkout");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    fake.fail = Some("private-assets".to_owned());
    let report = repos(&project, &Args::default(), &mut fake);
    assert!(report.ok, "{report:?}");
    assert!(report.warnings[0].contains("`shuyi private-assets` once git can read"));
    fake.fail = None;
    let second = repos(&project, &Args::default(), &mut fake);
    let made: Vec<_> = statuses(&second)
        .into_iter()
        .filter(|(_, s)| *s == Status::Made)
        .collect();
    assert_eq!(made, [("checkout", Status::Made)]);
}

#[test]
fn without_gh_or_signed_out_it_fails_with_its_code() {
    let scratch = Scratch::new("tools");
    let project = scratch.project(true);
    let run = |change: &dyn Fn(&mut Fake)| {
        let mut fake = scratch.fake();
        change(&mut fake);
        let report = repos(&project, &Args::default(), &mut fake);
        assert!(!report.ok);
        assert_eq!(fake.changes(), Vec::<String>::new());
        report
    };
    let no_gh = run(&|f| f.missing.push("gh"));
    assert_eq!(
        (no_gh.code.as_deref(), no_gh.exit_code()),
        (Some("SH1007"), 3)
    );
    assert!(
        no_gh
            .help
            .as_deref()
            .unwrap()
            .contains("https://cli.github.com")
    );
    let out = run(&|f| f.signed_in = false);
    assert_eq!((out.code.as_deref(), out.exit_code()), (Some("SH1008"), 3));
    assert!(out.help.as_deref().unwrap().contains("gh auth login"));
    let scope = run(&|f| f.scopes = Some(vec!["repo"]));
    assert_eq!(scope.code.as_deref(), Some("SH1009"));
    assert_eq!(
        scope.help.as_deref(),
        Some("run `gh auth refresh -h github.com -s workflow`, then run this again")
    );
    let keygen = run(&|f| f.missing.push("ssh-keygen"));
    assert_eq!(keygen.code.as_deref(), Some("SH1003"));
    let git = run(&|f| f.missing.push("git"));
    assert_eq!(git.code.as_deref(), Some("SH1003"));
    let theirs = run(&|f| f.repo(PRIVATE).admin = false);
    assert_eq!(theirs.code.as_deref(), Some("SH3013"));
    assert!(
        theirs
            .error
            .as_deref()
            .unwrap()
            .contains("moon-garden-assets")
    );
    let taken = run(&|f| {
        let game = f.repo(GAME);
        game.admin = true;
        game.branches.push("main".to_owned());
    });
    assert_eq!(
        (taken.code.as_deref(), taken.exit_code()),
        (Some("SH0006"), 2)
    );
    // A token whose scopes gh doesn't list isn't refused for them.
    let mut fake = scratch.fake();
    fake.scopes = None;
    assert!(repos(&project, &parsed(&["--dry-run"]), &mut fake).ok);
}

#[test]
fn without_private_assets_it_makes_only_the_games_repository() {
    let scratch = Scratch::new("public");
    let project = scratch.project(false);
    let mut fake = scratch.fake();
    // No key is made, so ssh-keygen isn't needed.
    fake.missing.push("ssh-keygen");
    let report = repos(&project, &parsed(&["--visibility", "private"]), &mut fake);
    assert!(report.ok, "{report:?}");
    assert_eq!(
        statuses(&report),
        [
            ("git", Status::Made),
            ("game-repository", Status::Made),
            ("push", Status::Made)
        ]
    );
    assert!(fake.repos[GAME].private);
    assert_eq!(fake.repos.len(), 1);
    assert_eq!(report.private, None);
    assert!(report.warnings[0].contains("this game has no private files"));
    assert!(report.warnings[0].contains("--set private-files=yes"));
}

#[test]
fn an_existing_origin_is_left_as_it_is() {
    let scratch = Scratch::new("origin");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    fake.git = Git::Committed;
    fake.branch = "trunk".to_owned();
    fake.origin = Some("git@github.com:studio/moon.git".to_owned());
    fake.repo("studio/moon").admin = true;
    let report = repos(&project, &Args::default(), &mut fake);
    assert!(report.ok, "{report:?}");
    assert_eq!(
        report.game.as_deref(),
        Some("https://github.com/studio/moon")
    );
    assert_eq!(report.steps[1].status, Status::There);
    assert_eq!(
        report.steps[1].about,
        "origin: git@github.com:studio/moon.git, left as it is"
    );
    let moon = &fake.repos["studio/moon"];
    assert_eq!(moon.branches, ["trunk"]);
    assert!(moon.actions.contains_key(SECRET) && moon.dependabot.contains_key(SECRET));
    // The keys are named from the repository's name.
    assert_eq!(fake.repos[PRIVATE].keys[0].1, "moon builds (read-only)");
}

#[test]
fn the_private_repositorys_first_commit_explains_and_keeps_its_folders() {
    let scratch = Scratch::new("first-commit");
    let project = scratch.project(true);
    let mut fake = scratch.fake();
    let asked = parsed(&["--private-folders", "text,sprites/units"]);
    assert!(repos(&project, &asked, &mut fake).ok);
    let files = &fake.repos[PRIVATE].files;
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [
            ".gitattributes",
            ".gitignore",
            "README.md",
            "docs/.gitkeep",
            "files/sprites/units/.gitkeep",
            "files/text/.gitkeep",
            "library/.gitkeep"
        ]
    );
    let readme = &files["README.md"];
    assert!(
        readme.starts_with("# Moon Garden: private files\n"),
        "{readme}"
    );
    assert!(readme.contains("**It must stay private.**"));
    assert!(
        readme.contains("https://github.com/you/moon-garden;"),
        "{readme}"
    );
    assert!(
        readme.contains("| `files/text/` | The game's writing"),
        "{readme}"
    );
    assert!(
        readme.contains("| `files/sprites/units/` | Files the game reads"),
        "{readme}"
    );
    assert!(readme.contains("`shuyi private-assets --pin`"));
    assert!(!readme.contains("{{"), "{readme}");
    assert!(files[".gitattributes"].contains("*.png binary"));
    // No README inside files/: every file there ships in the game.
    assert!(
        files
            .keys()
            .all(|f| !f.starts_with("files/") || f.ends_with(".gitkeep"))
    );
    // With no kind folder, files/ keeps itself.
    let bare = private_files("Moon", &slug(GAME), "shuyi", &[]);
    assert!(bare.iter().any(|(path, _)| path == "files/.gitkeep"));
}

#[test]
fn the_temporary_folder_goes_whatever_happens() {
    let scratch = Scratch::new("temp-dir");
    let path = {
        let temp = TempDir::new(&scratch.temp(), "key").unwrap();
        fs::write(temp.path().join("key"), "half").unwrap();
        temp.path().to_path_buf()
    };
    assert!(!path.exists());
    let caught = std::panic::catch_unwind(|| {
        let temp = TempDir::new(&scratch.temp(), "key").unwrap();
        fs::write(temp.path().join("key"), "half").unwrap();
        panic!("half-way");
    });
    assert!(caught.is_err());
    assert!(scratch.temp_is_empty());
}

#[test]
fn this_machine_runs_a_program_with_its_stdin() {
    let dir = std::env::temp_dir();
    let mut host = ThisMachine;
    let ran = host
        .run("git", &["hash-object", "--stdin"], &dir, Some("hello\n"))
        .unwrap();
    assert!(ran.ok, "{ran:?}");
    assert_eq!(
        ran.stdout.trim(),
        "ce013625030ba8dba906f756967f9e9ca394464a"
    );
    let failed = host.run("git", &["no-such-command"], &dir, None).unwrap();
    assert!(!failed.ok && !failed.said().is_empty());
    assert!(
        host.run("no-such-program-anywhere", &[], &dir, None)
            .is_err()
    );
    assert_eq!(host.temp_root(), std::env::temp_dir());
}
