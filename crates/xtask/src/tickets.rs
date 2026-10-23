//! `cargo xtask ticket-lint`: validates ticket files under `tickets/{open,done}`
//! against the conventions in `tickets/README.md`.
//!
//! Ported from the game's lint (its `crates/xtask/src/tickets.rs`), with this
//! repository's differences: `blocked_by` may name a game ticket as
//! `game#NNNN`, `Parked:` tickets have their own rules, and the valid `type`s
//! are this repository's.
//!
//! It also warns, never fails, on a clause in an open ticket that waits on a
//! ticket already done ("if NNNN is done", ticket 2017): the PR that closed
//! it should have settled the clause (the `work-ticket` skill, §5).

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_norway::Value;

const REQUIRED_KEYS: &[&str] = &[
    "id",
    "title",
    "type",
    "milestone",
    "model",
    "effort",
    "status",
    "blocked_by",
    "nick_input",
];

const VALID_TYPES: &[&str] = &["feature", "infra", "decision", "bug", "research"];
const VALID_MODELS: &[&str] = &["haiku-4.5", "sonnet-5", "opus-5.5", "fable-5.1"];
const VALID_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const VALID_STATUSES: &[&str] = &["todo", "in-progress", "blocked", "done"];
const VALID_NICK_INPUTS: &[&str] = &["none", "decision", "setup", "sign-off"];

/// The prefix of a game ticket in `blocked_by`.
const GAME_PREFIX: &str = "game#";
/// The prefix of a parked ticket's title.
const PARKED_TITLE: &str = "Parked:";
/// The start of the note a parked ticket has right under its heading.
const PARKED_NOTE: &str = "> **Parked**";
/// The game repository, for `game#NNNN` ids.
const GAME_REPO: &str = "Zafnok/visions-of-shuyi";

/// Which of the two ticket folders a file lives in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Folder {
    Open,
    Done,
}

impl Folder {
    const fn dirname(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
        }
    }
}

/// The fields pulled out of a ticket's frontmatter that later checks need.
#[derive(Default)]
struct TicketFields {
    id: Option<String>,
    status: Option<String>,
    completed: Option<String>,
    blocked_by: Option<Vec<String>>,
}

/// One ticket file: where it is, what it says, and every problem found in
/// isolation (i.e. not requiring knowledge of other tickets).
struct Ticket {
    /// Path relative to the repo root, for error messages.
    display_path: String,
    folder: Folder,
    fields: TicketFields,
    errors: Vec<String>,
    /// The clauses that wait on other tickets, read from open tickets only.
    clauses: Vec<Clause>,
}

/// A clause in a ticket's body that waits on other tickets, such as "if
/// 2204 is done".
#[derive(Debug, PartialEq, Eq)]
struct Clause {
    /// The clause's first line in the file, frontmatter counted.
    line: usize,
    /// The clause's words, as read.
    text: String,
    /// The ticket ids it waits on.
    ids: Vec<String>,
}

/// Where `game#NNNN` ids are checked.
pub enum GameSource<'a> {
    /// A local checkout of the game repository (`--game-repo <path>`).
    Checkout(&'a Path),
    /// The game repository on GitHub, through `gh api`.
    GitHub,
}

/// The game's ticket ids, or why they couldn't be read.
enum GameTickets {
    Known(HashSet<String>),
    Unavailable(String),
}

/// What the lint found: errors fail it, warnings don't.
#[derive(Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Runs every ticket-lint rule against `repo_root`. `pr_branch` is the
/// `--pr-branch` value, if any. The game's ticket ids are read from `game`
/// only when some ticket names one.
pub fn run(repo_root: &Path, pr_branch: Option<&str>, game: &GameSource) -> Report {
    let mut tickets = Vec::new();
    let mut report = Report::default();

    for folder in [Folder::Open, Folder::Done] {
        let dir = repo_root.join("tickets").join(folder.dirname());
        match read_ticket_files(&dir) {
            Ok(files) => {
                for (filename, content) in files {
                    let display_path = format!("tickets/{}/{filename}", folder.dirname());
                    tickets.push(parse_ticket(display_path, folder, &filename, &content));
                }
            }
            Err(e) => report.errors.push(format!("{}: {e}", dir.display())),
        }
    }

    for ticket in &tickets {
        for err in &ticket.errors {
            report
                .errors
                .push(format!("{}: {err}", ticket.display_path));
        }
    }

    report.errors.extend(check_unique_ids(&tickets));

    let names_a_game_ticket = tickets
        .iter()
        .filter_map(|t| t.fields.blocked_by.as_ref())
        .flatten()
        .any(|dep| dep.starts_with(GAME_PREFIX));
    let game_tickets = if names_a_game_ticket {
        game_ticket_ids(game)
    } else {
        GameTickets::Known(HashSet::new())
    };
    let (errors, warnings) = check_blocked_by_exists(&tickets, &game_tickets);
    report.errors.extend(errors);
    report.warnings.extend(warnings);

    if let Some(branch) = pr_branch {
        report.errors.extend(check_pr_branch(&tickets, branch));
    }

    report.warnings.extend(check_clauses(&tickets));
    report
}

fn read_ticket_files(dir: &Path) -> std::io::Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if Path::new(name).extension().is_none_or(|ext| ext != "md") || name == "README.md" {
            continue;
        }
        let content = fs::read_to_string(entry.path())?;
        files.push((name.to_string(), content));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

