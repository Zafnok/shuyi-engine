//! `cargo xtask changelog`: the changelog, written from the repository
//! (ticket 2804).
//!
//! Nobody writes it by hand. Its sources are the tickets in `tickets/done/`
//! (`id`, `title`, `type`, `completed` and the completion notes' `Breaking:`
//! lines and `migration` blocks) and `git log --first-parent`, whose one
//! commit per PR is titled `[NNNN] …` for a ticket. A commit's decoration
//! names the release tag (`vX.Y.Z`) it is, and every older commit belongs to
//! that release. Before any tag the entries group by month.
//!
//! Three outputs: Markdown on stdout, the same as Zola pages (`--site`), and
//! the Keep a Changelog shape for `CHANGELOG.md`. The one call to `git` goes
//! through a function the tests replace with a fake history.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::tickets::{breaking_lines, extract_frontmatter, read_ticket_files};

/// The repository, for the links to a ticket and a PR.
const REPO_URL: &str = "https://github.com/Zafnok/shuyi-engine";
/// A release post's introduction, if Nick or Claude wrote one, is
/// `INTRO_DIR/<tag>.md` (relative to the repository's root).
const INTRO_DIR: &str = "site/content/news/intro";
/// Where `--site` writes the posts, below its content folder.
const POSTS_DIR: &str = "news/generated";

/// The `git log` the history is read from, one line a commit: its date, its
/// decoration and its subject.
const LOG_ARGS: &[&str] = &["log", "--first-parent", "HEAD", "--format=%cs%x09%D%x09%s"];

/// The output's shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Markdown,
    KeepAChangelog,
}

/// What `cargo xtask changelog` was asked for.
pub struct Options {
    /// The Zola content folder to write the pages in, instead of stdout.
    pub site: Option<PathBuf>,
    pub format: Format,
}

impl Options {
    /// Reads `[--site <dir>] [--format markdown|keep-a-changelog]`, or `None`
    /// when they are wrong (`--site` has its own shape).
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut site = None;
        let mut format = Format::Markdown;
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match (arg.as_str(), rest.next()) {
                ("--site", Some(dir)) => site = Some(PathBuf::from(dir)),
                ("--format", Some(f)) if f == "markdown" => format = Format::Markdown,
                ("--format", Some(f)) if f == "keep-a-changelog" => {
                    format = Format::KeepAChangelog;
                }
                _ => return None,
            }
        }
        if site.is_some() && format != Format::Markdown {
            return None;
        }
        Some(Self { site, format })
    }
}

/// A `migration` block of a ticket's completion notes (ticket 2804): how a
/// game moves over one break. 2523 reuses the reader.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Migration {
    /// The engine version that first has the change, `X.Y.Z`.
    pub since: String,
    /// One sentence.
    pub what: String,
    /// The old path or signature.
    pub old: String,
    /// The new one.
    pub new: String,
    /// A game's code before.
    pub before: String,
    /// A game's code after.
    pub after: String,
    /// Literal strings in `before` that are not in `after`, marking a call
    /// site in a game's code.
    #[serde(default)]
    pub find: Vec<String>,
}

impl Migration {
    fn check(&self) -> Result<(), String> {
        let parts: Vec<&str> = self.since.split('.').collect();
        if parts.len() != 3
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(format!("`since` is `{}`, not `X.Y.Z`", self.since));
        }
        for needle in &self.find {
            if !self.before.contains(needle.as_str()) {
                return Err(format!("`find` string `{needle}` is not in `before`"));
            }
            if self.after.contains(needle.as_str()) {
                return Err(format!("`find` string `{needle}` is in `after`"));
            }
        }
        Ok(())
    }
}

/// The `migration` blocks under `## Completion notes` of a ticket's file, in
/// order. A block that doesn't parse is an error naming `file` and the line
/// its fence is on.
pub(crate) fn migrations(file: &str, content: &str) -> Result<Vec<Migration>, String> {
    let content = content.replace("\r\n", "\n");
    let mut found = Vec::new();
    let mut in_notes = false;
    let mut lines = content.lines().enumerate();
    while let Some((n, line)) = lines.next() {
        if line.starts_with("# ") || line.starts_with("## ") {
            in_notes = line.trim_end() == "## Completion notes";
            continue;
        }
        let text = line.trim_start();
        if !in_notes || text.trim_end() != "```migration" {
            continue;
        }
        let at = format!("{file}:{}", n + 1);
        let indent = line.len() - text.len();
        let mut yaml = String::new();
        let mut closed = false;
        for (_, inner) in lines.by_ref() {
            if inner.trim() == "```" {
                closed = true;
                break;
            }
            let cut = inner
                .char_indices()
                .take(indent)
                .take_while(|(_, c)| *c == ' ')
                .count();
            yaml.push_str(&inner[cut..]);
            yaml.push('\n');
        }
        if !closed {
            return Err(format!("{at}: migration block is never closed"));
        }
        let block: Migration =
            serde_norway::from_str(&yaml).map_err(|e| format!("{at}: migration block: {e}"))?;
        block
            .check()
            .map_err(|e| format!("{at}: migration block: {e}"))?;
        found.push(block);
    }
    Ok(found)
}

