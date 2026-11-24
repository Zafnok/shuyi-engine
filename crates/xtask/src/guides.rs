//! The guides, `docs/guides/` (ADR-0009 §5, ticket 2004): a direction
//! guide for each answer a game picks its kind by, and a move guide for
//! each move between two directions.
//!
//! A guide starts with its title (`# …`), then a list of its facts, each
//! `- **Key:** value`:
//!
//! - `Covers:` the answers a direction guide covers, as `concern/answer`
//!   ids from `docs/modules.ron`, each in backticks; or `Move:` a move
//!   guide's two, `` `a/b` to `c/d` ``;
//! - `Status:` `ready` or `planned`;
//! - `Tickets:` the tickets that build it, by id, `, ` between (none for a
//!   ready guide with nothing left to build).
//!
//! [`check`], part of `cargo xtask check-modules`, fails on:
//!
//! 1. a guide whose facts don't read;
//! 2. an answer a guide names that `docs/modules.ron` doesn't have;
//! 3. an answer of [`NEEDED`] that no direction guide covers;
//! 4. a relative link to a file or folder that isn't there;
//! 5. a ticket a guide names that is neither open nor done, and a planned
//!    guide that names no open ticket;
//! 6. a guide the README doesn't link;
//! 7. a guide with a fenced Rust block that [`DOC_TESTS`] doesn't include
//!    as a doc test.
//!
//! [`write_site`], part of `cargo xtask site-data`, copies them into the
//! book (ADR-0090 §1–§2): the README as the section's `_index.md`, each
//! guide as a page, in the README's order, with front matter, a link
//! between guides made a Zola link and any other relative link one to the
//! file on GitHub.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::modules::{CONCERNS, Concerns};

#[cfg(test)]
mod tests;

/// The guides' folder, from the repository's root.
pub const GUIDES: &str = "docs/guides";

/// The guides' landing page, in [`GUIDES`].
const README: &str = "README.md";

/// The book's section the guides are copied into, under the site's folder.
const SECTION: &str = "content/learn/book/guides";

/// The file whose doc tests run the guides' Rust, each guide included by
/// `include_str!("../../../docs/guides/<name>")`.
pub const DOC_TESTS: &str = "crates/shuyi/src/lib.rs";

/// Where a relative link that leaves the guides points on the site.
const BLOB: &str = "https://github.com/Zafnok/shuyi-engine/blob/main/";

/// The concerns every answer of which needs a direction guide, with the
/// answers left out: `flat` is every 2D direction's drawing.
pub const NEEDED: [(&str, &[&str]); 3] = [
    ("perspective", &[]),
    ("kind", &[]),
    ("world-drawing", &["flat"]),
];

/// A guide's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Built, with its example.
    Ready,
    /// Waiting for its tickets.
    Planned,
}

/// What a guide names in its first lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// Its title, from its `# ` line.
    pub title: String,
    /// The answers a direction guide covers.
    pub covers: Vec<String>,
    /// A move guide's two answers, from and to.
    pub moves: Vec<String>,
    /// Its status.
    pub status: Status,
    /// The tickets it names.
    pub tickets: Vec<String>,
}

/// The ids in backticks in `value`.
fn ids(value: &str) -> Vec<String> {
    value
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The facts of the guide `name`, read from its `text`; or why they don't
/// read.
pub fn facts(name: &str, text: &str) -> Result<Facts, String> {
    let mut lines = text.lines().skip_while(|line| line.trim().is_empty());
    let title = lines
        .next()
        .and_then(|line| line.strip_prefix("# "))
        .ok_or_else(|| format!("{GUIDES}/{name}: its first line isn't its title, `# …`"))?
        .trim()
        .to_owned();
    let (mut covers, mut moves, mut status, mut tickets) = (None, None, None, None);
    for line in lines
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| line.starts_with("- **"))
    {
        let Some((key, value)) = line
            .strip_prefix("- **")
            .and_then(|rest| rest.split_once(":** "))
        else {
            return Err(format!(
                "{GUIDES}/{name}: `{line}` isn't a fact, `- **Key:** value`"
            ));
        };
        match key {
            "Covers" => covers = Some(ids(value)),
            "Move" => moves = Some(ids(value)),
            "Status" => {
                status = Some(match value.trim() {
                    "ready" => Status::Ready,
                    "planned" => Status::Planned,
                    other => {
                        return Err(format!(
                            "{GUIDES}/{name}: its status is `{other}`, not `ready` or `planned`"
                        ));
                    }
                });
            }
            "Tickets" => {
                tickets = Some(
                    value
                        .split(',')
                        .map(|id| id.trim().to_owned())
                        .filter(|id| !id.is_empty())
                        .collect(),
                );
            }
            other => {
                return Err(format!(
                    "{GUIDES}/{name}: `{other}` is no fact of a guide (Covers, Move, Status, \
                     Tickets)"
                ));
            }
        }
    }
    let Some(status) = status else {
        return Err(format!(
            "{GUIDES}/{name}: no `- **Status:** ready` or `planned` under its title"
        ));
    };
    let (covers, moves) = match (covers, moves) {
        (Some(covers), None) if !covers.is_empty() => (covers, Vec::new()),
        (None, Some(moves)) if moves.len() == 2 => (Vec::new(), moves),
        (None, Some(_)) => {
            return Err(format!(
                "{GUIDES}/{name}: a move names two answers, `` `a/b` to `c/d` ``"
            ));
        }
        _ => {
            return Err(format!(
                "{GUIDES}/{name}: say what it covers under its title, `- **Covers:** \
                 `concern/answer`, …` for a direction or `- **Move:** `a/b` to `c/d`` for a move"
            ));
        }
    };
    Ok(Facts {
        title,
        covers,
        moves,
        status,
        tickets: tickets.unwrap_or_default(),
    })
}

