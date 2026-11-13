//! `shuyi new`'s questions: whether to include the skills' worked examples
//! (ADR-0113), then about the game's repositories (ADR-0112): whether to
//! put the game on GitHub, each repository's visibility, and which kinds of
//! files stay private. Each question is a function from a [`Terminal`]
//! (its input and its writer) to its answer, so a test feeds it lines; a
//! flag answers each one ([`Flags`]), and none is asked off a terminal
//! ([`on_a_terminal`]).
//!
//! With `--github`, [`new_with`] composes the private part with the private
//! repository's address (`https://github.com/<owner>/<crate name>-assets.git`),
//! writes the game, then runs `repos` ([`crate::repos`]) in its folder. The
//! contract's `new` (`--json`) and the MCP server's `new` tool never make a
//! repository: they act on no account.

use std::fmt::Write as _;
use std::io::{self, BufRead, IsTerminal as _, Write};
use std::path::Path;

use super::{NewInput, NewOutput, PRIVATE_FILES, answers_of, name_of, new, parse_args};
use crate::contract::codes::{self, Code};
use crate::contract::{Answer, Op};
use crate::project::Project;
use crate::repos::{self, DEFAULT_FOLDERS, Host, Report, Slug, Visibility};
use crate::template::{self, EXAMPLES};

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
const TAKES_A_VALUE: [&str; 7] = [
    "--template",
    "--answers",
    "--set",
    "--name",
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
            flag if TAKES_A_VALUE.contains(&flag) => {
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

/// Where the questions are asked: what the developer types, and where
/// the questions go.
pub struct Terminal<'a> {
    /// The answers, a line each.
    pub input: &'a mut dyn BufRead,
    /// The questions.
    pub out: &'a mut dyn Write,
}

/// Whether `new` runs where someone can answer: stdin and stdout a
/// terminal. Never in a test, whose stdin may be one.
pub fn on_a_terminal() -> bool {
    !cfg!(test) && io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Asks `question`, then on a line of its own the `choices` and what
/// pressing Enter picks, `default`; until `parse` takes the line typed
/// (trimmed; empty for the default), saying why it didn't each time. No
/// line left is an error.
fn ask<T>(
    terminal: &mut Terminal<'_>,
    question: &str,
    choices: &str,
    default: &str,
    parse: impl Fn(&str) -> Result<T, String>,
) -> io::Result<T> {
    writeln!(terminal.out, "{question}")?;
    loop {
        write!(terminal.out, "  {choices}, or press Enter for {default}: ")?;
        terminal.out.flush()?;
        let mut line = String::new();
        if terminal.input.read_line(&mut line)? == 0 {
            writeln!(terminal.out)?;
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("no answer to: {question}"),
            ));
        }
        match parse(line.trim()) {
            Ok(answer) => return Ok(answer),
            Err(e) => writeln!(terminal.out, "  {e}")?,
        }
    }
}

/// Yes or no (or the start of either), `default` for an empty line.
fn yes_or_no(default: bool) -> impl Fn(&str) -> Result<bool, String> {
    move |line| {
        let word = line.to_ascii_lowercase();
        if word.is_empty() {
            Ok(default)
        } else if "yes".starts_with(&word) {
            Ok(true)
        } else if "no".starts_with(&word) {
            Ok(false)
        } else {
            Err(format!("`{line}`: yes or no"))
        }
    }
}

/// Public or private (or their first two letters or more), `default` for
/// an empty line.
fn public_or_private(default: Visibility) -> impl Fn(&str) -> Result<Visibility, String> {
    move |line| {
        let word = line.to_ascii_lowercase();
        if word.is_empty() {
            Ok(default)
        } else if word.len() >= 2 && "public".starts_with(&word) {
            Ok(Visibility::Public)
        } else if word.len() >= 2 && "private".starts_with(&word) {
            Ok(Visibility::Private)
        } else {
            Err(format!("`{line}`: public or private"))
        }
    }
}

/// `yes` or `no`, as a default's name.
fn word(yes: bool) -> &'static str {
    if yes { "yes" } else { "no" }
}

