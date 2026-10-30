//! `NOTICES.txt`: the notices a package of a game owes for the works in
//! it, written beside the game. `shuyi web` writes one into the web build
//! ([`crate::web`]). It lists the crates compiled in ([`crates`]), the web
//! shell's files and the game's own license texts ([`game_texts`]), then
//! each license text once ([`write()`]).
//!
//! It copies what the packages say and judges nothing: every license of an
//! `OR` is listed, and a crate with no license file gets the standard text
//! of each license it declares that this tool can write.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::project::Notices;

/// The file's name.
pub const FILE: &str = "NOTICES.txt";

/// The engine's repository: its crates' `repository`.
const ENGINE_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The engine's license texts, from the repository's root.
const ENGINE_MIT: &str = include_str!("../../../LICENSE-MIT");
const ENGINE_APACHE: &str = include_str!("../../../LICENSE-APACHE");

/// What a license file's name starts with, in capitals.
const LICENSE_FILES: [&str; 6] = [
    "LICENSE",
    "LICENCE",
    "COPYING",
    "NOTICE",
    "COPYRIGHT",
    "UNLICENSE",
];

/// Which part of `NOTICES.txt` a work is listed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Section {
    /// The crates compiled into the game.
    Crates,
    /// The web shell's files beside the page.
    Shell,
    /// The engine's files a crate compiles in that aren't crates.
    Data,
    /// The game's own files whose texts it names in `shuyi.ron`.
    Game,
}

impl Section {
    /// Its heading.
    fn heading(self) -> &'static str {
        match self {
            Self::Crates => "The crates compiled in",
            Self::Shell => "The web shell's files",
            Self::Data => "Data compiled in",
            Self::Game => "The game's files",
        }
    }
}

/// What a package is built for, as [`FILES`] says which carry a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    /// The web build: the shell's files beside the page.
    Web,
    /// A native build (Windows, Linux, macOS).
    Native,
}

/// A file of the engine's that reaches a game's package and isn't a
/// crate. Every one has its row in `THIRD_PARTY_ASSETS.md` (a test).
#[derive(Debug, Clone, Copy)]
pub struct EngineFile {
    /// The file in the engine's repository, or the folder of the crate it
    /// is compiled in from.
    pub file: &'static str,
    /// The work it comes from.
    pub work: &'static str,
    /// The work's version, if it has one.
    pub version: &'static str,
    /// Where it comes from.
    pub source: &'static str,
    /// Its license.
    pub license: &'static str,
    /// Its license text's file in the repository, and the text.
    pub text: (&'static str, &'static str),
    /// The builds that carry it.
    pub builds: &'static [Build],
    /// The crate that compiles it in, if a crate does: it is carried only
    /// when that crate is.
    pub within: Option<&'static str>,
}

/// The engine's files that aren't crates, as data: the web shell's
/// vendored scripts, and the controller database `gilrs` compiles into
/// native builds (which the crates' walk misses, in a subfolder of its
/// package).
pub const FILES: [EngineFile; 4] = [
    EngineFile {
        file: "crates/cli/web/mq_js_bundle.js",
        work: "macroquad",
        version: "0.4.16",
        source: "https://github.com/not-fl3/macroquad",
        license: "MIT",
        text: (
            "crates/cli/web/mq_js_bundle-LICENSE-MIT.txt",
            include_str!("../web/mq_js_bundle-LICENSE-MIT.txt"),
        ),
        builds: &[Build::Web],
        within: None,
    },
    EngineFile {
        file: "crates/cli/web/sapp_jsutils.js",
        work: "sapp-jsutils",
        version: "0.1.7",
        source: "https://github.com/not-fl3/sapp-jsutils",
        license: "MIT",
        text: (
            "crates/cli/web/sapp_jsutils-LICENSE-MIT.txt",
            include_str!("../web/sapp_jsutils-LICENSE-MIT.txt"),
        ),
        builds: &[Build::Web],
        within: None,
    },
    EngineFile {
        file: "crates/cli/web/quad-storage.js",
        work: "quad-storage",
        version: "0.1.3",
        source: "https://github.com/optozorax/quad-storage",
        license: "MIT",
        text: (
            "crates/cli/web/quad-storage-LICENSE-MIT.txt",
            include_str!("../web/quad-storage-LICENSE-MIT.txt"),
        ),
        builds: &[Build::Web],
        within: None,
    },
    EngineFile {
        file: "gamecontrollerdb.txt",
        work: "SDL_GameControllerDB",
        version: "",
        source: "https://github.com/mdqinc/SDL_GameControllerDB",
        license: "Zlib",
        text: (
            "crates/app/SDL_GameControllerDB-LICENSE.txt",
            include_str!("../../app/SDL_GameControllerDB-LICENSE.txt"),
        ),
        builds: &[Build::Native],
        within: Some("gilrs"),
    },
];

