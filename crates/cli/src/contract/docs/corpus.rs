//! The corpus `docs` searches (ADR-0153), cut into chunks: the Learn
//! pages and the skills at their `##` and `###` headings, each example's
//! `.rs` or `.md` file whole, and each problem code's page. Every chunk is
//! text this `shuyi` embeds (`build.rs`), at the engine's revision it was
//! built from.
//!
//! A Learn page is read as its twin is (`cargo xtask llms`, ticket 2808):
//! its front matter dropped, a `snippet` shortcode replaced by the code it
//! shows, the bodies of `newcomer`, `check` and `exercise` kept without
//! their markers, and `try_it` dropped. [`learn_page`] is that text, which
//! the MCP server also serves as a resource.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::HitKind;
use crate::contract::codes;
use crate::skills::Pack;

/// The Learn pages, from `site/content/learn`.
static LEARN: &[(&str, &[u8])] = shuyi_embed::files!("learn");
/// The book's snippets, from `site/snippets`.
static SNIPPETS: &[(&str, &[u8])] = shuyi_embed::files!("snippets");
/// The tutorial's steps' files a snippet shows, from `site/tutorial`.
static TUTORIAL: &[(&str, &[u8])] = shuyi_embed::files!("tutorial");
/// The examples' `.rs` and `.md` files, from `examples`.
static EXAMPLES: &[(&str, &[u8])] = shuyi_embed::files!("examples");

/// Where the Learn pages are in the engine's repository.
pub const LEARN_ROOT: &str = "site/content/learn";
/// Where the skills are.
pub const SKILLS_ROOT: &str = "skills";
/// Where the examples are.
pub const EXAMPLES_ROOT: &str = "examples";
/// Where the problem codes' pages are.
pub const CODES_ROOT: &str = "crates/cli/errors";

/// The bytes of the embedded tables the corpus reads, together: the
/// Learn pages, the snippets, the tutorial's files and the examples'.
pub fn embedded_bytes() -> [(&'static str, usize); 4] {
    let size = |table: &[(&str, &[u8])]| table.iter().map(|(_, bytes)| bytes.len()).sum();
    [
        ("learn", size(LEARN)),
        ("snippets", size(SNIPPETS)),
        ("tutorial", size(TUTORIAL)),
        ("examples", size(EXAMPLES)),
    ]
}

/// One chunk of the corpus: what a hit names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// What it is from.
    pub kind: HitKind,
    /// Its title: the page's or skill's, and its heading's.
    pub title: String,
    /// Its source in the engine's repository (`site/content/learn/book/actions.md`).
    pub path: String,
    /// Its heading's anchor, Zola's slug of it; none for a page's start.
    pub anchor: Option<String>,
    /// The URL of its Learn page's Markdown twin.
    pub url: Option<String>,
    /// Its place in its file, from 0.
    pub position: u32,
    /// Its text, without its heading.
    pub text: String,
}

/// A Learn page, cleaned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Its path under `site/content/learn` (`book/actions.md`).
    pub path: String,
    /// Its title, from its front matter.
    pub title: String,
    /// Its body as its twin reads (the module's docs).
    pub text: String,
    /// The shortcodes cleaning didn't know, or couldn't expand, each with
    /// why: none in a page the corpus reads right.
    pub unknown: Vec<String>,
}

/// A part of a page cut at its headings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// Its heading's text, none for the text before the first heading.
    pub heading: Option<String>,
    /// Its anchor: Zola's slug of its heading, made unique in its page.
    pub anchor: Option<String>,
    /// Its text after its heading.
    pub text: String,
}

/// Whether `path`'s extension is `extension`, in any case.
fn has_extension(path: &str, extension: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(extension))
}

/// A file of `table` at `path`, as text with LF line ends.
fn text_of(table: &[(&str, &[u8])], path: &str) -> Option<String> {
    table
        .binary_search_by(|(p, _)| (*p).cmp(path))
        .ok()
        .map(|at| String::from_utf8_lossy(table[at].1).replace("\r\n", "\n"))
}

/// The front matter's fields the corpus reads.
#[derive(Debug, Default, Deserialize)]
struct Front {
    title: Option<String>,
}

