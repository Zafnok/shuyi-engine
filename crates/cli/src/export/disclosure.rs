//! The last step of every export: the final package read back, after every
//! copy its target makes, and written down in `exposure-report.json` beside
//! it: each file with its kind, its source, its size and its SHA-256, each
//! file of each pack, the debug file, whatever else is beside it, and what
//! the package was built from.
//!
//! A game whose `shuyi.ron` has a `disclosure` part ([`Disclosure`]) is held
//! to it: a file of the game or of a pack that the part doesn't allow, a
//! path it forbids, a canary of it anywhere (as UTF-8 or UTF-16, in what a
//! pack unpacks to as well), or a file beside the package that this export
//! didn't make refuses the export. A refused export leaves the report and
//! no package, and the web build's folder emptied.
//!
//! The report names a canary by its place in the part, never by its text,
//! so it can be read by people who shouldn't read the canaries.
//!
//! It finds what is written plainly. Bytes a game compresses or encrypts
//! itself, what a compiler makes of a rule, and a `pdc` image or sound aren't
//! searched (a Playdate's program is, in the ELF `pdc` packs it from). A
//! package that passes holds no declared text, not "nothing unreleased".

mod sha256;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::pack::{self, Pack, PackError};

use super::{ExportCx, Exporter, archive};
use crate::contract::codes::{self, Code};
use crate::contract::export::REPORT as EXPORT_REPORT;
use crate::project::{self, Disclosure};

/// The report's file, beside the package.
pub const REPORT: &str = "exposure-report.json";

/// The shortest canary, in bytes: a shorter one is found by chance in any
/// program.
pub const MIN_CANARY: usize = 6;

/// The longest canary, in bytes.
pub const MAX_CANARY: usize = 256;

/// The biggest file read, 2 GiB: a bigger one refuses the export, since it
/// can't be said to hold nothing.
pub const MAX_FILE_BYTES: u64 = 2 << 30;

/// The source of a file this export didn't make.
const NOT_MADE: &str = "not made by this export";

/// The findings an error names; the report has them all.
const NAMED: usize = 20;

/// What a file is to the package.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum FileKind {
    /// A built program.
    Program,
    /// A file the exporter writes itself.
    Written,
    /// A pack the exporter bakes.
    Pack,
    /// A file of the game the exporter copies: `disclosure.allowed_files`
    /// names it.
    LooseAsset,
    /// A file in the package no exporter made or `allowed_files` names.
    Undeclared,
    /// The archive players download: its files are those of the package.
    Archive,
    /// The program with its debug sections, beside the package.
    DebugCompanion,
    /// A file beside the package that this export didn't make.
    Beside,
    /// What a shipped file was made from by a tool that compresses it, read
    /// for the canaries it would hide.
    BuildInput,
}

/// What an export wrote and where, as its exporter says: the rest of a
/// package's files are the inspector's to find.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Shipped {
    /// The package's folder.
    pub tree: PathBuf,
    /// The archive made of it, if any.
    pub archive: Option<PathBuf>,
    /// What starts the name of each file in the archive.
    pub prefix: String,
    /// The debug file beside it, if any.
    pub debug: Option<PathBuf>,
    /// Each file the exporter knows of, by its path in the package, with its
    /// kind and source.
    pub sources: BTreeMap<String, (FileKind, String)>,
    /// What shipped files were made from, as [`FileKind::BuildInput`]: each
    /// one's name, what it is, and the file.
    pub inputs: Vec<(String, String, PathBuf)>,
    /// Other folders this export published the same bytes in, emptied with
    /// the package.
    pub public: Vec<PathBuf>,
    /// The files a tool writes differently each build, by path, with what
    /// starts the line it changes: their hashes leave that line out, and the
    /// archive's hash is of its files, so the same input gives the same
    /// report.
    pub varying: BTreeMap<String, String>,
}

impl Shipped {
    /// The package in `tree`, with `archive` made of it under `prefix`.
    pub fn new(tree: PathBuf, archive: Option<PathBuf>, prefix: impl Into<String>) -> Self {
        Self {
            tree,
            archive,
            prefix: prefix.into(),
            debug: None,
            sources: BTreeMap::new(),
            inputs: Vec::new(),
            public: Vec::new(),
            varying: BTreeMap::new(),
        }
    }

