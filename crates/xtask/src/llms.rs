//! `cargo xtask llms`: the docs in the shape agents read best (ticket 2808,
//! <https://llmstxt.org/>), written into Zola's output after `zola build`:
//!
//! - a Markdown twin of every Learn page (`site/content/learn/`, drafts
//!   aside) at its URL with `index.md` on the end: its title, a `Source:`
//!   line, then its text with each `snippet` shortcode replaced by the code
//!   it shows and its links made absolute;
//! - `llms.txt`: `site/llms-header.md`, the version and the commit, then the
//!   links, a list per Learn section, the contract, the API docs, the
//!   released versions' copies and the optional reading;
//! - `llms-full.txt`: every twin in one file, under [`FULL_BUDGET_BYTES`];
//! - the contract's schemas at `/contract/schemas/v<N>/`, with an index.
//!
//! The URL rule is Zola's for names already in slug form, and only that: a
//! page that sets `slug` or `path`, or a name not in slug form, is an
//! error, and so is a shortcode this file doesn't expand. A shortcode added
//! to Learn needs its expansion here.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Command;

use pulldown_cmark::{Event, LinkType, Options as Extensions, Parser, Tag, TagEnd};

#[cfg(test)]
mod tests;

/// The most `llms-full.txt` may weigh, in bytes: about 100 000 tokens at
/// four bytes each. Raising it is a change in a PR, on purpose.
pub const FULL_BUDGET_BYTES: usize = 400_000;

/// The repository, as GitHub's addresses name it.
const REPO: &str = "Zafnok/shuyi-engine";
/// Zola's content, from the repository's root.
const CONTENT: &str = "site/content";
/// Learn's folder, from [`CONTENT`], as a section's folder is written.
const LEARN: &str = "learn/";
/// The book's code, which the `snippet` shortcode reads.
const SNIPPETS: &str = "site/snippets";
/// The start of `llms.txt`, written by hand.
const HEADER: &str = "site/llms-header.md";
/// The contract's schemas, and the list of its versions.
const SCHEMAS: &str = "crates/cli/schemas";

/// What `cargo xtask llms` was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Where it writes: Zola's output (`--public`), or `--full-only`'s
    /// `--out`.
    pub dir: PathBuf,
    /// `--full-only`: `llms-full.txt` alone, and no check that reads Zola's
    /// output.
    pub full_only: bool,
    /// The site's address, ending in `/`.
    pub base_url: String,
    /// The commit documented.
    pub commit: String,
}

impl Options {
    /// `--public <dir> --base-url <url> --commit <sha>`, or `--full-only
    /// --out <dir>` with the same two; `None` for anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        let (mut public, mut out, mut base_url, mut commit) = (None, None, None, None);
        let mut full_only = false;
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--full-only" => full_only = true,
                "--public" => public = Some(PathBuf::from(rest.next()?)),
                "--out" => out = Some(PathBuf::from(rest.next()?)),
                "--base-url" => base_url = Some(rest.next()?.clone()),
                "--commit" => commit = Some(rest.next()?.clone()),
                _ => return None,
            }
        }
        let ((false, Some(dir), None) | (true, None, Some(dir))) = (full_only, public, out) else {
            return None;
        };
        let mut base_url: String = base_url.filter(|url| !url.is_empty())?;
        if !base_url.ends_with('/') {
            base_url.push('/');
        }
        let commit = commit.filter(|commit| !commit.is_empty())?;
        Some(Self {
            dir,
            full_only,
            base_url,
            commit,
        })
    }
}

/// What [`run`] reads besides the repository.
pub struct Env<'a> {
    /// The date of a commit, `YYYY-MM-DD` ([`git_date`]).
    pub date: &'a dyn Fn(&str) -> Result<String, String>,
    /// The most `llms-full.txt` may weigh ([`FULL_BUDGET_BYTES`]).
    pub budget: usize,
}

/// What [`run`] found: what fails it, and what it prints.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Every check that failed (empty means it passed).
    pub errors: Vec<String>,
    /// What it says, on stdout: the full file's size, the links it didn't
    /// check.
    pub notes: Vec<String>,
}