/// `source` without its TOML front matter (`+++` … `+++`), and its title.
fn split_front(source: &str) -> (Option<String>, &str) {
    let Some(rest) = source.strip_prefix("+++\n") else {
        return (None, source);
    };
    let Some(end) = rest.find("\n+++") else {
        return (None, source);
    };
    let front: Front = toml_edit::de::from_str(&rest[..end]).unwrap_or_default();
    let body = rest[end + 4..].trim_start_matches('\n');
    (front.title, body)
}

/// The Learn page at `path` under `site/content/learn`, cleaned.
pub fn learn_page(path: &str) -> Option<Page> {
    if !has_extension(path, "md") {
        return None;
    }
    let source = text_of(LEARN, path)?;
    let (title, body) = split_front(&source);
    let (text, unknown) = clean(body, &embedded_file);
    Some(Page {
        path: path.to_owned(),
        title: title.unwrap_or_else(|| path.to_owned()),
        text,
        unknown,
    })
}

/// Every Learn page, in path order.
pub fn learn_pages() -> Vec<Page> {
    LEARN
        .iter()
        .filter_map(|(path, _)| learn_page(path))
        .collect()
}

/// A snippet's file: of `site/tutorial/<step>/` with `step`, else of
/// `site/snippets/`.
fn embedded_file(step: Option<&str>, file: &str) -> Option<String> {
    match step {
        Some(step) => text_of(TUTORIAL, &format!("{step}/{file}")),
        None => text_of(SNIPPETS, file),
    }
}

/// The blocks whose body is kept, and their markers dropped.
const BLOCKS: [&str; 3] = ["newcomer", "check", "exercise"];

/// `body`, a Learn page's, with its shortcodes as its twin reads them
/// (the module's docs), `file` giving a snippet's file; and each shortcode
/// it couldn't read, with why.
pub fn clean(
    body: &str,
    file: &dyn Fn(Option<&str>, &str) -> Option<String>,
) -> (String, Vec<String>) {
    let mut out = String::with_capacity(body.len());
    let mut unknown = Vec::new();
    let mut at = 0;
    while let Some(found) = body.as_bytes()[at..]
        .windows(2)
        .position(|pair| pair[0] == b'{' && matches!(pair[1], b'{' | b'%'))
    {
        let open = at + found;
        let opener = &body[open..open + 2];
        let closer = if opener == "{{" { "}}" } else { "%}" };
        out.push_str(&body[at..open]);
        let after = &body[open + 2..];
        // The escaped form, `{{/*` … `*/}}`: plain text.
        if let Some(inner) = after.strip_prefix("/*")
            && let Some(end) = inner.find(&format!("*/{closer}"))
        {
            out.push_str(opener);
            out.push_str(&inner[..end]);
            out.push_str(closer);
            at = open + 4 + end + 2 + closer.len();
            continue;
        }
        if opener == "{%"
            && let Some(rest) = after.trim_start().strip_prefix("end")
            && let Some(rest) = rest.trim_start().strip_prefix("%}")
        {
            at = body.len() - rest.len();
            continue;
        }
        let Some((name, args, len)) = call(after, closer) else {
            out.push_str(opener);
            at = open + 2;
            continue;
        };
        at = open + 2 + len;
        match (opener, name) {
            ("{{", "snippet") => match snippet(args, file) {
                Ok(code) => out.push_str(&code),
                Err(e) => unknown.push(e),
            },
            ("{{", "try_it") => {}
            ("{%", block) if BLOCKS.contains(&block) => {}
            _ => unknown.push(format!(
                "the shortcode `{name}` has no reading in the corpus"
            )),
        }
    }
    out.push_str(&body[at..]);
    (out, unknown)
}

/// The shortcode call after an opener: its name, its arguments and its
/// length to the end of `closer`; none when it isn't one.
fn call<'a>(after: &'a str, closer: &str) -> Option<(&'a str, &'a str, usize)> {
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
    let close = close?;
    let tail = rest[close + 1..].trim_start();
    let tail = tail.strip_prefix(closer)?;
    Some((name, &rest[1..close], after.len() - tail.len()))
}