impl EngineFile {
    /// Its name as `NOTICES.txt` lists it: the file and its work.
    pub fn name(&self) -> String {
        let file = self.file.rsplit('/').next().unwrap_or(self.file);
        match self.within {
            Some(krate) => format!("{file}, from {}, compiled in by {krate}", self.work),
            None => format!("{file}, from {}", self.work),
        }
    }

    /// Its license text's file name, without its folders.
    pub fn text_name(&self) -> &'static str {
        self.text.0.rsplit('/').next().unwrap_or(self.text.0)
    }

    /// Its work in `section`.
    pub fn work(&self, section: Section) -> Work {
        Work {
            section,
            name: self.name(),
            version: self.version.to_owned(),
            license: self.license.to_owned(),
            source: self.source.to_owned(),
            texts: vec![Text::new(self.text_name(), self.text.1)],
        }
    }
}

/// The works of [`FILES`] that a `build` package carries whose crates are
/// `crates` (the [`crates`] compiled in), apart from the web shell's,
/// which `web` lists with the shell.
pub fn engine_files(build: Build, crates: &[Work]) -> Vec<Work> {
    FILES
        .iter()
        .filter(|file| file.builds.contains(&build))
        .filter_map(|file| {
            let krate = file.within?;
            crates
                .iter()
                .any(|work| work.name == krate)
                .then(|| file.work(Section::Data))
        })
        .collect()
}

/// A license or notice text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Text {
    /// What it is: the file it was read from, or how it was written.
    pub label: String,
    /// The text.
    pub body: String,
}

impl Text {
    /// The text `body`, called `label`.
    pub fn new(label: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            body: body.into(),
        }
    }
}

/// A work in the package, and its texts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Work {
    /// Where it is listed.
    pub section: Section,
    /// Its name.
    pub name: String,
    /// Its version, if it has one.
    pub version: String,
    /// Its license, as it declares it, if it does.
    pub license: String,
    /// Where it comes from, if known.
    pub source: String,
    /// Its license and notice texts.
    pub texts: Vec<Text>,
}

impl Work {
    /// The work `name` in `section`, with no version, license, source or
    /// text yet.
    pub fn new(section: Section, name: impl Into<String>) -> Self {
        Self {
            section,
            name: name.into(),
            version: String::new(),
            license: String::new(),
            source: String::new(),
            texts: Vec::new(),
        }
    }
}

/// The engine's own texts, for its crates and its files.
pub fn engine_texts() -> Vec<Text> {
    vec![
        Text::new("the engine's LICENSE-MIT", ENGINE_MIT),
        Text::new("the engine's LICENSE-APACHE", ENGINE_APACHE),
    ]
}

/// The crates the package `package` of the workspace at `root` compiles
/// in for `triple` with `features` (its own) turned on: its normal
/// dependencies, all the way down, by `cargo metadata`. Dev- and
/// build-dependencies don't ship; the game's own crates (workspace members
/// outside the engine's repository) are the game's work, not a notice.
pub fn crates(
    root: &Path,
    package: &str,
    triple: &str,
    features: &[String],
) -> Result<Vec<Work>, String> {
    let cargo = crate::cargo();
    let mut args: Vec<String> = [
        "metadata",
        "--format-version",
        "1",
        "--filter-platform",
        triple,
    ]
    .map(str::to_owned)
    .to_vec();
    for feature in features {
        args.extend(["--features".to_owned(), format!("{package}/{feature}")]);
    }
    let output = Command::new(cargo)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("spawn cargo metadata: {e}"))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed: {}", error.trim()));
    }
    let json = String::from_utf8_lossy(&output.stdout);
    crates_of(&json, package)
}