/// A commit's date, from `git log` in the checkout at `root`.
pub fn git_date(root: &Path) -> impl Fn(&str) -> Result<String, String> + '_ {
    move |commit| {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["log", "-1", "--format=%cs", commit, "--"])
            .output()
            .map_err(|e| format!("git log: {e}"))?;
        let date = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if output.status.success() && !date.is_empty() {
            Ok(date)
        } else {
            Err(format!(
                "git log -1 --format=%cs {commit}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
}

/// Writes the twins, `llms.txt`, `llms-full.txt` and the schemas into
/// `options.dir` (or `llms-full.txt` alone, with `--full-only`) from the
/// checkout at `root`, and checks them.
pub fn run(root: &Path, options: &Options, env: &Env) -> Outcome {
    let mut out = Outcome::default();
    let facts = match facts(root, options, env) {
        Ok(facts) => facts,
        Err(e) => {
            out.errors.push(e);
            return out;
        }
    };
    let (pages, errors) = read_pages(root);
    out.errors.extend(errors);
    let sections = sections(&pages);
    let listed: Vec<&Page> = sections
        .iter()
        .flat_map(|(section, pages)| std::iter::once(*section).chain(pages.iter().copied()))
        .collect();
    if listed.is_empty() {
        out.errors.push(format!(
            "{CONTENT}/{LEARN}_index.md: no Learn section to list"
        ));
    }

    let twins: BTreeMap<String, String> = pages
        .iter()
        .map(|page| (page.source.clone(), page.url.clone()))
        .collect();
    let site = Site {
        base: &facts.base,
        twins: &twins,
    };
    let snippets = root.join(SNIPPETS);
    let mut texts = Vec::new();
    let before = out.errors.len();
    for page in &listed {
        let (text, errors) = twin(page, &site, &snippets);
        out.errors.extend(errors);
        out.errors
            .extend(left_shortcodes(&format!("/{}index.md", page.url), &text));
        texts.push(text);
    }

    let full = full_text(&facts, &texts);
    // A twin's error is already named at its twin; the full file has only
    // its own start to add.
    if out.errors.len() == before {
        out.errors.extend(left_shortcodes("llms-full.txt", &full));
    }
    out.notes.push(format!(
        "llms-full.txt: {} bytes, about {} tokens, {} pages (the budget is {} bytes)",
        full.len(),
        full.len() / 4,
        texts.len(),
        env.budget
    ));
    if full.len() > env.budget {
        out.errors.push(format!(
            "llms-full.txt is {} bytes, over its budget of {} (FULL_BUDGET_BYTES in crates/xtask/src/llms.rs): \
             shorten Learn, or raise the budget in a PR that says why",
            full.len(),
            env.budget
        ));
    }
    out.errors
        .extend(write(&options.dir.join("llms-full.txt"), &full));
    if !options.full_only {
        let written = Written {
            facts: &facts,
            sections: &sections,
            listed: &listed,
            texts: &texts,
            full_bytes: full.len(),
        };
        write_site(root, &options.dir, &written, &mut out);
    }
    out
}

/// What [`run`] made, for [`write_site`].
struct Written<'a> {
    facts: &'a Facts,
    sections: &'a [(&'a Page, Vec<&'a Page>)],
    /// The pages in `llms.txt`'s order, with their twins' texts.
    listed: &'a [&'a Page],
    texts: &'a [String],
    full_bytes: usize,
}

/// Writes the twins, the schemas and `llms.txt` into Zola's output,
/// `public`, and checks them against it.
fn write_site(root: &Path, public: &Path, written: &Written, out: &mut Outcome) {
    let (listed, texts, facts) = (written.listed, written.texts, written.facts);
    for (page, text) in listed.iter().zip(texts) {
        let html = public.join(&page.url).join("index.html");
        if !html.is_file() {
            out.errors.push(format!(
                "{}:1: Zola built no /{}index.html, the URL the twins' rule gives it: the rule in \
                 crates/xtask/src/llms.rs and Zola's disagree",
                page.file(),
                page.url
            ));
        }
        out.errors
            .extend(write(&public.join(&page.url).join("index.md"), text));
    }
    let schemas = match copy_schemas(root, public, facts) {
        Ok(version) => version,
        Err(errors) => {
            out.errors.extend(errors);
            0
        }
    };
    let crates = match api_crates(root) {
        Ok(crates) => crates,
        Err(errors) => {
            out.errors.extend(errors);
            Vec::new()
        }
    };
    let header = match fs::read_to_string(root.join(HEADER)) {
        Ok(text) => text.replace("\r\n", "\n"),
        Err(e) => {
            out.errors.push(format!("{HEADER}: {e}"));
            String::new()
        }
    };
    let llms = llms_txt(&Listing {
        header: &header,
        facts,
        sections: written.sections,
        full_bytes: written.full_bytes,
        schemas,
        crates: &crates,
        released: &released(public),
    });
    out.errors.extend(write(&public.join("llms.txt"), &llms));
    let api_built = public.join("api").is_dir();
    if !api_built {
        out.notes.push(format!(
            "llms.txt: the API links weren't checked, since {} has no api/ (pages.yml puts cargo doc's there)",
            public.display()
        ));
    }
    out.errors
        .extend(check_links(&llms, facts, root, public, api_built));
}

/// What `llms.txt` and `llms-full.txt` say they document.
struct Facts {
    /// `[workspace.package] version` in the root `Cargo.toml`.
    version: String,
    commit: String,
    /// The commit's date, `YYYY-MM-DD`.
    date: String,
    /// The site's address, ending in `/`.
    base: String,
}

fn facts(root: &Path, options: &Options, env: &Env) -> Result<Facts, String> {
    let manifest = read_toml(&root.join("Cargo.toml"), "Cargo.toml")?;
    let version = manifest
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or("Cargo.toml: no `version` in `[workspace.package]`")?;
    Ok(Facts {
        version: version.to_owned(),
        commit: options.commit.clone(),
        date: (env.date)(&options.commit)?,
        base: options.base_url.clone(),
    })
}

fn read_toml(path: &Path, shown: &str) -> Result<toml::Table, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{shown}: {e}"))?;
    text.parse::<toml::Table>()
        .map_err(|e| format!("{shown}: {e}"))
}

/// Writes `text` to `path`, making its folders; the error, if any.
fn write(path: &Path, text: &str) -> Option<String> {
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(path, text));
    written.err().map(|e| format!("{}: {e}", path.display()))
}

// -- the pages --

