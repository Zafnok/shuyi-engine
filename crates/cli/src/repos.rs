//! `repos`: puts a game on GitHub with the `gh` its developer is signed in
//! to (ADR-0112). It makes the game's repository and, for a game with
//! private files (`shuyi.ron`'s `private_assets`, ADR-0111), the private
//! repository its `url` names, with its first commit; pins the game to it
//! and checks it out; makes the two read-only deploy keys whose private
//! halves are the game's secret `PRIVATE_ASSETS_KEY`, for Actions and for
//! Dependabot; puts a ruleset on the private repository's default branch;
//! and pushes the game, last, so its first CI run has the key.
//!
//! Each step is skipped when what it makes is there, so a second run makes
//! nothing and a run after a failure finishes the rest. It acts on the
//! developer's account, so it is a command of the CLI only, never an
//! operation of the contract or a tool of the MCP server.
//!
//! The plan is data: [`observe`] looks with read-only calls, [`plan`] is a
//! pure function from what was asked ([`Inputs`]) and what is there
//! ([`State`]) to the [`Step`]s, and every program runs through a
//! [`Host`], which a test scripts.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::contract::codes::{self, Code};
use crate::contract::new::crate_name;
use crate::private_assets::PRIVATE_FILES;
use crate::project::{PrivateAssets, Project};

mod host;

pub use host::{Host, Ran, TempDir, ThisMachine};

/// The game's secret holding the private half of a deploy key, for
/// Actions and for Dependabot (ADR-0111 §5).
pub const SECRET: &str = "PRIVATE_ASSETS_KEY";

/// The name of the ruleset `repos` puts on the private repository.
pub const RULESET: &str = "Protect main";

/// The kind folders of `files/` when `--private-folders` names none.
pub const DEFAULT_FOLDERS: [&str; 5] = ["text", "sprites", "sounds", "music", "data"];

/// The scopes `gh`'s sign-in needs: `repo` for the repositories, keys,
/// secrets and ruleset, `workflow` to push the game's workflows.
pub const SCOPES: [&str; 2] = ["repo", "workflow"];

/// The private repository's folder of story and design documents.
const DOCS: &str = "docs";

/// The private repository's first commit: its README (with
/// placeholders), `.gitattributes` and `.gitignore`.
const README: &str = include_str!("repos/private-readme.txt");
const GITATTRIBUTES: &str = include_str!("repos/gitattributes.txt");
const GITIGNORE: &str = include_str!("repos/gitignore.txt");

/// Who may see a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Anyone.
    Public,
    /// Its owner and who they let in.
    Private,
}

impl Visibility {
    /// Its name, as `gh` and the flags write it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }

    pub fn parse(flag: &str, value: &str) -> Result<Self, String> {
        match value {
            "public" => Ok(Self::Public),
            "private" => Ok(Self::Private),
            other => Err(format!("{flag} is public or private, not `{other}`")),
        }
    }
}

/// A repository on GitHub: its owner and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slug {
    /// The user or organization.
    pub owner: String,
    /// The repository's name.
    pub name: String,
}

