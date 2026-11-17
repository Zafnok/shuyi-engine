//! The kit: its manifest, `prepare`, which writes it, and `verify`, which
//! checks it anywhere.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::{Failed, Salts, text};
use crate::contract::codes;
use crate::project::{Project, Reveal};
use crate::sha256;

/// The manifest's file.
pub const MANIFEST: &str = "reveal.ron";
/// The seal: the manifest's SHA-256, as `sha256sum` writes it.
pub const SEAL: &str = "reveal.ron.sha256";
/// The evidence index, sealed through the manifest.
pub const INDEX: &str = "EVIDENCE.md";
/// The steps before the reveal: a working copy, never sealed.
pub const CHECKLIST: &str = "CHECKLIST.md";
/// Keeps git from changing the kit's line endings: never sealed.
pub const ATTRIBUTES: &str = ".gitattributes";
/// The captures' folder.
pub const CAPTURES: &str = "captures";
/// The receipts' folder.
pub const RECEIPTS: &str = "receipts";
/// The manifest's version. A kit of a newer one is refused, never misread.
pub const VERSION: u32 = 1;

/// What `.gitattributes` says: every file as bytes, so a checkout on
/// Windows changes none.
const ATTRIBUTES_TEXT: &str = "# Every file of the reveal kit is kept byte for byte.\n* -text\n";

/// The manifest, `reveal.ron`: what the kit holds and each file's SHA-256.
/// Its own SHA-256 is the kit's digest, in [`SEAL`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// [`VERSION`].
    pub version: u32,
    /// The game's name, as the public sees it.
    pub game: String,
    /// Words a copy's title or text might use.
    pub keywords: Vec<String>,
    /// The canonical store page.
    pub store_url: String,
    /// The day it was prepared, `YYYY-MM-DD`, by the preparer's clock: a
    /// claim, which a published digest dates.
    pub prepared: String,
    /// The build the captures came from.
    pub build: Build,
    /// The approved captures.
    pub captures: Vec<Capture>,
    /// The builds whose hashes it records; none for a store page alone.
    pub artifacts: Vec<Artifact>,
    /// The receipts, kept in whole.
    pub receipts: Vec<Kept>,
    /// The salted commitments' digests.
    pub commitments: Vec<Commitment>,
    /// The evidence index, [`INDEX`].
    pub index: Kept,
}

/// The build the captures came from: the game's version and its commit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// `shuyi.ron`'s `app.version`, if it says one.
    pub version: Option<String>,
    /// The game's git commit, if it is in git.
    pub commit: Option<String>,
    /// Whether its folder had changes not committed.
    pub dirty: bool,
}

impl Build {
    /// The build of the project at `root`: its version, and its commit if
    /// `git` is at hand and it is in git.
    pub fn of(root: &Path) -> Self {
        let git = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .ok()
                .filter(|out| out.status.success())
                .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        };
        let commit = git(&["rev-parse", "HEAD"]).filter(|commit| !commit.is_empty());
        let dirty = commit.is_some()
            && git(&["status", "--porcelain"]).is_some_and(|status| !status.is_empty());
        let version = Project::load(root)
            .ok()
            .and_then(|project| project.config.app.and_then(|app| app.version));
        Self {
            version,
            commit,
            dirty,
        }
    }
}

/// A file the kit holds, by its path in the kit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// Its path in the kit, with `/`.
    pub path: String,
    /// Its SHA-256.
    pub sha256: String,
    /// Its length.
    pub bytes: u64,
}

/// An approved capture: the original and each platform's re-encode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    /// What it shows.
    pub what: String,
    /// The original.
    pub original: Kept,
    /// The re-encodes.
    pub encodes: Vec<Kept>,
    /// The `frame-png` arguments that made it, if it was made so.
    pub args: Vec<String>,
}

/// A build the kit records by its hash, never holding it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// Its file's name.
    pub name: String,
    /// Its SHA-256.
    pub sha256: String,
    /// Its length, when the kit read the file (not from `SHA256SUMS`).
    pub bytes: Option<u64>,
}

/// A salted commitment's digest, opened later with the file and its salt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commitment {
    /// Its label (the file's path when it was made).
    pub label: String,
    /// The digest.
    pub sha256: String,
}