/// The targets of the Markdown links of `text`, `[…](target)`, outside its
/// fenced blocks and its code spans.
pub fn links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let prose: String = line.split('`').step_by(2).collect();
        let mut rest = prose.as_str();
        while let Some(at) = rest.find("](") {
            rest = &rest[at + 2..];
            let Some(end) = rest.find(')') else { break };
            out.push(rest[..end].to_owned());
            rest = &rest[end..];
        }
    }
    out
}

/// Whether `target` leaves the repository's files: a web address, a mail
/// or an anchor on the page.
fn outside(target: &str) -> bool {
    target.starts_with('#') || target.contains("://") || target.starts_with("mailto:")
}

/// The path from the repository's root a link `target` (its anchor cut)
/// from a page of [`GUIDES`] names, or `None` when it climbs out of it.
pub fn resolve(target: &str) -> Option<String> {
    let path = target.split('#').next().unwrap_or(target);
    let mut parts: Vec<&str> = GUIDES.split('/').collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// The tickets of the repository, by id.
#[derive(Debug, Default)]
pub struct Tickets {
    /// The ids in `tickets/open/`.
    pub open: BTreeSet<String>,
    /// The ids in `tickets/done/`.
    pub done: BTreeSet<String>,
}

/// Whether `text` has a fenced Rust block.
fn has_rust(text: &str) -> bool {
    text.lines().any(|line| {
        let fence = line.trim_start().trim_start_matches(['`', '~']);
        line.trim_start().starts_with("```")
            && (fence.starts_with("rust") || fence.starts_with("rs"))
    })
}

/// Every error of the guides: `readme` is the README's text, `guides` each
/// other page's file name and text, `doc_tests` the text of [`DOC_TESTS`].
/// `exists` says whether a path from the repository's root is a file or a
/// folder.
pub fn check(
    concerns: &Concerns,
    readme: &str,
    guides: &[(String, String)],
    tickets: &Tickets,
    doc_tests: &str,
    exists: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut errors = Vec::new();
    let answers: BTreeSet<String> = concerns
        .concerns
        .iter()
        .flat_map(|c| c.answers.iter().map(move |a| format!("{}/{}", c.id, a.id)))
        .collect();
    let mut covered = BTreeSet::new();
    let linked: BTreeSet<String> = links(readme)
        .iter()
        .filter(|t| !outside(t))
        .filter_map(|t| t.split('#').next().map(str::to_owned))
        .collect();

    for (name, text) in guides {
        // Rule 7.
        if has_rust(text) && !doc_tests.contains(&format!("\"../../../{GUIDES}/{name}\"")) {
            errors.push(format!(
                "{GUIDES}/{name} has Rust, and {DOC_TESTS} doesn't run it: add                  `#[cfg(doctest)] #[doc = include_str!(\"../../../{GUIDES}/{name}\")]` on an                  item there"
            ));
        }
        // Rule 6.
        if !linked.contains(name) {
            errors.push(format!(
                "{GUIDES}/{name} isn't linked from {GUIDES}/{README}: list it there"
            ));
        }
        // Rule 1.
        let facts = match facts(name, text) {
            Ok(facts) => facts,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        // Rule 2.
        for id in facts.covers.iter().chain(&facts.moves) {
            if !answers.contains(id) {
                errors.push(format!(
                    "{GUIDES}/{name} names `{id}`, which isn't a `concern/answer` of {CONCERNS}"
                ));
            }
        }
        covered.extend(facts.covers.iter().cloned());
        // Rule 5.
        for id in &facts.tickets {
            if !tickets.open.contains(id) && !tickets.done.contains(id) {
                errors.push(format!(
                    "{GUIDES}/{name} names ticket {id}, which is in neither tickets/open/ nor \
                     tickets/done/"
                ));
            }
        }
        if facts.status == Status::Planned
            && !facts.tickets.iter().any(|id| tickets.open.contains(id))
        {
            errors.push(format!(
                "{GUIDES}/{name} is planned and names no open ticket: name the tickets that \
                 build it, or write it in full and mark it ready"
            ));
        }
    }

    // Rule 3.
    for (concern, left_out) in NEEDED {
        let Some(concern) = concerns.concerns.iter().find(|c| c.id == concern) else {
            continue;
        };
        for answer in concern
            .answers
            .iter()
            .filter(|a| !left_out.contains(&a.id.as_str()))
        {
            let id = format!("{}/{}", concern.id, answer.id);
            if !covered.contains(&id) {
                errors.push(format!(
                    "no guide in {GUIDES}/ covers `{id}`: write one (a planned page naming its \
                     tickets, until it is built), with `- **Covers:** `{id}`` under its title \
                     (ADR-0009 §5)"
                ));
            }
        }
    }

    // Rule 4.
    let pages = std::iter::once((README, readme)).chain(
        guides
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str())),
    );
    for (name, text) in pages {
        for target in links(text).iter().filter(|t| !outside(t)) {
            match resolve(target) {
                Some(path) if exists(&path) => {}
                Some(path) => errors.push(format!(
                    "{GUIDES}/{name} links `{target}`, and {path} isn't there"
                )),
                None => errors.push(format!(
                    "{GUIDES}/{name} links `{target}`, which climbs out of the repository"
                )),
            }
        }
    }
    errors
}