/// `cargo metadata`'s answer, the fields [`crates`] reads.
#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    resolve: Resolve,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    license: Option<String>,
    license_file: Option<String>,
    repository: Option<String>,
    source: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    manifest_path: PathBuf,
}

#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<Dep>,
}

#[derive(Deserialize)]
struct Dep {
    pkg: String,
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct DepKind {
    kind: Option<String>,
}

/// [`crates`] from `cargo metadata`'s answer, `json`.
fn crates_of(json: &str, package: &str) -> Result<Vec<Work>, String> {
    let meta: Metadata =
        serde_json::from_str(json).map_err(|e| format!("read cargo metadata: {e}"))?;
    let member = |p: &Package| meta.workspace_members.contains(&p.id);
    let start = meta
        .packages
        .iter()
        .filter(|p| p.name == package)
        .find(|p| member(p))
        .ok_or_else(|| format!("cargo metadata: no package `{package}` in the workspace"))?;
    let nodes: HashMap<&str, &Node> = meta
        .resolve
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut seen = BTreeSet::new();
    let mut stack = vec![start.id.as_str()];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(node) = nodes.get(id) else { continue };
        let normal = node
            .deps
            .iter()
            .filter(|dep| dep.dep_kinds.iter().any(|kind| kind.kind.is_none()));
        stack.extend(normal.map(|dep| dep.pkg.as_str()));
    }
    let engine = |p: &Package| p.repository.as_deref() == Some(ENGINE_REPOSITORY);
    let mut shipped: Vec<&Package> = meta
        .packages
        .iter()
        .filter(|p| seen.contains(p.id.as_str()) && (engine(p) || !member(p)))
        .collect();
    shipped.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(shipped.into_iter().map(crate_work).collect())
}

/// A crate's work: what it declares, and its texts.
fn crate_work(package: &Package) -> Work {
    let mut work = Work::new(Section::Crates, &package.name);
    work.version.clone_from(&package.version);
    work.license = package.license.clone().unwrap_or_default();
    work.source = package
        .repository
        .clone()
        .or_else(|| {
            let registry = package.source.as_deref()?;
            registry
                .contains("crates.io-index")
                .then(|| format!("https://crates.io/crates/{}", package.name))
        })
        .unwrap_or_default();
    work.texts = license_files(package);
    if work.texts.is_empty() {
        work.texts = if package.repository.as_deref() == Some(ENGINE_REPOSITORY) {
            engine_texts()
        } else {
            written_texts(&work.license, &package.authors)
        };
    }
    work
}

/// The license files at the top of a crate's folder, and its
/// `license_file`, by name.
fn license_files(package: &Package) -> Vec<Text> {
    let Some(dir) = package.manifest_path.parent() else {
        return Vec::new();
    };
    let mut names = BTreeSet::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let upper = name.to_uppercase();
            if entry.path().is_file() && LICENSE_FILES.iter().any(|p| upper.starts_with(p)) {
                names.insert(name);
            }
        }
    }
    if let Some(file) = &package.license_file {
        names.insert(file.replace('\\', "/"));
    }
    names
        .into_iter()
        .filter_map(|name| {
            let bytes = fs::read(dir.join(&name)).ok()?;
            Some(Text::new(name, String::from_utf8_lossy(&bytes)))
        })
        .collect()
}

/// The ids of a license expression, once each, in order: `MIT OR
/// Apache-2.0`, `(MIT OR Apache-2.0) AND Unicode-3.0` and the old
/// `MIT/Apache-2.0` alike.
fn license_ids(expression: &str) -> Vec<&str> {
    let mut ids: Vec<&str> = Vec::new();
    let words = expression
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '/'))
        .filter(|word| !word.is_empty() && !matches!(*word, "OR" | "AND" | "WITH"));
    for word in words {
        if !ids.contains(&word) {
            ids.push(word);
        }
    }
    ids
}