    /// Says what the file at `path` in the package is and where it is from.
    pub fn source(&mut self, path: impl Into<String>, kind: FileKind, source: impl Into<String>) {
        self.sources.insert(path.into(), (kind, source.into()));
    }
}

/// What the package was built from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildInputs {
    /// The target.
    pub target: String,
    /// The target profile its check ran with.
    pub profile: String,
    /// The game's package.
    pub package: String,
    /// The program, or the library on the Playdate.
    pub bin: String,
    /// The game's version.
    pub version: String,
    /// Whether it was a release build.
    pub release: bool,
    /// The `cargo` arguments of each build.
    pub builds: Vec<Vec<String>>,
    /// Whether the builds keep the package's default features.
    pub default_features: bool,
    /// The features of the game's package they turn on, with what those turn
    /// on.
    pub features: Vec<String>,
    /// `rustc -V`.
    pub toolchain: String,
    /// The SHA-256 of `shuyi.ron`, which holds the `disclosure` part.
    pub project_file: Option<String>,
    /// The SHA-256 of the workspace's `Cargo.lock`.
    pub cargo_lock: Option<String>,
}

/// A file of the export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExposedFile {
    /// Its path: from the package's root for a file of the package, else from
    /// the export's folder.
    pub path: String,
    /// What it is.
    pub kind: FileKind,
    /// Where it is from.
    pub source: String,
    /// Its size in bytes.
    pub bytes: u64,
    /// Its SHA-256.
    pub sha256: String,
    /// What the hash leaves out, if a tool writes part of the file differently
    /// each build. The hash is then of what is left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub varies: Option<String>,
}

/// A file of a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExposedEntry {
    /// The file holding the pack.
    pub file: String,
    /// Where the pack starts in that file: above 0, the file embeds it.
    pub offset: u64,
    /// Its path in the pack.
    pub path: String,
    /// The kinds it was baked as: none for a file kept as its own bytes.
    pub kinds: Vec<String>,
    /// Its size in bytes, unpacked.
    pub bytes: u64,
    /// The SHA-256 of what it unpacks to.
    pub sha256: String,
}

/// What a finding is.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// A path of `disclosure.forbidden_paths`, as a path or as a program's
    /// text.
    ForbiddenPath,
    /// A canary of `disclosure.canaries`.
    Canary,
    /// A file `disclosure.allowed_files` doesn't name.
    UndeclaredFile,
    /// A file of a pack `disclosure.allowed_pack_entries` doesn't name.
    UndeclaredPackEntry,
    /// A file beside the package that this export didn't make.
    Beside,
    /// A pack that doesn't read, so its files can't be listed.
    UnreadablePack,
    /// The archive and the package's folder differ.
    ArchiveDiffers,
    /// A file over [`MAX_FILE_BYTES`].
    TooBig,
}

/// One thing that refuses the export.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    /// The file: its [`ExposedFile::path`].
    pub path: String,
    /// The path in the pack, if it is in one.
    pub entry: Option<String>,
    /// What it is.
    pub kind: FindingKind,
    /// Where the file is from.
    pub source: String,
    /// Which place of `disclosure.canaries`, for a canary. The report never
    /// holds its text.
    pub canary: Option<usize>,
}

/// `exposure-report.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExposureReport {
    /// Whether the package stands: false when a finding removed it.
    pub ok: bool,
    /// Whether `shuyi.ron` has a `disclosure` part to hold it to. Without one
    /// nothing is refused, and there are no findings.
    pub declared: bool,
    /// What it was built from.
    pub inputs: BuildInputs,
    /// Every file of the package, the archive and what is beside them, by
    /// path.
    pub files: Vec<ExposedFile>,
    /// Every file of every pack.
    pub pack_entries: Vec<ExposedEntry>,
    /// What refused it.
    pub findings: Vec<Finding>,
}