/// A kit just written.
#[derive(Debug, Clone)]
pub struct Prepared {
    /// Its manifest.
    pub manifest: Manifest,
    /// The kit's digest.
    pub digest: String,
    /// What it noticed, not wrong.
    pub notes: Vec<String>,
}

/// What [`verify`] found.
#[derive(Debug, Clone, Default)]
pub struct Verified {
    /// The manifest's SHA-256, if it could be read.
    pub digest: Option<String>,
    /// The manifest, if it read.
    pub manifest: Option<Manifest>,
    /// Each problem, naming its file; none when it verifies.
    pub problems: Vec<String>,
}

/// Today, `YYYY-MM-DD`, by this machine's clock (UTC).
pub fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let (y, m, d) = civil(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The day `days` after 1970-01-01, as year, month and day (Howard
/// Hinnant's `civil_from_days`).
pub(super) fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

/// "3 captures, no build, 2 receipts, 1 commitment".
pub fn counts(manifest: &Manifest) -> String {
    let count = |n: usize, one: &str, many: &str| match n {
        0 => format!("no {one}"),
        1 => format!("1 {one}"),
        n => format!("{n} {many}"),
    };
    format!(
        "{}, {}, {}, {}",
        count(manifest.captures.len(), "capture", "captures"),
        count(manifest.artifacts.len(), "build", "builds"),
        count(manifest.receipts.len(), "receipt", "receipts"),
        count(manifest.commitments.len(), "commitment", "commitments"),
    )
}

/// A file failure, naming `path`.
fn file_failed(path: &Path, what: &str, e: &std::io::Error) -> Failed {
    Failed::new(codes::FILE, format!("{what} {}: {e}", path.display()))
}

/// A setting of `shuyi.ron`'s `reveal` that is wrong.
fn setting(message: &str) -> Failed {
    Failed::new(
        codes::PROJECT_FILE,
        format!("{}'s `reveal`: {message}", crate::project::FILE),
    )
}

/// The name of the file at `path`, as the kit keeps it.
fn file_name(path: &str) -> Result<String, Failed> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| setting(&format!("`{path}` names no file")))
}

/// Copies `source` (from the project's root) into `folder` of the kit at
/// `dir`, refusing a name already taken, and returns its entry.
fn keep(
    root: &Path,
    dir: &Path,
    folder: &str,
    source: &str,
    taken: &mut BTreeSet<String>,
) -> Result<Kept, Failed> {
    let path = format!("{folder}/{}", file_name(source)?);
    if !taken.insert(path.clone()) {
        return Err(setting(&format!(
            "two files would be kept as `{path}`: rename one"
        )));
    }
    let from = root.join(source);
    let to = dir.join(&path);
    fs::copy(&from, &to).map_err(|e| file_failed(&from, "can't copy", &e))?;
    let (sha256, bytes) = sha256::file(&to).map_err(|e| file_failed(&to, "can't read", &e))?;
    Ok(Kept {
        path,
        sha256,
        bytes,
    })
}

/// The builds a release's `SHA256SUMS` lists, `name` naming it in errors.
fn sums(text: &str, name: &str) -> Result<Vec<Artifact>, Failed> {
    let mut artifacts = Vec::new();
    for (at, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed = line.split_once(' ').and_then(|(hash, rest)| {
            let name = rest.strip_prefix(' ').or_else(|| rest.strip_prefix('*'))?;
            (is_sha256(hash) && !name.is_empty()).then(|| Artifact {
                name: name.to_owned(),
                sha256: hash.to_ascii_lowercase(),
                bytes: None,
            })
        });
        artifacts.push(parsed.ok_or_else(|| {
            setting(&format!(
                "{name}, line {}: not `<sha256>  <file>` as `sha256sum` writes",
                at + 1
            ))
        })?);
    }
    Ok(artifacts)
}