/// A shortcode's arguments, `name=value` split by commas, a value quoted
/// or bare; none when they can't be read.
fn arguments(args: &str) -> Option<Vec<(&str, &str)>> {
    let mut out = Vec::new();
    let mut rest = args.trim();
    while !rest.is_empty() {
        let eq = rest.find('=')?;
        let key = rest[..eq].trim();
        let value = rest[eq + 1..].trim_start();
        let (value, after) = if let Some(q @ ('"' | '\'' | '`')) = value.chars().next() {
            let end = value[1..].find(q)?;
            (&value[1..=end], &value[end + 2..])
        } else {
            let end = value.find(',').unwrap_or(value.len());
            (value[..end].trim(), &value[end..])
        };
        out.push((key, value));
        let after = after.trim_start();
        rest = after.strip_prefix(',').unwrap_or(after).trim_start();
    }
    Some(out)
}

/// The `snippet` shortcode with `args`: its file's lines (between its
/// `anchor`'s marks, if it names one, without other anchors' marks) as a
/// fenced block in its `language`.
fn snippet(
    args: &str,
    file: &dyn Fn(Option<&str>, &str) -> Option<String>,
) -> Result<String, String> {
    let read = arguments(args).ok_or_else(|| format!("snippet: can't read `{args}`"))?;
    let (mut name, mut anchor, mut language, mut step) = (None, None, "rust", None);
    for (key, value) in read {
        match key {
            "file" => name = Some(value),
            "anchor" => anchor = Some(value),
            "language" => language = value,
            "step" => step = Some(value),
            _ => return Err(format!("snippet: no argument `{key}`")),
        }
    }
    let name = name.ok_or("snippet: no `file`")?;
    let source = file(step, name).ok_or_else(|| format!("snippet: no file {name}"))?;
    let lines = match anchor {
        Some(anchor) => anchored(&source, anchor)
            .ok_or_else(|| format!("snippet: no anchor {anchor} in {name}"))?,
        None => source.split('\n').filter(|line| !is_mark(line)).collect(),
    };
    Ok(format!("```{language}\n{}\n```", lines.join("\n")))
}

/// Whether `line` is an anchor's mark.
fn is_mark(line: &str) -> bool {
    let bare = line.trim();
    bare.starts_with("// ANCHOR: ") || bare.starts_with("// ANCHOR_END: ")
}

/// The lines of `source` inside its first `anchor`, without other marks.
fn anchored<'a>(source: &'a str, anchor: &str) -> Option<Vec<&'a str>> {
    let begin = format!("// ANCHOR: {anchor}");
    let end = format!("// ANCHOR_END: {anchor}");
    let mut lines = Vec::new();
    let mut inside = false;
    for line in source.split('\n') {
        let bare = line.trim();
        if inside {
            if bare == end {
                return Some(lines);
            }
            if !is_mark(line) {
                lines.push(line);
            }
        } else if bare == begin {
            inside = true;
        }
    }
    None
}

/// Zola's slug of a heading's Markdown `heading`: its text (code, links'
/// words, no emphasis) lowercased, each run of other characters a `-`.
pub fn slug(heading: &str) -> String {
    let mut text = String::new();
    let mut chars = heading.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // A link's target is no part of its text.
            ']' if chars.peek() == Some(&'(') => {
                for c in chars.by_ref() {
                    if c == ')' {
                        break;
                    }
                }
            }
            '`' | '*' | '[' | ']' => {}
            _ => text.push(c),
        }
    }
    let mut slug = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            slug.extend(c.to_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_owned()
}

/// The heading `line` starts, of level 2 or 3: its text.
fn heading_of(line: &str) -> Option<&str> {
    let text = line
        .strip_prefix("### ")
        .or_else(|| line.strip_prefix("## "))?;
    Some(text.trim().trim_end_matches('#').trim_end())
}

/// Whether `line` opens or closes a fenced code block.
fn is_fence(line: &str) -> bool {
    let bare = line.trim_start();
    bare.starts_with("```") || bare.starts_with("~~~")
}

/// The `##` and `###` headings of `text`, outside fenced code, in order.
pub fn headings(text: &str) -> Vec<&str> {
    let mut fenced = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if is_fence(line) {
            fenced = !fenced;
        } else if !fenced && let Some(heading) = heading_of(line) {
            out.push(heading);
        }
    }
    out
}