impl Slug {
    /// `owner/name`.
    pub fn path(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// Its page, `https://github.com/owner/name`.
    pub fn url(&self) -> String {
        format!("https://github.com/{}", self.path())
    }

    /// The repository a GitHub address names: `https://github.com/o/n`,
    /// `git@github.com:o/n` or `ssh://git@github.com/o/n`, with or without
    /// `.git`.
    pub fn of_url(url: &str) -> Option<Self> {
        let url = url.trim();
        let rest = [
            "https://github.com/",
            "git@github.com:",
            "ssh://git@github.com/",
        ]
        .iter()
        .find_map(|prefix| url.strip_prefix(prefix))?;
        let rest = rest.trim_end_matches('/');
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        let (owner, name) = rest.split_once('/')?;
        (check_owner(owner).is_ok() && check_name(name).is_ok()).then(|| Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }
}

/// Whether GitHub takes `name` as a repository's name: letters, digits,
/// `.`, `-` and `_`, at most 100, not `.` or `..`, not ending in `.git`.
pub fn check_name(name: &str) -> Result<(), String> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_');
    if name.is_empty() || name.len() > 100 {
        return Err(format!(
            "`{name}`: a repository's name is 1 to 100 characters"
        ));
    }
    if !name.chars().all(allowed)
        || name == "."
        || name == ".."
        || name.to_ascii_lowercase().ends_with(".git")
    {
        return Err(format!(
            "`{name}` isn't a name GitHub takes: letters, digits, `.`, `-` and `_`, not `.` or \
             `..`, and not ending in `.git`"
        ));
    }
    Ok(())
}

/// Whether `owner` can be a GitHub login or organization: letters, digits
/// and single `-` between them, at most 39.
pub fn check_owner(owner: &str) -> Result<(), String> {
    let ok = !owner.is_empty()
        && owner.len() <= 39
        && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !owner.starts_with('-')
        && !owner.ends_with('-')
        && !owner.contains("--");
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{owner}` isn't a GitHub login or organization: letters, digits and single `-` \
             between them"
        ))
    }
}

/// The kind folders `list` names (`text,sprites/units`), each a path of
/// letters, digits, `-` and `_` under `files/`. `docs` and `library`, the
/// private repository's other folders, are there anyway: naming them adds
/// nothing.
pub fn folders(list: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for folder in list.split(',').map(str::trim).filter(|f| !f.is_empty()) {
        let segment_ok = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        };
        if !folder.split('/').all(segment_ok) {
            return Err(format!(
                "--private-folders: `{folder}` isn't a folder: a path of letters, digits, `-` and \
                 `_` under files/, its parts joined by `/`"
            ));
        }
        if folder == PRIVATE_FILES {
            return Err(format!(
                "--private-folders: `{folder}/` is always there: name the folders inside it"
            ));
        }
        if folder == DOCS || folder == "library" {
            continue;
        }
        if out.iter().any(|f| f == folder) {
            return Err(format!("--private-folders: `{folder}` twice"));
        }
        out.push(folder.to_owned());
    }
    Ok(out)
}

/// The command line of `repos`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// `--owner`: the game's repository's owner; the signed-in user if
    /// none.
    pub owner: Option<String>,
    /// `--name`: the game's repository's name; from `shuyi.ron`'s
    /// `app.name` if none.
    pub name: Option<String>,
    /// `--visibility`: the game's repository's.
    pub visibility: Visibility,
    /// `--assets-visibility`: the private repository's.
    pub assets_visibility: Visibility,
    /// `--private-folders`: the kind folders of `files/`.
    pub folders: Vec<String>,
    /// `--new-keys`: replace both deploy keys and both secrets.
    pub new_keys: bool,
    /// `--dry-run`: the plan, and change nothing.
    pub dry_run: bool,
    /// `--json`: the plan or the outcome as JSON.
    pub json: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            owner: None,
            name: None,
            visibility: Visibility::Public,
            assets_visibility: Visibility::Private,
            folders: DEFAULT_FOLDERS.iter().map(|f| (*f).to_owned()).collect(),
            new_keys: false,
            dry_run: false,
            json: false,
        }
    }
}

/// The usage line, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} repos [--owner <login>] [--name <repository>] [--visibility \
         public|private]\n         [--assets-visibility private|public --yes] [--private-folders \
         <list>] [--new-keys]\n         [--dry-run] [--json]"
    )
}

/// What `--help` prints.
pub fn help(command: &str) -> String {
    format!(
        "{usage}

Puts the game on GitHub with the gh you are signed in to. It CREATES
REPOSITORIES, DEPLOY KEYS AND SECRETS ON YOUR ACCOUNT: run it with
--dry-run first to see what it would make.

It makes the game's repository (named from shuyi.ron's app name, public by
default) unless the folder already has an `origin`; for a game with private
files (shuyi.ron's private_assets), the private repository its url names
(private by default) with a README and the folders of files/, pins the game
to its commit and checks it out; two read-only deploy keys there, whose
private halves are the game's secret {SECRET} for Actions and for
Dependabot; a ruleset on its default branch; then pushes the game, so its
first CI run reads the private files. What is there already is left alone:
run it again to finish what a failure left.

  --owner <login>              the game's repository's owner (default: you)
  --name <repository>          its name (default: from shuyi.ron's app name)
  --visibility public|private  the game's repository (default: public, free CI)
  --assets-visibility private|public
                               the private files' repository (default: private;
                               public needs --yes, and makes the files anyone's)
  --private-folders <list>     the folders of files/ (default: {folders})
  --new-keys                   replace both deploy keys and both secrets
  --dry-run                    print the plan and change nothing
  --json                       the plan or the outcome as JSON",
        usage = usage(command),
        folders = DEFAULT_FOLDERS.join(",")
    )
}

/// What the command line asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    /// Run with these arguments.
    Run(Args),
    /// Print the help.
    Help,
}

/// Parses the arguments after `repos`. The error says what is wrong.
pub fn parse_args(args: &[String]) -> Result<Parsed, String> {
    let mut out = Args::default();
    let mut yes = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = |flag: &str| {
            rest.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match arg.as_str() {
            "--help" | "-h" => return Ok(Parsed::Help),
            "--owner" => {
                let owner = value(arg)?;
                check_owner(&owner)?;
                out.owner = Some(owner);
            }
            "--name" => {
                let name = value(arg)?;
                check_name(&name)?;
                out.name = Some(name);
            }
            "--visibility" => out.visibility = Visibility::parse(arg, &value(arg)?)?,
            "--assets-visibility" => {
                out.assets_visibility = Visibility::parse(arg, &value(arg)?)?;
            }
            "--private-folders" => out.folders = folders(&value(arg)?)?,
            "--new-keys" => out.new_keys = true,
            "--dry-run" => out.dry_run = true,
            "--json" => out.json = true,
            "--yes" => yes = true,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if out.assets_visibility == Visibility::Public && !yes {
        return Err(
            "--assets-visibility public makes the private files' repository public: anyone could \
             copy the game's writing, art and sounds, which is what it keeps from them. Add --yes \
             if that is what you want"
                .to_owned(),
        );
    }
    Ok(Parsed::Run(out))
}

/// The private repository, as asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Private {
    /// The repository `private_assets.url` names.
    pub repo: Slug,
    /// Who may see it.
    pub visibility: Visibility,
    /// The kind folders of its `files/`.
    pub folders: Vec<String>,
}

/// What `repos` was asked, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inputs {
    /// The game's repository: `origin`'s, else the owner and name asked.
    pub game: Slug,
    /// Who may see it, when it is made.
    pub visibility: Visibility,
    /// The private repository, for a game with private files.
    pub private: Option<Private>,
    /// Replace both deploy keys and both secrets.
    pub new_keys: bool,
}

/// How far the game's folder is a git repository of its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Git {
    /// It isn't one (or is only inside another's).
    #[default]
    None,
    /// It is, with no commit yet.
    NoCommit,
    /// It is, with a commit.
    Committed,
}

/// A deploy key of the private repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployKey {
    /// GitHub's id of it.
    pub id: u64,
    /// Its title.
    pub title: String,
}

/// What is there, as [`observe`] found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// The game's folder as a git repository.
    pub git: Git,
    /// Its branch (`main` for one `repos` makes).
    pub branch: String,
    /// Its `origin`, if it has one.
    pub origin: Option<String>,
    /// The game's repository, if it is on GitHub.
    pub game_repo: Option<GameRepo>,
    /// The private repository, if it is on GitHub.
    pub private_repo: Option<PrivateRepo>,
    /// The pin file.
    pub pin: Pin,
    /// Whether the private files are checked out.
    pub checkout: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            git: Git::None,
            branch: "main".to_owned(),
            origin: None,
            game_repo: None,
            private_repo: None,
            pin: Pin::Missing,
            checkout: false,
        }
    }
}

/// The game's repository on GitHub.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameRepo {
    /// Whether the folder's branch is there.
    pub pushed: bool,
    /// Who has the secret [`SECRET`]: Actions, Dependabot.
    pub secrets: Vec<Use>,
}

/// The private repository on GitHub.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivateRepo {
    /// Its deploy keys.
    pub keys: Vec<DeployKey>,
    /// Whether it has the ruleset [`RULESET`].
    pub ruleset: bool,
}

/// The pin file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Pin {
    /// Missing or empty: no pin yet.
    #[default]
    Missing,
    /// It names something, committed.
    Committed,
    /// It names something, with a change that isn't committed.
    Changed,
}

/// Who reads the private repository with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// The game's Actions runs.
    Builds,
    /// Its Dependabot runs, which read only Dependabot secrets.
    Dependabot,
}

impl Use {
    /// Its name, in the key's title and the step's id.
    pub fn name(self) -> &'static str {
        match self {
            Self::Builds => "builds",
            Self::Dependabot => "dependabot",
        }
    }

    /// The deploy key's title and comment, for the game's repository
    /// named `game`.
    pub fn title(self, game: &str) -> String {
        format!("{game} {} (read-only)", self.name())
    }
}

/// What a step makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thing {
    /// The game's folder as a git repository, with its first commit.
    Git,
    /// The game's repository on GitHub, as `origin`.
    GameRepo,
    /// The private repository, with its first commit.
    PrivateRepo,
    /// The pin file, at the private repository's `main`.
    Pin,
    /// The checkout of the private files.
    Checkout,
    /// A read-only deploy key and its secret.
    Key(Use),
    /// The ruleset on the private repository's default branch.
    Ruleset,
    /// The pin file committed.
    PinCommit,
    /// The game's branch pushed.
    Push,
}

impl Thing {
    /// Its id in the report.
    pub fn id(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::GameRepo => "game-repository",
            Self::PrivateRepo => "private-repository",
            Self::Pin => "pin",
            Self::Checkout => "checkout",
            Self::Key(Use::Builds) => "builds-key",
            Self::Key(Use::Dependabot) => "dependabot-key",
            Self::Ruleset => "ruleset",
            Self::PinCommit => "pin-commit",
            Self::Push => "push",
        }
    }
}

/// One step of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// What it makes.
    pub thing: Thing,
    /// Whether it is to be made (else it is there).
    pub make: bool,
    /// The deploy keys it deletes first: those of its title.
    pub replaces: Vec<u64>,
}

impl Step {
    fn new(thing: Thing, make: bool) -> Self {
        Self {
            thing,
            make,
            replaces: Vec::new(),
        }
    }
}

/// The steps, in order, from what was asked and what is there: each to be
/// made, or there.
pub fn plan(inputs: &Inputs, state: &State) -> Vec<Step> {
    let mut steps = vec![
        Step::new(Thing::Git, state.git != Git::Committed),
        Step::new(Thing::GameRepo, state.origin.is_none()),
    ];
    if inputs.private.is_some() {
        let private = state.private_repo.as_ref();
        steps.push(Step::new(Thing::PrivateRepo, private.is_none()));
        steps.push(Step::new(Thing::Pin, state.pin == Pin::Missing));
        steps.push(Step::new(Thing::Checkout, !state.checkout));
        for key in [Use::Builds, Use::Dependabot] {
            let title = key.title(&inputs.game.name);
            let old: Vec<u64> = private
                .iter()
                .flat_map(|p| &p.keys)
                .filter(|k| k.title == title)
                .map(|k| k.id)
                .collect();
            let secret = state
                .game_repo
                .as_ref()
                .is_some_and(|g| g.secrets.contains(&key));
            let make = inputs.new_keys || !secret || old.is_empty();
            steps.push(Step {
                thing: Thing::Key(key),
                make,
                replaces: if make { old } else { Vec::new() },
            });
        }
        let ruleset = private.is_some_and(|p| p.ruleset);
        steps.push(Step::new(Thing::Ruleset, !ruleset));
        steps.push(Step::new(Thing::PinCommit, state.pin != Pin::Committed));
    }
    let pushed = state.game_repo.as_ref().is_some_and(|g| g.pushed);
    steps.push(Step::new(Thing::Push, !pushed));
    steps
}

/// An error, with its problem code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// Its code.
    pub code: Code,
    /// What went wrong.
    pub message: String,
    /// How to fix it, when the code's own help isn't enough.
    pub help: Option<String>,
}

impl Failure {
    fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            help: None,
        }
    }

    fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// A step on GitHub or in git that failed: `what`, and what the
    /// program said.
    fn step(what: &str, ran: &Ran) -> Self {
        Self::new(
            codes::GITHUB,
            format!("{what} failed\n  it said: {}", ran.said()),
        )
    }
}

/// Runs `program` and fails with [`codes::TOOLCHAIN`] if it doesn't
/// start.
fn run(
    host: &mut dyn Host,
    program: &str,
    args: &[&str],
    dir: &Path,
    stdin: Option<&str>,
) -> Result<Ran, Failure> {
    host.run(program, args, dir, stdin)
        .map_err(|e| Failure::new(codes::TOOLCHAIN, format!("{program} didn't start: {e}")))
}

/// Runs `program`, which must succeed at `what`.
fn must(
    host: &mut dyn Host,
    what: &str,
    program: &str,
    args: &[&str],
    dir: &Path,
) -> Result<String, Failure> {
    let ran = run(host, program, args, dir, None)?;
    if ran.ok {
        Ok(ran.stdout)
    } else {
        Err(Failure::step(what, &ran))
    }
}

/// What `gh api` answered.
enum Got {
    /// JSON.
    Json(Value),
    /// Not found: HTTP 404 (or 409, an empty repository's branch).
    Missing,
}

/// `gh api <path>`, a read.
fn gh_get(host: &mut dyn Host, dir: &Path, path: &str) -> Result<Got, Failure> {
    let ran = run(host, "gh", &["api", path], dir, None)?;
    if ran.ok {
        return serde_json::from_str(&ran.stdout)
            .map(Got::Json)
            .map_err(|e| {
                Failure::new(
                    codes::GITHUB,
                    format!("gh api {path} answered what isn't JSON: {e}"),
                )
            });
    }
    let said = ran.said();
    if said.contains("(HTTP 404)") || said.contains("(HTTP 409)") {
        Ok(Got::Missing)
    } else {
        Err(Failure::step(&format!("gh api {path}"), &ran))
    }
}

/// The names in a list of secrets (`{"secrets": [{"name": …}]}`).
fn secret_names(json: &Value) -> Vec<&str> {
    json["secrets"]
        .as_array()
        .map(|list| list.iter().filter_map(|s| s["name"].as_str()).collect())
        .unwrap_or_default()
}

/// Whether `repo` is on GitHub, failing when the signed-in user can't
/// administer it.
fn repo_there(host: &mut dyn Host, dir: &Path, repo: &Slug) -> Result<bool, Failure> {
    match gh_get(host, dir, &format!("repos/{}", repo.path()))? {
        Got::Missing => Ok(false),
        Got::Json(json) if json["permissions"]["admin"].as_bool() == Some(true) => Ok(true),
        Got::Json(_) => Err(Failure::new(
            codes::NOT_YOURS,
            format!(
                "{} exists, and you can't administer it: its keys, secrets and rulesets need an \
                 admin",
                repo.url()
            ),
        )),
    }
}

/// The scopes in `gh auth status`'s text for the active account, or
/// `None` when it names none (a token whose scopes aren't listed).
fn scopes(status: &str) -> Option<Vec<String>> {
    let blocks: Vec<&str> = status.split("Logged in to").skip(1).collect();
    let block = blocks
        .iter()
        .find(|b| b.contains("Active account: true"))
        .or_else(|| blocks.first())?;
    let line = block.lines().find(|l| l.contains("Token scopes:"))?;
    let list = line.split_once("Token scopes:")?.1;
    Some(
        list.split(',')
            .map(|s| s.trim().trim_matches(|c| c == '\'' || c == '"').to_owned())
            .filter(|s| !s.is_empty() && s != "none")
            .collect(),
    )
}

/// Checks that `gh`, `git` and (for `keys`) `ssh-keygen` are installed,
/// and that `gh` is signed in with [`SCOPES`]. Returns the login.
fn check_tools(host: &mut dyn Host, dir: &Path, keys: bool) -> Result<String, Failure> {
    if host.run("gh", &["--version"], dir, None).is_err() {
        return Err(Failure::new(
            codes::NO_GH,
            "gh, GitHub's CLI, isn't installed",
        ));
    }
    if host.run("git", &["--version"], dir, None).is_err() {
        return Err(Failure::new(codes::TOOLCHAIN, "git isn't installed")
            .help("install git (https://git-scm.com), then run this again"));
    }
    if keys && host.run("ssh-keygen", &["-?"], dir, None).is_err() {
        return Err(
            Failure::new(codes::TOOLCHAIN, "ssh-keygen isn't installed").help(
                "it comes with OpenSSH: on Windows the optional feature OpenSSH Client, or Git \
                 for Windows (its usr/bin on PATH); on Linux the package openssh-client; macOS \
                 has it. Then run this again",
            ),
        );
    }
    let status = run(
        host,
        "gh",
        &["auth", "status", "--hostname", "github.com"],
        dir,
        None,
    )?;
    let text = format!("{}\n{}", status.stdout, status.stderr);
    if !text.contains("Logged in to github.com") {
        return Err(Failure::new(
            codes::GH_SIGNED_OUT,
            "gh isn't signed in to github.com",
        ));
    }
    if let Some(scopes) = scopes(&text) {
        let missing: Vec<&str> = SCOPES
            .iter()
            .copied()
            .filter(|s| !scopes.iter().any(|have| have == s))
            .collect();
        if !missing.is_empty() {
            return Err(Failure::new(
                codes::GH_SCOPE,
                format!("gh's sign-in lacks the scope {}", missing.join(", ")),
            )
            .help(format!(
                "run `gh auth refresh -h github.com -s {}`, then run this again",
                missing.join(",")
            )));
        }
    }
    let login = must(
        host,
        "gh api user",
        "gh",
        &["api", "user", "--jq", ".login"],
        dir,
    )?;
    Ok(login.trim().to_owned())
}

/// The game's folder as a git repository, its branch and its `origin`.
fn local_git(host: &mut dyn Host, root: &Path) -> Result<(Git, String, Option<String>), Failure> {
    let top = run(host, "git", &["rev-parse", "--show-toplevel"], root, None)?;
    let same = |top: &str| {
        let canonical = |p: &Path| fs::canonicalize(p).ok();
        canonical(Path::new(top.trim())).is_some()
            && canonical(Path::new(top.trim())) == canonical(root)
    };
    if !top.ok || !same(&top.stdout) {
        return Ok((Git::None, "main".to_owned(), None));
    }
    let head = run(
        host,
        "git",
        &["rev-parse", "--verify", "--quiet", "HEAD"],
        root,
        None,
    )?;
    let git = if head.ok {
        Git::Committed
    } else {
        Git::NoCommit
    };
    let branch = run(
        host,
        "git",
        &["symbolic-ref", "--short", "HEAD"],
        root,
        None,
    )?;
    let branch = if branch.ok && !branch.stdout.trim().is_empty() {
        branch.stdout.trim().to_owned()
    } else {
        "main".to_owned()
    };
    let origin = run(host, "git", &["remote", "get-url", "origin"], root, None)?;
    let origin = origin.ok.then(|| origin.stdout.trim().to_owned());
    Ok((git, branch, origin))
}

/// The game's repository: `origin`'s, else the owner and name asked (the
/// signed-in user, and the crate name of `shuyi.ron`'s `app.name`).
fn game_slug(
    project: &Project,
    args: &Args,
    login: &str,
    origin: Option<&str>,
) -> Result<Slug, Failure> {
    if let Some(origin) = origin {
        return Slug::of_url(origin).ok_or_else(|| {
            Failure::new(
                codes::GITHUB,
                format!("this folder's origin, {origin}, isn't a repository on GitHub"),
            )
            .help("repos makes repositories on GitHub only: remove origin (`git remote remove origin`) to make one there")
        });
    }
    let name = match (&args.name, &project.config.app) {
        (Some(name), _) => name.clone(),
        (None, Some(app)) => crate_name(&app.name).map_err(|e| {
            Failure::new(codes::REPO_NAME, e).help("pass the repository's name as --name")
        })?,
        (None, None) => {
            return Err(Failure::new(
                codes::PROJECT_FILE,
                "shuyi.ron has no `app` whose name names the repository",
            )
            .help("pass the repository's name as --name"));
        }
    };
    Ok(Slug {
        owner: args.owner.clone().unwrap_or_else(|| login.to_owned()),
        name,
    })
}

/// The private repository `rules.url` names.
fn private_slug(rules: &PrivateAssets) -> Result<Slug, Failure> {
    Slug::of_url(&rules.url).ok_or_else(|| {
        Failure::new(
            codes::PROJECT_FILE,
            format!(
                "shuyi.ron's private_assets.url, `{}`, isn't a repository on GitHub",
                rules.url
            ),
        )
        .help("write it as https://github.com/<owner>/<repository>.git")
    })
}

/// The pin file: missing or empty, else whether it has a change that
/// isn't committed.
fn pin_state(
    host: &mut dyn Host,
    root: &Path,
    rules: &PrivateAssets,
    git: Git,
) -> Result<Pin, Failure> {
    let pin =
        fs::read_to_string(root.join(&rules.pin_file)).is_ok_and(|text| !text.trim().is_empty());
    if !pin {
        return Ok(Pin::Missing);
    }
    if git == Git::None {
        return Ok(Pin::Changed);
    }
    let status = must(
        host,
        "git status",
        "git",
        &["status", "--porcelain", "--", &rules.pin_file],
        root,
    )?;
    Ok(if status.trim().is_empty() {
        Pin::Committed
    } else {
        Pin::Changed
    })
}

/// The game's repository on GitHub, if it is there. A repository with
/// the folder's branch, when the folder has no `origin`, is taken.
fn observe_game(
    host: &mut dyn Host,
    root: &Path,
    game: &Slug,
    state: &State,
) -> Result<Option<GameRepo>, Failure> {
    if !repo_there(host, root, game)? {
        return Ok(None);
    }
    let branch = format!("repos/{}/branches/{}", game.path(), state.branch);
    let pushed = matches!(gh_get(host, root, &branch)?, Got::Json(_));
    if state.origin.is_none() && pushed {
        return Err(Failure::new(
            codes::REPO_NAME,
            format!(
                "{} is taken: it already has a branch {}, and this folder has no origin",
                game.url(),
                state.branch
            ),
        )
        .help(format!(
            "if it is this game's, add it (`git remote add origin {}.git`); else pick another \
             name with --name. Then run this again",
            game.url()
        )));
    }
    let mut secrets = Vec::new();
    for (key, path) in [(Use::Builds, "actions"), (Use::Dependabot, "dependabot")] {
        let list = format!("repos/{}/{path}/secrets?per_page=100", game.path());
        if let Got::Json(json) = gh_get(host, root, &list)?
            && secret_names(&json).contains(&SECRET)
        {
            secrets.push(key);
        }
    }
    Ok(Some(GameRepo { pushed, secrets }))
}

/// The private repository on GitHub, if it is there.
fn observe_private(
    host: &mut dyn Host,
    root: &Path,
    repo: &Slug,
) -> Result<Option<PrivateRepo>, Failure> {
    if !repo_there(host, root, repo)? {
        return Ok(None);
    }
    let keys = deploy_keys(host, root, repo)?;
    // A private repository of a free personal account has no rulesets:
    // GitHub refuses to list them.
    let rulesets = format!("repos/{}/rulesets", repo.path());
    let ruleset = match gh_get(host, root, &rulesets) {
        Ok(Got::Json(json)) => json
            .as_array()
            .is_some_and(|list| list.iter().any(|r| r["name"] == RULESET)),
        Ok(Got::Missing) | Err(_) => false,
    };
    Ok(Some(PrivateRepo { keys, ruleset }))
}

/// Looks, with read-only calls only, at the tools, the game's folder and
/// GitHub: what was asked, resolved, and what is there.
pub fn observe(
    host: &mut dyn Host,
    project: &Project,
    args: &Args,
) -> Result<(Inputs, State), Failure> {
    let root = project.root.as_path();
    let rules = project.config.private_assets.as_ref();
    let login = check_tools(host, root, rules.is_some())?;
    let (git, branch, origin) = local_git(host, root)?;
    let game = game_slug(project, args, &login, origin.as_deref())?;
    let mut state = State {
        git,
        branch,
        origin,
        ..State::default()
    };
    state.game_repo = observe_game(host, root, &game, &state)?;
    let private = match rules {
        None => None,
        Some(rules) => {
            let repo = private_slug(rules)?;
            state.private_repo = observe_private(host, root, &repo)?;
            state.pin = pin_state(host, root, rules, state.git)?;
            state.checkout = root.join(&rules.checkout).join(".git").exists();
            Some(Private {
                repo,
                visibility: args.assets_visibility,
                folders: args.folders.clone(),
            })
        }
    };
    let inputs = Inputs {
        game,
        visibility: args.visibility,
        private,
        new_keys: args.new_keys,
    };
    Ok((inputs, state))
}

/// The deploy keys of `repo`.
fn deploy_keys(host: &mut dyn Host, dir: &Path, repo: &Slug) -> Result<Vec<DeployKey>, Failure> {
    let path = format!("repos/{}/keys?per_page=100", repo.path());
    let Got::Json(json) = gh_get(host, dir, &path)? else {
        return Ok(Vec::new());
    };
    Ok(json
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|k| {
                    Some(DeployKey {
                        id: k["id"].as_u64()?,
                        title: k["title"].as_str()?.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// The ruleset `repos` puts on the private repository's default branch:
/// no deletion, no force push, a pull request required, and the
/// repository's admin role may bypass it.
pub fn ruleset_json() -> Value {
    serde_json::json!({
        "name": RULESET,
        "target": "branch",
        "enforcement": "active",
        "conditions": {
            "ref_name": { "include": ["~DEFAULT_BRANCH"], "exclude": [] }
        },
        "rules": [
            { "type": "deletion" },
            { "type": "non_fast_forward" },
            {
                "type": "pull_request",
                "parameters": {
                    "required_approving_review_count": 0,
                    "dismiss_stale_reviews_on_push": false,
                    "require_code_owner_review": false,
                    "require_last_push_approval": false,
                    "required_review_thread_resolution": false
                }
            }
        ],
        // GitHub's id of the repository role `admin`.
        "bypass_actors": [
            { "actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always" }
        ]
    })
}

/// What the private repository's first commit holds: each file's path
/// and text.
pub fn private_files(
    game_name: &str,
    game: &Slug,
    command: &str,
    folders: &[String],
) -> Vec<(String, String)> {
    let mut kinds = String::new();
    for folder in folders {
        let holds = match folder.as_str() {
            "text" => "The game's writing: its text and dialogue.",
            "sprites" => "Pictures: sprites, tiles, portraits.",
            "sounds" => "Sound effects.",
            "music" => "Music.",
            "data" => "The game's data: its stages, units and tables.",
            _ => "Files the game reads from that folder.",
        };
        // Writing to a `String` can't fail.
        let _ = writeln!(kinds, "| `{PRIVATE_FILES}/{folder}/` | {holds} |");
    }
    let readme = README
        .replace("{{name}}", game_name)
        .replace("{{game}}", &game.url())
        .replace("{{command}}", command)
        .replace("{{kinds}}\n", &kinds);
    let mut files = vec![
        ("README.md".to_owned(), readme),
        (".gitattributes".to_owned(), GITATTRIBUTES.to_owned()),
        (".gitignore".to_owned(), GITIGNORE.to_owned()),
    ];
    let mut keep = |folder: String| files.push((format!("{folder}/.gitkeep"), String::new()));
    if folders.is_empty() {
        keep(PRIVATE_FILES.to_owned());
    }
    for folder in folders {
        keep(format!("{PRIVATE_FILES}/{folder}"));
    }
    keep(DOCS.to_owned());
    keep("library".to_owned());
    files
}

/// How a step went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// To be made (`--dry-run`).
    Planned,
    /// Made by this run.
    Made,
    /// There already.
    There,
    /// Failed: the run stopped here.
    Failed,
    /// Not run, after a failure.
    NotRun,
}

impl Status {
    fn word(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Made => "made",
            Self::There => "there",
            Self::Failed => "FAILED",
            Self::NotRun => "not run",
        }
    }
}

/// A step in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Line {
    /// Its id ([`Thing::id`]).
    pub step: &'static str,
    /// How it went.
    pub status: Status,
    /// What it makes, or found.
    pub about: String,
}

/// What `repos` did, or would do.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Whether it succeeded (warnings or not).
    pub ok: bool,
    /// Whether it only planned.
    pub dry_run: bool,
    /// The game's repository.
    pub game: Option<String>,
    /// The private repository.
    pub private: Option<String>,
    /// Each step.
    pub steps: Vec<Line>,
    /// What didn't fail the run but needs a look.
    pub warnings: Vec<String>,
    /// The problem code of the error.
    pub code: Option<String>,
    /// The error.
    pub error: Option<String>,
    /// How to fix it.
    pub help: Option<String>,
    /// What to do next.
    pub next: Option<String>,
}

impl Report {
    fn failed(mut self, failure: Failure) -> Self {
        self.ok = false;
        self.help = failure
            .help
            .or_else(|| codes::help_of(failure.code.0).map(str::to_owned));
        self.code = Some(failure.code.0.to_owned());
        self.error = Some(failure.message);
        self
    }

    /// The exit code: 0 for success, else the error's class's.
    pub fn exit_code(&self) -> u8 {
        if self.ok {
            0
        } else {
            codes::exit_code(self.code.as_deref())
        }
    }

    /// For people.
    pub fn text(&self) -> String {
        let mut out = String::new();
        if self.dry_run {
            out.push_str("repos --dry-run: the plan; nothing was changed\n");
        }
        // Writing to a `String` can't fail.
        if let Some(game) = &self.game {
            let _ = writeln!(out, "the game:          {game}");
        }
        if let Some(private) = &self.private {
            let _ = writeln!(out, "its private files: {private}");
        }
        for line in &self.steps {
            let _ = writeln!(out, "  {:<8} {}", line.status.word(), line.about);
        }
        if self.ok && self.steps.iter().all(|l| l.status == Status::There) {
            out.push_str("everything is there: nothing was made\n");
        }
        for warning in &self.warnings {
            let _ = writeln!(out, "warning: {warning}");
        }
        if let Some(next) = &self.next {
            let _ = writeln!(out, "next: {next}");
        }
        out
    }
}

/// What a step makes, or found, in a line.
fn about(step: &Step, inputs: &Inputs, state: &State, rules: Option<&PrivateAssets>) -> String {
    let game = &inputs.game;
    let private = inputs.private.as_ref();
    let private_url = private.map(|p| p.repo.url()).unwrap_or_default();
    let pin_file = rules.map_or("private.rev", |r| r.pin_file.as_str());
    match step.thing {
        Thing::Git => "the game's folder as a git repository, with a first commit".to_owned(),
        Thing::GameRepo => match (&state.origin, step.make) {
            (Some(origin), _) => format!("origin: {origin}, left as it is"),
            (None, _) => format!("{} ({}), as origin", game.url(), inputs.visibility.name()),
        },
        Thing::PrivateRepo => {
            let visibility = private.map_or("private", |p| p.visibility.name());
            let folders = private.map(|p| p.folders.join(", ")).unwrap_or_default();
            format!(
                "{private_url} ({visibility}), its README and files/ ({folders}), docs/, library/"
            )
        }
        Thing::Pin => format!("{pin_file} names a commit of {private_url}"),
        Thing::Checkout => format!(
            "the private files checked out in {}/",
            rules.map_or("private", |r| r.checkout.as_str())
        ),
        Thing::Key(key) => {
            let kind = match key {
                Use::Builds => "Actions",
                Use::Dependabot => "Dependabot",
            };
            let replaced = if step.replaces.is_empty() {
                String::new()
            } else {
                format!(", replacing {} old one(s)", step.replaces.len())
            };
            format!(
                "the read-only deploy key \"{}\" of {}, its private half the {kind} secret \
                 {SECRET} of {}{replaced}",
                key.title(&game.name),
                private.map(|p| p.repo.path()).unwrap_or_default(),
                game.path()
            )
        }
        Thing::Ruleset => format!(
            "the ruleset \"{RULESET}\" on {}'s default branch: no deletion, no force push, a pull \
             request, admins may bypass",
            private.map(|p| p.repo.path()).unwrap_or_default()
        ),
        Thing::PinCommit => format!("{pin_file} committed (\"Pin the private files\")"),
        Thing::Push => format!("{} pushed to origin", state.branch),
    }
}

/// Everything `make` needs.
struct Ctx<'a> {
    host: &'a mut dyn Host,
    project: &'a Project,
    inputs: &'a Inputs,
    state: &'a State,
}

impl Ctx<'_> {
    fn root(&self) -> &Path {
        &self.project.root
    }

    fn git(&mut self, what: &str, args: &[&str]) -> Result<(), Failure> {
        let root = self.project.root.clone();
        must(self.host, what, "git", args, &root).map(drop)
    }

    fn gh(&mut self, what: &str, args: &[&str], stdin: Option<&str>) -> Result<Ran, Failure> {
        let root = self.project.root.clone();
        let ran = run(self.host, "gh", args, &root, stdin)?;
        if ran.ok {
            Ok(ran)
        } else {
            Err(Failure::step(what, &ran))
        }
    }

    fn private(&self) -> Result<(&Private, &PrivateAssets), Failure> {
        let rules = self.project.config.private_assets.as_ref();
        match (&self.inputs.private, rules) {
            (Some(private), Some(rules)) => Ok((private, rules)),
            _ => Err(Failure::new(
                codes::PROJECT_FILE,
                "shuyi.ron has no private_assets",
            )),
        }
    }
}

/// Makes what `step` makes. A warning is what didn't fail the run but
/// needs a look.
fn make(ctx: &mut Ctx, step: &Step) -> Result<Option<String>, Failure> {
    match step.thing {
        Thing::Git => {
            if ctx.state.git == Git::None {
                ctx.git("git init", &["init", "--quiet", "-b", "main"])?;
            }
            ctx.git("git add", &["add", "--all"])?;
            ctx.git(
                "the first commit",
                &["commit", "--quiet", "-m", "First commit"],
            )?;
        }
        Thing::GameRepo => make_game_repo(ctx)?,
        Thing::PrivateRepo => make_private_repo(ctx)?,
        Thing::Pin => make_pin(ctx)?,
        Thing::Checkout => {
            let (private, rules) = ctx.private()?;
            let (url, checkout) = (private.repo.url(), rules.checkout.clone());
            let command = ctx.project.config.command().to_owned();
            if let Err(e) = ctx.host.fetch_private(ctx.project) {
                return Ok(Some(format!(
                    "the private files weren't checked out into {checkout}/: {e}\n  run `{command} \
                     private-assets` once git can read {url} (`gh auth setup-git` lets git use \
                     gh's sign-in)"
                )));
            }
        }
        Thing::Key(key) => make_key(ctx, key, &step.replaces)?,
        Thing::Ruleset => return make_ruleset(ctx),
        Thing::PinCommit => {
            let pin_file = ctx.private()?.1.pin_file.clone();
            ctx.git("git add", &["add", "--", &pin_file])?;
            ctx.git(
                "committing the pin",
                &[
                    "commit",
                    "--quiet",
                    "-m",
                    "Pin the private files",
                    "--",
                    &pin_file,
                ],
            )?;
        }
        Thing::Push => {
            let branch = ctx.state.branch.clone();
            ctx.git(
                "the push",
                &["push", "--quiet", "--set-upstream", "origin", &branch],
            )
            .map_err(|f| {
                f.help(
                    "fix what git said (`gh auth setup-git` lets git use gh's sign-in), then run \
                     this again: it pushes only what isn't",
                )
            })?;
        }
    }
    Ok(None)
}

fn make_game_repo(ctx: &mut Ctx) -> Result<(), Failure> {
    let game = ctx.inputs.game.clone();
    if ctx.state.game_repo.is_some() {
        // Made by a run that stopped before it was added: still empty.
        let url = format!("{}.git", game.url());
        return ctx.git("git remote add", &["remote", "add", "origin", &url]);
    }
    let visibility = format!("--{}", ctx.inputs.visibility.name());
    ctx.gh(
        "gh repo create",
        &[
            "repo",
            "create",
            &game.path(),
            &visibility,
            "--source",
            ".",
            "--remote",
            "origin",
        ],
        None,
    )
    .map(drop)
}

fn make_private_repo(ctx: &mut Ctx) -> Result<(), Failure> {
    let (private, _) = ctx.private()?;
    let private = private.clone();
    let name = ctx
        .project
        .config
        .app
        .as_ref()
        .map_or_else(|| ctx.inputs.game.name.clone(), |app| app.name.clone());
    let files = private_files(
        &name,
        &ctx.inputs.game,
        ctx.project.config.command(),
        &private.folders,
    );
    let temp =
        TempDir::new(&ctx.host.temp_root(), "private").map_err(|e| Failure::new(codes::FILE, e))?;
    let dir = temp.path();
    for (path, text) in &files {
        let file = dir.join(path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|e| write_failed(&file, &e))?;
        }
        fs::write(&file, text).map_err(|e| write_failed(&file, &e))?;
    }
    let git = |host: &mut dyn Host, what: &str, args: &[&str]| must(host, what, "git", args, dir);
    git(ctx.host, "git init", &["init", "--quiet", "-b", "main"])?;
    git(ctx.host, "git add", &["add", "--all"])?;
    git(
        ctx.host,
        "the private repository's first commit",
        &["commit", "--quiet", "-m", "First commit"],
    )?;
    let visibility = format!("--{}", private.visibility.name());
    let description = format!("Private files of {name}. Must stay private.");
    let ran = run(
        ctx.host,
        "gh",
        &[
            "repo",
            "create",
            &private.repo.path(),
            &visibility,
            "--source",
            ".",
            "--remote",
            "origin",
            "--push",
            "--description",
            &description,
        ],
        dir,
        None,
    )?;
    if ran.ok {
        Ok(())
    } else {
        Err(Failure::step("gh repo create", &ran))
    }
}

fn write_failed(file: &Path, e: &std::io::Error) -> Failure {
    Failure::new(codes::FILE, format!("can't write {}: {e}", file.display()))
}

fn make_pin(ctx: &mut Ctx) -> Result<(), Failure> {
    let (private, rules) = ctx.private()?;
    let (repo, pin_file) = (private.repo.clone(), rules.pin_file.clone());
    let path = format!("repos/{}/commits/main", repo.path());
    let ran = ctx.gh(
        "finding the private repository's main",
        &["api", &path, "--jq", ".sha"],
        None,
    )?;
    let sha = ran.stdout.trim();
    let is_hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
    if sha.len() != 40 || !sha.chars().all(is_hex) {
        return Err(Failure::new(
            codes::GITHUB,
            format!("{}'s main is `{sha}`, not a commit id", repo.path()),
        ));
    }
    let file = ctx.root().join(&pin_file);
    fs::write(&file, format!("{sha}\n")).map_err(|e| write_failed(&file, &e))
}

/// Makes the deploy key for `key` and sets its secret, after deleting the
/// old keys `replaces` names. The private half goes to `gh` on stdin only,
/// and its folder is removed whatever happens.
fn make_key(ctx: &mut Ctx, key: Use, replaces: &[u64]) -> Result<(), Failure> {
    let (private, _) = ctx.private()?;
    let repo = private.repo.path();
    let game = ctx.inputs.game.clone();
    for id in replaces {
        let path = format!("repos/{repo}/keys/{id}");
        ctx.gh(
            "deleting an old deploy key",
            &["api", "-X", "DELETE", &path],
            None,
        )?;
    }
    let title = key.title(&game.name);
    let temp =
        TempDir::new(&ctx.host.temp_root(), "key").map_err(|e| Failure::new(codes::FILE, e))?;
    let file = temp.path().join("key");
    let file_arg = file.to_string_lossy().into_owned();
    must(
        ctx.host,
        "ssh-keygen",
        "ssh-keygen",
        &[
            "-q", "-t", "ed25519", "-N", "", "-C", &title, "-f", &file_arg,
        ],
        temp.path(),
    )?;
    let public = file.with_file_name("key.pub");
    let secret = fs::read_to_string(&file)
        .map_err(|e| Failure::new(codes::FILE, format!("ssh-keygen wrote no key: {e}")))?;
    let public_arg = public.to_string_lossy().into_owned();
    ctx.gh(
        "gh repo deploy-key add",
        &[
            "repo",
            "deploy-key",
            "add",
            &public_arg,
            "--repo",
            &repo,
            "--title",
            &title,
        ],
        None,
    )?;
    drop(temp);
    let mut args = vec!["secret", "set", SECRET, "--repo"];
    let game_path = game.path();
    args.push(&game_path);
    if key == Use::Dependabot {
        args.extend(["--app", "dependabot"]);
    }
    ctx.gh("gh secret set", &args, Some(&secret)).map(drop)
}

fn make_ruleset(ctx: &mut Ctx) -> Result<Option<String>, Failure> {
    let (private, _) = ctx.private()?;
    let repo = private.repo.path();
    let path = format!("repos/{repo}/rulesets");
    let body = ruleset_json().to_string();
    let root = ctx.root().to_path_buf();
    let ran = run(
        ctx.host,
        "gh",
        &["api", &path, "-X", "POST", "--input", "-"],
        &root,
        Some(&body),
    )?;
    if ran.ok {
        return Ok(None);
    }
    if ran.said().contains("(HTTP 403)") {
        return Ok(Some(format!(
            "GitHub refused the ruleset on {repo} (a private repository of a free personal \
             account has none): pushing to its main is not blocked, so make each change there \
             by pull request"
        )));
    }
    Err(Failure::step("making the ruleset", &ran))
}

/// What to say for a game with no private files.
fn no_private(command: &str) -> String {
    format!(
        "this game has no private files (shuyi.ron has no private_assets): only its repository \
         was set up. A game whose writing, art or sounds must stay private is made with `{command} \
         new … --set private-files=yes --private-url https://github.com/<you>/<repo>.git` (the \
         book's quick start, *Make a game*)"
    )
}