/// What the part says that `shuyi.ron`'s reader can't: each line that can't
/// match anything, or can't be meant.
pub fn problems(part: &Disclosure) -> Vec<String> {
    let mut problems = Vec::new();
    let lists = [
        ("allowed_files", &part.allowed_files),
        ("allowed_pack_entries", &part.allowed_pack_entries),
        ("forbidden_paths", &part.forbidden_paths),
    ];
    for (name, list) in lists {
        let mut seen = BTreeSet::new();
        for (at, path) in list.iter().enumerate() {
            if let Some(why) = path_problem(path) {
                problems.push(format!("disclosure.{name}[{at}], `{path}`: {why}"));
            }
            if !seen.insert(path) {
                problems.push(format!("disclosure.{name}[{at}], `{path}`: listed twice"));
            }
        }
    }
    for (name, list) in &lists[..2] {
        for path in list
            .iter()
            .filter(|path| part.forbidden_paths.contains(path))
        {
            problems.push(format!(
                "`{path}` is in both disclosure.{name} and disclosure.forbidden_paths"
            ));
        }
    }
    let mut seen = BTreeSet::new();
    for (at, canary) in part.canaries.iter().enumerate() {
        // The texts stay out of what is printed, as out of the report.
        if canary.len() < MIN_CANARY {
            problems.push(format!(
                "disclosure.canaries[{at}]: under {MIN_CANARY} bytes, which any program holds by \
                 chance"
            ));
        }
        if canary.len() > MAX_CANARY {
            problems.push(format!(
                "disclosure.canaries[{at}]: over {MAX_CANARY} bytes"
            ));
        }
        if !seen.insert(canary) {
            problems.push(format!("disclosure.canaries[{at}]: listed twice"));
        }
    }
    problems
}

/// Why `path` isn't a path a package or a pack can hold, if so.
fn path_problem(path: &str) -> Option<&'static str> {
    if path.contains('\\') {
        return Some("write it with `/`");
    }
    if path.contains(['*', '?', '[', ']']) {
        return Some("a path matches whole, with no pattern");
    }
    if path.starts_with('/') || path.contains(':') {
        return Some("write it from the package's root or the pack's");
    }
    if path.split('/').any(|part| matches!(part, "" | "." | "..")) {
        return Some("it has an empty part, `.` or `..`");
    }
    None
}

/// Writes the report of what `shipped` holds, and refuses the export with
/// what `part` finds there: on a refusal, or a package that can't be read,
/// no package is left. The error is the problem code and the message.
pub fn gate(
    cx: &ExportCx,
    exporter: &dyn Exporter,
    part: Option<&Disclosure>,
    shipped: &Shipped,
) -> Result<(), (Code, String)> {
    let checked = build_inputs(cx, exporter)
        .and_then(|inputs| inspect(&cx.out, part, shipped, MAX_FILE_BYTES, inputs));
    let report = match checked {
        Ok(report) => report,
        Err(error) => {
            remove_package(&cx.out, shipped);
            return Err((codes::PACKAGE, format!("inspecting the package: {error}")));
        }
    };
    let json = serde_json::to_string_pretty(&report).unwrap_or_default();
    let file = cx.out.join(REPORT);
    if let Err(e) = fs::write(&file, format!("{json}\n")) {
        remove_package(&cx.out, shipped);
        return Err((codes::FILE, format!("write {}: {e}", file.display())));
    }
    if report.ok {
        return Ok(());
    }
    remove_package(&cx.out, shipped);
    Err((codes::DISCLOSURE, refusal(&report.findings)))
}

/// What the error of a refused export says.
fn refusal(findings: &[Finding]) -> String {
    let mut text = format!(
        "the package breaks shuyi.ron's `disclosure`, and was removed ({REPORT} lists all it held):"
    );
    for finding in findings.iter().take(NAMED) {
        let _ = write!(text, "\n  {}", said(finding));
    }
    if findings.len() > NAMED {
        let _ = write!(text, "\n  and {} more, in {REPORT}", findings.len() - NAMED);
    }
    text
}

/// One finding, for people.
fn said(finding: &Finding) -> String {
    let place = match &finding.entry {
        Some(entry) => format!("{entry} in the pack in {}", finding.path),
        None => finding.path.clone(),
    };
    let what = match finding.kind {
        FindingKind::ForbiddenPath => "is or names a path of disclosure.forbidden_paths".to_owned(),
        FindingKind::Canary => format!(
            "holds disclosure.canaries[{}]",
            finding.canary.unwrap_or_default()
        ),
        FindingKind::UndeclaredFile => "isn't in disclosure.allowed_files".to_owned(),
        FindingKind::UndeclaredPackEntry => "isn't in disclosure.allowed_pack_entries".to_owned(),
        FindingKind::Beside => {
            "is beside the package, and this export didn't make it: remove it".to_owned()
        }
        FindingKind::UnreadablePack => {
            "holds a pack that doesn't read, so what it holds can't be listed".to_owned()
        }
        FindingKind::ArchiveDiffers => "differs between the archive and its folder".to_owned(),
        FindingKind::TooBig => format!("is over {MAX_FILE_BYTES} bytes, too big to read"),
    };
    format!("{place} ({}) {what}", finding.source)
}