/// The first question: whether to include the engine skills' worked
/// examples, yes by default (ADR-0113).
pub fn examples(terminal: &mut Terminal<'_>) -> io::Result<bool> {
    ask(
        terminal,
        "Include the engine skills' worked examples (bots.rs, playtest.ron…)? They show each \
         skill at work; a game runs without them.",
        "yes or no",
        "yes",
        yes_or_no(true),
    )
}

/// Question 1: whether to put the game `name` on GitHub; no by default
/// when `gh` isn't installed.
pub fn github(terminal: &mut Terminal<'_>, name: &str, gh: bool) -> io::Result<bool> {
    let question = if gh {
        format!("Put {name} on GitHub? It needs the GitHub CLI (gh), signed in.")
    } else {
        format!(
            "Put {name} on GitHub? It needs the GitHub CLI (gh), signed in, and gh isn't \
             installed (https://cli.github.com)."
        )
    };
    ask(terminal, &question, "yes or no", word(gh), yes_or_no(gh))
}

/// Question 2: the game's repository `repo`'s visibility.
pub fn game_visibility(terminal: &mut Terminal<'_>, repo: &str) -> io::Result<Visibility> {
    let question =
        format!("Should {repo} be public or private? GitHub's CI is free on a public repository.");
    ask(
        terminal,
        &question,
        "public or private",
        "public",
        public_or_private(Visibility::Public),
    )
}

/// Question 3: whether to keep some files in the private repository
/// `repo`.
pub fn private(terminal: &mut Terminal<'_>, repo: &str) -> io::Result<bool> {
    let question = format!(
        "Keep some files in a second, private repository, {repo}, so they aren't in the public \
         code?"
    );
    ask(terminal, &question, "yes or no", "yes", yes_or_no(true))
}

/// Question 4: which kinds of files to keep private, at least one.
pub fn which_kinds(terminal: &mut Terminal<'_>) -> io::Result<Vec<String>> {
    let mut question = String::from("Which kinds of files go there?");
    for (at, (kind, holds)) in KINDS.iter().enumerate() {
        // Writing to a `String` can't fail.
        let _ = write!(
            question,
            "\n  {}. {:<16} {holds}",
            at + 1,
            format!("files/{kind}/")
        );
    }
    ask(
        terminal,
        &question,
        "numbers or names, several with commas",
        "all five",
        |line| {
            if line.is_empty() {
                return Ok(DEFAULT_FOLDERS.map(str::to_owned).to_vec());
            }
            match kinds(line)? {
                picked if picked.is_empty() => Err(
                    "pick one kind at least: to keep none, answer no to the question before"
                        .to_owned(),
                ),
                picked => Ok(picked),
            }
        },
    )
}

/// Question 5: the developer's own folders under `files/`.
pub fn other_folders(terminal: &mut Terminal<'_>) -> io::Result<Vec<String>> {
    ask(
        terminal,
        "Any other folders under files/ to keep there?",
        "folders, several with commas (levels, maps)",
        "none",
        own_folders,
    )
}

