//! `cargo xtask docs-rules`: the docs map's rows as checked rules (ticket
//! 3534, ADR-0149).
//!
//! [`RULES`] names, rule by rule, the paths whose change asks for a home of
//! the docs to change in the same PR: a schema of the contract asks for the
//! skill pack and the book, the template for the same, a format for the
//! skill pack and `docs/formats.md`. A rule asked for whose homes didn't
//! change passes when the done ticket of the PR (`[NNNN] …`) has a line in
//! its `## Completion notes` starting with the rule's mark, saying why not
//! (`Skills: none, <why>`). Otherwise it fails, naming the rule, the files
//! that asked, the homes and the line to write.
//!
//! A PR for no ticket's work (ticket-only, a bot's, a Sonar batch, a game
//! ticket's small change) is reported, not failed, as `pr-size` does; so is
//! a local run, which reads the ticket from `tickets/open/` too.
//!
//! The public API's rule for the book stays CI's `api` job
//! ([`crate::api::book_rule`], ticket 2815). It says what this check says,
//! in the same words: both build them with [`ways`] and judge the ticket's
//! notes with [`waived`].

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::tickets::{self, Pr, PrKind};

/// The rules, as data.
pub const RULES: &str = "tools/docs-rules.ron";

/// What the command was asked.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// The revision the PR is measured from: by default where `HEAD` left
    /// `origin/main`, else `main`.
    pub base: Option<String>,
    /// The revision measured, without the working tree. By default the
    /// working tree, untracked files included.
    pub head: Option<String>,
    pub pr_branch: Option<String>,
    pub pr_title: Option<String>,
}

impl Options {
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut options = Options::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let value = Some(rest.next()?.clone());
            match arg.as_str() {
                "--base" => options.base = value,
                "--head" => options.head = value,
                "--pr-branch" => options.pr_branch = value,
                "--pr-title" => options.pr_title = value,
                _ => return None,
            }
        }
        Some(options)
    }
}

/// The file [`RULES`] holds.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesFile {
    rules: Vec<Rule>,
}

/// One row of the docs map, as a rule.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// What a failure names.
    pub name: String,
    /// What a change to [`Rule::triggers`] is, as a sentence's subject.
    pub changes: String,
    /// The globs whose change asks for the rule.
    pub triggers: Vec<String>,
    /// The globs one of which changes with them.
    pub homes: Vec<String>,
    /// The home as a whole, for the failure's words ("the skill pack").
    pub home: String,
    /// The piece of it to change ("the skill").
    pub part: String,
    /// The start of the waiver line in a ticket's notes (`Skills:`).
    pub mark: String,
}

/// The rules in `text`, or why they don't read: a rule with no trigger, no
/// home, or a mark that isn't a word and a colon fails too.
pub fn parse(text: &str) -> Result<Vec<Rule>, String> {
    let file: RulesFile = ron::from_str(text).map_err(|e| format!("{RULES}: {e}"))?;
    let mut errors = Vec::new();
    let mut names = std::collections::HashSet::new();
    for rule in &file.rules {
        if !names.insert(rule.name.as_str()) {
            errors.push(format!("{RULES}: two rules are named `{}`", rule.name));
        }
        if rule.triggers.is_empty() || rule.homes.is_empty() {
            errors.push(format!(
                "{RULES}: rule `{}` needs a trigger and a home",
                rule.name
            ));
        }
        let word = rule.mark.strip_suffix(':').unwrap_or_default();
        if word.is_empty() || word.contains(':') || word.trim() != word {
            errors.push(format!(
                "{RULES}: rule `{}`'s mark `{}` is not a word and a colon, as `Skills:`",
                rule.name, rule.mark
            ));
        }
    }
    if errors.is_empty() {
        Ok(file.rules)
    } else {
        Err(errors.join("\n"))
    }
}

/// Whether `path` matches `glob`: segments split at `/`, a `*` in one
/// matching any characters but `/`, and a `**` segment any number of
/// segments, none included.
pub fn matches(glob: &str, path: &str) -> bool {
    let glob: Vec<&str> = glob.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    segments_match(&glob, &path)
}

fn segments_match(glob: &[&str], path: &[&str]) -> bool {
    match glob.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|skip| segments_match(rest, &path[skip..])),
        Some((first, rest)) => path.split_first().is_some_and(|(segment, others)| {
            segment_matches(first, segment) && segments_match(rest, others)
        }),
    }
}

fn segment_matches(glob: &str, segment: &str) -> bool {
    match glob.split_once('*') {
        None => glob == segment,
        Some((before, after)) => {
            let Some(rest) = segment.strip_prefix(before) else {
                return false;
            };
            rest.char_indices()
                .map(|(at, _)| at)
                .chain(std::iter::once(rest.len()))
                .any(|at| segment_matches(after, &rest[at..]))
        }
    }
}