/// Removes what a refused export made, and the report of the last one that
/// wasn't: the folder, its archive, the debug file and the same bytes
/// anywhere else it published them. The folder of those stays, as a server
/// may be running in it.
fn remove_package(out: &Path, shipped: &Shipped) {
    let _ = fs::remove_dir_all(&shipped.tree);
    for file in shipped.archive.iter().chain(&shipped.debug) {
        let _ = fs::remove_file(file);
    }
    let _ = fs::remove_file(out.join(EXPORT_REPORT));
    for folder in &shipped.public {
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            let path = entry.path();
            let _ = if path.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            };
        }
    }
}

/// What `exporter`'s package of `cx` is built from.
fn build_inputs(cx: &ExportCx, exporter: &dyn Exporter) -> Result<BuildInputs, String> {
    let builds = exporter.cargo_args(cx);
    let default_features = !builds
        .iter()
        .flatten()
        .any(|arg| arg == "--no-default-features");
    let asked: Vec<&str> = builds
        .iter()
        .flat_map(|args| args.windows(2))
        .filter(|pair| pair[0] == "--features" || pair[0] == "-F")
        .flat_map(|pair| pair[1].split([',', ' ']))
        .collect();
    let (_, _, map) = super::metadata(&cx.root, &cx.package).map_err(|e| e.message())?;
    let toolchain = Command::new("rustc")
        .arg("-V")
        .current_dir(&cx.root)
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default();
    let hash = |file: PathBuf| fs::read(file).ok().map(|bytes| sha256::hex(&bytes));
    Ok(BuildInputs {
        target: cx.target.clone(),
        profile: cx.profile.clone(),
        package: cx.package.clone(),
        bin: cx.bin.clone(),
        version: cx.version.clone(),
        release: cx.release,
        features: resolved_features(&map, default_features, &asked),
        builds,
        default_features,
        toolchain,
        project_file: hash(cx.root.join(project::FILE)),
        cargo_lock: hash(cx.workspace.join("Cargo.lock")),
    })
}

/// The features `asked`, and `default` if `default_features`, with what they
/// turn on by `map` (a package's `[features]`), sorted. A dependency's
/// (`dep:x`, `x/y`) is listed as it is written.
fn resolved_features(
    map: &BTreeMap<String, Vec<String>>,
    default_features: bool,
    asked: &[&str],
) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut waiting: Vec<&str> = asked.to_vec();
    if default_features && map.contains_key("default") {
        waiting.push("default");
    }
    while let Some(feature) = waiting.pop() {
        if found.insert(feature.to_owned()) {
            let on = map.get(feature).into_iter().flatten();
            waiting.extend(on.map(String::as_str));
        }
    }
    found.into_iter().collect()
}

/// One text looked for in a file's bytes.
struct Needle {
    bytes: Vec<u8>,
    kind: FindingKind,
    canary: Option<usize>,
    /// Whether it is looked for in programs alone: a path, which any text may
    /// mention.
    programs_only: bool,
}

/// The texts of `part`: each canary as UTF-8 and UTF-16, and each forbidden
/// path with either slash, since a Windows build of `include_dir!` keeps a
/// `\`.
fn needles(part: Option<&Disclosure>) -> Vec<Needle> {
    let Some(part) = part else {
        return Vec::new();
    };
    let mut needles = Vec::new();
    for (at, canary) in part.canaries.iter().enumerate() {
        let wide = canary.encode_utf16().flat_map(u16::to_le_bytes).collect();
        for bytes in [canary.as_bytes().to_vec(), wide] {
            needles.push(Needle {
                bytes,
                kind: FindingKind::Canary,
                canary: Some(at),
                programs_only: false,
            });
        }
    }
    for path in &part.forbidden_paths {
        for form in [path.clone(), path.replace('/', "\\")] {
            needles.push(Needle {
                bytes: form.into_bytes(),
                kind: FindingKind::ForbiddenPath,
                canary: None,
                programs_only: true,
            });
        }
    }
    needles
}