/// `text` cut at its `##` and `###` headings outside fenced code: the
/// text before the first, then each heading's. An anchor met twice gets
/// `-1`, `-2` … as Zola gives it.
pub fn sections(text: &str) -> Vec<Section> {
    let mut out = vec![Section {
        heading: None,
        anchor: None,
        text: String::new(),
    }];
    let mut seen: BTreeMap<String, u32> = BTreeMap::new();
    let mut fenced = false;
    for line in text.lines() {
        if is_fence(line) {
            fenced = !fenced;
        } else if !fenced && let Some(heading) = heading_of(line) {
            let base = slug(heading);
            let count = seen.entry(base.clone()).or_insert(0);
            let anchor = if *count == 0 {
                base
            } else {
                format!("{base}-{count}")
            };
            *count += 1;
            out.push(Section {
                heading: Some(heading.to_owned()),
                anchor: Some(anchor),
                text: String::new(),
            });
            continue;
        }
        if let Some(last) = out.last_mut() {
            last.text.push_str(line);
            last.text.push('\n');
        }
    }
    for section in &mut out {
        section.text = section.text.trim().to_owned();
    }
    out
}

/// The URL of the twin of the Learn page at `path` under
/// `site/content/learn`: Zola's URL of `learn/<path>` with `index.md`.
pub fn twin_url(path: &str) -> String {
    let stem = path.strip_suffix(".md").unwrap_or(path);
    let mut parts: Vec<&str> = std::iter::once("learn").chain(stem.split('/')).collect();
    if matches!(parts.last(), Some(&("_index" | "index"))) {
        parts.pop();
    }
    format!(
        "{}/{}/index.md",
        crate::contract::new::ENGINE_SITE,
        parts.join("/")
    )
}

/// Chunks of `sections`, of kind `kind` from `path`, titled `title` and
/// each heading after it; a section with neither heading nor text is left
/// out.
fn chunked(
    kind: HitKind,
    title: &str,
    path: &str,
    url: Option<&str>,
    sections: Vec<Section>,
) -> Vec<Chunk> {
    sections
        .into_iter()
        .filter(|s| s.heading.is_some() || !s.text.is_empty())
        .enumerate()
        .map(|(position, s)| Chunk {
            kind,
            title: s
                .heading
                .as_ref()
                .map_or_else(|| title.to_owned(), |h| format!("{title}: {h}")),
            path: path.to_owned(),
            anchor: s.anchor,
            url: url.map(str::to_owned),
            position: u32::try_from(position).unwrap_or(u32::MAX),
            text: s.text,
        })
        .collect()
}

/// The chunks of a Learn page.
fn learn_chunks(page: &Page) -> Vec<Chunk> {
    let path = format!("{LEARN_ROOT}/{}", page.path);
    let url = twin_url(&page.path);
    chunked(
        HitKind::Learn,
        &page.title,
        &path,
        Some(&url),
        sections(&page.text),
    )
}

/// A skill's `SKILL.md` without its YAML front matter, and its
/// description.
fn skill_body(text: &str) -> (Option<String>, &str) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return (None, text);
    };
    let Some(end) = rest.find("\n---") else {
        return (None, text);
    };
    let description = rest[..end].lines().find_map(|line| {
        line.strip_prefix("description:")
            .map(|d| d.trim().trim_matches('"').to_owned())
    });
    (description, rest[end + 4..].trim_start_matches('\n'))
}

