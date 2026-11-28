//! `shuyi new` from the command line: the wizard's questions
//! ([`prompts::ask`]: the name, the id, the perspectives, the catalog's,
//! the template's with the worked examples (ADR-0113), the targets), then
//! the questions about the game's repositories (ADR-0112): whether to put
//! the game on GitHub, each repository's visibility, and which kinds of
//! files stay private. Each is asked through a [`Prompter`], so a test
//! answers from a script; a flag answers each one ([`Flags`]), and none is
//! asked off a terminal ([`on_a_terminal`]). `--questions` (the input's
//! `questions`) lists them all
//! ([`listed`]), the GitHub ones as [`github_questions`] makes them.
//!
//! With `--github`, [`new_with`] composes the private part with the private
//! repository's address (`https://github.com/<owner>/<crate name>-assets.git`),
//! writes the game, then runs `repos` ([`crate::repos`]) in its folder. The
//! contract's `new` (`--json`) and the MCP server's `new` tool never make a
//! repository: they act on no account.

use std::fmt::Write as _;
use std::io::{self, IsTerminal as _};
use std::path::Path;

use super::prompts::{self, Prompter, pick};
use super::wizard::{Question, Questions, Takes, engine_list, engine_text};
use super::{NewInput, NewOutput, PRIVATE_FILES, name_of, new, parse_args};
use crate::contract::codes::{self, Code};
use crate::contract::{Answer, Op};
use crate::project::Project;
use crate::repos::{self, DEFAULT_FOLDERS, Host, Report, Slug, Visibility};
use crate::template::EXAMPLES;

/// The kinds of files a game may keep private, each the folder of
/// `files/` it makes, with what it holds.
pub const KINDS: [(&str, &str); 5] = [
    ("text", "writing: dialogue, scripts…"),
    ("sprites", "pictures: sprites, tiles, portraits…"),
    ("sounds", "sound effects"),
    ("music", "music"),
    ("data", "data: stages, units, tables…"),
];

/// `new`'s options taking a value, whose value is never one of [`Flags`]'.
/// The wizard's flags of the catalog's concerns take one too
/// ([`super::wizard::concern_flag`]).
const TAKES_A_VALUE: [&str; 10] = [
    "--template",
    "--answers",
    "--set",
    "--name",
    "--id",
    "--storage",
    "--targets",
    "--engine-rev",
    "--engine-path",
    "--private-url",
];

/// The answers the command line gives to the questions; `None` for one it
/// leaves to ask.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flags {
    /// `--examples` or `--no-examples`: the template's `examples` question.
    pub examples: Option<bool>,
    /// `--github` or `--no-github`.
    pub github: Option<bool>,
    /// `--owner`: the repositories' owner; the signed-in user if none.
    pub owner: Option<String>,
    /// `--visibility`: the game's repository's.
    pub visibility: Option<Visibility>,
    /// `--assets-visibility`: the private files' repository's.
    pub assets_visibility: Option<Visibility>,
    /// `--private`: the kinds kept private ([`KINDS`]), empty for `none`.
    pub private: Option<Vec<String>>,
    /// `--private-folder`s: the developer's own folders, under `files/`.
    pub folders: Option<Vec<String>>,
    /// `--no-input`: nothing asked, as off a terminal: every question
    /// the flags leave takes its default, and no repository is made.
    pub no_input: bool,
}

/// Reads `new`'s arguments (not `--json`): [`parse_args`]' and the flags
/// that answer the questions.
pub fn parse(args: &[String]) -> Result<(NewInput, Flags), String> {
    let mut flags = Flags::default();
    let mut rest = Vec::new();
    let mut folders = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--examples" => flags.examples = Some(true),
            "--no-examples" => flags.examples = Some(false),
            "--github" => flags.github = Some(true),
            "--no-github" => flags.github = Some(false),
            "--owner" => {
                let owner = value()?;
                repos::check_owner(&owner)?;
                flags.owner = Some(owner);
            }
            "--visibility" => flags.visibility = Some(Visibility::parse(arg, &value()?)?),
            "--assets-visibility" => {
                flags.assets_visibility = Some(Visibility::parse(arg, &value()?)?);
            }
            "--private" => {
                flags.private = Some(kinds(&value()?).map_err(|e| format!("--private: {e}"))?);
            }
            "--private-folder" => folders.push(value()?),
            "--no-input" => flags.no_input = true,
            flag if TAKES_A_VALUE.contains(&flag)
                || super::wizard::concern_flag(flag).is_some() =>
            {
                rest.push(arg.clone());
                rest.push(value()?);
            }
            _ => rest.push(arg.clone()),
        }
    }
    if !folders.is_empty() {
        flags.folders = Some(own_folders(&folders.join(","))?);
    }
    Ok((parse_args(&rest)?, flags))
}