/// Where `needle` starts in `bytes` at `from` or after.
fn find(bytes: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    let (first, rest) = needle.split_first()?;
    let mut at = from;
    while let Some(tail) = bytes.get(at..) {
        at += tail.iter().position(|byte| byte == first)?;
        if bytes.get(at + 1..at + needle.len()) == Some(rest) {
            return Some(at);
        }
        at += 1;
    }
    None
}

/// Where the report is being made.
struct Inspector<'a> {
    part: Option<&'a Disclosure>,
    needles: Vec<Needle>,
    files: Vec<ExposedFile>,
    entries: Vec<ExposedEntry>,
    findings: BTreeSet<Finding>,
}

/// What a file being read is.
struct Seen<'a> {
    path: &'a str,
    kind: FileKind,
    source: &'a str,
    /// What starts the line its hash leaves out, if any.
    varying: Option<&'a str>,
}

/// The SHA-256 of `bytes`, less the lines that start with `varying`, and what
/// that leaves out.
fn hash_of(bytes: &[u8], varying: Option<&str>) -> (String, Option<String>) {
    let Some(start) = varying else {
        return (sha256::hex(bytes), None);
    };
    let kept: Vec<&[u8]> = bytes
        .split_inclusive(|&byte| byte == b'\n')
        .filter(|line| !line.starts_with(start.as_bytes()))
        .collect();
    let said = format!("the line that starts `{start}`");
    (sha256::hex(&kept.concat()), Some(said))
}