/// Whether the file `name` is Markdown.
fn markdown(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// The ids of the tickets in `dir`, from their file names (`NNNN-slug.md`).
fn ticket_ids(dir: &Path) -> Result<BTreeSet<String>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| markdown(name))
        .filter_map(|name| name.split_once('-').map(|(id, _)| id.to_owned()))
        .collect())
}

/// The README's text and every other guide's file name and text, by name.
pub fn read(root: &Path) -> Result<(String, Vec<(String, String)>), String> {
    let dir = root.join(GUIDES);
    let readme =
        fs::read_to_string(dir.join(README)).map_err(|e| format!("{GUIDES}/{README}: {e}"))?;
    let mut guides = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("{GUIDES}: {e}"))? {
        let entry = entry.map_err(|e| format!("{GUIDES}: {e}"))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name == README || !markdown(&name) {
            continue;
        }
        let text = fs::read_to_string(entry.path()).map_err(|e| format!("{GUIDES}/{name}: {e}"))?;
        guides.push((name, text));
    }
    guides.sort();
    Ok((readme, guides))
}

/// Checks the guides of the repository at `root` against `concerns`.
pub fn run(root: &Path, concerns: &Concerns) -> Vec<String> {
    let read_all = || -> Result<_, String> {
        let (readme, guides) = read(root)?;
        let tickets = Tickets {
            open: ticket_ids(&root.join("tickets/open"))?,
            done: ticket_ids(&root.join("tickets/done"))?,
        };
        let doc_tests =
            fs::read_to_string(root.join(DOC_TESTS)).map_err(|e| format!("{DOC_TESTS}: {e}"))?;
        Ok((readme, guides, tickets, doc_tests))
    };
    match read_all() {
        Ok((readme, guides, tickets, doc_tests)) => {
            check(concerns, &readme, &guides, &tickets, &doc_tests, |path| {
                root.join(path).exists()
            })
        }
        Err(e) => vec![e],
    }
}