/// The kinds `list` picks, in [`KINDS`]' order: names or numbers from 1,
/// joined by commas or spaces (`text, 2`); `none` alone for none.
pub fn kinds(list: &str) -> Result<Vec<String>, String> {
    let words: Vec<&str> = list
        .split([',', ' '])
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .collect();
    let names = KINDS.map(|(kind, _)| kind).join(", ");
    match words.as_slice() {
        [] => return Err(format!("name the kinds ({names}), or none")),
        ["none"] => return Ok(Vec::new()),
        _ => {}
    }
    let mut picked = [false; KINDS.len()];
    for word in words {
        let name = word.strip_prefix("files/").unwrap_or(word);
        let name = name.trim_end_matches('/');
        let at = name
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .filter(|at| *at < KINDS.len())
            .or_else(|| KINDS.iter().position(|(kind, _)| *kind == name))
            .ok_or_else(|| {
                if word == "none" {
                    "`none` goes alone".to_owned()
                } else {
                    format!(
                        "`{word}` is no kind: pick from {names}, or 1 to {}",
                        KINDS.len()
                    )
                }
            })?;
        picked[at] = true;
    }
    Ok(KINDS
        .iter()
        .zip(picked)
        .filter(|(_, picked)| *picked)
        .map(|((kind, _), _)| (*kind).to_owned())
        .collect())
}

/// The developer's own folders `list` names, a comma list, each under
/// `files/` (written with or without it), checked as `repos` checks them
/// ([`repos::folders`]): `docs` and `library` are there anyway.
pub fn own_folders(list: &str) -> Result<Vec<String>, String> {
    let list: Vec<&str> = list
        .split(',')
        .map(|f| {
            let f = f.trim().trim_end_matches('/');
            f.strip_prefix("files/").unwrap_or(f)
        })
        .collect();
    repos::folders(&list.join(","))
        .map_err(|e| e.replacen("--private-folders", "--private-folder", 1))
}

/// The private repository's address for `owner`'s game of crate `krate`.
pub fn private_url(owner: &str, krate: &str) -> String {
    format!("https://github.com/{owner}/{krate}-assets.git")
}

/// The first GitHub question's id: whether to put the game on GitHub.
pub const GITHUB: &str = "github";
/// The game's repository's visibility.
pub const VISIBILITY: &str = "visibility";
/// Whether to keep some files in a private repository.
pub const KEEP_PRIVATE: &str = "keep-private";
/// The kinds of files kept private.
pub const PRIVATE: &str = "private";
/// The developer's own folders kept private.
pub const PRIVATE_FOLDER: &str = "private-folder";
/// The private repository's visibility.
pub const ASSETS_VISIBILITY: &str = "assets-visibility";

/// What `github`'s [`Question::agent`] says.
pub const GITHUB_AGENT: &str = "ask your user, and never answer yes yourself: --github makes \
                                repositories, keys and secrets on their GitHub account; without \
                                it new makes none";

const YES_NO: [(&str, &str); 2] = [("yes", "yes"), ("no", "no")];