/// An error for each glob of `rules` that matches none of `tracked`.
pub fn unmatched_globs(rules: &[Rule], tracked: &[String]) -> Vec<String> {
    let mut errors = Vec::new();
    for rule in rules {
        for (role, globs) in [("trigger", &rule.triggers), ("home", &rule.homes)] {
            for glob in globs {
                if !tracked.iter().any(|path| matches(glob, path)) {
                    errors.push(format!(
                        "{RULES}: rule `{}`'s {role} `{glob}` matches no tracked file: \
                         a folder moved or a file was renamed; point the rule at where it \
                         is now, or take the glob out",
                        rule.name
                    ));
                }
            }
        }
    }
    errors
}

/// A glob as a failure shows it: `skills/**` as `skills/`.
fn shown(glob: &str) -> &str {
    glob.strip_suffix("**").unwrap_or(glob)
}

/// The way out of a rule of the docs, in the words every such rule uses:
/// `what` changed and none of `homes` did, so change `part` of `home`, or
/// write the line starting with `mark`.
pub fn ways(what: &str, homes: &[&str], home: &str, part: &str, mark: &str) -> String {
    format!(
        "{what} changed, and nothing in {} did. Either change {part} the docs map \
         (docs/README.md) names for it, or say why {home} needs no change in a line of \
         the ticket's `## Completion notes`, as:\n\n    {mark} none, <why>",
        homes.join(" or ")
    )
}

/// Whether ticket `id`, whose done file's text is `body` (`None` when it is
/// not in `tickets/done/`), has a line starting with `mark` in its notes:
/// `Ok` quotes it, and `Err` says what is missing, then `ways`.
pub fn waived(id: &str, body: Option<&str>, mark: &str, ways: &str) -> Result<String, String> {
    let Some(body) = body else {
        return Err(format!(
            "ticket {id} is not in tickets/done/, so it has no `{mark}` line.\n{ways}"
        ));
    };
    match tickets::notes_lines(body, mark).first() {
        Some(line) => Ok(format!("ticket {id}'s notes say: {line}")),
        None => Err(format!(
            "ticket {id}'s `## Completion notes` have no `{mark}` line.\n{ways}"
        )),
    }
}

/// What one rule made of a PR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// None of its triggers changed.
    Quiet,
    /// A trigger changed, and so did these homes.
    HomeChanged(Vec<String>),
    /// A trigger changed and no home did, and the ticket says why.
    Waived(String),
    /// A trigger changed and no home did, and the PR names no ticket's work.
    Exempt(String),
    /// A trigger changed and no home did, and nothing says why.
    Missing(String),
}

/// `rule`'s verdict on a PR of `kind` that changed `changed`. `ticket`
/// reads a ticket's text by id.
pub fn judge(
    rule: &Rule,
    kind: &PrKind,
    changed: &[String],
    ticket: &dyn Fn(&str) -> Option<String>,
) -> Verdict {
    let in_any = |globs: &[String]| -> Vec<String> {
        changed
            .iter()
            .filter(|path| globs.iter().any(|glob| matches(glob, path)))
            .cloned()
            .collect()
    };
    let triggered = in_any(&rule.triggers);
    if triggered.is_empty() {
        return Verdict::Quiet;
    }
    let homes = in_any(&rule.homes);
    if !homes.is_empty() {
        return Verdict::HomeChanged(homes);
    }
    let what = format!("{} ({})", rule.changes, triggered.join(", "));
    let shown_homes: Vec<&str> = rule.homes.iter().map(|glob| shown(glob)).collect();
    let ways = ways(&what, &shown_homes, &rule.home, &rule.part, &rule.mark);
    let id = match kind {
        PrKind::Work(id) => id,
        PrKind::TicketsOnly(_) => return Verdict::Exempt(format!("a ticket-only PR. {ways}")),
        PrKind::GameSmall(id) => {
            return Verdict::Exempt(format!("game ticket {id}'s small change. {ways}"));
        }
        PrKind::Bot => return Verdict::Exempt(format!("a bot's PR. {ways}")),
        PrKind::Sonar => return Verdict::Exempt(format!("a Sonar batch. {ways}")),
        PrKind::None => return Verdict::Exempt(format!("no ticket is named. {ways}")),
    };
    match waived(id, ticket(id).as_deref(), &rule.mark, &ways) {
        Ok(said) => Verdict::Waived(said),
        Err(why) => Verdict::Missing(why),
    }
}