/// `text` as a TOML basic string, quotes included.
fn toml_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `text` with each Markdown link `[words](target)` made its words.
fn unlinked(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find("](").map(|i| open + i) else {
            break;
        };
        let Some(end) = rest[close..].find(')').map(|i| close + i) else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(&rest[open + 1..close]);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// A page's description: the first sentence of its first paragraph after
/// its title and its facts, its links made words.
pub fn description(text: &str) -> String {
    let paragraph: Vec<&str> = text
        .lines()
        .skip_while(|line| !line.starts_with("# "))
        .skip(1)
        .skip_while(|line| line.trim().is_empty() || line.starts_with("- **"))
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect();
    let paragraph = unlinked(&paragraph.join(" "));
    match paragraph.find(". ") {
        Some(at) => paragraph[..=at].to_owned(),
        None => paragraph,
    }
}

/// The target a link of a guide points at on the site.
pub fn site_target(target: &str) -> String {
    if outside(target) {
        return target.to_owned();
    }
    let (path, anchor) = match target.split_once('#') {
        Some((path, anchor)) => (path, format!("#{anchor}")),
        None => (target, String::new()),
    };
    if !path.contains('/') && markdown(path) {
        let page = if path == README { "_index.md" } else { path };
        return format!("@/learn/book/guides/{page}{anchor}");
    }
    match resolve(path) {
        Some(repo) => format!("{BLOB}{repo}{anchor}"),
        None => target.to_owned(),
    }
}

/// `text` with every link's target made [`site_target`]'s, outside its
/// fenced blocks and code spans, and its title line taken off.
pub fn site_body(text: &str) -> String {
    let mut out = String::new();
    let mut fenced = false;
    let mut titled = false;
    for line in text.lines() {
        if !titled && line.starts_with("# ") {
            titled = true;
            continue;
        }
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if fenced || line.trim_start().starts_with("```") {
            out.push_str(line);
        } else {
            for (i, piece) in line.split('`').enumerate() {
                if i > 0 {
                    out.push('`');
                }
                if i % 2 == 1 {
                    out.push_str(piece);
                    continue;
                }
                let mut rest = piece;
                while let Some(at) = rest.find("](") {
                    out.push_str(&rest[..at + 2]);
                    rest = &rest[at + 2..];
                    let Some(end) = rest.find(')') else { break };
                    out.push_str(&site_target(&rest[..end]));
                    rest = &rest[end..];
                }
                out.push_str(rest);
            }
        }
        out.push('\n');
    }
    out.trim_start_matches('\n').to_owned()
}

/// The pages of the book's guides: each file's name under [`SECTION`] and
/// its text. The README is `_index.md`; the guides follow in the order it
/// links them, then any it doesn't, by name.
pub fn site_pages(readme: &str, guides: &[(String, String)]) -> Vec<(String, String)> {
    let order: Vec<String> = links(readme)
        .into_iter()
        .filter(|t| !outside(t))
        .filter_map(|t| t.split('#').next().map(str::to_owned))
        .collect();
    let rank = |name: &str| order.iter().position(|n| n == name).unwrap_or(order.len());
    let mut sorted: Vec<&(String, String)> = guides.iter().collect();
    sorted.sort_by(|a, b| rank(&a.0).cmp(&rank(&b.0)).then(a.0.cmp(&b.0)));

    let title = |text: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix("# "))
            .unwrap_or("Guides")
            .trim()
            .to_owned()
    };
    let mut pages = Vec::new();
    let mut index = String::from("+++\n");
    let _ = writeln!(index, "title = {}", toml_string(&title(readme)));
    let _ = writeln!(index, "description = {}", toml_string(&description(readme)));
    index.push_str(
        "sort_by = \"weight\"\ntemplate = \"guides.html\"\npage_template = \"chapter.html\"\n\
         +++\n\n",
    );
    index.push_str(
        "<!-- Copied from docs/guides/README.md by `cargo xtask site-data`: edit that. -->\n\n",
    );
    index.push_str(&site_body(readme));
    pages.push(("_index.md".to_owned(), index));
    for (n, (name, text)) in sorted.into_iter().enumerate() {
        let mut page = String::from("+++\n");
        let _ = writeln!(page, "title = {}", toml_string(&title(text)));
        let _ = writeln!(page, "description = {}", toml_string(&description(text)));
        let _ = writeln!(page, "weight = {}", (n + 1) * 10);
        page.push_str("+++\n\n");
        let _ = writeln!(
            page,
            "<!-- Copied from docs/guides/{name} by `cargo xtask site-data`: edit that. -->\n"
        );
        page.push_str(&site_body(text));
        pages.push((name.clone(), page));
    }
    pages
}

/// Writes the guides of the repository at `root` into the book of the site
/// in `site`, its section made again from nothing.
pub fn write_site(root: &Path, site: &Path) -> Result<(), String> {
    let (readme, guides) = read(root)?;
    let dir = site.join(SECTION);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (name, text) in site_pages(&readme, &guides) {
        let path = dir.join(&name);
        fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}