/// A ticket of `tickets/done/`, as the changelog needs it.
#[derive(Debug, Clone)]
struct Entry {
    id: String,
    title: String,
    kind: String,
    file: String,
    completed: String,
    /// The PR that merged it, if the history has one.
    pr: Option<u64>,
    breaking: Vec<String>,
    migrations: Vec<Migration>,
}

#[derive(Deserialize)]
struct Front {
    id: String,
    title: String,
    #[serde(rename = "type")]
    kind: String,
    completed: Option<String>,
}

fn read_entry(file: &str, content: &str) -> Result<Entry, String> {
    let path = format!("tickets/done/{file}");
    let content = content.replace("\r\n", "\n");
    let (yaml, body) =
        extract_frontmatter(&content).ok_or_else(|| format!("{path}: no frontmatter"))?;
    let front: Front =
        serde_norway::from_str(yaml).map_err(|e| format!("{path}: frontmatter: {e}"))?;
    let completed = front
        .completed
        .filter(|c| !c.is_empty())
        .ok_or_else(|| format!("{path}: a done ticket has no `completed` date"))?;
    Ok(Entry {
        id: front.id,
        title: front.title,
        kind: front.kind,
        file: file.to_string(),
        completed,
        pr: None,
        breaking: breaking_lines(body)
            .into_iter()
            .map(|l| l.trim_start_matches("Breaking:").trim().to_string())
            .collect(),
        migrations: migrations(&path, &content)?,
    })
}

/// One release of the changelog.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// A tag, `vX.Y.Z`.
    Tag(String),
    /// After the last tag.
    Unreleased,
    /// Before any tag: `YYYY-MM`.
    Month(String),
}

#[derive(Debug, Clone)]
struct Release {
    kind: Kind,
    /// The tag's date, or the newest date among a month's entries.
    date: Option<String>,
    tickets: Vec<Entry>,
    sonar: usize,
    bumps: usize,
    other: usize,
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl Release {
    fn new(kind: Kind, date: Option<String>) -> Self {
        Self {
            kind,
            date,
            tickets: Vec::new(),
            sonar: 0,
            bumps: 0,
            other: 0,
        }
    }

    /// The heading's name: `v0.1.0`, `Unreleased` or `2026-10`.
    fn name(&self) -> &str {
        match &self.kind {
            Kind::Tag(tag) => tag,
            Kind::Unreleased => "Unreleased",
            Kind::Month(month) => month,
        }
    }

    /// The post's title.
    fn title(&self) -> String {
        match &self.kind {
            Kind::Tag(tag) => format!("Shuyi Engine {tag}"),
            Kind::Unreleased => "Unreleased changes".to_string(),
            Kind::Month(month) => {
                let (year, number) = month.split_once('-').unwrap_or((month, "1"));
                let name = number
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| MONTHS.get(n.wrapping_sub(1)))
                    .copied()
                    .unwrap_or(number);
                format!("Changes in {name} {year}")
            }
        }
    }

    /// The post's file name, or `None` for the one that isn't posted.
    fn slug(&self) -> Option<String> {
        match &self.kind {
            Kind::Tag(tag) => Some(format!("release-{}", tag.replace('.', "-"))),
            Kind::Unreleased => None,
            Kind::Month(month) => Some(format!("changes-{month}")),
        }
    }

    fn breaking_count(&self) -> usize {
        self.tickets.iter().map(|t| t.breaking.len()).sum()
    }

    fn description(&self) -> String {
        let n = self.tickets.len();
        let mut text = format!("{n} ticket{} done", plural(n));
        let b = self.breaking_count();
        if b > 0 {
            let _ = write!(text, ", {b} breaking change{}", plural(b));
        }
        text.push('.');
        text
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

struct Commit<'a> {
    date: &'a str,
    tag: Option<&'a str>,
    subject: &'a str,
}

/// `vX.Y.Z`, with numbers.
fn is_release_tag(tag: &str) -> bool {
    tag.strip_prefix('v').is_some_and(|v| {
        let parts: Vec<&str> = v.split('.').collect();
        parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    })
}

fn parse_log(log: &str) -> Result<Vec<Commit<'_>>, String> {
    log.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut parts = line.splitn(3, '\t');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(date), Some(refs), Some(subject)) => Ok(Commit {
                    date,
                    tag: refs
                        .split(", ")
                        .filter_map(|r| r.strip_prefix("tag: "))
                        .find(|t| is_release_tag(t)),
                    subject,
                }),
                _ => Err(format!("git log: unexpected line `{line}`")),
            }
        })
        .collect()
}