/// A page of Learn that isn't a draft.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Page {
    /// From `site/content/`, `/`-separated (`learn/book/quick-start.md`).
    source: String,
    /// Its URL from the site's base (`learn/book/quick-start/`).
    url: String,
    /// Whether it is a section's `_index.md`.
    is_section: bool,
    /// A section's own folder, or the folder of the section a page is
    /// listed in (`learn/book/`).
    folder: String,
    /// A section's: the folder of the section above it; none for Learn.
    parent: Option<String>,
    title: String,
    description: Option<String>,
    weight: i64,
    /// The text after the front matter.
    body: String,
    /// The number of the body's first line in the file.
    body_line: usize,
}

impl Page {
    /// Its file, from the repository's root.
    fn file(&self) -> String {
        format!("{CONTENT}/{}", self.source)
    }
}

/// Every page of Learn but the drafts, and every error reading them.
fn read_pages(root: &Path) -> (Vec<Page>, Vec<String>) {
    let content = root.join(CONTENT);
    let mut files = Vec::new();
    let mut errors = Vec::new();
    if let Err(e) = markdown_under(&content.join(LEARN), &mut files) {
        errors.push(format!("{CONTENT}/{LEARN}: {e}"));
    }
    files.sort();
    let mut pages = Vec::new();
    for file in files {
        let source = file
            .strip_prefix(&content)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let shown = format!("{CONTENT}/{source}");
        let text = match fs::read_to_string(&file) {
            Ok(text) => text.replace("\r\n", "\n"),
            Err(e) => {
                errors.push(format!("{shown}: {e}"));
                continue;
            }
        };
        match page(&source, &shown, &text) {
            Ok(Some(page)) => pages.push(page),
            Ok(None) => {}
            Err(e) => errors.push(e),
        }
    }
    let sections: BTreeSet<String> = pages
        .iter()
        .filter(|page| page.is_section)
        .map(|page| page.folder.clone())
        .collect();
    for page in &mut pages {
        if page.is_section {
            page.parent = parent_folder(&page.folder).and_then(|up| nearest(&sections, up));
        } else if let Some(folder) = nearest(&sections, &page.folder) {
            page.folder = folder;
        } else {
            errors.push(format!(
                "{}: no section holds it: no folder above it has an `_index.md`",
                page.file()
            ));
        }
    }
    (pages, errors)
}

fn markdown_under(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            markdown_under(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// The page in the file `source` (shown as `shown`) holding `text`; none
/// for a draft. Its `folder` is the folder it is in, until [`read_pages`]
/// finds its section.
fn page(source: &str, shown: &str, text: &str) -> Result<Option<Page>, String> {
    let front = front_matter(shown, text)?;
    if front.draft {
        return Ok(None);
    }
    let url = url_of(source).map_err(|e| format!("{shown}: {e}"))?;
    let name = source.rsplit('/').next().unwrap_or(source);
    let is_section = name == "_index.md";
    let folder = if is_section {
        url.clone()
    } else if name == "index.md" {
        parent_folder(&url).unwrap_or_default().to_owned()
    } else {
        source[..source.len() - name.len()].to_owned()
    };
    Ok(Some(Page {
        source: source.to_owned(),
        url,
        is_section,
        folder,
        parent: None,
        title: front.title,
        description: front.description,
        weight: front.weight,
        body: text[front.body..].to_owned(),
        body_line: front.body_line,
    }))
}

/// The folder above `folder` (`learn/` above `learn/book/`); none above
/// the top.
fn parent_folder(folder: &str) -> Option<&str> {
    let trimmed = folder.strip_suffix('/')?;
    trimmed.rfind('/').map(|at| &trimmed[..=at])
}

/// The nearest of `sections` that is `folder` or above it.
fn nearest(sections: &BTreeSet<String>, mut folder: &str) -> Option<String> {
    loop {
        if sections.contains(folder) {
            return Some(folder.to_owned());
        }
        folder = parent_folder(folder)?;
    }
}

/// What a page's front matter gives.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Front {
    title: String,
    description: Option<String>,
    weight: i64,
    draft: bool,
    /// Where its body starts, in bytes.
    body: usize,
    /// Its body's first line's number.
    body_line: usize,
}

/// The TOML between the `+++` lines at the start of `text`, the file
/// `shown`.
fn front_matter(shown: &str, text: &str) -> Result<Front, String> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().unwrap_or_default();
    if first.trim_end() != "+++" {
        return Err(format!(
            "{shown}:1: no front matter: a page of Learn starts with a `+++` line"
        ));
    }
    let mut at = first.len();
    let mut end = None;
    for (index, line) in lines.enumerate() {
        if line.trim_end() == "+++" {
            end = Some((at, at + line.len(), index + 3));
            break;
        }
        at += line.len();
    }
    let Some((toml_end, body, body_line)) = end else {
        return Err(format!(
            "{shown}:1: its front matter has no closing `+++` line"
        ));
    };
    let toml_text = &text[first.len()..toml_end];
    let table: toml::Table = toml_text
        .parse()
        .map_err(|e| format!("{shown}: its front matter isn't TOML: {e}"))?;
    for key in ["slug", "path"] {
        if table.contains_key(key) {
            let line = toml_text
                .lines()
                .position(|line| {
                    line.trim_start()
                        .strip_prefix(key)
                        .is_some_and(|rest| rest.trim_start().starts_with('='))
                })
                .map_or(1, |index| index + 2);
            return Err(format!(
                "{shown}:{line}: sets `{key}`, and the twins' URL rule follows only the file's name: \
                 drop it, or add its rule to crates/xtask/src/llms.rs"
            ));
        }
    }
    let title = table
        .get("title")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("{shown}:1: no `title` in its front matter"))?;
    Ok(Front {
        title: title.to_owned(),
        description: table
            .get("description")
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        weight: table
            .get("weight")
            .and_then(toml::Value::as_integer)
            .unwrap_or(0),
        draft: table
            .get("draft")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false),
        body,
        body_line,
    })
}