/// Whether `text` is a SHA-256 in hexadecimal.
pub(super) fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Readies the folder `dir` for a new kit: makes it, or empties it if it
/// holds an older kit, and refuses any other folder that isn't empty.
fn ready(dir: &Path) -> Result<(), Failed> {
    if dir.join(MANIFEST).is_file() {
        fs::remove_dir_all(dir).map_err(|e| file_failed(dir, "can't empty", &e))?;
    } else if fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some()) {
        return Err(Failed::new(
            codes::FILE,
            format!(
                "{} holds files and no kit: name an empty folder (`reveal`'s `out`, or --out)",
                dir.display()
            ),
        ));
    }
    for folder in [CAPTURES, RECEIPTS] {
        let path = dir.join(folder);
        fs::create_dir_all(&path).map_err(|e| file_failed(&path, "can't make", &e))?;
    }
    Ok(())
}

/// Writes `bytes` to `path`.
fn write(path: &Path, bytes: &[u8]) -> Result<(), Failed> {
    fs::write(path, bytes).map_err(|e| file_failed(path, "can't write", &e))
}

/// Copies the captures of `reveal` into the kit at `dir`, each with the
/// `frame-png` arguments that made it: its own, or the `preview` part's for
/// `preview.png`.
fn captures_of(
    project: &Project,
    reveal: &Reveal,
    dir: &Path,
    taken: &mut BTreeSet<String>,
) -> Result<Vec<Capture>, Failed> {
    let root = &project.root;
    let preview_args = project
        .config
        .preview
        .as_ref()
        .map(|preview| preview.args.clone());
    let mut captures = Vec::new();
    for capture in &reveal.captures {
        let original = keep(root, dir, CAPTURES, &capture.file, taken)?;
        let encodes = capture
            .encodes
            .iter()
            .map(|encode| keep(root, dir, CAPTURES, encode, taken))
            .collect::<Result<Vec<_>, _>>()?;
        let mut args = capture.args.clone();
        if args.is_empty() && capture.file == crate::frame_png::preview::FILE {
            args = preview_args.clone().unwrap_or_default();
        }
        captures.push(Capture {
            what: capture.what.clone(),
            original,
            encodes,
            args,
        });
    }
    Ok(captures)
}

/// The builds `reveal` names, by hash, and the receipts it keeps (a
/// release's `SHA256SUMS` first), copied into the kit at `dir`.
fn builds_of(
    root: &Path,
    reveal: &Reveal,
    dir: &Path,
    taken: &mut BTreeSet<String>,
) -> Result<(Vec<Artifact>, Vec<Kept>), Failed> {
    let mut artifacts = Vec::new();
    for artifact in &reveal.artifacts {
        let path = root.join(artifact);
        let (sha256, bytes) =
            sha256::file(&path).map_err(|e| file_failed(&path, "can't read", &e))?;
        artifacts.push(Artifact {
            name: file_name(artifact)?,
            sha256,
            bytes: Some(bytes),
        });
    }
    let mut receipts = Vec::new();
    if let Some(sums_file) = &reveal.sums {
        let path = root.join(sums_file);
        let text = fs::read_to_string(&path).map_err(|e| file_failed(&path, "can't read", &e))?;
        artifacts.extend(sums(&text, sums_file)?);
        receipts.push(keep(root, dir, RECEIPTS, sums_file, taken)?);
    }
    for receipt in &reveal.receipts {
        receipts.push(keep(root, dir, RECEIPTS, receipt, taken)?);
    }
    Ok((artifacts, receipts))
}