/// The ticket id of a `[NNNN] …` subject.
fn subject_ticket(subject: &str) -> Option<&str> {
    let id = subject.strip_prefix('[')?.split_once(']')?.0;
    (id.len() == 4 && id.chars().all(|c| c.is_ascii_digit())).then_some(id)
}

/// The PR number of a squash or merge subject ending in `(#N)`.
fn subject_pr(subject: &str) -> Option<u64> {
    subject
        .trim_end()
        .strip_suffix(')')?
        .rsplit_once("(#")?
        .1
        .parse()
        .ok()
}

/// The release of a month, made if it isn't there yet.
fn month_release<'a>(releases: &'a mut Vec<Release>, date: &str) -> &'a mut Release {
    let month = date.get(..7).unwrap_or(date).to_string();
    let at = releases
        .iter()
        .position(|r| r.kind == Kind::Month(month.clone()))
        .unwrap_or_else(|| {
            releases.push(Release::new(Kind::Month(month), None));
            releases.len() - 1
        });
    &mut releases[at]
}

fn newer(date: &mut Option<String>, seen: &str) {
    if date.as_deref().is_none_or(|d| d < seen) {
        *date = Some(seen.to_string());
    }
}

/// Groups the done tickets and the history into releases, newest first.
///
/// A ticket is in the release of the commit titled `[NNNN] …`. One with no
/// such commit (it came with a game ticket's move, or the history is short)
/// is in the first release dated on or after its `completed` date.
fn build(tickets: &[(String, String)], log: &str) -> Result<Vec<Release>, Vec<String>> {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for (file, content) in tickets {
        match read_entry(file, content) {
            Ok(entry) => entries.push(entry),
            Err(e) => errors.push(e),
        }
    }
    let commits = parse_log(log).map_err(|e| vec![e])?;
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut by_id: HashMap<String, usize> = HashMap::new();
    for (at, entry) in entries.iter().enumerate() {
        by_id.entry(entry.id.clone()).or_insert(at);
    }

    let has_tags = commits.iter().any(|c| c.tag.is_some());
    let mut releases: Vec<Release> = Vec::new();
    let mut placed = vec![false; entries.len()];
    for commit in &commits {
        let release = if let Some(tag) = commit.tag {
            releases.push(Release::new(
                Kind::Tag(tag.to_string()),
                Some(commit.date.to_string()),
            ));
            releases.last_mut()
        } else if !has_tags {
            Some(month_release(&mut releases, commit.date))
        } else {
            if releases.is_empty() {
                releases.push(Release::new(Kind::Unreleased, None));
            }
            releases.last_mut()
        };
        let Some(release) = release else { continue };
        if !has_tags {
            newer(&mut release.date, commit.date);
        }
        let ticket = subject_ticket(commit.subject)
            .and_then(|id| by_id.get(id))
            .copied()
            .filter(|at| !placed[*at]);
        if let Some(at) = ticket {
            placed[at] = true;
            let mut entry = entries[at].clone();
            entry.pr = subject_pr(commit.subject);
            release.tickets.push(entry);
        } else if commit.subject.to_lowercase().contains("sonar") {
            release.sonar += 1;
        } else if commit.subject.starts_with("Bump ") {
            release.bumps += 1;
        } else {
            release.other += 1;
        }
    }

    for (at, entry) in entries.iter().enumerate().filter(|(at, _)| !placed[*at]) {
        let release = if has_tags {
            place_by_date(&mut releases, &entry.completed)
        } else {
            let release = month_release(&mut releases, &entry.completed);
            newer(&mut release.date, &entry.completed);
            release
        };
        release.tickets.push(entries[at].clone());
    }
    if !has_tags {
        releases.sort_by(|a, b| b.name().cmp(a.name()));
    }
    for release in &mut releases {
        release
            .tickets
            .sort_by(|a, b| (area(&a.id).0, &a.id).cmp(&(area(&b.id).0, &b.id)));
    }
    Ok(releases)
}

/// The oldest tag dated on or after `completed`, or Unreleased.
fn place_by_date<'a>(releases: &'a mut Vec<Release>, completed: &str) -> &'a mut Release {
    let tagged = releases
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r.kind, Kind::Tag(_)))
        .filter(|(_, r)| r.date.as_deref().is_some_and(|d| d >= completed))
        .min_by(|a, b| a.1.date.cmp(&b.1.date))
        .map(|(at, _)| at);
    let at = tagged.unwrap_or_else(|| {
        releases
            .iter()
            .position(|r| r.kind == Kind::Unreleased)
            .unwrap_or_else(|| {
                releases.insert(0, Release::new(Kind::Unreleased, None));
                0
            })
    });
    &mut releases[at]
}