fn parse_ticket(display_path: String, folder: Folder, filename: &str, content: &str) -> Ticket {
    let mut errors = Vec::new();

    if !is_valid_ticket_filename(filename) {
        errors.push(format!(
            "filename `{filename}` must match `^\\d{{4}}-[a-z0-9-]+\\.md$`"
        ));
    }

    let mut fields = TicketFields::default();
    // A checkout with CRLF line endings reads the same as one with LF.
    let content = content.replace("\r\n", "\n");
    // Done tickets are history: their clauses are never read.
    let clauses = match (folder, extract_frontmatter(&content)) {
        (Folder::Open, Some((_, body))) => find_clauses(body, first_line_of(&content, body)),
        _ => Vec::new(),
    };

    match extract_frontmatter(&content) {
        None => errors.push("missing or malformed frontmatter (no `---` delimiters)".to_string()),
        Some((fm_text, body)) => match serde_norway::from_str::<Value>(fm_text) {
            Err(e) => errors.push(format!("frontmatter is not valid YAML: {e}")),
            Ok(value) => {
                let Value::Mapping(map) = &value else {
                    errors.push("frontmatter must be a YAML mapping".to_string());
                    return Ticket {
                        display_path,
                        folder,
                        fields,
                        errors,
                        clauses,
                    };
                };

                for key in REQUIRED_KEYS {
                    if !map.contains_key(Value::from(*key)) {
                        errors.push(format!("frontmatter is missing required key `{key}`"));
                    }
                }

                fields.id = scalar_string(map, "id");
                fields.status = scalar_string(map, "status");
                fields.completed = scalar_string(map, "completed");
                fields.blocked_by = string_list(map, "blocked_by");

                if map.contains_key(Value::from("blocked_by"))
                    && fields.blocked_by.is_none()
                    && !matches!(
                        map.get(Value::from("blocked_by")),
                        Some(Value::Sequence(seq)) if seq.is_empty()
                    )
                {
                    errors.push("`blocked_by` must be a list of ticket id strings".to_string());
                }

                if let Some(id) = &fields.id {
                    if let Some(stem) = filename.strip_suffix(".md") {
                        let prefix = stem.split('-').next().unwrap_or(stem);
                        if prefix != id {
                            errors.push(format!(
                                "`id: \"{id}\"` does not match filename prefix `{prefix}`"
                            ));
                        }
                    }
                } else if map.contains_key(Value::from("id")) {
                    errors.push("`id` must be a string".to_string());
                }

                check_enum(map, "type", VALID_TYPES, &mut errors);
                check_enum(map, "model", VALID_MODELS, &mut errors);
                check_enum(map, "effort", VALID_EFFORTS, &mut errors);
                check_enum(map, "status", VALID_STATUSES, &mut errors);
                check_enum(map, "nick_input", VALID_NICK_INPUTS, &mut errors);

                if scalar_string(map, "title").is_some_and(|t| t.starts_with(PARKED_TITLE)) {
                    errors.extend(check_parked(fields.status.as_deref(), body));
                }

                match folder {
                    Folder::Done => {
                        if fields.status.as_deref() != Some("done") {
                            errors.push(
                                "tickets in `tickets/done/` must have `status: done`".to_string(),
                            );
                        }
                        if fields.completed.as_deref().is_none_or(str::is_empty) {
                            errors.push(
                                "tickets in `tickets/done/` must have a `completed:` date"
                                    .to_string(),
                            );
                        }
                    }
                    Folder::Open => {
                        if fields.status.as_deref() == Some("done") {
                            errors.push(
                                "tickets in `tickets/open/` must not have `status: done`"
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        },
    }

    Ticket {
        display_path,
        folder,
        fields,
        errors,
        clauses,
    }
}

/// The line, counted from 1, on which `body` (a suffix of `content`) starts.
fn first_line_of(content: &str, body: &str) -> usize {
    let before = &content[..content.len() - body.len()];
    before.matches('\n').count() + 1
}

/// One word of a ticket's body: lower-cased and its edges stripped of
/// Markdown and punctuation, whether a comma was among what was stripped from
/// its end, and its line.
struct Word {
    text: String,
    comma: bool,
    line: usize,
}

/// What is stripped from a word's edges.
fn is_edge(c: char) -> bool {
    matches!(
        c,
        '*' | '_' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ':' | ';' | '.'
    )
}

/// The words of `body` outside fenced code blocks, each with its line.
/// `first_line` is the body's first line in the file.
fn words(body: &str, first_line: usize) -> Vec<Word> {
    let mut words = Vec::new();
    let mut fenced = false;
    for (n, line) in body.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        for raw in line.split_whitespace() {
            let tail = &raw[raw.trim_end_matches(is_edge).len()..];
            words.push(Word {
                text: raw.trim_matches(is_edge).to_lowercase(),
                comma: tail.contains(','),
                line: first_line + n,
            });
        }
    }
    words
}

/// Whether `words`, from `at`, read `expected` word for word.
fn reads(words: &[Word], at: usize, expected: &[&str]) -> bool {
    expected
        .iter()
        .enumerate()
        .all(|(i, e)| words.get(at + i).is_some_and(|w| w.text == *e))
}

/// The ids from `at`: one four-digit id, or several joined by `,`, `and` or
/// `or`. Returns them and the index of the word after the last one; no ids
/// when the word at `at` isn't one.
fn ids_at(words: &[Word], at: usize) -> (Vec<String>, usize) {
    let mut ids = Vec::new();
    let mut end = at;
    // Whether the next word may be an id: the first one, or one after a
    // comma or a joining word. A `for` over the words, so it always ends.
    let mut open = true;
    for (i, word) in words.iter().enumerate().skip(at) {
        let joins = matches!(word.text.as_str(), "and" | "or");
        if open && is_four_digits(&word.text) {
            ids.push(word.text.clone());
            end = i + 1;
            open = word.comma;
        } else if joins && i == end && !ids.is_empty() {
            open = true;
        } else {
            break;
        }
    }
    (ids, end)
}

/// The clause `words[start..end]`, which waits on `ids`.
fn clause(words: &[Word], start: usize, end: usize, ids: Vec<String>) -> Clause {
    let last = end - 1;
    let text = words[start..end]
        .iter()
        .zip(start..)
        .map(|(w, i)| {
            if w.comma && i < last {
                format!("{},", w.text)
            } else {
                w.text.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    Clause {
        line: words[start].line,
        text,
        ids,
    }
}

/// The clause that starts at `words[start]`, if one does, in one of the
/// forms `find_clauses` names.
fn clause_at(words: &[Word], start: usize) -> Option<Clause> {
    let word = words[start].text.as_str();
    if is_four_digits(word) && words[start].comma && reads(words, start + 1, &["if", "done"]) {
        return Some(clause(words, start, start + 3, vec![word.to_string()]));
    }
    let (ids, end) = match word {
        "either" if reads(words, start, &["either", "order", "with"]) => ids_at(words, start + 3),
        "either" if reads(words, start, &["either", "order", "works", "with"]) => {
            ids_at(words, start + 4)
        }
        "if" | "when" => {
            let (ids, end) = ids_at(words, start + 1);
            let verbs: &[&str] = if word == "if" {
                &["is", "are"]
            } else {
                &["is"]
            };
            let done = verbs.iter().any(|v| reads(words, end, &[*v, "done"]));
            (if done { ids } else { Vec::new() }, end + 2)
        }
        _ => return None,
    };
    (!ids.is_empty()).then(|| clause(words, start, end, ids))
}

/// The clauses in `body` that wait on other tickets, in exactly these forms,
/// in any case, across line breaks and Markdown emphasis:
///
/// 1. `either order with <ids>` (and `either order works with <ids>`);
/// 2. `if <ids> is done` (and `are done`);
/// 3. `when <ids> is done`;
/// 4. `<id>, if done`, as in "the book (NNNN, if done)".
///
/// `<ids>` is one four-digit id, or several joined by `,`, `and` or `or`.
/// Fenced code blocks are skipped. `first_line` is the body's first line in
/// the file.
fn find_clauses(body: &str, first_line: usize) -> Vec<Clause> {
    let words = words(body, first_line);
    (0..words.len())
        .filter_map(|start| clause_at(&words, start))
        .collect()
}

/// One warning per file, line and id for each clause of an open ticket that
/// waits on a ticket in `tickets/done/`.
fn check_clauses(tickets: &[Ticket]) -> Vec<String> {
    let done: HashSet<&str> = tickets
        .iter()
        .filter(|t| t.folder == Folder::Done)
        .filter_map(|t| t.fields.id.as_deref())
        .collect();
    let mut seen = HashSet::new();
    let mut warnings = Vec::new();
    for ticket in tickets {
        for clause in &ticket.clauses {
            for id in clause.ids.iter().filter(|id| done.contains(id.as_str())) {
                if seen.insert((&ticket.display_path, clause.line, id)) {
                    warnings.push(format!(
                        "{}:{}: \"{}\": {id} is done, so settle the clause (work-ticket §5)",
                        ticket.display_path, clause.line, clause.text
                    ));
                }
            }
        }
    }
    warnings
}

/// A `Parked:` ticket has `status: blocked` and a `> **Parked**` note as the
/// first thing under its `# ` heading (`tickets/README.md`, *Parked tickets*).
fn check_parked(status: Option<&str>, body: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if status != Some("blocked") {
        errors.push(format!(
            "a `{PARKED_TITLE}` ticket must have `status: blocked`, not `{}`",
            status.unwrap_or("(none)")
        ));
    }
    let mut lines = body.lines().skip_while(|l| !l.starts_with("# "));
    let has_heading = lines.next().is_some();
    let note = lines.find(|l| !l.trim().is_empty());
    if !has_heading || !note.is_some_and(|l| l.starts_with(PARKED_NOTE)) {
        errors.push(format!(
            "a `{PARKED_TITLE}` ticket must have a `{PARKED_NOTE}` note right under its `# ` heading"
        ));
    }
    errors
}

fn check_enum(map: &serde_norway::Mapping, key: &str, valid: &[&str], errors: &mut Vec<String>) {
    let Some(value) = scalar_string(map, key) else {
        return;
    };
    if !valid.contains(&value.as_str()) {
        errors.push(format!(
            "`{key}: {value}` is not one of {valid:?} (see tickets/README.md)"
        ));
    }
}

/// Reads `map[key]` as a plain scalar string, if present and scalar.
fn scalar_string(map: &serde_norway::Mapping, key: &str) -> Option<String> {
    match map.get(Value::from(key))? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Reads `map[key]` as a list of plain strings, if present and every element
/// is a string.
fn string_list(map: &serde_norway::Mapping, key: &str) -> Option<Vec<String>> {
    let Value::Sequence(seq) = map.get(Value::from(key))? else {
        return None;
    };
    seq.iter()
        .map(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// Exactly four ASCII digits.
fn is_four_digits(s: &str) -> bool {
    s.len() == 4 && s.bytes().all(|b| b.is_ascii_digit())
}

/// `NNNN-slug.md`: exactly four ASCII digits, a hyphen, then one or more
/// lowercase-kebab characters.
fn is_valid_ticket_filename(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    let bytes = stem.as_bytes();
    if bytes.len() < 6 || bytes[4] != b'-' || !bytes[..4].iter().all(u8::is_ascii_digit) {
        return false;
    }
    let slug = &stem[5..];
    !slug.is_empty()
        && slug
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase() || c == '-')
}

/// Splits a ticket into the YAML text between the first `---` line and the
/// next line that starts with `---`, and the body after that line.
fn extract_frontmatter(content: &str) -> Option<(&str, &str)> {
    let after_open = content.strip_prefix("---\n")?;
    let end = after_open.find("\n---")?;
    let rest = &after_open[end + 1..];
    let body = rest.split_once('\n').map_or("", |(_, body)| body);
    Some((&after_open[..end], body))
}

fn check_unique_ids(tickets: &[Ticket]) -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut errors = Vec::new();
    for ticket in tickets {
        let Some(id) = &ticket.fields.id else {
            continue;
        };
        if !seen.insert(id.as_str()) {
            errors.push(format!(
                "{}: duplicate ticket id `{id}`",
                ticket.display_path
            ));
        }
    }
    errors
}

/// Every `blocked_by` id is a ticket of this repository, or a `game#NNNN`
/// that is a ticket of the game. Returns the errors and the warnings (a
/// `game#` id that can't be checked).
fn check_blocked_by_exists(tickets: &[Ticket], game: &GameTickets) -> (Vec<String>, Vec<String>) {
    let known_ids: HashSet<&str> = tickets
        .iter()
        .filter_map(|t| t.fields.id.as_deref())
        .collect();

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for ticket in tickets {
        let Some(blocked_by) = &ticket.fields.blocked_by else {
            continue;
        };
        for dep in blocked_by {
            let path = &ticket.display_path;
            if let Some(game_id) = dep.strip_prefix(GAME_PREFIX) {
                if !is_four_digits(game_id) {
                    errors.push(format!(
                        "{path}: `blocked_by` entry `{dep}` must be `game#NNNN`"
                    ));
                    continue;
                }
                match game {
                    GameTickets::Known(ids) if !ids.contains(game_id) => errors.push(format!(
                        "{path}: `blocked_by` references unknown game ticket `{dep}`"
                    )),
                    GameTickets::Known(_) => {}
                    GameTickets::Unavailable(why) => warnings.push(format!(
                        "{path}: `blocked_by` entry `{dep}` not checked: {why}"
                    )),
                }
            } else if !known_ids.contains(dep.as_str()) {
                errors.push(format!(
                    "{path}: `blocked_by` references unknown ticket id `{dep}`"
                ));
            }
        }
    }
    (errors, warnings)
}

/// The game's ticket ids, from the file names in its `tickets/open` and
/// `tickets/done`.
fn game_ticket_ids(source: &GameSource) -> GameTickets {
    let mut names = Vec::new();
    for folder in [Folder::Open, Folder::Done] {
        let listed = match source {
            GameSource::Checkout(root) => list_dir(&root.join("tickets").join(folder.dirname())),
            GameSource::GitHub => list_on_github(folder.dirname()),
        };
        match listed {
            Ok(found) => names.extend(found),
            Err(why) => return GameTickets::Unavailable(why),
        }
    }
    GameTickets::Known(ticket_ids_from_names(&names))
}

fn list_dir(dir: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect())
}

/// One folder's file names through `gh api` (the contents API lists up to
/// 1000 files in a folder). What it does with `gh`'s answer is
/// `gh_listing`, tested without `gh`.
fn list_on_github(folder: &str) -> Result<Vec<String>, String> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{GAME_REPO}/contents/tickets/{folder}"),
            "--jq",
            ".[].name",
        ])
        .output()
        .map_err(|e| format!("`gh` is not available ({e}); pass --game-repo <path>"))?;
    gh_listing(
        folder,
        output.status.success(),
        &output.stdout,
        &output.stderr,
    )
}

/// The file names `gh api --jq '.[].name'` printed, one per line, or why it
/// failed.
fn gh_listing(
    folder: &str,
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<Vec<String>, String> {
    if !success {
        return Err(format!(
            "`gh api` could not list the game's tickets/{folder} ({}); pass --game-repo <path>",
            String::from_utf8_lossy(stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

/// The ids of the ticket files among `names`.
fn ticket_ids_from_names(names: &[String]) -> HashSet<String> {
    names
        .iter()
        .filter(|n| is_valid_ticket_filename(n))
        .map(|n| n[..4].to_string())
        .collect()
}

/// With `--pr-branch t<NNNN>-<slug>`, ticket `NNNN` must be in `tickets/done/`
/// (that PR is expected to be the one archiving it). Branches that don't
/// follow the `t<NNNN>-` convention (e.g. `main`, `dependabot/...`) are
/// ignored.
fn check_pr_branch(tickets: &[Ticket], branch: &str) -> Vec<String> {
    let Some(id) = ticket_id_from_branch(branch) else {
        return Vec::new();
    };

    let ticket = tickets.iter().find(|t| t.fields.id.as_deref() == Some(id));
    match ticket {
        None => vec![format!(
            "branch `{branch}` names ticket `{id}`, but no ticket with that id exists"
        )],
        Some(t) if t.folder != Folder::Done => vec![format!(
            "branch `{branch}` names ticket `{id}`, which is not in tickets/done/ \
             (the PR for a ticket must move it to done/)"
        )],
        Some(_) => Vec::new(),
    }
}

/// The ticket a branch named `t<NNNN>-<slug>` is for, or `None` for any
/// other branch (`main`, `sonar/…`, `dependabot/…`, `game-NNNN-…`).
pub(crate) fn ticket_id_from_branch(branch: &str) -> Option<&str> {
    let rest = branch.strip_prefix('t')?;
    let (id, _slug) = rest.split_once('-')?;
    is_four_digits(id).then_some(id)
}

/// The heading of the section a ticket's `Breaking:` lines go in.
const COMPLETION_NOTES: &str = "## Completion notes";

/// The `Breaking:` lines of a ticket (ADR-0038): each line under `## Completion
/// notes` that starts with `Breaking:`, after an optional `- ` list marker,
/// returned from `Breaking:` on. Lines in any other section don't count.
pub(crate) fn breaking_lines(body: &str) -> Vec<&str> {
    let mut in_notes = false;
    let mut found = Vec::new();
    // `lines` drops a CR before each LF too.
    for line in body.lines() {
        if line.starts_with("# ") || line.starts_with("## ") {
            in_notes = line.trim_end() == COMPLETION_NOTES;
            continue;
        }
        if !in_notes {
            continue;
        }
        let text = line.trim_start();
        let text = text.strip_prefix("- ").unwrap_or(text);
        if text.starts_with("Breaking:") {
            found.push(text);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(id: &str, folder: Folder, status: &str, blocked_by: &[&str]) -> Ticket {
        Ticket {
            display_path: format!("tickets/{}/{id}-x.md", folder.dirname()),
            folder,
            fields: TicketFields {
                id: Some(id.to_string()),
                status: Some(status.to_string()),
                completed: None,
                blocked_by: Some(blocked_by.iter().map(|s| (*s).to_string()).collect()),
            },
            errors: Vec::new(),
            clauses: Vec::new(),
        }
    }

    fn game(ids: &[&str]) -> GameTickets {
        GameTickets::Known(ids.iter().map(|s| (*s).to_string()).collect())
    }

    // -- filename --

    #[test]
    fn accepts_valid_filenames() {
        assert!(is_valid_ticket_filename("0106-branch-protection.md"));
        assert!(is_valid_ticket_filename("0001-a.md"));
        assert!(is_valid_ticket_filename("0001-a1-b2.md"));
    }

    #[test]
    fn rejects_bad_filenames() {
        assert!(!is_valid_ticket_filename("106-branch-protection.md")); // 3 digits
        assert!(!is_valid_ticket_filename("01006-branch-protection.md")); // 5 digits
        assert!(!is_valid_ticket_filename("0106_branch_protection.md")); // underscores
        assert!(!is_valid_ticket_filename("0106-Branch.md")); // uppercase
        assert!(!is_valid_ticket_filename("0106-.md")); // empty slug
        assert!(!is_valid_ticket_filename("0106-branch-protection.txt")); // wrong ext
        assert!(!is_valid_ticket_filename("readme.md"));
    }

    // -- frontmatter extraction --

    #[test]
    fn extracts_frontmatter_between_delimiters() {
        let content = "---\nid: \"1\"\nfoo: bar\n---\n\n# body\n";
        assert_eq!(
            extract_frontmatter(content),
            Some(("id: \"1\"\nfoo: bar", "\n# body\n"))
        );
    }

    #[test]
    fn missing_delimiters_returns_none() {
        assert_eq!(extract_frontmatter("# no frontmatter here\n"), None);
        assert_eq!(extract_frontmatter("id: \"1\"\n---\n"), None);
    }

    // -- full parse_ticket: required keys / values --

    const VALID_FRONTMATTER: &str = "\
---
id: \"2106\"
title: \"Test ticket\"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: [\"2102\", \"game#1131\"]
nick_input: none
completed:
---

# body
";

    #[test]
    fn accepts_well_formed_open_ticket() {
        let t = parse_ticket(
            "tickets/open/2106-x.md".to_string(),
            Folder::Open,
            "2106-x.md",
            VALID_FRONTMATTER,
        );
        assert!(t.errors.is_empty(), "unexpected errors: {:?}", t.errors);
        assert_eq!(t.fields.id.as_deref(), Some("2106"));
        assert_eq!(
            t.fields.blocked_by.as_deref(),
            Some(["2102".to_string(), "game#1131".to_string()].as_slice())
        );
    }

    #[test]
    fn accepts_crlf_line_endings() {
        let crlf = VALID_FRONTMATTER.replace('\n', "\r\n");
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &crlf);
        assert!(t.errors.is_empty(), "unexpected errors: {:?}", t.errors);
    }

    #[test]
    fn flags_missing_required_key() {
        let fm = "---\nid: \"2106\"\ntitle: x\n---\n";
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", fm);
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("missing required key `type`"))
        );
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("missing required key `blocked_by`"))
        );
    }

    #[test]
    fn flags_malformed_yaml() {
        let fm = "---\nid: \"2106\n---\n";
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", fm);
        assert!(t.errors.iter().any(|e| e.contains("not valid YAML")));
    }

    #[test]
    fn flags_bad_enum_value() {
        let fm = VALID_FRONTMATTER.replace("type: infra", "type: sidequest");
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &fm);
        assert!(t.errors.iter().any(|e| e.contains("`type: sidequest`")));
    }

    #[test]
    fn accepts_each_type_of_this_repository() {
        for ty in ["feature", "infra", "decision", "bug", "research"] {
            let fm = VALID_FRONTMATTER.replace("type: infra", &format!("type: {ty}"));
            let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &fm);
            assert!(t.errors.is_empty(), "type {ty}: {:?}", t.errors);
        }
    }

    #[test]
    fn rejects_the_games_types_this_repository_dropped() {
        for ty in ["design-decision", "content", "tuning", "playtest"] {
            let fm = VALID_FRONTMATTER.replace("type: infra", &format!("type: {ty}"));
            let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &fm);
            assert!(
                t.errors
                    .iter()
                    .any(|e| e.contains(&format!("`type: {ty}`"))),
                "type {ty} was accepted"
            );
        }
    }

    #[test]
    fn flags_blocked_by_that_is_not_a_list() {
        let fm = VALID_FRONTMATTER.replace(
            "blocked_by: [\"2102\", \"game#1131\"]",
            "blocked_by: \"2102\"",
        );
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &fm);
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("must be a list of ticket id strings"))
        );
    }

    #[test]
    fn flags_id_filename_mismatch() {
        let t = parse_ticket(
            "tickets/open/2107-x.md".to_string(),
            Folder::Open,
            "2107-x.md",
            VALID_FRONTMATTER,
        );
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("does not match filename prefix"))
        );
    }

    #[test]
    fn open_ticket_cannot_be_status_done() {
        let fm = VALID_FRONTMATTER.replace("status: todo", "status: done");
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &fm);
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("must not have `status: done`"))
        );
    }

    #[test]
    fn done_ticket_requires_status_done_and_completed_date() {
        let fm = VALID_FRONTMATTER.replace("status: todo", "status: done");
        let t = parse_ticket("p".to_string(), Folder::Done, "2106-x.md", &fm);
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("must have a `completed:` date"))
        );

        let fm_ok = fm.replace("completed:", "completed: 2026-09-25");
        let t_ok = parse_ticket("p".to_string(), Folder::Done, "2106-x.md", &fm_ok);
        assert!(
            t_ok.errors.is_empty(),
            "unexpected errors: {:?}",
            t_ok.errors
        );
    }

    #[test]
    fn done_ticket_wrong_status_is_flagged() {
        let t = parse_ticket(
            "p".to_string(),
            Folder::Done,
            "2106-x.md",
            VALID_FRONTMATTER, // status: todo
        );
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("must have `status: done`"))
        );
    }

    // -- parked tickets --

    fn parked(status: &str, body: &str) -> String {
        VALID_FRONTMATTER
            .replace("\"Test ticket\"", "\"Parked: test ticket\"")
            .replace("status: todo", &format!("status: {status}"))
            .replace("\n# body\n", body)
    }

    const PARKED_BODY: &str =
        "\n# 2106 — Test\n\n> **Parked** (Nick, 2026-10-17): later.\n\n## Context\n";

    #[test]
    fn accepts_a_parked_ticket_with_its_note() {
        let t = parse_ticket(
            "p".to_string(),
            Folder::Open,
            "2106-x.md",
            &parked("blocked", PARKED_BODY),
        );
        assert!(t.errors.is_empty(), "unexpected errors: {:?}", t.errors);
    }

    #[test]
    fn a_parked_ticket_must_be_blocked() {
        let t = parse_ticket(
            "p".to_string(),
            Folder::Open,
            "2106-x.md",
            &parked("todo", PARKED_BODY),
        );
        assert_eq!(t.errors.len(), 1, "{:?}", t.errors);
        assert!(t.errors[0].contains("must have `status: blocked`, not `todo`"));
    }

    #[test]
    fn a_parked_ticket_needs_the_note_right_under_its_heading() {
        let missing = "\n# 2106 — Test\n\n## Context\n";
        let late = "\n# 2106 — Test\n\nSome text first.\n\n> **Parked**: later.\n";
        let no_heading = "\n> **Parked**: later.\n";
        for body in [missing, late, no_heading] {
            let t = parse_ticket(
                "p".to_string(),
                Folder::Open,
                "2106-x.md",
                &parked("blocked", body),
            );
            assert_eq!(t.errors.len(), 1, "body {body:?}: {:?}", t.errors);
            assert!(t.errors[0].contains("`> **Parked**` note right under"));
        }
    }

    #[test]
    fn a_ticket_not_parked_has_no_parked_rules() {
        let fm = VALID_FRONTMATTER.replace("status: todo", "status: blocked");
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &fm);
        assert!(t.errors.is_empty(), "unexpected errors: {:?}", t.errors);
    }

    // -- cross-file checks --

    #[test]
    fn detects_duplicate_ids() {
        let tickets = vec![
            ticket("2100", Folder::Open, "todo", &[]),
            ticket("2100", Folder::Open, "todo", &[]),
        ];
        let errors = check_unique_ids(&tickets);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("duplicate ticket id `2100`"));
    }

    #[test]
    fn detects_unknown_blocked_by() {
        let tickets = vec![ticket("2100", Folder::Open, "todo", &["2099"])];
        let (errors, warnings) = check_blocked_by_exists(&tickets, &game(&[]));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("unknown ticket id `2099`"));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn allows_blocked_by_that_exists() {
        let tickets = vec![
            ticket("2100", Folder::Done, "done", &[]),
            ticket("2101", Folder::Open, "todo", &["2100"]),
        ];
        let (errors, warnings) = check_blocked_by_exists(&tickets, &game(&[]));
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_plain_id_is_never_looked_up_in_the_game() {
        let tickets = vec![ticket("2100", Folder::Open, "todo", &["1131"])];
        let (errors, _) = check_blocked_by_exists(&tickets, &game(&["1131"]));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("unknown ticket id `1131`"));
    }

    #[test]
    fn allows_a_game_ticket_the_game_has() {
        let tickets = vec![ticket("2100", Folder::Open, "todo", &["game#1131"])];
        let (errors, warnings) = check_blocked_by_exists(&tickets, &game(&["1131"]));
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn detects_a_game_ticket_the_game_lacks() {
        let tickets = vec![ticket("2100", Folder::Open, "todo", &["game#9999"])];
        let (errors, warnings) = check_blocked_by_exists(&tickets, &game(&["1131"]));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("unknown game ticket `game#9999`"));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn detects_a_malformed_game_id() {
        for dep in ["game#131", "game#11311", "game#abcd", "game#"] {
            let tickets = vec![ticket("2100", Folder::Open, "todo", &[dep])];
            let (errors, _) = check_blocked_by_exists(&tickets, &game(&["1131"]));
            assert_eq!(errors.len(), 1, "{dep}");
            assert!(errors[0].contains("must be `game#NNNN`"), "{dep}");
        }
    }

    #[test]
    fn warns_and_skips_when_the_game_is_unavailable() {
        let tickets = vec![ticket("2100", Folder::Open, "todo", &["game#9999"])];
        let unavailable = GameTickets::Unavailable("no gh".to_string());
        let (errors, warnings) = check_blocked_by_exists(&tickets, &unavailable);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("`game#9999` not checked: no gh"));
    }

    #[test]
    fn game_ids_come_from_ticket_file_names_only() {
        let names = [
            "1131-the-code.md",
            "0903-x.md",
            "README.md",
            "TEMPLATE.md",
            "notes.txt",
        ]
        .map(str::to_string);
        let ids = ticket_ids_from_names(&names);
        let expected: HashSet<String> = ["1131", "0903"].map(str::to_string).into();
        assert_eq!(ids, expected);
    }

    #[test]
    fn game_ids_from_a_local_checkout() {
        let root = make_temp_tickets_dir("game-checkout");
        fs::write(root.join("tickets/open/1131-a.md"), "").unwrap();
        fs::write(root.join("tickets/done/0903-b.md"), "").unwrap();
        let GameTickets::Known(ids) = game_ticket_ids(&GameSource::Checkout(&root)) else {
            panic!("the checkout was not read");
        };
        let expected: HashSet<String> = ["1131", "0903"].map(str::to_string).into();
        assert_eq!(ids, expected);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn gh_listing_reads_one_name_per_line() {
        assert_eq!(
            gh_listing(
                "open",
                true,
                b"1131-a.md
README.md
",
                b""
            ),
            Ok(vec!["1131-a.md".to_string(), "README.md".to_string()])
        );
    }

    #[test]
    fn gh_listing_says_why_gh_failed() {
        let why = gh_listing(
            "done",
            false,
            b"1131-a.md
",
            b" HTTP 404 
",
        )
        .unwrap_err();
        assert!(why.contains("tickets/done (HTTP 404)"), "{why}");
        assert!(why.contains("--game-repo"), "{why}");
    }

    #[test]
    fn a_missing_game_checkout_is_unavailable() {
        let root = std::env::temp_dir().join("xtask-no-such-game-checkout");
        assert!(matches!(
            game_ticket_ids(&GameSource::Checkout(&root)),
            GameTickets::Unavailable(_)
        ));
    }

    // -- pr-branch --

    #[test]
    fn pr_branch_ok_when_ticket_is_done() {
        let tickets = vec![ticket("2304", Folder::Done, "done", &[])];
        let left = check_pr_branch(&tickets, "t2304-pathfinding");
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn pr_branch_fails_when_ticket_still_open() {
        let tickets = vec![ticket("2304", Folder::Open, "todo", &[])];
        let errors = check_pr_branch(&tickets, "t2304-pathfinding");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not in tickets/done/"));
    }

    #[test]
    fn pr_branch_fails_when_ticket_unknown() {
        let tickets: Vec<Ticket> = Vec::new();
        let errors = check_pr_branch(&tickets, "t9999-ghost");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("no ticket with that id exists"));
    }

    #[test]
    fn pr_branch_ignores_non_ticket_branches() {
        let tickets: Vec<Ticket> = Vec::new();
        let left = check_pr_branch(&tickets, "main");
        assert!(left.is_empty(), "{left:?}");
        let left = check_pr_branch(&tickets, "dependabot/cargo/foo-1.2.3");
        assert!(left.is_empty(), "{left:?}");
        let left = check_pr_branch(&tickets, "claude/some-work");
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn ticket_id_from_branch_parses_prefix() {
        assert_eq!(ticket_id_from_branch("t2304-pathfinding"), Some("2304"));
        assert_eq!(ticket_id_from_branch("t304-pathfinding"), None);
        assert_eq!(ticket_id_from_branch("main"), None);
        assert_eq!(ticket_id_from_branch("t2304"), None);
    }

    // -- Breaking: lines --

    #[test]
    fn breaking_lines_reads_the_completion_notes_with_or_without_a_marker() {
        let body = "# 2100 — A\n\n## Scope\n\nBreaking: shuyi-ui: not here\n\n\
                    ## Completion notes\n\nDone.\n\n\
                    Breaking: shuyi-core: `Grid::new`: pass the size.\n\
                    - Breaking: shuyi-ui, shuyi-app: `Frame`: build it with `Frame::new`.\n\
                    Not Breaking: this one.\n";
        assert_eq!(
            breaking_lines(body),
            vec![
                "Breaking: shuyi-core: `Grid::new`: pass the size.",
                "Breaking: shuyi-ui, shuyi-app: `Frame`: build it with `Frame::new`.",
            ]
        );
    }

    #[test]
    fn breaking_lines_reads_crlf_line_endings() {
        let body = "## Completion notes\r\n\r\n- Breaking: shuyi-core: `f`: call `g`.\r\n";
        assert_eq!(
            breaking_lines(body),
            vec!["Breaking: shuyi-core: `f`: call `g`."]
        );
    }

    #[test]
    fn breaking_lines_stop_at_the_next_section() {
        let body = "## Completion notes\n\nBreaking: shuyi-core: `f`: call `g`.\n\n\
                    ## Later\n\nBreaking: shuyi-ui: `h`: gone.\n";
        assert_eq!(
            breaking_lines(body),
            vec!["Breaking: shuyi-core: `f`: call `g`."]
        );
    }

    #[test]
    fn breaking_lines_are_none_without_completion_notes() {
        assert_eq!(
            breaking_lines("## Scope\n\nBreaking: shuyi-core: `f`: call `g`.\n"),
            Vec::<&str>::new()
        );
    }

    // -- clauses about other tickets --

    /// The ids of each clause `find_clauses` finds in `body`.
    fn clause_ids(body: &str) -> Vec<Vec<String>> {
        find_clauses(body, 1).into_iter().map(|c| c.ids).collect()
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn finds_each_form_in_any_case() {
        for (body, text) in [
            ("Either order with 2204: fine.", "either order with 2204"),
            (
                "EITHER ORDER WORKS WITH 2204.",
                "either order works with 2204",
            ),
            ("Then, if 2204 is done, use it.", "if 2204 is done"),
            ("If 2204 IS DONE use it.", "if 2204 is done"),
            ("if 2204 are done", "if 2204 are done"),
            ("When 2204 is done, use it.", "when 2204 is done"),
            ("the book (2204, if done).", "2204, if done"),
            ("The book (2204, IF DONE).", "2204, if done"),
        ] {
            let found = find_clauses(body, 1);
            assert_eq!(found.len(), 1, "{body}: {found:?}");
            assert_eq!(found[0].text, text, "{body}");
            assert_eq!(found[0].ids, ids(&["2204"]), "{body}");
        }
    }

    #[test]
    fn reads_ids_joined_by_commas_and_or() {
        assert_eq!(
            clause_ids("if 2001, 2002 and 2003 are done"),
            vec![ids(&["2001", "2002", "2003"])]
        );
        assert_eq!(
            clause_ids("either order with 2001 or 2002."),
            vec![ids(&["2001", "2002"])]
        );
        assert_eq!(
            clause_ids("Either order with 2001, 2002, and 2003: x"),
            vec![ids(&["2001", "2002", "2003"])]
        );
        let found = find_clauses("if 2001, 2002 and 2003 are done", 1);
        assert_eq!(found[0].text, "if 2001, 2002 and 2003 are done");
    }

    #[test]
    fn ids_stop_where_no_id_follows() {
        assert_eq!(
            clause_ids("either order with 2001 and the rest."),
            vec![ids(&["2001"])]
        );
        assert_eq!(
            clause_ids("either order with 2001, then x"),
            vec![ids(&["2001"])]
        );
        let found = find_clauses("either order with 2001 and the rest", 1);
        assert_eq!(found[0].text, "either order with 2001");
        assert_eq!(
            clause_ids("either order with 2001 and and 2002"),
            vec![ids(&["2001"])]
        );
        assert_eq!(
            clause_ids("either order with 2001 2002"),
            vec![ids(&["2001"])]
        );
        assert_eq!(
            clause_ids("either order with 2001 the 2002"),
            vec![ids(&["2001"])]
        );
        let left = clause_ids("either order with and 2001");
        assert!(left.is_empty(), "{left:?}");
        let left = clause_ids("if 2001 2002 is done");
        assert!(left.is_empty(), "{left:?}");
        let left = clause_ids("if 2001 then is done");
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn needs_the_whole_form() {
        for body in [
            "if 2204 is",
            "if 2204 is ready",
            "if 2204 were done",
            "when 2204 are done",
            "when 2204",
            "if done",
            "if 22044 is done",
            "if two is done",
            "either order",
            "either order with",
            "either way with 2204",
            "either order about 2204",
            "either order works for 2204",
            "2204 if done",
            "2204, if",
            "2204, when done",
            "2204, if ready",
            "abcd, if done",
            "then 2204 is done",
            "2204",
        ] {
            assert!(find_clauses(body, 1).is_empty(), "{body}");
        }
    }

    #[test]
    fn a_clause_may_run_across_lines_and_emphasis() {
        let body = "Intro.\n- **Either order\nwith 2204:** text.\n";
        let found = find_clauses(body, 10);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 11);
        assert_eq!(found[0].ids, ids(&["2204"]));
        assert_eq!(clause_ids("If `2204`\nis _done_, x"), vec![ids(&["2204"])]);
        assert_eq!(clause_ids("(**2204**), if done"), vec![ids(&["2204"])]);
    }

    #[test]
    fn skips_fenced_code_blocks() {
        let body =
            "```text\nif 2204 is done\n```\n  ```\nif 2205 is done\n  ```\nif 2206 is done\n";
        let found = find_clauses(body, 1);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].ids, ids(&["2206"]));
        assert_eq!(found[0].line, 7);
    }

    #[test]
    fn counts_the_frontmatter_in_the_line() {
        let content = VALID_FRONTMATTER.replace("# body", "# body\n\nif 2204 is done");
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &content);
        assert_eq!(t.clauses.len(), 1);
        assert_eq!(t.clauses[0].line, 16);
        let crlf = content.replace('\n', "\r\n");
        let t = parse_ticket("p".to_string(), Folder::Open, "2106-x.md", &crlf);
        assert_eq!(t.clauses[0].line, 16);
    }

    #[test]
    fn a_done_ticket_has_no_clauses() {
        let content = VALID_FRONTMATTER.replace("# body", "if 2204 is done");
        let t = parse_ticket("p".to_string(), Folder::Done, "2106-x.md", &content);
        assert!(t.clauses.is_empty(), "{:?}", t.clauses);
    }

    fn with_clauses(id: &str, folder: Folder, body: &str) -> Ticket {
        Ticket {
            clauses: find_clauses(body, 1),
            ..ticket(id, folder, "todo", &[])
        }
    }

    #[test]
    fn warns_on_done_ids_only() {
        let tickets = vec![
            ticket("2001", Folder::Done, "done", &[]),
            ticket("2002", Folder::Open, "todo", &[]),
            ticket("2003", Folder::Done, "done", &[]),
            with_clauses(
                "2100",
                Folder::Open,
                "x\nif 2001, 2002 and 2003 are done, y; 2003 is old. 2002, if done",
            ),
        ];
        let warnings = check_clauses(&tickets);
        assert_eq!(
            warnings,
            vec![
                "tickets/open/2100-x.md:2: \"if 2001, 2002 and 2003 are done\": 2001 is done, \
                 so settle the clause (work-ticket §5)",
                "tickets/open/2100-x.md:2: \"if 2001, 2002 and 2003 are done\": 2003 is done, \
                 so settle the clause (work-ticket §5)",
            ]
        );
    }

    #[test]
    fn warns_once_per_file_line_and_id() {
        let tickets = vec![
            ticket("2001", Folder::Done, "done", &[]),
            with_clauses(
                "2100",
                Folder::Open,
                "if 2001 is done (2001, if done)\nif 2001 is done",
            ),
            with_clauses("2101", Folder::Open, "if 2001 is done"),
        ];
        let warnings = check_clauses(&tickets);
        assert_eq!(warnings.len(), 3, "{warnings:#?}");
        assert!(warnings[0].starts_with("tickets/open/2100-x.md:1: "));
        assert!(warnings[1].starts_with("tickets/open/2100-x.md:2: "));
        assert!(warnings[2].starts_with("tickets/open/2101-x.md:1: "));
    }

    #[test]
    fn run_warns_on_a_stale_clause_without_failing() {
        let root = make_temp_tickets_dir("run-stale-clause");
        let open = VALID_FRONTMATTER
            .replace("\"2102\", \"game#1131\"", "")
            .replace(
                "# body",
                "# body\n\nIf 2102 is done, x.\n\n```\nif 2102 is done\n```",
            );
        fs::write(root.join("tickets/open/2106-x.md"), open).unwrap();
        let done = VALID_FRONTMATTER
            .replace("2106", "2102")
            .replace("\"2102\", \"game#1131\"", "")
            .replace("status: todo", "status: done")
            .replace("completed:", "completed: 2026-10-21")
            .replace("# body", "if 2106 is done");
        fs::write(root.join("tickets/done/2102-x.md"), done).unwrap();

        let report = run(&root, None, &GameSource::Checkout(&root));
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(
            report.warnings,
            vec![
                "tickets/open/2106-x.md:16: \"if 2102 is done\": 2102 is done, so settle the \
                 clause (work-ticket §5)"
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    // -- end to end over the real repo --

    #[test]
    fn real_repo_tickets_pass() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask");
        // Without the game: its ids are checked by `cargo xtask ticket-lint`.
        let no_game = repo_root.join("target/xtask-no-game");
        let report = run(repo_root, None, &GameSource::Checkout(&no_game));
        assert!(
            report.errors.is_empty(),
            "ticket-lint errors: {:#?}",
            report.errors
        );
    }

    // -- reading a directory for real (not just in-memory frontmatter) --

    fn make_temp_tickets_dir(unique: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xtask-ticket-lint-test-{unique}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tickets/open")).unwrap();
        fs::create_dir_all(root.join("tickets/done")).unwrap();
        root
    }

    #[test]
    fn read_ticket_files_skips_non_md_and_readme() {
        let root = make_temp_tickets_dir("read-skips");
        let dir = root.join("tickets/open");
        fs::write(dir.join("2100-a.md"), "a").unwrap();
        fs::write(dir.join("notes.txt"), "not a ticket").unwrap();
        fs::write(dir.join("README.md"), "not a ticket either").unwrap();

        let files = read_ticket_files(&dir).unwrap();

        assert_eq!(
            files.into_iter().map(|(name, _)| name).collect::<Vec<_>>(),
            vec!["2100-a.md".to_string()]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_reports_real_errors_from_a_directory_tree() {
        let root = make_temp_tickets_dir("run-real-errors");
        let open_dir = root.join("tickets/open");
        fs::write(open_dir.join("notes.txt"), "ignored, wrong extension").unwrap();
        fs::write(
            open_dir.join("2200-bad.md"),
            "---\nid: \"2200\"\n---\n\n# body\n",
        )
        .unwrap();

        let report = run(&root, None, &GameSource::Checkout(&root));
        let errors = report.errors;

        assert!(!errors.is_empty(), "{errors:?}");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("missing required key `title`")),
            "errors: {errors:#?}"
        );
        assert!(
            !errors.iter().any(|e| e.contains("notes.txt")),
            "notes.txt should have been skipped, not linted: {errors:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_checks_game_ids_against_a_checkout() {
        let root = make_temp_tickets_dir("run-game-ids");
        let game_root = make_temp_tickets_dir("run-game-ids-game");
        fs::write(game_root.join("tickets/done/1131-x.md"), "").unwrap();
        let ticket = VALID_FRONTMATTER.replace("\"2102\", ", "");
        fs::write(root.join("tickets/open/2106-x.md"), &ticket).unwrap();
        let ok = run(&root, None, &GameSource::Checkout(&game_root));
        assert!(
            ok.errors.is_empty() && ok.warnings.is_empty(),
            "{:?}",
            ok.errors
        );

        fs::write(
            root.join("tickets/open/2106-x.md"),
            ticket.replace("game#1131", "game#1132"),
        )
        .unwrap();
        let bad = run(&root, None, &GameSource::Checkout(&game_root));
        assert_eq!(bad.errors.len(), 1, "{:?}", bad.errors);
        assert!(bad.errors[0].contains("unknown game ticket `game#1132`"));

        let missing = run(&root, None, &GameSource::Checkout(&root.join("nowhere")));
        assert!(missing.errors.is_empty(), "{:?}", missing.errors);
        assert_eq!(missing.warnings.len(), 1);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&game_root);
    }

    // -- scalar_string / string_list on raw YAML values --

    fn mapping_from(yaml: &str) -> serde_norway::Mapping {
        match serde_norway::from_str::<Value>(yaml).unwrap() {
            Value::Mapping(m) => m,
            other => panic!("expected a mapping, got {other:?}"),
        }
    }

    #[test]
    fn scalar_string_reads_every_scalar_kind() {
        let map = mapping_from("s: hello\nn: 42\nb: true\nl: [1, 2]\n");
        assert_eq!(scalar_string(&map, "s"), Some("hello".to_string()));
        assert_eq!(scalar_string(&map, "n"), Some("42".to_string()));
        assert_eq!(scalar_string(&map, "b"), Some("true".to_string()));
        assert_eq!(scalar_string(&map, "l"), None);
        assert_eq!(scalar_string(&map, "missing"), None);
    }

    #[test]
    fn string_list_requires_a_sequence_of_strings() {
        let map = mapping_from("l: [\"a\", \"b\"]\nmixed: [\"a\", 1]\nnot_a_list: x\n");
        assert_eq!(
            string_list(&map, "l"),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(string_list(&map, "mixed"), None);
        assert_eq!(string_list(&map, "not_a_list"), None);
        assert_eq!(string_list(&map, "missing"), None);
    }
}