/// Question 6: the private repository `repo`'s visibility.
pub fn assets_visibility(terminal: &mut Terminal<'_>, repo: &str) -> io::Result<Visibility> {
    let question =
        format!("Should {repo} be private or public? Public makes these files anyone's.");
    ask(
        terminal,
        &question,
        "private or public",
        "private",
        public_or_private(Visibility::Private),
    )
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

/// Asks the examples question on `terminal` unless `input`'s answers (from
/// `cwd`) give it, and writes the answer into its `set`. Answers that make
/// no game (an unknown short name, a missing file) ask nothing: `new` says
/// what is wrong.
fn ask_examples(
    cwd: &Path,
    input: &mut NewInput,
    terminal: &mut Terminal<'_>,
) -> Result<(), Refused> {
    let unanswered = template::built_in().is_ok_and(|template| {
        answers_of(cwd, &template, input).is_ok_and(|a| !a.0.contains_key(EXAMPLES))
    });
    if unanswered {
        let answer = word(asked(examples(terminal))?);
        input
            .set
            .0
            .insert(EXAMPLES.to_owned(), vec![answer.to_owned()]);
    }
    Ok(())
}

/// Settles the questions, asking on `terminal` those no flag answers, and
/// sets `input`'s private part: what `repos` is to make, if anything.
fn settle(
    cwd: &Path,
    input: &mut NewInput,
    flags: &Flags,
    json: bool,
    mut terminal: Option<Terminal<'_>>,
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
        terminal = None;
    }
    flag_examples(input, flags.examples)?;
    // A name that makes no game: `new` says so, with no question first.
    let Ok((name, krate)) = name_of(cwd, input) else {
        return Ok(None);
    };
    if let Some(terminal) = terminal.as_mut() {
        ask_examples(cwd, input, terminal)?;
    }
    let github = match (flags.github, terminal.as_mut()) {
        (Some(github), _) => github,
        (None, Some(terminal)) => {
            let gh = host
                .run("gh", &["--version"], cwd, None)
                .is_ok_and(|ran| ran.ok);
            asked(github(terminal, &name, gh))?
        }
        (None, None) => false,
    };
    if !github {
        if flags.github.is_some() || terminal.is_none() {
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
    let visibility = match (flags.visibility, terminal.as_mut()) {
        (Some(visibility), _) => visibility,
        (None, Some(terminal)) => asked(game_visibility(terminal, &format!("{owner}/{krate}")))?,
        (None, None) => Visibility::Public,
    };
    let url = input
        .private_url
        .clone()
        .unwrap_or_else(|| private_url(&owner, &krate));
    let assets = Slug::of_url(&url).map_or_else(|| format!("{owner}/{krate}-assets"), |s| s.path());
    let keep = match (&flags.private, set_private(input), terminal.as_mut()) {
        (Some(kinds), _, _) => !kinds.is_empty(),
        (None, Some(private), _) => private,
        (None, None, _) if flags.folders.is_some() => true,
        (None, None, Some(terminal)) => asked(private(terminal, &assets))?,
        (None, None, None) => true,
    };
    let mut args = repos::Args {
        owner: Some(owner),
        visibility,
        folders: Vec::new(),
        ..repos::Args::default()
    };
    if keep {
        (args.folders, args.assets_visibility) = kept(flags, &mut terminal, &assets)?;
        input
            .set
            .0
            .insert(PRIVATE_FILES.to_owned(), vec!["yes".to_owned()]);
        input.private_url = Some(url);
    }
    Ok(Some(GitHub { args }))
}

/// The private repository `assets`' folders under `files/` (the kinds,
/// then the developer's own) and its visibility, asking on `terminal` what
/// no flag answers.
fn kept(
    flags: &Flags,
    terminal: &mut Option<Terminal<'_>>,
    assets: &str,
) -> Result<(Vec<String>, Visibility), Refused> {
    let mut folders = match (&flags.private, terminal.as_mut()) {
        (Some(kinds), _) => kinds.clone(),
        (None, Some(terminal)) => asked(which_kinds(terminal))?,
        (None, None) => DEFAULT_FOLDERS.map(str::to_owned).to_vec(),
    };
    let own = match (&flags.folders, terminal.as_mut()) {
        (Some(own), _) => own.clone(),
        (None, Some(terminal)) => asked(other_folders(terminal))?,
        (None, None) => Vec::new(),
    };
    let visibility = match (flags.assets_visibility, terminal.as_mut()) {
        (Some(visibility), _) => visibility,
        (None, Some(terminal)) => asked(assets_visibility(terminal, assets))?,
        (None, None) => Visibility::Private,
    };
    for folder in own {
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    Ok((folders, visibility))
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

/// `new` from the command line: settles the questions (asking on
/// `terminal` those no flag answers), makes the game under `cwd`, then,
/// with `--github`, runs `repos` in its folder through `host`.
pub fn new_with(
    cwd: &Path,
    mut input: NewInput,
    flags: &Flags,
    json: bool,
    terminal: Option<Terminal<'_>>,
    host: &mut dyn Host,
) -> Made {
    let github = match settle(cwd, &mut input, flags, json, terminal, host) {
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

/// `new` on this machine, from the command line: prints what it made
/// as `json` says, then `repos`' report; the exit code.
pub fn main(cwd: &Path, input: NewInput, flags: &Flags, json: bool) -> Made {
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    let mut stdout = io::stdout();
    let terminal = (!json && on_a_terminal()).then_some(Terminal {
        input: &mut lock,
        out: &mut stdout,
    });
    new_with(cwd, input, flags, json, terminal, &mut repos::ThisMachine)
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