fn ids(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

/// The first GitHub question: whether to put the game `name` on GitHub,
/// `default` unless answered; it says when `gh` isn't installed.
pub fn github_question(name: &str, gh: bool, default: bool) -> Question {
    let text = if gh {
        format!("Put {name} on GitHub? It needs the GitHub CLI (gh), signed in.")
    } else {
        format!(
            "Put {name} on GitHub? It needs the GitHub CLI (gh), signed in, and gh isn't \
             installed (https://cli.github.com)."
        )
    };
    let mut question = engine_list(
        GITHUB,
        &text,
        Takes::One,
        &YES_NO,
        ids(&[word(default)]),
        "--github",
    );
    question.agent = Some(GITHUB_AGENT.to_owned());
    question
}

/// The game's repository `repo`'s visibility, `default` unless answered.
pub fn visibility_question(repo: &str, default: Visibility) -> Question {
    engine_list(
        VISIBILITY,
        &format!("Should {repo} be public or private? GitHub's CI is free on a public repository."),
        Takes::One,
        &[("public", "public"), ("private", "private")],
        ids(&[default.name()]),
        "--visibility",
    )
}

/// Whether to keep some files in the private repository `repo`.
pub fn keep_question(repo: &str, default: bool) -> Question {
    engine_list(
        KEEP_PRIVATE,
        &format!(
            "Keep some files in a second, private repository, {repo}, so they aren't in the \
             public code?"
        ),
        Takes::One,
        &YES_NO,
        ids(&[word(default)]),
        "--private",
    )
}

/// Which kinds of files to keep private ([`KINDS`]), `default` unless
/// answered.
pub fn kinds_question(default: &[String]) -> Question {
    let shown: Vec<(String, String)> = KINDS
        .iter()
        .map(|(kind, holds)| ((*kind).to_owned(), format!("files/{kind}/: {holds}")))
        .collect();
    let answers: Vec<(&str, &str)> = shown
        .iter()
        .map(|(id, text)| (id.as_str(), text.as_str()))
        .collect();
    engine_list(
        PRIVATE,
        "Which kinds of files go there?",
        Takes::Many,
        &answers,
        default.to_vec(),
        "--private",
    )
}

/// The developer's own folders under `files/` to keep private.
pub fn folders_question(default: &[String]) -> Question {
    engine_text(
        PRIVATE_FOLDER,
        "Any other folders under files/ to keep there? Several with commas (levels, maps), or \
         none.",
        &default.join(","),
        "--private-folder",
    )
}

/// The private repository `repo`'s visibility, `default` unless answered.
pub fn assets_visibility_question(repo: &str, default: Visibility) -> Question {
    engine_list(
        ASSETS_VISIBILITY,
        &format!("Should {repo} be private or public? Public makes these files anyone's."),
        Takes::One,
        &[("private", "private"), ("public", "public")],
        ids(&[default.name()]),
        "--assets-visibility",
    )
}

/// The GitHub questions for the game of crate `krate` named `name`, as
/// `flags` and `input` answer them so far, for `--questions`: each at the
/// answer it takes with nothing asked, the repositories' after `github`
/// skipped unless `--github`.
pub fn github_questions(name: &str, krate: &str, input: &NewInput, flags: &Flags) -> Vec<Question> {
    let owner = flags.owner.as_deref().unwrap_or("<owner>");
    let repo = format!("{owner}/{krate}");
    let url = input
        .private_url
        .clone()
        .unwrap_or_else(|| private_url(owner, krate));
    let assets = Slug::of_url(&url).map_or_else(|| format!("{owner}/{krate}-assets"), |s| s.path());
    let github = flags.github == Some(true);
    let keep = match (&flags.private, set_private(input)) {
        (Some(kinds), _) => !kinds.is_empty(),
        (None, Some(private)) => private,
        (None, None) => true,
    };
    let kinds = flags
        .private
        .clone()
        .filter(|kinds| !kinds.is_empty())
        .unwrap_or_else(|| DEFAULT_FOLDERS.map(str::to_owned).to_vec());
    let mut questions = vec![
        github_question(name, true, github),
        visibility_question(&repo, flags.visibility.unwrap_or(Visibility::Public)),
        keep_question(&assets, keep),
        kinds_question(&kinds),
        folders_question(flags.folders.as_deref().unwrap_or_default()),
        assets_visibility_question(
            &assets,
            flags.assets_visibility.unwrap_or(Visibility::Private),
        ),
    ];
    for (at, question) in questions.iter_mut().enumerate().skip(1) {
        question.skipped = if !github {
            Some("asked only when the game goes on GitHub (--github)".to_owned())
        } else if at > 2 && !keep {
            Some("no files are kept private (--private none)".to_owned())
        } else {
            None
        };
    }
    questions
}

/// Whether the yes-or-no question `question` is answered yes through
/// `prompter`.
fn yes(prompter: &mut dyn Prompter, question: &Question) -> io::Result<bool> {
    Ok(pick(prompter, question)? == ["yes"])
}

/// The visibility question `question` answered through `prompter`.
fn visibility(prompter: &mut dyn Prompter, question: &Question) -> io::Result<Visibility> {
    Ok(if pick(prompter, question)? == ["private"] {
        Visibility::Private
    } else {
        Visibility::Public
    })
}

/// The kinds of files kept private, asked through `prompter` until one is
/// ticked at least.
fn which_kinds(prompter: &mut dyn Prompter) -> io::Result<Vec<String>> {
    let question = kinds_question(&DEFAULT_FOLDERS.map(str::to_owned));
    loop {
        let picked = pick(prompter, &question)?;
        if !picked.is_empty() {
            return Ok(picked);
        }
        prompter.say("Tick one kind at least: to keep none, answer no to the question before.")?;
    }
}

/// The developer's own folders kept private, typed through `prompter`.
fn other_folders(prompter: &mut dyn Prompter) -> io::Result<Vec<String>> {
    let question = folders_question(&[]);
    let check = |line: &str| own_folders(line).map(|_| ());
    let line = prompter.text(&question.text, "", &check)?;
    own_folders(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}

/// What `new` made, and what `repos` did after it.
#[derive(Debug)]
pub struct Made {
    /// `new`'s answer.
    pub answer: Answer<NewOutput>,
    /// `repos`' report, with `--github` and a game written.
    pub repos: Option<Report>,
    /// After a failure of `repos`: how to finish.
    pub finish: Option<String>,
}

impl Made {
    /// The exit code: `new`'s, else `repos`'.
    pub fn exit_code(&self) -> u8 {
        match &self.repos {
            Some(report) if self.answer.ok => report.exit_code(),
            _ => self.answer.exit_code(),
        }
    }
}

/// What `repos` is asked to make.
struct GitHub {
    args: repos::Args,
}

type Refused = (Code, String);

fn asked<T>(answer: io::Result<T>) -> Result<T, Refused> {
    answer.map_err(|e| (codes::WRONG_ARGUMENTS, e.to_string()))
}

/// The signed-in user's login, from `gh`.
fn login(host: &mut dyn Host, cwd: &Path) -> Result<String, Refused> {
    let why = match host.run("gh", &["api", "user", "--jq", ".login"], cwd, None) {
        Ok(ran) if ran.ok && !ran.stdout.trim().is_empty() => {
            return Ok(ran.stdout.trim().to_owned());
        }
        Ok(ran) => format!("`gh api user` failed: {}", ran.said()),
        Err(e) => format!("gh didn't start ({e}): install the GitHub CLI, https://cli.github.com"),
    };
    Err((
        codes::GITHUB,
        format!(
            "--github needs your GitHub login: {why}. Sign in with `gh auth login`, or pass \
             --owner, or --no-github"
        ),
    ))
}

/// Whether `input`'s `--set` answers `private-files`, and how.
fn set_private(input: &NewInput) -> Option<bool> {
    input
        .set
        .0
        .get(PRIVATE_FILES)
        .map(|picked| picked.iter().any(|answer| answer == "yes"))
}

/// `--examples` or `--no-examples` (`flag`) written into `input`'s `set`;
/// refused when `--set examples=…` says otherwise.
fn flag_examples(input: &mut NewInput, flag: Option<bool>) -> Result<(), Refused> {
    let Some(examples) = flag else {
        return Ok(());
    };
    let answer = word(examples);
    if input
        .set
        .0
        .get(EXAMPLES)
        .is_some_and(|set| set != &[answer])
    {
        return Err((
            codes::WRONG_ARGUMENTS,
            format!(
                "--{}examples and --set {EXAMPLES}=… disagree",
                if examples { "" } else { "no-" }
            ),
        ));
    }
    input
        .set
        .0
        .insert(EXAMPLES.to_owned(), vec![answer.to_owned()]);
    Ok(())
}

/// Settles the questions, asking through `prompter` those no flag
/// answers (the wizard's, [`prompts::ask`], then the GitHub ones), and sets
/// `input`'s private part: what `repos` is to make, if anything.
fn settle(
    cwd: &Path,
    input: &mut NewInput,
    flags: &Flags,
    json: bool,
    mut prompter: Option<&mut dyn Prompter>,
    host: &mut dyn Host,
) -> Result<Option<GitHub>, Refused> {
    let refused = |e: &str| (codes::WRONG_ARGUMENTS, e.to_owned());
    let none = flags.private.as_ref().is_some_and(Vec::is_empty);
    if none && set_private(input) == Some(true) {
        return Err(refused(
            "--private none and --set private-files=yes disagree",
        ));
    }
    if none && flags.folders.is_some() {
        return Err(refused(
            "--private none makes no private repository: drop --private-folder",
        ));
    }
    if json && flags.github == Some(true) {
        return Err(refused(
            "--github isn't for --json: the contract's new makes no repository. Run it without \
             --json",
        ));
    }
    if json {
        prompter = None;
    }
    flag_examples(input, flags.examples)?;
    if let Some(prompter) = prompter.as_deref_mut() {
        asked(prompts::ask(cwd, input, prompter))?;
    }
    // A name that makes no game: `new` says so, with no question first.
    let Ok((name, krate)) = name_of(cwd, input) else {
        return Ok(None);
    };
    let github = match (flags.github, prompter.as_deref_mut()) {
        (Some(github), _) => github,
        (None, Some(prompter)) => {
            let gh = host
                .run("gh", &["--version"], cwd, None)
                .is_ok_and(|ran| ran.ok);
            asked(yes(prompter, &github_question(&name, gh, gh)))?
        }
        (None, None) => false,
    };
    if !github {
        if flags.github.is_some() || prompter.is_none() {
            let given = [
                ("--owner", flags.owner.is_some()),
                ("--visibility", flags.visibility.is_some()),
                ("--assets-visibility", flags.assets_visibility.is_some()),
            ];
            if let Some((flag, _)) = given.iter().find(|(_, given)| *given) {
                return Err(refused(&format!("{flag} is for --github")));
            }
        }
        let wants =
            flags.private.as_ref().is_some_and(|k| !k.is_empty()) || flags.folders.is_some();
        if wants && input.private_url.is_some() && set_private(input).is_none() {
            input
                .set
                .0
                .insert(PRIVATE_FILES.to_owned(), vec!["yes".to_owned()]);
        }
        return Ok(None);
    }
    let owner = match &flags.owner {
        Some(owner) => owner.clone(),
        None => login(host, cwd)?,
    };
    let visibility = match (flags.visibility, prompter.as_deref_mut()) {
        (Some(visibility), _) => visibility,
        (None, Some(prompter)) => asked(self::visibility(
            prompter,
            &visibility_question(&format!("{owner}/{krate}"), Visibility::Public),
        ))?,
        (None, None) => Visibility::Public,
    };
    let url = input
        .private_url
        .clone()
        .unwrap_or_else(|| private_url(&owner, &krate));
    let assets = Slug::of_url(&url).map_or_else(|| format!("{owner}/{krate}-assets"), |s| s.path());
    let keep = match (&flags.private, set_private(input), prompter.as_deref_mut()) {
        (Some(kinds), _, _) => !kinds.is_empty(),
        (None, Some(private), _) => private,
        (None, None, _) if flags.folders.is_some() => true,
        (None, None, Some(prompter)) => asked(yes(prompter, &keep_question(&assets, true)))?,
        (None, None, None) => true,
    };
    let mut args = repos::Args {
        owner: Some(owner),
        visibility,
        folders: Vec::new(),
        ..repos::Args::default()
    };
    if keep {
        (args.folders, args.assets_visibility) = kept(flags, prompter, &assets)?;
        input
            .set
            .0
            .insert(PRIVATE_FILES.to_owned(), vec!["yes".to_owned()]);
        input.private_url = Some(url);
    }
    Ok(Some(GitHub { args }))
}

/// The private repository `assets`' folders under `files/` (the kinds,
/// then the developer's own) and its visibility, asking through `prompter`
/// what no flag answers.
fn kept(
    flags: &Flags,
    mut prompter: Option<&mut dyn Prompter>,
    assets: &str,
) -> Result<(Vec<String>, Visibility), Refused> {
    let mut folders = match (&flags.private, prompter.as_deref_mut()) {
        (Some(kinds), _) => kinds.clone(),
        (None, Some(prompter)) => asked(which_kinds(prompter))?,
        (None, None) => DEFAULT_FOLDERS.map(str::to_owned).to_vec(),
    };
    let own = match (&flags.folders, prompter.as_deref_mut()) {
        (Some(own), _) => own.clone(),
        (None, Some(prompter)) => asked(other_folders(prompter))?,
        (None, None) => Vec::new(),
    };
    let visibility = match (flags.assets_visibility, prompter) {
        (Some(visibility), _) => visibility,
        (None, Some(prompter)) => asked(self::visibility(
            prompter,
            &assets_visibility_question(assets, Visibility::Private),
        ))?,
        (None, None) => Visibility::Private,
    };
    for folder in own {
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    Ok((folders, visibility))
}

/// Whether `new` runs where someone can answer: stdin, stdout and stderr
/// (where the prompts are drawn) a terminal. Never in a test, whose stdin
/// may be one.
pub fn on_a_terminal() -> bool {
    !cfg!(test)
        && io::stdin().is_terminal()
        && io::stdout().is_terminal()
        && io::stderr().is_terminal()
}

/// `yes` or `no`, as an answer's id.
fn word(yes: bool) -> &'static str {
    if yes { "yes" } else { "no" }
}

/// How to finish what a failed `repos` left: the command to run in the
/// game's folder `dir`, with the same answers.
fn finish(command: &str, dir: &str, args: &repos::Args) -> String {
    let mut line = format!("{command} repos");
    if let Some(owner) = &args.owner {
        let _ = write!(line, " --owner {owner}");
    }
    if args.visibility != Visibility::Public {
        let _ = write!(line, " --visibility {}", args.visibility.name());
    }
    if args.assets_visibility != Visibility::Private {
        let _ = write!(
            line,
            " --assets-visibility {} --yes",
            args.assets_visibility.name()
        );
    }
    if !args.folders.is_empty() && args.folders != DEFAULT_FOLDERS.map(str::to_owned) {
        let _ = write!(line, " --private-folders {}", args.folders.join(","));
    }
    format!(
        "the game is written in {dir}, but not yet on GitHub: fix what failed, then run `{line}` \
         in {dir} to finish (it keeps what was made)"
    )
}

/// `new` from the command line: settles the questions (asking through
/// `prompter` those no flag answers), makes the game under `cwd`, then,
/// with `--github`, runs `repos` in its folder through `host`.
pub fn new_with(
    cwd: &Path,
    mut input: NewInput,
    flags: &Flags,
    json: bool,
    prompter: Option<&mut dyn Prompter>,
    host: &mut dyn Host,
) -> Made {
    let github = match settle(cwd, &mut input, flags, json, prompter, host) {
        Ok(github) => github,
        Err((code, e)) => {
            return Made {
                answer: Answer::failed(Op::New, code, e),
                repos: None,
                finish: None,
            };
        }
    };
    let answer = new(cwd, &input);
    let Some(GitHub { args }) = github.filter(|_| answer.ok) else {
        return Made {
            answer,
            repos: None,
            finish: None,
        };
    };
    let report = match Project::load(&cwd.join(&input.dir)) {
        Ok(project) => {
            let report = repos::repos(&project, &args, host);
            let command = project.config.command().to_owned();
            (report, command)
        }
        Err(e) => (
            Report {
                code: Some(codes::PROJECT_FILE.0.to_owned()),
                error: Some(e),
                ..Report::default()
            },
            "shuyi".to_owned(),
        ),
    };
    let (report, command) = report;
    let finish = (!report.ok).then(|| finish(&command, &input.dir, &args));
    Made {
        answer,
        repos: Some(report),
        finish,
    }
}

/// `new <dir> --questions` (and the MCP `new` tool with `questions`, with
/// [`Flags::default`]): the questions for the answers `input` and `flags`
/// give so far, from `cwd` ([`super::wizard::listed`]), then the GitHub
/// questions ([`github_questions`]); nothing is asked or made.
pub fn listed(cwd: &Path, mut input: NewInput, flags: &Flags) -> Answer<Questions> {
    if let Err((code, e)) = flag_examples(&mut input, flags.examples) {
        return Answer::failed(Op::New, code, e);
    }
    let mut listed = super::wizard::listed(cwd, &input);
    if let (Some(questions), Ok((name, krate))) = (listed.result.as_mut(), name_of(cwd, &input)) {
        questions
            .questions
            .extend(github_questions(&name, &krate, &input, flags));
    }
    listed
}

/// `new` on this machine, from the command line: prints what it made
/// as `json` says, then `repos`' report; the exit code.
pub fn main(cwd: &Path, input: NewInput, flags: &Flags, json: bool) -> Made {
    let mut terminal = prompts::Terminal::new();
    let prompter =
        (!json && !flags.no_input && on_a_terminal()).then_some(&mut terminal as &mut dyn Prompter);
    new_with(cwd, input, flags, json, prompter, &mut repos::ThisMachine)
}

/// `repos`' part of `made`, for people: its report, its error and how to
/// finish, the error and the finish on stderr.
pub fn print_repos(made: &Made) {
    let Some(report) = &made.repos else {
        return;
    };
    print!("{}", report.text());
    if let (Some(code), Some(error)) = (&report.code, &report.error) {
        eprint!(
            "{}",
            codes::said("repos", code, error, report.help.as_deref())
        );
    }
    if let Some(finish) = &made.finish {
        eprintln!("{finish}");
    }
}

#[cfg(test)]
mod tests;