/// The URL of the page in `source` (from `site/content/`), Zola's rule for
/// names in slug form: `learn/book/x.md` is `learn/book/x/`, and an
/// `_index.md` or an `index.md` is its folder's.
fn url_of(source: &str) -> Result<String, String> {
    let stem = source
        .strip_suffix(".md")
        .ok_or_else(|| format!("`{source}` isn't a Markdown file"))?;
    let mut parts: Vec<&str> = stem.split('/').collect();
    if matches!(parts.last(), Some(&("_index" | "index"))) {
        parts.pop();
    }
    for part in &parts {
        let dated = part.len() > 10
            && part[..10].bytes().enumerate().all(|(i, b)| {
                if i == 4 || i == 7 {
                    b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            });
        if part.is_empty()
            || dated
            || !part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(format!(
                "`{part}` isn't a name the twins' URL rule knows (lowercase ASCII letters, digits \
                 and `-`, and no date first): rename it, or add its rule to crates/xtask/src/llms.rs"
            ));
        }
    }
    let mut url = String::new();
    for part in parts {
        url.push_str(part);
        url.push('/');
    }
    Ok(url)
}

/// The sections in `llms.txt`'s order, each with its pages: Learn first,
/// then each section's sections after it, in `weight` order and then by
/// file name, as are the pages.
fn sections(pages: &[Page]) -> Vec<(&Page, Vec<&Page>)> {
    fn visit<'a>(section: &'a Page, pages: &'a [Page], out: &mut Vec<(&'a Page, Vec<&'a Page>)>) {
        let key = |page: &&Page| (page.weight, page.source.clone());
        let mut listed: Vec<&Page> = pages
            .iter()
            .filter(|page| !page.is_section && page.folder == section.folder)
            .collect();
        listed.sort_by_key(key);
        out.push((section, listed));
        let mut below: Vec<&Page> = pages
            .iter()
            .filter(|page| page.is_section && page.parent.as_deref() == Some(&section.folder))
            .collect();
        below.sort_by_key(key);
        for section in below {
            visit(section, pages, out);
        }
    }
    let mut out = Vec::new();
    if let Some(top) = pages
        .iter()
        .find(|page| page.is_section && page.folder == LEARN)
    {
        visit(top, pages, &mut out);
    }
    out
}

// -- the twins --

/// Where the twins are: the site's base, and each twin's URL by its page's
/// file (`learn/book/x.md`).
struct Site<'a> {
    base: &'a str,
    twins: &'a BTreeMap<String, String>,
}

/// The twin of `page`, and every error making it.
fn twin(page: &Page, site: &Site, snippets: &Path) -> (String, Vec<String>) {
    let shown = page.file();
    let (expanded, mut errors) = expand(&shown, &page.body, page.body_line, snippets);
    let (linked, link_errors) = rewrite_links(&shown, &expanded, &page.url, site);
    errors.extend(link_errors);
    let text = format!(
        "# {}\n\nSource: {}{}\n\n{}\n",
        page.title,
        site.base,
        page.url,
        linked.trim()
    );
    (text, errors)
}

/// `body`, the text of `shown` from its line `first_line`, with Zola's
/// shortcodes expanded, as Zola reads them: before the Markdown, so in code
/// too. `snippet` becomes its code; the escaped form, `{{/*` … `*/}}`,
/// becomes plain `{{` … `}}`; any other is an error.
fn expand(shown: &str, body: &str, first_line: usize, snippets: &Path) -> (String, Vec<String>) {
    let mut out = String::with_capacity(body.len());
    let mut errors = Vec::new();
    let mut at = 0;
    while let Some(found) = body.as_bytes()[at..]
        .windows(2)
        .position(|pair| pair[0] == b'{' && matches!(pair[1], b'{' | b'%'))
    {
        let open = at + found;
        let opener = &body[open..open + 2];
        let closer = if opener == "{{" { "}}" } else { "%}" };
        out.push_str(&body[at..open]);
        let line = first_line + body[..open].matches('\n').count();
        let after = &body[open + 2..];
        if let Some(inner) = after.strip_prefix("/*")
            && let Some(end) = inner.find(&format!("*/{closer}"))
        {
            out.push_str(opener);
            out.push_str(&inner[..end]);
            out.push_str(closer);
            at = open + 2 + 2 + end + 2 + closer.len();
            continue;
        }
        match call(after, closer) {
            None => {
                out.push_str(opener);
                at = open + 2;
            }
            Some(Err(e)) => {
                errors.push(format!("{shown}:{line}: {e}"));
                out.push_str(opener);
                at = open + 2;
            }
            Some(Ok(call)) => {
                at = open + 2 + call.len;
                if opener == "{{" && call.name == "snippet" {
                    match snippet(call.args, snippets) {
                        Ok(code) => out.push_str(&code),
                        Err(e) => errors.push(format!("{shown}:{line}: {e}")),
                    }
                } else {
                    errors.push(format!(
                        "{shown}:{line}: the shortcode `{}` has no expansion in the twins: add one to \
                         crates/xtask/src/llms.rs",
                        call.name
                    ));
                }
            }
        }
    }
    out.push_str(&body[at..]);
    (out, errors)
}