/// The standard texts of `license`'s ids this tool can write, for a crate
/// with no license file: Apache-2.0 as it is, and MIT with its copyright
/// line naming `authors` (none without them).
fn written_texts(license: &str, authors: &[String]) -> Vec<Text> {
    let label = |id: &str| {
        format!("{id}, written from the declared license: the package has no license file")
    };
    license_ids(license)
        .into_iter()
        .filter_map(|id| match id {
            "Apache-2.0" => Some(Text::new(label(id), ENGINE_APACHE)),
            "MIT" if !authors.is_empty() => {
                Some(Text::new(label(id), mit_text(&authors.join(", "))))
            }
            _ => None,
        })
        .collect()
}

/// The MIT text with its copyright line naming `holders`.
fn mit_text(holders: &str) -> String {
    ENGINE_MIT
        .lines()
        .map(|line| {
            if line.starts_with("Copyright") {
                format!("Copyright (c) {holders}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect()
}

/// The game's own license texts that `notices` names, each a work of its
/// own read whole from the project at `root`.
pub fn game_texts(root: &Path, notices: &Notices) -> Result<Vec<Work>, String> {
    notices
        .texts
        .iter()
        .map(|path| {
            let body =
                fs::read(root.join(path)).map_err(|e| format!("notices: read {path}: {e}"))?;
            let mut work = Work::new(Section::Game, path);
            work.texts = vec![Text::new(path, String::from_utf8_lossy(&body))];
            Ok(work)
        })
        .collect()
}

/// A text as it is compared and written: `\n` line ends, no trailing
/// blank lines.
fn normalized(body: &str) -> String {
    let text = body.replace("\r\n", "\n");
    let lines: Vec<&str> = text.lines().collect();
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map_or(0, |last| last + 1);
    let mut out = lines[..end].join("\n");
    out.push('\n');
    out
}

/// Writes `NOTICES.txt` into `dist` for the game called `title`.
pub fn write(dist: &Path, title: &str, works: &[Work]) -> Result<(), String> {
    let path = dist.join(FILE);
    fs::write(&path, render(title, works)).map_err(|e| format!("write {}: {e}", path.display()))
}

/// `NOTICES.txt`'s text: the works by section, then each text once,
/// headed by the works it covers. The same works give the same bytes.
fn render(title: &str, works: &[Work]) -> String {
    let mut sorted: Vec<&Work> = works.iter().collect();
    sorted.sort_by_key(|work| work.section);
    let heading = format!("Notices for {title}");
    let mut out = format!("{heading}\n{}\n\n", "=".repeat(heading.chars().count()));
    out.push_str(
        "This build holds the works listed below. Each license or notice\n\
         text they come with follows the list, once for all the works it\n\
         covers.\n",
    );
    let mut section = None;
    for work in &sorted {
        if section != Some(work.section) {
            section = Some(work.section);
            let heading = work.section.heading();
            let _ = write!(out, "\n{heading}\n{}\n\n", "-".repeat(heading.len()));
        }
        let mut line = format!("- {}", named(work));
        for part in [&work.license, &work.source] {
            if !part.is_empty() {
                let _ = write!(line, ", {part}");
            }
        }
        if work.texts.is_empty() {
            line.push_str(" (no license text found)");
        }
        out.push_str(&line);
        out.push('\n');
    }
    // Each text once, in the order the works first name it.
    let mut texts: Vec<(String, Vec<String>)> = Vec::new();
    for work in &sorted {
        for text in &work.texts {
            let body = normalized(&text.body);
            let cover = if text.label == work.name {
                named(work)
            } else {
                format!("{} ({})", named(work), text.label)
            };
            match texts.iter_mut().find(|(known, _)| *known == body) {
                Some((_, covers)) => covers.push(cover),
                None => texts.push((body, vec![cover])),
            }
        }
    }
    let rule = "=".repeat(72);
    out.push_str("\n\nLicense texts\n=============\n");
    for (n, (body, covers)) in texts.iter().enumerate() {
        let _ = write!(out, "\n{rule}\nText {} covers:\n", n + 1);
        for cover in covers {
            let _ = writeln!(out, "  {cover}");
        }
        let _ = write!(out, "{rule}\n\n{body}");
    }
    out
}

/// A work's name and version.
fn named(work: &Work) -> String {
    if work.version.is_empty() {
        work.name.clone()
    } else {
        format!("{} {}", work.name, work.version)
    }
}

#[cfg(test)]
mod tests;