/// Runs `repos` on `project` with `args`, through `host`.
pub fn repos(project: &Project, args: &Args, host: &mut dyn Host) -> Report {
    let mut report = Report {
        dry_run: args.dry_run,
        ..Report::default()
    };
    let (inputs, state) = match observe(host, project, args) {
        Ok(found) => found,
        Err(failure) => return report.failed(failure),
    };
    report.game = Some(inputs.game.url());
    report.private = inputs.private.as_ref().map(|p| p.repo.url());
    let rules = project.config.private_assets.as_ref();
    let command = project.config.command();
    if rules.is_none() {
        report.warnings.push(no_private(command));
    }
    let steps = plan(&inputs, &state);
    let mut ctx = Ctx {
        host,
        project,
        inputs: &inputs,
        state: &state,
    };
    let mut failure = None;
    for step in &steps {
        let status = if failure.is_some() {
            Status::NotRun
        } else if !step.make {
            Status::There
        } else if args.dry_run {
            Status::Planned
        } else {
            match make(&mut ctx, step) {
                Ok(warning) => {
                    report.warnings.extend(warning);
                    Status::Made
                }
                Err(f) => {
                    failure = Some(f);
                    Status::Failed
                }
            }
        };
        report.steps.push(Line {
            step: step.thing.id(),
            status,
            about: about(step, &inputs, &state, rules),
        });
    }
    if let Some(failure) = failure {
        return report.failed(failure);
    }
    report.ok = true;
    if !args.dry_run {
        report.next = Some(format!(
            "`{command} run` plays the game; its CI runs at {}/actions",
            inputs.game.url()
        ));
    }
    report
}

/// `repos` from the command line, on the machine: the exit code.
pub fn main(project: &Project, args: &[String]) -> u8 {
    let command = project.config.command();
    let args = match parse_args(args) {
        Ok(Parsed::Run(args)) => args,
        Ok(Parsed::Help) => {
            println!("{}", help(command));
            return 0;
        }
        Err(e) => {
            eprint!(
                "{}",
                codes::said("repos", codes::WRONG_ARGUMENTS.0, &e, None)
            );
            eprintln!("{}", usage(command));
            return 2;
        }
    };
    let report = repos(project, &args, &mut ThisMachine);
    if args.json {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("repos: {e}"),
        }
    } else {
        print!("{}", report.text());
        if let (Some(code), Some(error)) = (&report.code, &report.error) {
            eprint!(
                "{}",
                codes::said("repos", code, error, report.help.as_deref())
            );
        }
    }
    report.exit_code()
}

#[cfg(test)]
pub(crate) mod fake;
#[cfg(test)]
mod tests;