/// A shortcode's call: `name(args)` and its closer.
struct Call<'a> {
    name: &'a str,
    args: &'a str,
    /// Its length in bytes after the opener, the closer included.
    len: usize,
}

/// The call after a `{{` or `{%`: none when it isn't one (Zola leaves that
/// text as it is), an error when it starts as one and doesn't end.
fn call<'a>(after: &'a str, closer: &str) -> Option<Result<Call<'a>, String>> {
    let trimmed = after.trim_start();
    let name_len = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
    let name = &trimmed[..name_len];
    let rest = trimmed[name_len..].trim_start();
    if name.is_empty() || !rest.starts_with('(') {
        return None;
    }
    let mut quote = None;
    let mut close = None;
    for (at, c) in rest.char_indices().skip(1) {
        match quote {
            Some(q) if c == q => quote = None,
            None if matches!(c, '"' | '\'' | '`') => quote = Some(c),
            None if c == ')' => {
                close = Some(at);
                break;
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return Some(Err(format!("the shortcode `{name}` has no closing `)`")));
    };
    let tail = rest[close + 1..].trim_start();
    if !tail.starts_with(closer) {
        return Some(Err(format!(
            "the shortcode `{name}` doesn't end with `{closer}`"
        )));
    }
    Some(Ok(Call {
        name,
        args: &rest[1..close],
        len: after.len() - tail.len() + closer.len(),
    }))
}

/// A shortcode's arguments, `name=value` split by commas, a value quoted
/// with `"`, `'` or `` ` `` or bare.
fn arguments(args: &str) -> Result<Vec<(&str, &str)>, String> {
    let mut out = Vec::new();
    let mut rest = args.trim();
    while !rest.is_empty() {
        let eq = rest
            .find('=')
            .ok_or_else(|| format!("`{rest}` isn't `name=value`"))?;
        let key = rest[..eq].trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("`{key}` isn't an argument's name"));
        }
        let value = rest[eq + 1..].trim_start();
        let (value, after) = if let Some(q @ ('"' | '\'' | '`')) = value.chars().next() {
            let end = value[1..]
                .find(q)
                .ok_or_else(|| format!("the value of `{key}` has no closing {q}"))?;
            (&value[1..=end], &value[end + 2..])
        } else {
            let end = value.find(',').unwrap_or(value.len());
            (value[..end].trim(), &value[end..])
        };
        out.push((key, value));
        let after = after.trim_start();
        rest = match after.strip_prefix(',') {
            Some(next) => next.trim_start(),
            None if after.is_empty() => after,
            None => return Err(format!("no `,` after the value of `{key}`")),
        };
    }
    Ok(out)
}

/// The `snippet` shortcode (`site/templates/shortcodes/snippet.md`): the
/// lines of `site/snippets/<file>`, or with `step` of
/// `site/tutorial/<step>/<file>`, between `// ANCHOR: <anchor>` and
/// `// ANCHOR_END: <anchor>` (with no `anchor`, of the whole file), other
/// anchors' lines left out, as a fenced block in `language` (`rust` unless
/// given).
fn snippet(args: &str, snippets: &Path) -> Result<String, String> {
    let (mut file, mut anchor, mut language, mut step) = (None, None, "rust", None);
    for (key, value) in arguments(args).map_err(|e| format!("snippet: {e}"))? {
        match key {
            "file" => file = Some(value),
            "anchor" => anchor = Some(value),
            "language" => language = value,
            "step" => step = Some(value),
            _ => {
                return Err(format!(
                    "snippet: no argument `{key}` (it takes `file`, `anchor`, `language` and `step`)"
                ));
            }
        }
    }
    let file = file.ok_or("snippet: no `file`")?;
    let (path, shown) = match step {
        Some(step) => (
            snippets.with_file_name("tutorial").join(step).join(file),
            format!("site/tutorial/{step}/{file}"),
        ),
        None => (snippets.join(file), format!("{SNIPPETS}/{file}")),
    };
    let source = fs::read_to_string(path).map_err(|e| format!("snippet: {shown}: {e}"))?;
    let lines = match anchor {
        Some(anchor) => anchored(&source, anchor).ok_or_else(|| {
            format!("snippet: no `// ANCHOR: {anchor}` … `// ANCHOR_END: {anchor}` in {shown}")
        })?,
        None => whole(&source),
    };
    Ok(format!("```{language}\n{}\n```", lines.join("\n")))
}

/// Every line of `source`, without the anchors' lines, as the shortcode
/// shows a file named with no anchor. The empty line after its last
/// newline stays, as the shortcode's split keeps it.
fn whole(source: &str) -> Vec<&str> {
    source
        .split('\n')
        .map(|raw| raw.trim_end_matches('\r'))
        .filter(|line| {
            let bare = line.trim();
            !bare.starts_with("// ANCHOR: ") && !bare.starts_with("// ANCHOR_END: ")
        })
        .collect()
}