/// What a run found: the report, and the errors and warnings for `report`
/// in `main.rs`.
#[derive(Debug, Default)]
pub struct Outcome {
    pub report: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Every rule's verdict on a PR of `kind` that changed `changed`: a
/// missing line is an error when `strict` (CI, with the PR's title), else a
/// warning, and an exempt PR's is a warning.
pub fn check(
    rules: &[Rule],
    kind: &PrKind,
    changed: &[String],
    ticket: &dyn Fn(&str) -> Option<String>,
    strict: bool,
) -> Outcome {
    let mut outcome = Outcome::default();
    for rule in rules {
        let name = &rule.name;
        let line = match judge(rule, kind, changed, ticket) {
            Verdict::Quiet => continue,
            Verdict::HomeChanged(homes) => {
                format!("{name}: its home changed ({})", homes.join(", "))
            }
            Verdict::Waived(said) => format!("{name}: {said}"),
            Verdict::Exempt(why) => {
                outcome
                    .warnings
                    .push(format!("rule `{name}`, reported, not failed, for {why}"));
                format!("{name}: reported, not failed")
            }
            Verdict::Missing(why) => {
                let problem = format!("rule `{name}` ({RULES}): {why}");
                if strict {
                    outcome.errors.push(problem);
                } else {
                    outcome.warnings.push(problem);
                }
                format!("{name}: no home changed and no `{}` line", rule.mark)
            }
        };
        let _ = writeln!(outcome.report, "{line}");
    }
    if outcome.report.is_empty() {
        outcome.report.push_str("No rule's paths changed.\n");
    }
    outcome
}

fn git(repo_root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("git did not run ({e})"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn paths(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The files changed from where `head` (the working tree, untracked files
/// included, when `None`) left `base`. A rename is its two paths.
fn changed_files(repo_root: &Path, base: &str, head: Option<&str>) -> Result<Vec<String>, String> {
    let from = git(repo_root, &["merge-base", base, head.unwrap_or("HEAD")])?;
    let from = from.trim();
    let mut args = vec!["diff", "--name-only", "--no-renames", from];
    args.extend(head);
    let mut changed = paths(&git(repo_root, &args)?);
    if head.is_none() {
        changed.extend(paths(&git(
            repo_root,
            &["ls-files", "--others", "--exclude-standard"],
        )?));
    }
    Ok(changed)
}

/// The base a run counts from: `--base`, else `origin/main`, else `main`.
fn base_of(repo_root: &Path, base: Option<&str>) -> String {
    base.map_or_else(
        || {
            if git(repo_root, &["rev-parse", "--verify", "-q", "origin/main"]).is_ok() {
                "origin/main".to_owned()
            } else {
                "main".to_owned()
            }
        },
        str::to_owned,
    )
}

/// The text of ticket `id` in `tickets/done/`, or, when `open_too`, in
/// `tickets/open/` (a session checking before it archives the ticket).
fn ticket_text(repo_root: &Path, id: &str, open_too: bool) -> Option<String> {
    let folders: &[&str] = if open_too {
        &["done", "open"]
    } else {
        &["done"]
    };
    folders.iter().find_map(|folder| {
        tickets::read_ticket_files(&repo_root.join("tickets").join(folder))
            .ok()?
            .into_iter()
            .find(|(name, _)| name.starts_with(&format!("{id}-")))
            .map(|(_, body)| body)
    })
}

/// Reads the rules and checks their globs against the tracked files.
pub fn rules_of(repo_root: &Path) -> Result<Vec<Rule>, Vec<String>> {
    let text =
        fs::read_to_string(repo_root.join(RULES)).map_err(|e| vec![format!("{RULES}: {e}")])?;
    let rules = parse(&text).map_err(|e| vec![e])?;
    let tracked = paths(&git(repo_root, &["ls-files"]).map_err(|e| vec![e])?);
    let errors = unmatched_globs(&rules, &tracked);
    if errors.is_empty() {
        Ok(rules)
    } else {
        Err(errors)
    }
}

/// Checks `repo_root`'s PR against the rules.
pub fn run(repo_root: &Path, options: &Options) -> Outcome {
    let failed = |errors: Vec<String>| Outcome {
        errors,
        ..Outcome::default()
    };
    let rules = match rules_of(repo_root) {
        Ok(rules) => rules,
        Err(errors) => return failed(errors),
    };
    let base = base_of(repo_root, options.base.as_deref());
    let changed = match changed_files(repo_root, &base, options.head.as_deref()) {
        Ok(changed) => changed,
        Err(e) => return failed(vec![e]),
    };
    let (branch, strict) = match &options.pr_branch {
        Some(branch) => (branch.clone(), true),
        None => match git(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
            Ok(branch) => (branch.trim().to_owned(), false),
            Err(e) => return failed(vec![e]),
        },
    };
    let pr = Pr {
        branch: &branch,
        title: options.pr_title.as_deref(),
    };
    match pr.kind() {
        Ok(kind) => check(
            &rules,
            &kind,
            &changed,
            &|id| ticket_text(repo_root, id, !strict),
            strict,
        ),
        Err(e) => failed(vec![e]),
    }
}

#[cfg(test)]
mod tests;