impl Inspector<'_> {
    /// Adds a finding, if the game has a part to hold it to.
    fn found(
        &mut self,
        seen: &Seen,
        entry: Option<&str>,
        kind: FindingKind,
        canary: Option<usize>,
    ) {
        if self.part.is_some() {
            self.findings.insert(Finding {
                path: seen.path.to_owned(),
                entry: entry.map(str::to_owned),
                kind,
                source: seen.source.to_owned(),
                canary,
            });
        }
    }

    /// Looks for every needle in `bytes`, of `seen` or of its `entry`.
    fn scan(&mut self, seen: &Seen, entry: Option<&str>, bytes: &[u8]) {
        let program = entry.is_none()
            && matches!(
                seen.kind,
                FileKind::Program
                    | FileKind::DebugCompanion
                    | FileKind::BuildInput
                    | FileKind::Undeclared
            );
        let hits: Vec<(FindingKind, Option<usize>)> = self
            .needles
            .iter()
            .filter(|needle| program || !needle.programs_only)
            .filter(|needle| find(bytes, &needle.bytes, 0).is_some())
            .map(|needle| (needle.kind, needle.canary))
            .collect();
        for (kind, canary) in hits {
            self.found(seen, entry, kind, canary);
        }
    }

    /// Lists the pack `pack`, at `offset` in `seen`, and reads its files.
    fn pack(&mut self, seen: &Seen, offset: usize, pack: &Pack) {
        for file in pack.files() {
            self.entries.push(ExposedEntry {
                file: seen.path.to_owned(),
                offset: offset as u64,
                path: file.path.clone(),
                kinds: file.kinds.clone(),
                bytes: file.bytes.len() as u64,
                sha256: sha256::hex(&file.bytes),
            });
            let entry = Some(file.path.as_str());
            if let Some(part) = self.part {
                if part.forbidden_paths.contains(&file.path) {
                    self.found(seen, entry, FindingKind::ForbiddenPath, None);
                }
                if !part.allowed_pack_entries.contains(&file.path) {
                    self.found(seen, entry, FindingKind::UndeclaredPackEntry, None);
                }
            }
            self.scan(seen, entry, &file.bytes);
        }
    }

    /// Finds the packs in `bytes`, whole or embedded. An embedded pack of
    /// another version can't be loaded, and the magic bytes alone are in any
    /// program that reads packs: neither counts.
    fn packs(&mut self, seen: &Seen, bytes: &[u8]) {
        let mut from = 0;
        while let Some(at) = find(bytes, &pack::MAGIC, from) {
            from = at + pack::MAGIC.len();
            let here = &bytes[at..];
            let ours = here.get(8..12) == Some(&pack::VERSION.to_le_bytes()[..]);
            if at > 0 && !ours {
                continue;
            }
            match open_pack(here) {
                Ok((length, pack)) => {
                    self.pack(seen, at, &pack);
                    from = at + length;
                }
                Err(_) => self.found(seen, None, FindingKind::UnreadablePack, None),
            }
        }
    }

    /// Reads one file.
    fn file(&mut self, seen: &Seen, bytes: &[u8]) {
        let (sha256, varies) = hash_of(bytes, seen.varying);
        self.files.push(ExposedFile {
            path: seen.path.to_owned(),
            kind: seen.kind,
            source: seen.source.to_owned(),
            bytes: bytes.len() as u64,
            sha256,
            varies,
        });
        if let Some(part) = self.part {
            if part.forbidden_paths.iter().any(|path| path == seen.path) {
                self.found(seen, None, FindingKind::ForbiddenPath, None);
            }
            if seen.kind == FileKind::Undeclared {
                self.found(seen, None, FindingKind::UndeclaredFile, None);
            }
            if seen.kind == FileKind::LooseAsset
                && !part.allowed_files.iter().any(|f| f == seen.path)
            {
                self.found(seen, None, FindingKind::UndeclaredFile, None);
            }
            if seen.kind == FileKind::Beside {
                self.found(seen, None, FindingKind::Beside, None);
            }
        }
        self.scan(seen, None, bytes);
        self.packs(seen, bytes);
    }

    /// Reads the file at `disk`, unless it is over `limit` bytes.
    fn file_at(&mut self, seen: &Seen, disk: &Path, limit: u64) -> Result<Option<Vec<u8>>, String> {
        let size = fs::metadata(disk)
            .map_err(|e| format!("stat {}: {e}", disk.display()))?
            .len();
        if size > limit {
            self.found(seen, None, FindingKind::TooBig, None);
            return Ok(None);
        }
        let bytes = fs::read(disk).map_err(|e| format!("read {}: {e}", disk.display()))?;
        self.file(seen, &bytes);
        Ok(Some(bytes))
    }

    /// The kind and source of the package's file at `path`.
    fn known(&self, shipped: &Shipped, path: &str) -> (FileKind, String) {
        shipped.sources.get(path).cloned().unwrap_or_else(|| {
            let allowed = self
                .part
                .is_some_and(|part| part.allowed_files.iter().any(|file| file == path));
            let kind = if allowed {
                FileKind::LooseAsset
            } else {
                FileKind::Undeclared
            };
            (kind, NOT_MADE.to_owned())
        })
    }
}

/// The pack at the start of `bytes` and how many bytes it takes.
fn open_pack(bytes: &[u8]) -> Result<(usize, Pack<'_>), PackError> {
    let length = pack::length(bytes)?;
    let whole = &bytes[..length];
    let stamp = pack::info(whole)?.stamp;
    Ok((length, Pack::open(whole, &stamp)?))
}