/// The lines of `source` inside its first `anchor`, without the lines of
/// other anchors; none when it has no such anchor, or it doesn't end.
fn anchored<'a>(source: &'a str, anchor: &str) -> Option<Vec<&'a str>> {
    let begin = format!("// ANCHOR: {anchor}");
    let end = format!("// ANCHOR_END: {anchor}");
    let mut lines = Vec::new();
    let mut inside = false;
    for raw in source.split('\n') {
        let line = raw.trim_end_matches('\r');
        let bare = line.trim();
        if inside {
            if bare == end {
                return Some(lines);
            }
            if !bare.starts_with("// ANCHOR: ") && !bare.starts_with("// ANCHOR_END: ") {
                lines.push(line);
            }
        } else if bare == begin {
            inside = true;
        }
    }
    None
}

/// The Markdown extensions Zola turns on that change where links are.
fn extensions() -> Extensions {
    Extensions::ENABLE_TABLES
        | Extensions::ENABLE_FOOTNOTES
        | Extensions::ENABLE_STRIKETHROUGH
        | Extensions::ENABLE_TASKLISTS
}

/// `text`, of the page at `page_url`, with each link's and image's target
/// made absolute: a Zola link (`@/…`) to a page with a twin goes to the
/// twin, any other to its HTML URL, and a relative one is read against the
/// page's URL. Only the targets' bytes change.
fn rewrite_links(shown: &str, text: &str, page_url: &str, site: &Site) -> (String, Vec<String>) {
    let parser = Parser::new_ext(text, extensions());
    let mut found: Vec<(Range<usize>, String, &str)> = parser
        .reference_definitions()
        .iter()
        .map(|(_, def)| (def.span.clone(), def.dest.to_string(), "]:"))
        .collect();
    for (event, range) in parser.into_offset_iter() {
        if let Event::Start(
            Tag::Link {
                link_type: LinkType::Inline,
                dest_url,
                ..
            }
            | Tag::Image {
                link_type: LinkType::Inline,
                dest_url,
                ..
            },
        ) = event
        {
            found.push((range, dest_url.to_string(), "]("));
        }
    }
    let mut changes = Vec::new();
    let mut errors = Vec::new();
    for (span, dest, marker) in found {
        match absolute(&dest, page_url, site) {
            Ok(None) => {}
            Ok(Some(url)) => match target_bytes(text, span, &dest, marker) {
                Some(bytes) => changes.push((bytes, url)),
                None => errors.push(format!(
                    "{shown}: the link to `{dest}` isn't written as its target: write it plainly"
                )),
            },
            Err(e) => errors.push(format!("{shown}: the link to `{dest}`: {e}")),
        }
    }
    changes.sort_by_key(|(bytes, _)| bytes.start);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (bytes, url) in changes {
        if bytes.start < at {
            continue;
        }
        out.push_str(&text[at..bytes.start]);
        out.push_str(&url);
        at = bytes.end;
    }
    out.push_str(&text[at..]);
    (out, errors)
}

/// Where in `text[span]` the target `dest` is written: after the last
/// `marker` (`](` or `]:`) it follows, spaces and a `<` aside.
fn target_bytes(text: &str, span: Range<usize>, dest: &str, marker: &str) -> Option<Range<usize>> {
    let slice = &text[span.clone()];
    slice.rmatch_indices(marker).find_map(|(at, _)| {
        let after = &slice[at + marker.len()..];
        let mut start = at + marker.len() + (after.len() - after.trim_start().len());
        if slice[start..].starts_with('<') {
            start += 1;
        }
        slice[start..]
            .starts_with(dest)
            .then(|| span.start + start..span.start + start + dest.len())
    })
}

/// `dest`, a link of the page at `page_url`, as an absolute URL; none when
/// it stays as it is (a web link, an anchor).
fn absolute(dest: &str, page_url: &str, site: &Site) -> Result<Option<String>, String> {
    if dest.is_empty()
        || dest.starts_with('#')
        || dest.contains("://")
        || dest.starts_with("mailto:")
    {
        return Ok(None);
    }
    if let Some(internal) = dest.strip_prefix("@/") {
        let (path, anchor) = internal.split_at(internal.find('#').unwrap_or(internal.len()));
        let url = match site.twins.get(path) {
            Some(url) => format!("{url}index.md"),
            None => url_of(path)?,
        };
        return Ok(Some(format!("{}{url}{anchor}", site.base)));
    }
    if let Some(rooted) = dest.strip_prefix('/') {
        return Ok(Some(format!("{}{rooted}", site.base)));
    }
    Ok(Some(format!("{}{}", site.base, resolve(page_url, dest))))
}

/// The relative `dest` read against the URL `page_url` (both from the
/// site's base).
fn resolve(page_url: &str, dest: &str) -> String {
    let (path, suffix) = dest.split_at(dest.find(['#', '?']).unwrap_or(dest.len()));
    let mut parts: Vec<&str> = page_url
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    let mut out = parts.join("/");
    if path.ends_with('/') && !out.is_empty() {
        out.push('/');
    }
    out + suffix
}

/// A `{{` or `{%` left in `text`, the file `shown`, outside code: a
/// shortcode nothing expanded.
fn left_shortcodes(shown: &str, text: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut code = 0_usize;
    for (event, range) in Parser::new_ext(text, extensions()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code += 1,
            Event::End(TagEnd::CodeBlock) => code = code.saturating_sub(1),
            Event::Text(t) | Event::Html(t) | Event::InlineHtml(t)
                if code == 0 && (t.contains("{{") || t.contains("{%")) =>
            {
                let line = text[..range.start].matches('\n').count() + 1;
                errors.push(format!(
                    "{shown}:{line}: a `{{{{` or `{{%` outside code: a shortcode left unexpanded \
                     (crates/xtask/src/llms.rs), or text to put in code"
                ));
            }
            _ => {}
        }
    }
    errors
}