/// The chunks of the skill pack: each `SKILL.md` cut at its headings, its
/// title its `#` heading and its description at its start; each other
/// `.md` file one chunk.
fn skill_chunks(pack: &Pack) -> Vec<Chunk> {
    let mut out = Vec::new();
    for skill in &pack.skills {
        for (file, bytes) in &skill.files {
            if !has_extension(file, "md") {
                continue;
            }
            let text = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
            let path = format!("{SKILLS_ROOT}/{}/{file}", skill.name);
            if file != crate::skills::FILE {
                let title = format!("{}: {file}", skill.name);
                out.push(Chunk {
                    kind: HitKind::Skill,
                    title,
                    path,
                    anchor: None,
                    url: None,
                    position: 0,
                    text: text.trim().to_owned(),
                });
                continue;
            }
            let (description, body) = skill_body(&text);
            let (heading, body) = match body.strip_prefix("# ") {
                Some(rest) => {
                    let (heading, body) = rest.split_once('\n').unwrap_or((rest, ""));
                    (Some(heading.trim()), body)
                }
                None => (None, body),
            };
            let title = match heading {
                Some(heading) => format!("{}: {heading}", skill.name),
                None => skill.name.clone(),
            };
            let mut parts = sections(body);
            if let (Some(description), Some(first)) = (description, parts.first_mut()) {
                format!("{description}\n\n{}", first.text)
                    .trim()
                    .clone_into(&mut first.text);
            }
            out.extend(chunked(HitKind::Skill, &title, &path, None, parts));
        }
    }
    out
}

/// What an example's Rust file says of itself: its `//!` lines, then the
/// names after `pub fn`, `pub struct`, `pub enum` and `impl`.
pub fn example_text(source: &str) -> String {
    let mut doc = Vec::new();
    let mut names = Vec::new();
    for line in source.lines() {
        let bare = line.trim_start();
        if let Some(said) = bare.strip_prefix("//!") {
            doc.push(said.strip_prefix(' ').unwrap_or(said));
            continue;
        }
        for item in ["pub fn ", "pub struct ", "pub enum "] {
            if let Some(rest) = bare.strip_prefix(item) {
                let end = rest
                    .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .unwrap_or(rest.len());
                if end > 0 {
                    names.push(rest[..end].to_owned());
                }
            }
        }
        if let Some(rest) = bare.strip_prefix("impl")
            && rest.starts_with([' ', '<'])
        {
            let end = rest.find(['{', ';']).unwrap_or(rest.len());
            let rest = rest[..end].split(" where").next().unwrap_or("");
            let signature = rest.trim();
            if !signature.is_empty() {
                let gap = if signature.starts_with('<') { "" } else { " " };
                names.push(format!("impl{gap}{signature}"));
            }
        }
    }
    let mut text = doc.join("\n").trim().to_owned();
    if !names.is_empty() {
        text.push_str("\n\n");
        text.push_str(&names.join("\n"));
    }
    text.trim().to_owned()
}

/// The chunks of the examples: each `.rs` file's [`example_text`], each
/// `.md` file whole.
fn example_chunks() -> Vec<Chunk> {
    let mut out = Vec::new();
    for (path, bytes) in EXAMPLES {
        let rust = has_extension(path, "rs");
        if !rust && !has_extension(path, "md") {
            continue;
        }
        let source = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
        let text = if rust {
            example_text(&source)
        } else {
            source.trim().to_owned()
        };
        out.push(Chunk {
            kind: HitKind::Example,
            title: (*path).to_owned(),
            path: format!("{EXAMPLES_ROOT}/{path}"),
            anchor: None,
            url: None,
            position: 0,
            text,
        });
    }
    out
}

/// The chunks of the problem codes: each one's page.
fn code_chunks() -> Vec<Chunk> {
    codes::TABLE
        .iter()
        .map(|entry| {
            let code = entry.code.0;
            Chunk {
                kind: HitKind::Code,
                title: format!("{code}: {}", entry.title),
                path: format!("{CODES_ROOT}/{code}.md"),
                anchor: None,
                url: None,
                position: 0,
                text: format!(
                    "{}\n\n{} ({})\n\n{}",
                    entry.page.replace("\r\n", "\n").trim(),
                    entry.name,
                    entry.class.name(),
                    entry.help
                ),
            }
        })
        .collect()
}

/// Every chunk of the corpus, by source path, then position.
pub fn chunks() -> Vec<Chunk> {
    let mut out: Vec<Chunk> = learn_pages().iter().flat_map(learn_chunks).collect();
    out.extend(skill_chunks(&Pack::embedded()));
    out.extend(example_chunks());
    out.extend(code_chunks());
    out.sort_by(|a, b| (&a.path, a.position).cmp(&(&b.path, b.position)));
    out
}

#[cfg(test)]
mod tests;