/// Writes the kit of `project`'s `reveal` part into `dir`, prepared on
/// `today` from `build`.
///
/// # Errors
///
/// A setting that is wrong ([`codes::PROJECT_FILE`]), or a file it reads
/// or writes ([`codes::FILE`]).
pub fn prepare(
    project: &Project,
    dir: &Path,
    today: &str,
    build: &Build,
) -> Result<Prepared, Failed> {
    let reveal = project
        .part(&project.config.reveal, "reveal")
        .map_err(|e| Failed::new(codes::PROJECT_FILE, e))?;
    let game = game_name(project, reveal)?;
    check_url(&reveal.store_url)?;
    let root = &project.root;
    ready(dir)?;
    let mut taken = BTreeSet::new();
    let mut notes = Vec::new();
    let captures = captures_of(project, reveal, dir, &mut taken)?;
    if captures.is_empty() {
        notes.push("no capture: list the approved ones in `reveal`'s `captures`".to_owned());
    }
    let (artifacts, receipts) = builds_of(root, reveal, dir, &mut taken)?;
    let commitments = match &reveal.salts {
        Some(salts) => Salts::load(&root.join(salts))?
            .commitments
            .into_iter()
            .map(|salted| Commitment {
                label: salted.label,
                sha256: salted.sha256,
            })
            .collect(),
        None => Vec::new(),
    };

    let mut manifest = Manifest {
        version: VERSION,
        game,
        keywords: reveal.keywords.clone(),
        store_url: reveal.store_url.clone(),
        prepared: today.to_owned(),
        build: build.clone(),
        captures,
        artifacts,
        receipts,
        commitments,
        index: Kept::default(),
    };
    let index = text::evidence(&manifest);
    write(&dir.join(INDEX), index.as_bytes())?;
    manifest.index = Kept {
        path: INDEX.to_owned(),
        sha256: sha256::hex(index.as_bytes()),
        bytes: index.len() as u64,
    };
    let ron = to_ron(&manifest)?;
    write(&dir.join(MANIFEST), ron.as_bytes())?;
    let digest = sha256::hex(ron.as_bytes());
    write(&dir.join(SEAL), seal(&digest).as_bytes())?;
    write(
        &dir.join(CHECKLIST),
        text::checklist(&manifest, &digest).as_bytes(),
    )?;
    write(&dir.join(ATTRIBUTES), ATTRIBUTES_TEXT.as_bytes())?;
    Ok(Prepared {
        manifest,
        digest,
        notes,
    })
}

/// The game's name: the `reveal` part's, else `app`'s.
fn game_name(project: &Project, reveal: &Reveal) -> Result<String, Failed> {
    reveal
        .name
        .clone()
        .or_else(|| project.config.app.as_ref().map(|app| app.name.clone()))
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| setting("no `name`, and no `app` to take it from"))
}

/// Refuses a store page that isn't an `https://` address with a host.
fn check_url(url: &str) -> Result<(), Failed> {
    let host = url
        .strip_prefix("https://")
        .map(|rest| rest.split(['/', '?', '#']).next().unwrap_or_default());
    if host.is_some_and(|host| !host.is_empty()) && !url.contains(char::is_whitespace) {
        Ok(())
    } else {
        Err(setting(&format!(
            "`store_url` is `{url}`: give the store page's whole `https://` address"
        )))
    }
}

/// The manifest as RON, its lines ended by `\n` on every system.
fn to_ron(manifest: &Manifest) -> Result<String, Failed> {
    let pretty = ron::ser::PrettyConfig::default().new_line("\n");
    let mut text = ron::ser::to_string_pretty(manifest, pretty)
        .map_err(|e| Failed::new(codes::FILE, format!("can't write the manifest: {e}")))?;
    text.push('\n');
    Ok(text)
}

/// The seal's text for `digest`, as `sha256sum` writes it.
fn seal(digest: &str) -> String {
    format!("{digest}  {MANIFEST}\n")
}

/// Whether `path` is a path the manifest may name: relative, with `/`,
/// under the captures, the receipts or the index, and no `..`.
fn safe(path: &str) -> bool {
    let mut parts = path.split('/');
    let first = parts.next().unwrap_or_default();
    let rest: Vec<&str> = parts.collect();
    let plain =
        |part: &str| !part.is_empty() && part != "." && part != ".." && !part.contains(['\\', ':']);
    match first {
        INDEX => rest.is_empty(),
        CAPTURES | RECEIPTS => !rest.is_empty() && rest.iter().all(|part| plain(part)),
        _ => false,
    }
}

/// Every file under `dir`, by its path from `dir` with `/`.
fn files(dir: &Path, from: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, from, out);
        } else if let Ok(rel) = path.strip_prefix(from) {
            let parts: Vec<String> = rel
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(parts.join("/"));
        }
    }
}