/// The report of what `shipped`, in the export's folder `out`, holds.
pub(crate) fn inspect(
    out: &Path,
    part: Option<&Disclosure>,
    shipped: &Shipped,
    limit: u64,
    inputs: BuildInputs,
) -> Result<ExposureReport, String> {
    let mut inspector = Inspector {
        part,
        needles: needles(part),
        files: Vec::new(),
        entries: Vec::new(),
        findings: BTreeSet::new(),
    };
    // The folder's files, and their hashes for the archive's.
    let mut hashes = BTreeMap::new();
    for entry in archive::entries(&shipped.tree, "", &|_| false)? {
        let (kind, source) = inspector.known(shipped, &entry.name);
        let seen = Seen {
            path: &entry.name,
            kind,
            source: &source,
            varying: shipped.varying.get(&entry.name).map(String::as_str),
        };
        if let Some(bytes) = inspector.file_at(&seen, &entry.path, limit)? {
            hashes.insert(entry.name.clone(), sha256::hex(&bytes));
        }
    }
    let mut known: BTreeSet<PathBuf> = [out.join(EXPORT_REPORT), out.join(REPORT)].into();
    if let Some(file) = &shipped.archive {
        known.insert(file.clone());
        read_archive(&mut inspector, shipped, file, &hashes, limit)?;
    }
    if let Some(file) = &shipped.debug {
        known.insert(file.clone());
        let name = beside_name(out, file);
        let seen = Seen {
            path: &name,
            kind: FileKind::DebugCompanion,
            source: "the built program, with its debug sections",
            varying: None,
        };
        inspector.file_at(&seen, file, limit)?;
    }
    for (name, source, file) in &shipped.inputs {
        // Not every build has them all: what a build leaves out, it can't ship.
        if file.is_file() {
            let seen = Seen {
                path: name,
                kind: FileKind::BuildInput,
                source,
                varying: None,
            };
            inspector.file_at(&seen, file, limit)?;
        }
    }
    for entry in archive::entries(out, "", &|_| false)? {
        if !entry.path.starts_with(&shipped.tree) && !known.contains(&entry.path) {
            let seen = Seen {
                path: &entry.name,
                kind: FileKind::Beside,
                source: NOT_MADE,
                varying: None,
            };
            inspector.file_at(&seen, &entry.path, limit)?;
        }
    }
    let Inspector {
        mut files,
        mut entries,
        findings,
        ..
    } = inspector;
    files.sort_by(|a, b| (&a.path, a.kind).cmp(&(&b.path, b.kind)));
    entries.sort_by(|a, b| (&a.file, a.offset, &a.path).cmp(&(&b.file, b.offset, &b.path)));
    Ok(ExposureReport {
        ok: findings.is_empty(),
        declared: part.is_some(),
        inputs,
        files,
        pack_entries: entries,
        findings: findings.into_iter().collect(),
    })
}

/// `file`'s name from the export's folder `out`, with `/`.
fn beside_name(out: &Path, file: &Path) -> String {
    file.strip_prefix(out)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Reads the archive at `file` against the folder's `hashes`: what players
/// download is this, and a file only here gets all a file gets.
fn read_archive(
    inspector: &mut Inspector,
    shipped: &Shipped,
    file: &Path,
    hashes: &BTreeMap<String, String>,
    limit: u64,
) -> Result<(), String> {
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let bytes = fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let mut left: BTreeSet<&str> = hashes.keys().map(String::as_str).collect();
    // What the archive's hash is of when a file in it varies.
    let mut listing = String::new();
    let mut unpacked = 0;
    for (full, inner) in archive::read(file, limit)? {
        let path = full.strip_prefix(&shipped.prefix).unwrap_or(&full);
        left.remove(path);
        let (kind, from) = inspector.known(shipped, path);
        let inner_seen = Seen {
            path,
            kind,
            source: &from,
            varying: shipped.varying.get(path).map(String::as_str),
        };
        let _ = writeln!(listing, "{}  {path}", hash_of(&inner, inner_seen.varying).0);
        unpacked += inner.len() as u64;
        match hashes.get(path) {
            Some(hash) if *hash == sha256::hex(&inner) => {}
            Some(_) => {
                inspector.found(&inner_seen, None, FindingKind::ArchiveDiffers, None);
                inspector.scan(&inner_seen, None, &inner);
                inspector.packs(&inner_seen, &inner);
            }
            None => {
                inspector.found(&inner_seen, None, FindingKind::ArchiveDiffers, None);
                inspector.file(&inner_seen, &inner);
            }
        }
    }
    for path in left {
        let (kind, from) = inspector.known(shipped, path);
        let seen = Seen {
            path,
            kind,
            source: &from,
            varying: None,
        };
        inspector.found(&seen, None, FindingKind::ArchiveDiffers, None);
    }
    // Its own bytes are compressed: only its size and hash are of use here.
    let (size, sha256, varies) = if shipped.varying.is_empty() {
        (bytes.len() as u64, sha256::hex(&bytes), None)
    } else {
        // Compressed, a changed line changes the size as well as the bytes.
        let said = "its own bytes: the size is of its files unpacked, and the hash of a                     `<hash>  <path>` line for each";
        let hash = sha256::hex(listing.as_bytes());
        (unpacked, hash, Some(said.to_owned()))
    };
    inspector.files.push(ExposedFile {
        path: name.into_owned(),
        kind: FileKind::Archive,
        source: "the package's folder, zipped".to_owned(),
        bytes: size,
        sha256,
        varies,
    });
    Ok(())
}

#[cfg(test)]
mod tests;