// -- the files --

/// `llms-full.txt`: a line on what it is, then each twin, split by `---`.
fn full_text(facts: &Facts, twins: &[String]) -> String {
    let mut out = format!(
        "# Shuyi Engine {}: every page of Learn\n\nFrom commit {} of {}. The list of them, and the \
         rest of the docs, are in {}llms.txt.\n",
        facts.version, facts.commit, facts.date, facts.base
    );
    for twin in twins {
        out.push_str("\n---\n\n");
        out.push_str(twin);
    }
    out
}

/// An engine crate whose API docs are at `/api/<lib>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Crate {
    /// Rustdoc's folder name (`shuyi_core`).
    lib: String,
    description: String,
}

/// Every `shuyi` and `shuyi-*` package in `crates/*/Cargo.toml`, by
/// rustdoc's folder name.
fn api_crates(root: &Path) -> Result<Vec<Crate>, Vec<String>> {
    let entries = fs::read_dir(root.join("crates")).map_err(|e| vec![format!("crates/: {e}")])?;
    let mut crates = Vec::new();
    let mut errors = Vec::new();
    for entry in entries.flatten() {
        let manifest = entry.path().join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let shown = format!("crates/{}/Cargo.toml", entry.file_name().to_string_lossy());
        let table = match read_toml(&manifest, &shown) {
            Ok(table) => table,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        let package = table.get("package");
        let Some(name) = package
            .and_then(|p| p.get("name"))
            .and_then(toml::Value::as_str)
        else {
            continue;
        };
        if name != "shuyi" && !name.starts_with("shuyi-") {
            continue;
        }
        let lib = table
            .get("lib")
            .and_then(|lib| lib.get("name"))
            .and_then(toml::Value::as_str)
            .map_or_else(|| name.replace('-', "_"), str::to_owned);
        let description = package
            .and_then(|p| p.get("description"))
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        crates.push(Crate { lib, description });
    }
    crates.sort_by(|a, b| a.lib.cmp(&b.lib));
    if errors.is_empty() {
        Ok(crates)
    } else {
        Err(errors)
    }
}

/// The released versions whose frozen copy is in `public`
/// (`v<x.y>/llms-full.txt`), newest first, as `x.y`.
fn released(public: &Path) -> Vec<String> {
    let mut found: Vec<(u64, u64)> = fs::read_dir(public)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().join("llms-full.txt").is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let (major, minor) = name.strip_prefix('v')?.split_once('.')?;
            Some((major.parse().ok()?, minor.parse().ok()?))
        })
        .collect();
    found.sort_unstable_by(|a, b| b.cmp(a));
    found
        .into_iter()
        .map(|(major, minor)| format!("{major}.{minor}"))
        .collect()
}

/// Copies the schemas to `public/contract/schemas/v<N>/` with an index;
/// N, the contract's last version.
fn copy_schemas(root: &Path, public: &Path, facts: &Facts) -> Result<u64, Vec<String>> {
    let schemas = root.join(SCHEMAS);
    let versions: serde_json::Value = fs::read_to_string(schemas.join("versions.json"))
        .map_err(|e| e.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
        .map_err(|e| vec![format!("{SCHEMAS}/versions.json: {e}")])?;
    let version = versions
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("version").and_then(serde_json::Value::as_u64))
        .max()
        .ok_or_else(|| vec![format!("{SCHEMAS}/versions.json: no version")])?;
    let mut names: Vec<String> = fs::read_dir(&schemas)
        .map_err(|e| vec![format!("{SCHEMAS}: {e}")])?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| Path::new(name).extension().is_some_and(|ext| ext == "json"))
        .collect();
    names.sort();
    let dest = public.join(format!("contract/schemas/v{version}"));
    let mut errors = Vec::new();
    if let Err(e) = fs::create_dir_all(&dest) {
        return Err(vec![format!("{}: {e}", dest.display())]);
    }
    for name in &names {
        if let Err(e) = fs::copy(schemas.join(name), dest.join(name)) {
            errors.push(format!("{SCHEMAS}/{name}: {e}"));
        }
    }
    errors.extend(write(
        &dest.join("index.md"),
        &schemas_index(facts, version, &names),
    ));
    if errors.is_empty() {
        Ok(version)
    } else {
        Err(errors)
    }
}

/// The schemas' `index.md`: each operation's input and output files.
fn schemas_index(facts: &Facts, version: u64, names: &[String]) -> String {
    let here = format!("{}contract/schemas/v{version}/", facts.base);
    let raw = raw_url(facts);
    let mut out = format!(
        "# The tool contract's schemas, version {version}\n\n\
         Source: https://github.com/{REPO}/tree/{}/{SCHEMAS}\n\n\
         The JSON Schema of each operation's input and of its answer, for version {version} of the \
         tool contract, as `shuyi schema` prints them. What each operation does is in [the tool \
         contract]({raw}docs/contract.md), and [versions.json]({here}versions.json) lists every \
         version with its fingerprint.\n\n\
         | Operation | Input | Output |\n| --------- | ----- | ------ |\n",
        facts.commit
    );
    let file = |name: String| {
        if names.contains(&name) {
            format!("[{name}]({here}{name})")
        } else {
            "none".to_owned()
        }
    };
    let ops: BTreeSet<&str> = names
        .iter()
        .filter_map(|name| {
            name.strip_suffix(".input.json")
                .or_else(|| name.strip_suffix(".output.json"))
        })
        .collect();
    for op in ops {
        let _ = writeln!(
            out,
            "| `{op}` | {} | {} |",
            file(format!("{op}.input.json")),
            file(format!("{op}.output.json"))
        );
    }
    out
}