/// Checks the kit in `dir`: the seal against the manifest (and against
/// `expect`, the digest published before, if given), the manifest's
/// version, every file it names, and that the folder holds nothing else.
pub fn verify(dir: &Path, expect: Option<&str>) -> Verified {
    let mut verified = Verified::default();
    let problems = &mut verified.problems;
    let manifest_path = dir.join(MANIFEST);
    let bytes = match fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            problems.push(format!("{MANIFEST}: can't read it ({e}): is this a kit?"));
            return verified;
        }
    };
    let digest = sha256::hex(&bytes);
    match fs::read_to_string(dir.join(SEAL)) {
        Ok(text) => {
            let sealed = text.split_whitespace().next().unwrap_or_default();
            if !sealed.eq_ignore_ascii_case(&digest) {
                problems.push(format!(
                    "{MANIFEST}: its SHA-256 is {digest}, and {SEAL} says {sealed}: the \
                     manifest changed after it was sealed"
                ));
            }
        }
        Err(e) => problems.push(format!("{SEAL}: can't read it ({e})")),
    }
    if let Some(expect) = expect
        && !expect.trim().eq_ignore_ascii_case(&digest)
    {
        problems.push(format!(
            "{MANIFEST}: its SHA-256 is {digest}, not the digest given, {}",
            expect.trim()
        ));
    }
    verified.digest = Some(digest);
    let Ok(text) = std::str::from_utf8(&bytes) else {
        verified
            .problems
            .push(format!("{MANIFEST}: not UTF-8 text"));
        return verified;
    };
    let manifest = match read_manifest(text) {
        Ok(manifest) => manifest,
        Err(e) => {
            verified.problems.push(format!("{MANIFEST}: {e}"));
            return verified;
        }
    };
    check_files(dir, &manifest, &mut verified.problems);
    verified.manifest = Some(manifest);
    verified
}

/// Reads a manifest, refusing one of a newer version by its version alone.
pub(super) fn read_manifest(text: &str) -> Result<Manifest, String> {
    #[derive(Deserialize)]
    struct Head {
        version: u32,
    }
    let head: Head = ron::from_str(text).map_err(|e| format!("can't read it: {e}"))?;
    if head.version > VERSION {
        return Err(format!(
            "version {} is newer than this shuyi reads ({VERSION}): verify it with a newer one",
            head.version
        ));
    }
    ron::from_str(text).map_err(|e| format!("can't read it: {e}"))
}

/// Checks each file `manifest` names in `dir`, and that `dir` holds no
/// other.
fn check_files(dir: &Path, manifest: &Manifest, problems: &mut Vec<String>) {
    let mut named = BTreeSet::new();
    let kept = manifest
        .captures
        .iter()
        .flat_map(|capture| std::iter::once(&capture.original).chain(&capture.encodes))
        .chain(&manifest.receipts)
        .chain(std::iter::once(&manifest.index));
    for entry in kept {
        if !safe(&entry.path) {
            problems.push(format!(
                "{MANIFEST}: names `{}`, not a path in the kit",
                entry.path
            ));
            continue;
        }
        if !named.insert(entry.path.clone()) {
            problems.push(format!("{MANIFEST}: names `{}` twice", entry.path));
            continue;
        }
        let path: PathBuf = dir.join(&entry.path);
        match sha256::file(&path) {
            Ok((sha256, bytes)) => {
                if !sha256.eq_ignore_ascii_case(&entry.sha256) {
                    problems.push(format!(
                        "{}: its SHA-256 is {sha256}, the manifest says {}",
                        entry.path, entry.sha256
                    ));
                } else if bytes != entry.bytes {
                    problems.push(format!(
                        "{}: it is {bytes} bytes, the manifest says {}",
                        entry.path, entry.bytes
                    ));
                }
            }
            Err(e) => problems.push(format!("{}: missing or unreadable ({e})", entry.path)),
        }
    }
    let mut found = Vec::new();
    files(dir, dir, &mut found);
    found.sort();
    for file in found {
        let unsealed = [MANIFEST, SEAL, CHECKLIST, ATTRIBUTES].contains(&file.as_str());
        if !unsealed && !named.contains(&file) {
            problems.push(format!(
                "{file}: in the folder, and not in the manifest: a kit holds only what it lists"
            ));
        }
    }
}