/// A ticket id's area: its block, as `tickets/README.md`'s *Numbering*.
fn area(id: &str) -> (u32, &'static str) {
    let block: u32 = id.get(..2).and_then(|b| b.parse().ok()).unwrap_or(0);
    let name = match block {
        11 => "Carved from the game",
        20 => "Repository and foundation",
        21 => "Core and simulation",
        22 => "Assets and formats",
        23 => "Frame, screens and skins",
        24 => "Input",
        25 => "Tools and AI support",
        26 => "Networking",
        27 => "Game styles and examples",
        28 => "Docs and community",
        29 => "Research",
        30 => "Export targets",
        31 => "3D and HD-2D",
        _ => "Other tickets",
    };
    (block, name)
}

// --- Rendering ---------------------------------------------------------

/// Escapes what Markdown or Zola would read as a link or HTML, outside code.
fn escape_md(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut code = false;
    for c in text.chars() {
        if c == '`' {
            code = !code;
        } else if !code && matches!(c, '<' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn ticket_url(entry: &Entry) -> String {
    format!("{REPO_URL}/blob/main/tickets/done/{}", entry.file)
}

/// One ticket's line: its title, linked to its file, and its PR.
fn ticket_line(entry: &Entry) -> String {
    let mut line = format!("[{}]({})", escape_md(&entry.title), ticket_url(entry));
    if let Some(pr) = entry.pr {
        let _ = write!(line, " ([#{pr}]({REPO_URL}/pull/{pr}))");
    }
    line
}

fn counts(release: &Release) -> Vec<String> {
    let mut lines = Vec::new();
    let mut count = |n: usize, one: &str, many: &str| {
        if n > 0 {
            lines.push(format!("{n} {}", if n == 1 { one } else { many }));
        }
    };
    count(release.sonar, "Sonar fix", "Sonar fixes");
    count(release.bumps, "dependency update", "dependency updates");
    count(release.other, "other pull request", "other pull requests");
    lines
}

fn heading(level: usize, text: &str) -> String {
    format!("{} {text}\n\n", "#".repeat(level))
}

/// The "Migration guide" section, if the release has a migration block.
fn migration_guide(release: &Release, level: usize) -> String {
    let blocks: Vec<(&Entry, &Migration)> = release
        .tickets
        .iter()
        .flat_map(|t| t.migrations.iter().map(move |m| (t, m)))
        .collect();
    if blocks.is_empty() {
        return String::new();
    }
    let mut out = heading(level, "Migration guide");
    for (entry, m) in blocks {
        out.push_str(&heading(level + 1, &escape_md(&m.what)));
        let _ = writeln!(
            out,
            "Since `{}`: `{}` → `{}` ([{}]({})).\n",
            m.since,
            m.old,
            m.new,
            escape_md(&entry.title),
            ticket_url(entry)
        );
        for (label, code) in [("Before:", &m.before), ("After:", &m.after)] {
            let _ = writeln!(out, "{label}\n\n```rust\n{}\n```\n", code.trim_end());
        }
    }
    out
}

/// A release's notes, as Markdown with its sections at `level`.
fn notes(release: &Release, style: Format, level: usize) -> String {
    let mut out = String::new();
    match style {
        Format::Markdown => {
            if release.breaking_count() > 0 {
                out.push_str(&heading(level, "Breaking changes"));
                for t in &release.tickets {
                    for b in &t.breaking {
                        let _ = writeln!(out, "- {} ({})", escape_md(b), ticket_line(t));
                    }
                }
                out.push('\n');
            }
            out.push_str(&migration_guide(release, level));
            let mut current = None;
            for t in &release.tickets {
                let (block, name) = area(&t.id);
                if current != Some(block) {
                    if current.is_some() {
                        out.push('\n');
                    }
                    out.push_str(&heading(level, name));
                    current = Some(block);
                }
                let _ = writeln!(out, "- {}", ticket_line(t));
            }
            if current.is_some() {
                out.push('\n');
            }
            let others = counts(release);
            if !others.is_empty() {
                out.push_str(&heading(level, "Other changes"));
                for line in others {
                    let _ = writeln!(out, "- {line}");
                }
                out.push('\n');
            }
        }
        Format::KeepAChangelog => {
            let mut added = Vec::new();
            let mut fixed = Vec::new();
            let mut changed = Vec::new();
            for t in &release.tickets {
                for b in &t.breaking {
                    changed.push(format!(
                        "**Breaking:** {} ({})",
                        escape_md(b),
                        ticket_line(t)
                    ));
                }
            }
            for t in &release.tickets {
                match t.kind.as_str() {
                    "feature" => added.push(ticket_line(t)),
                    "bug" => fixed.push(ticket_line(t)),
                    _ => changed.push(ticket_line(t)),
                }
            }
            changed.extend(counts(release));
            for (name, lines) in [("Added", added), ("Changed", changed), ("Fixed", fixed)] {
                if lines.is_empty() {
                    continue;
                }
                out.push_str(&heading(level, name));
                for line in lines {
                    let _ = writeln!(out, "- {line}");
                }
                out.push('\n');
            }
            out.push_str(&migration_guide(release, level));
        }
    }
    out
}

fn dated(release: &Release) -> String {
    match &release.date {
        Some(date) => format!("{} — {date}", release.name()),
        None => release.name().to_string(),
    }
}

/// The whole changelog as Markdown, with its title when `title` is set.
fn markdown(releases: &[Release], title: bool) -> String {
    let mut out = String::new();
    if title {
        out.push_str(&heading(1, "Changelog"));
    }
    out.push_str(
        "Every ticket done, by release and then by area, with the engine's breaking \
         changes first. Written from `tickets/done/` and the merges by `cargo xtask \
         changelog`.\n\n",
    );
    for release in releases {
        out.push_str(&heading(2, &dated(release)));
        out.push_str(&notes(release, Format::Markdown, 3));
    }
    out
}

/// `CHANGELOG.md`, in the Keep a Changelog 1.1.0 shape.
fn keep_a_changelog(releases: &[Release]) -> String {
    let mut out = String::from(
        "# Changelog\n\nAll notable changes to Shuyi Engine are in this file, written from \
         `tickets/done/` and the merges by `cargo xtask changelog`. The format is based on \
         [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).\n\n",
    );
    for release in releases {
        let name = match &release.kind {
            Kind::Tag(tag) => tag.trim_start_matches('v').to_string(),
            _ => release.name().to_string(),
        };
        match &release.date {
            Some(date) if matches!(release.kind, Kind::Tag(_)) => {
                out.push_str(&heading(2, &format!("[{name}] - {date}")));
            }
            _ => out.push_str(&heading(2, &format!("[{name}]"))),
        }
        out.push_str(&notes(release, Format::KeepAChangelog, 3));
    }
    let tags: Vec<&str> = releases
        .iter()
        .filter_map(|r| match &r.kind {
            Kind::Tag(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    for release in releases {
        let name = release.name().trim_start_matches('v');
        let link = match (
            &release.kind,
            tags.iter().position(|t| *t == release.name()),
        ) {
            (Kind::Unreleased, _) => tags
                .first()
                .map(|t| format!("{REPO_URL}/compare/{t}...HEAD")),
            (Kind::Tag(tag), Some(i)) => Some(match tags.get(i + 1) {
                Some(previous) => format!("{REPO_URL}/compare/{previous}...{tag}"),
                None => format!("{REPO_URL}/releases/tag/{tag}"),
            }),
            _ => None,
        };
        if let Some(link) = link {
            let _ = writeln!(out, "[{name}]: {link}");
        }
    }
    out
}

/// A TOML basic string.
fn toml_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The introduction Nick or Claude wrote for a release, without its Zola
/// front matter (`draft = true` keeps the file out of the site).
fn strip_front_matter(text: &str) -> &str {
    let text = text.trim_start_matches('\u{feff}');
    match text
        .strip_prefix("+++\n")
        .and_then(|rest| rest.split_once("\n+++"))
    {
        Some((_, rest)) => rest.trim_start_matches(['\r', '\n']),
        None => text,
    }
}

/// The pages `--site` writes, as `(path under the content folder, text)`:
/// `changelog.md`, and a post for each tag (or, before any tag, each month).
fn site_pages(
    releases: &[Release],
    intro: &dyn Fn(&str) -> Option<String>,
) -> Vec<(String, String)> {
    let mut pages = Vec::new();
    let page = format!(
        "+++\ntitle = \"Changelog\"\ndescription = \"Every ticket done in Shuyi Engine, by release \
         and by area, with the breaking changes and how to move a game over them.\"\n+++\n\n{}",
        markdown(releases, false)
    );
    pages.push(("changelog.md".to_string(), page));
    for release in releases {
        let (Some(slug), Some(date)) = (release.slug(), &release.date) else {
            continue;
        };
        let mut text = format!(
            "+++\ntitle = {}\ndate = {date}\ndescription = {}\n\n[taxonomies]\ntags = [\"release\"]\n+++\n\n",
            toml_string(&release.title()),
            toml_string(&release.description()),
        );
        if let Kind::Tag(tag) = &release.kind
            && let Some(intro) = intro(tag)
        {
            let _ = write!(
                text,
                "{}

",
                strip_front_matter(&intro).trim_end()
            );
        }
        text.push_str(&notes(release, Format::Markdown, 2));
        text.push_str(
            "The whole history is in the [changelog](@/changelog.md).
",
        );
        pages.push((format!("{POSTS_DIR}/{slug}.md"), text));
    }
    pages
}

// --- The command -------------------------------------------------------

/// Runs `git` in the repository, for the history.
pub(crate) fn system_git(root: &Path) -> impl Fn(&[&str]) -> Result<String, String> + '_ {
    move |args| {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
        if !output.status.success() {
            return Err(format!(
                "git {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// What the command writes: Markdown for stdout, or the site's pages.
pub enum Output {
    Stdout(String),
    Pages(Vec<(String, String)>),
}

/// Reads the done tickets under `root` and the history through `git`, and
/// writes `options`' output. The site's pages are written by the caller
/// with [`write_pages`].
pub fn run(
    root: &Path,
    options: &Options,
    git: &dyn Fn(&[&str]) -> Result<String, String>,
) -> Result<Output, Vec<String>> {
    let tickets = read_ticket_files(&root.join("tickets").join("done"))
        .map_err(|e| vec![format!("tickets/done: {e}")])?;
    let log = git(LOG_ARGS).map_err(|e| vec![e])?;
    let releases = build(&tickets, &log)?;
    Ok(match (&options.site, options.format) {
        (Some(_), _) => Output::Pages(site_pages(&releases, &|tag| {
            fs::read_to_string(root.join(INTRO_DIR).join(format!("{tag}.md"))).ok()
        })),
        (None, Format::Markdown) => Output::Stdout(markdown(&releases, true)),
        (None, Format::KeepAChangelog) => Output::Stdout(keep_a_changelog(&releases)),
    })
}

/// Writes the site's pages below `content`, after clearing the posts' folder
/// of an earlier run.
pub fn write_pages(content: &Path, pages: &[(String, String)]) -> Result<(), String> {
    let posts = content.join(POSTS_DIR);
    if posts.exists() {
        fs::remove_dir_all(&posts).map_err(|e| format!("{}: {e}", posts.display()))?;
    }
    for (path, text) in pages {
        let file = content.join(path);
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(id: &str, title: &str, kind: &str, completed: &str, notes: &str) -> (String, String) {
        let slug: String = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        (
            format!("{id}-{slug}.md"),
            format!(
                "---\nid: \"{id}\"\ntitle: \"{title}\"\ntype: {kind}\nstatus: done\n\
                 completed: {completed}\n---\n\n# {id} — {title}\n\n## Scope\n\nWork.\n\n\
                 ## Completion notes\n\n{notes}\n"
            ),
        )
    }

    const BREAKING: &str = "- Breaking: shuyi-core: `Timer::new`: pass the step as a `Step`\n\n\
        ```migration\n\
        since: 0.2.0\n\
        what: A timer takes its step as a Step, not a number.\n\
        old: Timer::new(u32)\n\
        new: Timer::new(Step)\n\
        before: |\n  let t = Timer::new(60);\n\
        after: |\n  let t = Timer::new(Step::from_ticks(60));\n\
        find: [\"Timer::new(60)\"]\n\
        ```\n";

    /// Two tags, tickets in six blocks (one with a break and a migration
    /// block, one with no PR), and three Sonar PRs.
    fn fake() -> (Vec<(String, String)>, String) {
        let tickets = vec![
            ticket("2001", "The workspace", "infra", "2026-11-01", "Done."),
            ticket("2801", "The website", "feature", "2026-10-20", "Done."),
            ticket("2301", "Draw the scene", "feature", "2026-11-10", "Done."),
            ticket(
                "2402",
                "Rebind keys at run time",
                "bug",
                "2026-11-20",
                "Done.",
            ),
            ticket(
                "2101",
                "Fixed-step timers",
                "feature",
                "2026-12-01",
                BREAKING,
            ),
            ticket(
                "2501",
                "A <tool> for [agents]",
                "feature",
                "2026-11-15",
                "Done.",
            ),
            ticket(
                "2203",
                "Hot reload for baked assets",
                "feature",
                "2026-12-05",
                "Done.",
            ),
        ];
        let log = "\
2026-12-05\tHEAD -> main, origin/main\t[2203] Hot reload for baked assets (#41)
2026-12-04\t\tFix SonarCloud code smells, batch 7 (#40)
2026-12-02\t\tBump serde from 1.0.1 to 1.0.2 (#39)
2026-12-01\ttag: v0.2.0\t[2101] Fixed-step timers (#38)
2026-11-20\t\t[2402] Rebind keys at run time (#37)
2026-11-18\t\tsonar(rust): 12 findings (#36)
2026-11-17\t\tsonar(rust): 7 findings (#35)
2026-11-10\t\t[2301] Draw the scene (#34)
2026-11-02\t\tGame 1200: a small game PR (#33)
2026-11-01\ttag: v0.1.0, origin/v0.1.0\t[2001] The workspace (#30)
2026-10-25\t\tsonar(rust): 3 findings (#28)
2026-10-20\t\t[2801] The website (#25)
";
        (tickets, log.to_string())
    }

    fn snapshot(name: &str, actual: &str) {
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/snapshots")
            .join(name);
        if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, actual).unwrap();
        }
        let expected = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("{}: {e} (UPDATE_SNAPSHOTS=1 writes it)", file.display()))
            .replace("\r\n", "\n");
        assert_eq!(
            expected, actual,
            "{name} differs (UPDATE_SNAPSHOTS=1 rewrites it)"
        );
    }

    fn built() -> Vec<Release> {
        let (tickets, log) = fake();
        build(&tickets, &log).unwrap()
    }

    #[test]
    fn groups_by_tag_newest_first_then_by_area() {
        let releases = built();
        let names: Vec<&str> = releases.iter().map(Release::name).collect();
        assert_eq!(names, ["Unreleased", "v0.2.0", "v0.1.0"]);
        let ids = |r: &Release| r.tickets.iter().map(|t| t.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&releases[0]), ["2203"]);
        // 2501 has no merge: the first tag dated on or after 2026-11-15.
        assert_eq!(ids(&releases[1]), ["2101", "2301", "2402", "2501"]);
        assert_eq!(ids(&releases[2]), ["2001", "2801"]);
        assert_eq!(releases[1].date.as_deref(), Some("2026-12-01"));
    }

    #[test]
    fn counts_the_prs_with_no_ticket() {
        let releases = built();
        assert_eq!((releases[0].sonar, releases[0].bumps), (1, 1));
        assert_eq!((releases[1].sonar, releases[1].other), (2, 1));
        assert_eq!((releases[2].sonar, releases[2].other), (1, 0));
        assert_eq!(
            counts(&releases[1]),
            ["2 Sonar fixes", "1 other pull request"]
        );
    }

    #[test]
    fn a_ticket_with_no_pr_is_listed_without_a_link() {
        let releases = built();
        let entry = releases[1].tickets.iter().find(|t| t.id == "2501").unwrap();
        assert_eq!(entry.pr, None);
        assert!(!ticket_line(entry).contains("/pull/"));
        let other = releases[1].tickets.iter().find(|t| t.id == "2301").unwrap();
        assert_eq!(other.pr, Some(34));
        assert!(ticket_line(other).contains("/pull/34"));
    }

    #[test]
    fn reads_breaking_lines_and_the_migration_block() {
        let releases = built();
        let entry = &releases[1].tickets[0];
        assert_eq!(
            entry.breaking,
            ["shuyi-core: `Timer::new`: pass the step as a `Step`"]
        );
        assert_eq!(entry.migrations.len(), 1);
        assert_eq!(entry.migrations[0].since, "0.2.0");
        assert_eq!(entry.migrations[0].find, ["Timer::new(60)"]);
    }

    #[test]
    fn before_any_tag_the_entries_group_by_month() {
        let (tickets, _) = fake();
        let log =
            "2026-11-10\t\t[2301] Draw the scene (#34)\n2026-10-20\t\t[2801] The website (#25)\n";
        let releases = build(&tickets, log).unwrap();
        let names: Vec<&str> = releases.iter().map(Release::name).collect();
        // The tickets with no merge sort into their `completed` months.
        assert_eq!(names, ["2026-12", "2026-11", "2026-10"]);
        assert_eq!(releases[1].title(), "Changes in November 2026");
        assert_eq!(releases[1].slug().as_deref(), Some("changes-2026-11"));
        assert_eq!(releases[0].tickets.len(), 2);
    }

    #[test]
    fn a_bad_migration_block_names_the_file_and_line() {
        let bad = "- Breaking: shuyi-core: x: y\n\n```migration\nsince: soon\nwhat: w\nold: a\nnew: b\n\
                   before: c\nafter: d\n```\n";
        let (file, content) = ticket("2101", "Timers", "feature", "2026-12-01", bad);
        let err = migrations(&format!("tickets/done/{file}"), &content).unwrap_err();
        assert!(err.starts_with("tickets/done/2101-timers.md:19: "), "{err}");
        assert!(err.contains("`since` is `soon`"), "{err}");

        let unknown = "```migration\nsince: 1.0.0\nwhat: w\nold: a\nnew: b\nbefore: c\nafter: d\nbogus: 1\n```\n";
        let (file, content) = ticket("2101", "Timers", "feature", "2026-12-01", unknown);
        let err = migrations(&file, &content).unwrap_err();
        assert!(err.contains("bogus"), "{err}");

        let open = "```migration\nsince: 1.0.0\n";
        let (file, content) = ticket("2101", "Timers", "feature", "2026-12-01", open);
        assert!(
            migrations(&file, &content)
                .unwrap_err()
                .contains("never closed")
        );
    }

    #[test]
    fn find_strings_must_mark_the_old_code() {
        let wrong = "```migration\nsince: 1.0.0\nwhat: w\nold: a\nnew: b\nbefore: c\nafter: c\nfind: [c]\n```\n";
        let (file, content) = ticket("2101", "Timers", "feature", "2026-12-01", wrong);
        assert!(
            migrations(&file, &content)
                .unwrap_err()
                .contains("is in `after`")
        );
        let missing = wrong.replace("find: [c]", "find: [z]");
        let (file, content) = ticket("2101", "Timers", "feature", "2026-12-01", &missing);
        assert!(
            migrations(&file, &content)
                .unwrap_err()
                .contains("not in `before`")
        );
    }

    #[test]
    fn migration_blocks_outside_the_completion_notes_are_not_read() {
        let content = "---\nid: \"1\"\n---\n\n## Scope\n\n```migration\nnot: yaml: [\n```\n";
        assert_eq!(migrations("x.md", content), Ok(Vec::new()));
    }

    #[test]
    fn an_indented_block_under_a_list_item_is_read() {
        let notes = "- Breaking: a: b: c\n\n  ```migration\n  since: 1.2.3\n  what: w\n  old: a\n  new: b\n  \
                     before: |\n    x\n  after: |\n    y\n  ```\n";
        let (file, content) = ticket("2101", "Timers", "feature", "2026-12-01", notes);
        let found = migrations(&file, &content).unwrap();
        assert_eq!(found[0].before, "x\n");
    }

    #[test]
    fn a_done_ticket_with_no_completed_date_is_an_error() {
        let content = "---\nid: \"2101\"\ntitle: T\ntype: infra\ncompleted:\n---\n\nBody\n";
        let err = build(&[("2101-t.md".to_string(), content.to_string())], "").unwrap_err();
        assert!(err[0].contains("no `completed` date"), "{err:?}");
    }

    #[test]
    fn parses_the_pr_number_and_the_ticket() {
        assert_eq!(subject_pr("[2804] x (#158)"), Some(158));
        assert_eq!(subject_pr("Game 1: x"), None);
        assert_eq!(subject_ticket("[2804] x"), Some("2804"));
        assert_eq!(subject_ticket("Ticket 2804: x"), None);
        assert!(is_release_tag("v1.22.3") && !is_release_tag("v1.2") && !is_release_tag("nightly"));
    }

    #[test]
    fn options_parse() {
        let args = |a: &[&str]| a.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert!(Options::parse(&args(&[])).is_some());
        let kac = Options::parse(&args(&["--format", "keep-a-changelog"])).unwrap();
        assert_eq!(kac.format, Format::KeepAChangelog);
        assert!(
            Options::parse(&args(&["--site", "c"]))
                .unwrap()
                .site
                .is_some()
        );
        assert!(Options::parse(&args(&["--site", "c", "--format", "keep-a-changelog"])).is_none());
        assert!(Options::parse(&args(&["--format", "html"])).is_none());
        assert!(Options::parse(&args(&["--site"])).is_none());
    }

    #[test]
    fn strips_a_posts_front_matter() {
        assert_eq!(
            strip_front_matter("+++\ndraft = true\n+++\n\nHello\n"),
            "Hello\n"
        );
        assert_eq!(strip_front_matter("Hello\n"), "Hello\n");
    }

    #[test]
    fn the_markdown_snapshot() {
        snapshot("changelog-markdown.snap", &markdown(&built(), true));
    }

    #[test]
    fn the_keep_a_changelog_snapshot() {
        snapshot(
            "changelog-keep-a-changelog.snap",
            &keep_a_changelog(&built()),
        );
    }

    #[test]
    fn the_site_snapshot() {
        let pages = site_pages(&built(), &|tag| {
            (tag == "v0.2.0")
                .then(|| "+++\ndraft = true\n+++\n\nThe timers moved to steps.\n".to_string())
        });
        let paths: Vec<&str> = pages.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            paths,
            [
                "changelog.md",
                "news/generated/release-v0-2-0.md",
                "news/generated/release-v0-1-0.md"
            ]
        );
        let mut all = String::new();
        for (path, text) in &pages {
            let _ = write!(all, "=== {path} ===\n{text}\n");
        }
        snapshot("changelog-site.snap", &all);
    }

    #[test]
    fn run_reads_the_tickets_and_asks_git_for_the_log() {
        let root = std::env::temp_dir().join(format!("xtask-changelog-{}", std::process::id()));
        let done = root.join("tickets").join("done");
        fs::create_dir_all(&done).unwrap();
        let (file, content) = ticket("2801", "The website", "feature", "2026-10-20", "Done.");
        fs::write(done.join(file), content).unwrap();
        let git = |args: &[&str]| {
            assert!(args.contains(&"--first-parent"));
            Ok("2026-10-20\t\t[2801] The website (#25)\n".to_string())
        };
        let options = Options {
            site: None,
            format: Format::Markdown,
        };
        let Ok(Output::Stdout(text)) = run(&root, &options, &git) else {
            panic!("no markdown");
        };
        assert!(text.contains("## 2026-10 — 2026-10-20"));
        assert!(text.contains("/pull/25"));

        let site = Options {
            site: Some(root.join("content")),
            format: Format::Markdown,
        };
        let Ok(Output::Pages(pages)) = run(&root, &site, &git) else {
            panic!("no pages");
        };
        write_pages(&root.join("content"), &pages).unwrap();
        assert!(root.join("content/changelog.md").is_file());
        assert!(
            root.join("content/news/generated/changes-2026-10.md")
                .is_file()
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