/// The repository's files at the commit, as raw text.
fn raw_url(facts: &Facts) -> String {
    format!("https://raw.githubusercontent.com/{REPO}/{}/", facts.commit)
}

/// What `llms.txt` lists.
struct Listing<'a> {
    header: &'a str,
    facts: &'a Facts,
    sections: &'a [(&'a Page, Vec<&'a Page>)],
    full_bytes: usize,
    /// The contract's version whose schemas are on the site.
    schemas: u64,
    crates: &'a [Crate],
    /// The released versions' copies, newest first (`0.2`).
    released: &'a [String],
}

/// `llms.txt`, in llmstxt.org's shape.
fn llms_txt(listing: &Listing) -> String {
    let facts = listing.facts;
    let base = &facts.base;
    let raw = raw_url(facts);
    let short = facts.commit.get(..7).unwrap_or(&facts.commit);
    let mut out = format!("{}\n\n", listing.header.trim_end());
    let _ = writeln!(
        out,
        "This copy documents Shuyi Engine {} at commit [`{short}`](https://github.com/{REPO}/commit/{}), \
         of {}. It follows the `main` branch and changes with each merge, so a game on a released \
         version reads its version's copy, under *Released versions*. Each page listed under \
         Learn's sections is also in one file, [llms-full.txt]({base}llms-full.txt): {} bytes, \
         about {} tokens.",
        facts.version,
        facts.commit,
        facts.date,
        listing.full_bytes,
        listing.full_bytes / 4
    );
    for (section, pages) in listing.sections {
        let _ = write!(out, "\n## {}\n\n", section.title);
        for page in std::iter::once(section).chain(pages) {
            let _ = write!(out, "- [{}]({base}{}index.md)", page.title, page.url);
            match &page.description {
                Some(description) => {
                    let _ = writeln!(out, ": {description}");
                }
                None => out.push('\n'),
            }
        }
    }
    let version = listing.schemas;
    let _ = write!(
        out,
        "\n## Contract\n\n\
         - [The tool contract]({raw}docs/contract.md): the operations an agent, CI or a person runs \
         on a game, each with a typed input and a typed answer, on the command line or over MCP\n\
         - [Its schemas, version {version}]({base}contract/schemas/v{version}/index.md): the JSON \
         Schema of each operation's input and answer\n\
         \n## API\n\n"
    );
    for krate in listing.crates {
        let _ = writeln!(
            out,
            "- [`{0}`]({base}api/{0}/index.html): rustdoc's HTML. {1}",
            krate.lib, krate.description
        );
    }
    out.push_str("\n## Released versions\n\n");
    if listing.released.is_empty() {
        out.push_str("None yet.\n");
    }
    for version in listing.released {
        let _ = writeln!(
            out,
            "- [Shuyi Engine {version}]({base}v{version}/llms-full.txt): every page of Learn, as released"
        );
    }
    let _ = write!(
        out,
        "\n## Optional\n\n\
         - [The ADRs]({raw}docs/adr/README.md): why each technical choice was made, a record each\n\
         - [The roadmap]({raw}docs/ROADMAP.md): the order the work is done in, and what is parked\n"
    );
    out
}

/// Every link of `llms` that names no file: one to the site, in `public`
/// (one to the API docs only when `api_built`), and one to the repository
/// at the commit, in the checkout at `root`.
fn check_links(
    llms: &str,
    facts: &Facts,
    root: &Path,
    public: &Path,
    api_built: bool,
) -> Vec<String> {
    let at_commit = [
        raw_url(facts),
        format!("https://github.com/{REPO}/blob/{}/", facts.commit),
        format!("https://github.com/{REPO}/tree/{}/", facts.commit),
    ];
    let mut errors = Vec::new();
    for (event, range) in Parser::new(llms).into_offset_iter() {
        let Event::Start(Tag::Link { dest_url, .. }) = event else {
            continue;
        };
        let line = llms[..range.start].matches('\n').count() + 1;
        let cut = |path: &str| path.split(['#', '?']).next().unwrap_or_default().to_owned();
        if let Some(rest) = dest_url.strip_prefix(facts.base.as_str()) {
            let mut path = cut(rest);
            if path.is_empty() || path.ends_with('/') {
                path.push_str("index.html");
            }
            if (api_built || !path.starts_with("api/")) && !public.join(&path).is_file() {
                errors.push(format!(
                    "llms.txt:{line}: {dest_url} names no file in {}",
                    public.display()
                ));
            }
        } else if let Some(rest) = at_commit
            .iter()
            .find_map(|prefix| dest_url.strip_prefix(prefix.as_str()))
        {
            let path = cut(rest);
            if !root.join(&path).exists() {
                errors.push(format!(
                    "llms.txt:{line}: {dest_url} names {path}, which isn't in the checkout"
                ));
            }
        }
    }
    errors
}
