//! A closed build's per-recipient overlay (ADR-0118): `export <target>
//! --release --recipient <id>` makes the ordinary, reproducible package
//! first (the public one, which a release attests), then a copy of it for one
//! recipient, in `dist/recipients/<id>/<target>`, whose program, pack and
//! picture carry the recipient's code:
//!
//! - (a) the program's field, which `shuyi_app::recipient` reads;
//! - (b) the pack's layout: its files' order and the bytes between them;
//! - (c) the lowest bits of the picture `shuyi.ron`'s `export.overlay_picture`
//!   names, in the pack, keeping its palette.
//!
//! Each holds the whole code with its check, so one mark that survives names
//! the recipient and two that agree say more. The copy is held to the game's
//! `disclosure` part as the public package is, and the ledger beside the
//! reveal kit, `recipients.ron`, gets an entry, signed with the game's SSH
//! key ([`ledger`]).
//!
//! `export --detect-recipient <file>` reads a copy that leaked (a package, a
//! folder, a program, a pack or a picture), says which marks it found and
//! the code each names, and whom the ledger gave that code.
//!
//! **A deterrent, never proof.** Every answer says a match is circumstantial
//! evidence. Two recipients who compare their copies find every mark and can
//! remove them; a repacked pack loses (b), a re-encoded or reduced picture
//! loses (c), and an overwritten field loses (a).

pub mod ledger;

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use shuyi_assets::image::{Image, PNG_SIGNATURE, mark};
use shuyi_assets::pack::{self, PackMark, Raw};
use shuyi_assets::recipient::{self as code, Code, FieldRead};

use super::disclosure::{self, Shipped};
use super::{ExportCx, ExportError, ExportReport, Exporter, archive};
use crate::contract::export::{FoundMark, MarkKind, RecipientCopy, RecipientFound};
use crate::project::{Disclosure, Project};
use crate::sha256;

/// The sentence every answer that names a code carries.
pub const CIRCUMSTANTIAL: &str = "A mark is circumstantial evidence, never proof: it says which \
                                  copy this was, not who leaked it, and two recipients who \
                                  compare their copies can find and remove every mark.";

/// The variable that names the SSH key the ledger is signed with, when
/// `--signing-key` doesn't.
pub const KEY_VARIABLE: &str = "SHUYI_SIGNING_KEY";

/// The folder, from the project's root, the recipients' copies go in.
pub const COPIES: &str = "dist/recipients";

/// The most bytes the detector reads of a package, unpacked.
const MAX_READ: u64 = 1 << 31;

/// What an overlay needs that the export's input gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    /// The recipient's id.
    pub recipient: String,
    /// Who they are, for the ledger.
    pub label: String,
    /// The SSH key the ledger is signed with.
    pub key: PathBuf,
}

/// Whether `id` can name a recipient: 1 to 64 letters, digits, `.`, `_` and
/// `-`, not starting with `.`.
pub fn plain_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The key `--signing-key` names (from `root`), else [`KEY_VARIABLE`]'s.
pub fn key_of(root: &Path, named: Option<&str>) -> Result<PathBuf, ExportError> {
    let named = named
        .map(str::to_owned)
        .or_else(|| std::env::var(KEY_VARIABLE).ok().filter(|v| !v.is_empty()));
    let Some(named) = named else {
        return Err(ExportError::Setup(format!(
            "--recipient signs the ledger: name the game's SSH private key with \
             --signing-key <file> or {KEY_VARIABLE} (a key kept outside the repository)"
        )));
    };
    let key = root.join(named);
    if key.is_file() {
        Ok(key)
    } else {
        Err(ExportError::Setup(format!(
            "the signing key {} isn't there",
            key.display()
        )))
    }
}

/// The reveal kit's folder of `project`: its `reveal` part's `out`, else
/// `dist/reveal`.
pub fn kit_of(project: &Project) -> PathBuf {
    let out = project
        .config
        .reveal
        .as_ref()
        .and_then(|reveal| reveal.out.clone())
        .unwrap_or_else(|| crate::reveal::DEFAULT_OUT.to_owned());
    project.root.join(out)
}

/// The SHA-256 of the package at `path`: a file's, or for a folder (a macOS
/// package) the SHA-256 of `sha256sum`'s lines for every file in it, by
/// path.
pub fn digest(path: &Path) -> Result<String, ExportError> {
    if path.is_file() {
        return sha256::file(path)
            .map(|(hex, _)| hex)
            .map_err(|e| ExportError::Build(format!("read {}: {e}", path.display())));
    }
    let entries = archive::entries(path, "", &|_| false).map_err(ExportError::Build)?;
    let mut lines = String::new();
    for entry in entries {
        let (hex, _) = sha256::file(&entry.path)
            .map_err(|e| ExportError::Build(format!("read {}: {e}", entry.path.display())))?;
        // Writing to a `String` can't fail.
        let _ = writeln!(lines, "{hex}  {}", entry.name);
    }
    Ok(sha256::hex(lines.as_bytes()))
}

/// Refuses an overlay `cx` can't have, before anything is built: a build
/// that isn't a release, or isn't packed, a target with no overlay, a game
/// that names no picture, a recipient's id that isn't plain, and a missing or
/// broken reveal kit.
pub fn refuse(
    cx: &ExportCx,
    exporter: &dyn Exporter,
    project: &Project,
    asked: &Asked,
) -> Result<(), ExportError> {
    if !plain_id(&asked.recipient) {
        return Err(ExportError::Setup(format!(
            "--recipient {}: an id is 1 to 64 letters, digits, `.`, `_` and `-`",
            asked.recipient
        )));
    }
    if !cx.release {
        return Err(ExportError::Setup(
            "--recipient marks a closed build's copy of a release: add --release".to_owned(),
        ));
    }
    if !exporter.overlays() {
        return Err(ExportError::Setup(format!(
            "--recipient: the {} package has no overlay (only windows, linux, macos and web)",
            exporter.id()
        )));
    }
    if !cx.packs() {
        return Err(ExportError::Setup(format!(
            "--recipient marks the pack beside the program: the package `{}` needs a `{}` \
             feature",
            cx.package,
            super::PACKED
        )));
    }
    if picture_of(project).is_none() {
        return Err(ExportError::Setup(format!(
            "--recipient marks one picture: name it in {}'s `export` part, `overlay_picture: \
             \"<its path in the pack>\"` (a picture of the game's, not its text)",
            crate::project::FILE
        )));
    }
    kit_digest(&kit_of(project)).map(drop)
}

/// The digest of the reveal kit in `kit`, once it verifies.
fn kit_digest(kit: &Path) -> Result<String, ExportError> {
    let verified = crate::reveal::verify(kit, None);
    match (&verified.digest, verified.problems.first()) {
        (Some(digest), None) => Ok(digest.clone()),
        (_, Some(problem)) => Err(ExportError::Setup(format!(
            "--recipient keeps its ledger beside the reveal kit, {}, which doesn't verify \
             ({problem}): run `shuyi reveal prepare` first",
            kit.display()
        ))),
        (None, None) => Err(ExportError::Setup(format!(
            "--recipient keeps its ledger beside the reveal kit, {}: run `shuyi reveal prepare` \
             first",
            kit.display()
        ))),
    }
}

/// The picture `project`'s `export` part names for the overlay.
fn picture_of(project: &Project) -> Option<&str> {
    project
        .config
        .export
        .as_ref()
        .and_then(|part| part.overlay_picture.as_deref())
}

/// The folder a recipient's copy of `cx` goes in, from the project's root.
fn copy_folder(cx: &ExportCx, recipient: &str) -> String {
    match &cx.edition {
        Some(edition) => format!("{COPIES}/{recipient}/{}/{}", edition.name, cx.target),
        None => format!("{COPIES}/{recipient}/{}", cx.target),
    }
}

/// The context of the copy: `cx` with its folder the copy's, made empty.
fn copy_cx(cx: &ExportCx, recipient: &str) -> Result<ExportCx, ExportError> {
    let mut copy = cx.clone();
    copy.shown = copy_folder(cx, recipient);
    copy.out = cx.root.join(&copy.shown);
    super::fresh(&copy.out)?;
    Ok(copy)
}

/// `report`, the base package's, as the copy in `copy` has it: the same
/// files from the copy's folder, no debug file (it stays with the base),
/// and the sizes the copy has.
fn copy_report(copy: &ExportCx, report: &ExportReport) -> Result<ExportReport, ExportError> {
    let mut out = report.clone();
    out.out = copy.shown.replace('\\', "/");
    out.debug = None;
    out.warnings = Vec::new();
    for size in out
        .sizes
        .iter_mut()
        .chain(&mut out.beside)
        .chain(&mut out.pack)
    {
        *size = copy.size(&copy.out.join(&size.file))?;
    }
    Ok(out)
}

/// Draws the code of `recipient`'s copy of the build whose digest is `build`:
/// signed by the key, so only its holder can draw it, and never one the
/// ledger gave someone else.
fn draw(
    key: &Path,
    recipient: &str,
    build: &str,
    ledger: Option<&ledger::Ledger>,
) -> Result<Code, ExportError> {
    for round in 0..16 {
        let Some(code) = Code::new(ledger::draw_code(key, recipient, build, round)?) else {
            continue;
        };
        let text = code.to_string();
        let taken = ledger.is_some_and(|ledger| {
            ledger
                .entries
                .iter()
                .any(|entry| entry.code == text && entry.recipient != recipient)
        });
        if !taken {
            return Ok(code);
        }
    }
    Err(ExportError::Build(
        "no free code after 16 draws: the ledger is broken".to_owned(),
    ))
}

/// Writes the code into the program at `program`: how many fields it wrote.
fn mark_program(program: &Path, code: Code) -> Result<usize, ExportError> {
    let mut bytes = fs::read(program)
        .map_err(|e| ExportError::Build(format!("read {}: {e}", program.display())))?;
    let written = code::write_field(&mut bytes, code);
    if written == 0 {
        return Err(ExportError::Build(format!(
            "{} has no recipient's field: build it with this engine's shuyi-app",
            program.display()
        )));
    }
    fs::write(program, &bytes)
        .map_err(|e| ExportError::Build(format!("write {}: {e}", program.display())))?;
    Ok(written)
}

/// The pixels of PNG `bytes`, as the engine decodes them.
fn pixels(bytes: &[u8]) -> Result<Image, String> {
    let (width, height, rgba) = shuyi_assets::bitmap::convert::png_pixels(bytes)?;
    Ok(Image {
        width,
        height,
        rgba,
    })
}

/// Marks the pack at `file`: the picture `picture` in it, then its layout.
fn mark_pack(file: &Path, picture: &str, code: Code) -> Result<(), ExportError> {
    let bytes =
        fs::read(file).map_err(|e| ExportError::Build(format!("read {}: {e}", file.display())))?;
    let mut raw =
        Raw::read(&bytes).map_err(|e| ExportError::Build(format!("{}: {e}", file.display())))?;
    let png = raw.file(picture).ok_or_else(|| {
        ExportError::Setup(format!(
            "`export.overlay_picture` names `{picture}`, which the pack doesn't keep as a \
             picture's own bytes"
        ))
    })?;
    let mut image = pixels(&png)
        .map_err(|e| ExportError::Setup(format!("`export.overlay_picture`, {picture}: {e}")))?;
    let before = mark::palette_count(&image);
    mark::mark(&mut image, code)
        .map_err(|e| ExportError::Setup(format!("`export.overlay_picture`, {picture}: {e}")))?;
    let encoded = crate::codec::encode_png(image.width, image.height, &image.rgba)
        .map_err(ExportError::Build)?;
    // Read back as the engine reads it: the mark and the palette held.
    let again = pixels(&encoded).map_err(ExportError::Build)?;
    if mark::read(&again) != mark::PictureRead::Code(code) || mark::palette_count(&again) != before
    {
        return Err(ExportError::Build(format!(
            "{picture}: the mark didn't read back from the PNG"
        )));
    }
    raw.replace(picture, &encoded);
    let marked = raw
        .write_marked(code)
        .ok_or_else(|| ExportError::Build(format!("{}: the pack has no file", file.display())))?;
    let read = Raw::read(&marked).map(|raw| raw.mark());
    if read
        != Ok(PackMark::Code {
            code,
            order_agrees: true,
        })
    {
        return Err(ExportError::Build(format!(
            "{}: the layout didn't read back",
            file.display()
        )));
    }
    fs::write(file, &marked)
        .map_err(|e| ExportError::Build(format!("write {}: {e}", file.display())))
}

/// Makes `asked.recipient`'s copy of the package `report` names, which `cx`
/// exported with `exporter`: copied, marked, packaged again, held to
/// `part`, and written in the ledger.
pub fn overlay(
    cx: &ExportCx,
    exporter: &dyn Exporter,
    project: &Project,
    part: Option<&Disclosure>,
    report: &ExportReport,
    asked: &Asked,
) -> Result<RecipientCopy, (crate::contract::codes::Code, String)> {
    let failed = |e: ExportError| (e.code(), e.message());
    let picture = picture_of(project).unwrap_or_default().to_owned();
    let kit = kit_of(project);
    let kit_digest = kit_digest(&kit).map_err(failed)?;
    let ledger_path = ledger::path_beside(&kit);
    let mut ledger = ledger::load(&ledger_path).map_err(failed)?;
    let signer = ledger::public_key(&asked.key).map_err(failed)?;
    if let Some(kept) = &ledger {
        if kept.signer != signer {
            return Err(failed(ExportError::Setup(format!(
                "{} is signed by another key ({}): sign with that one",
                ledger_path.display(),
                kept.signer
            ))));
        }
        if ledger::check(&ledger_path, kept) != Some(true) {
            return Err(failed(ExportError::Setup(format!(
                "{} doesn't verify against {}: it was changed after it was signed",
                ledger_path.display(),
                ledger::SIGNATURE
            ))));
        }
    }
    let build = digest(&cx.out.join(&report.package)).map_err(failed)?;
    let code = draw(&asked.key, &asked.recipient, &build, ledger.as_ref()).map_err(failed)?;
    let copy = copy_cx(cx, &asked.recipient).map_err(failed)?;
    let folder = copy.out.join(&report.folder);
    crate::web::copy_tree(&cx.out.join(&report.folder), &folder, &|_| true)
        .map_err(|e| failed(ExportError::Build(e)))?;
    let program = report
        .sizes
        .get(1)
        .map(|size| copy.out.join(&size.file))
        .ok_or_else(|| failed(ExportError::Build("the report names no program".to_owned())))?;
    let fields = mark_program(&program, code).map_err(failed)?;
    let pack_file = report
        .pack
        .as_ref()
        .map(|pack| copy.out.join(&pack.file))
        .ok_or_else(|| failed(ExportError::Build("the report names no pack".to_owned())))?;
    mark_pack(&pack_file, &picture, code).map_err(failed)?;
    exporter.repackage(&copy, report).map_err(failed)?;
    let copy_report = copy_report(&copy, report).map_err(failed)?;
    let mut shipped: Shipped = exporter.shipped(&copy, &copy_report);
    // The web build's folder is the public package's, not the copy's.
    shipped.public.clear();
    disclosure::gate(&copy, exporter, part, &shipped)?;
    let overlay = digest(&copy.out.join(&copy_report.package)).map_err(failed)?;
    let json = serde_json::to_string_pretty(&copy_report).unwrap_or_default();
    let written = copy.out.join(crate::contract::export::REPORT);
    fs::write(&written, format!("{json}\n")).map_err(|e| {
        failed(ExportError::Build(format!(
            "write {}: {e}",
            written.display()
        )))
    })?;
    let entry = ledger::Entry {
        recipient: asked.recipient.clone(),
        label: asked.label.clone(),
        date: crate::reveal::today(),
        code: code.to_string(),
        target: cx.target.clone(),
        edition: cx.edition.as_ref().map(|edition| edition.name.clone()),
        version: cx.version.clone(),
        build: build.clone(),
        overlay: overlay.clone(),
        kit: kit_digest,
    };
    let mut kept = ledger.take().unwrap_or_else(|| ledger::Ledger {
        about: ledger::ABOUT.to_owned(),
        signer,
        entries: Vec::new(),
    });
    let same = |old: &ledger::Entry| {
        old.recipient == entry.recipient && old.build == entry.build && old.overlay == entry.overlay
    };
    if !kept.entries.iter().any(same) {
        kept.entries.push(entry);
    }
    ledger::save(&ledger_path, &kept, &asked.key).map_err(failed)?;
    let program_mark = if fields > 1 {
        format!("program: its {fields} fields (one in each slice)")
    } else {
        "program: its field".to_owned()
    };
    let marks = vec![
        program_mark,
        "pack: the order of its files and the bytes between them".to_owned(),
        format!("picture: {picture}'s lowest bits, its palette kept"),
    ];
    Ok(RecipientCopy {
        recipient: asked.recipient.clone(),
        code: code.to_string(),
        out: copy.shown.replace('\\', "/"),
        package: copy_report.package,
        build,
        overlay,
        marks,
        ledger: shown_path(&project.root, &ledger_path),
        note: CIRCUMSTANTIAL.to_owned(),
    })
}

/// `path` from `root`, with `/`.
fn shown_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The files to read for `path`: a folder's, an archive's, or the file.
fn files_of(path: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    if path.is_dir() {
        let entries = archive::entries(path, "", &|_| false)?;
        return entries
            .into_iter()
            .map(|entry| {
                fs::read(&entry.path)
                    .map(|bytes| (entry.name, bytes))
                    .map_err(|e| format!("read {}: {e}", entry.path.display()))
            })
            .collect();
    }
    let name = path.to_string_lossy();
    if name.ends_with(".zip") || name.ends_with(".tar.gz") {
        return archive::read(path, MAX_READ);
    }
    fs::read(path)
        .map(|bytes| {
            let shown = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            vec![(shown, bytes)]
        })
        .map_err(|e| format!("read {}: {e}", path.display()))
}

/// What the picture `bytes` (`name`) holds, unless it is the game's own
/// picture `own` (its pixels the same): then no mark.
fn picture_mark(name: &str, bytes: &[u8], own: Option<&Image>) -> FoundMark {
    let found = |code: Option<Code>, said: String| FoundMark {
        mark: MarkKind::Picture,
        file: name.to_owned(),
        code: code.map(|code| code.to_string()),
        said,
    };
    let image = match pixels(bytes) {
        Ok(image) => image,
        Err(e) => return found(None, format!("a picture that doesn't decode: {e}")),
    };
    if own.is_some_and(|own| *own == image) {
        return found(
            None,
            "no mark: the game's own picture, unchanged".to_owned(),
        );
    }
    match mark::read(&image) {
        mark::PictureRead::Code(code) => found(
            Some(code),
            "its lowest bits hold a code, its check right".to_owned(),
        ),
        mark::PictureRead::NoMark => {
            found(None, "no mark: its lowest bits hold no code".to_owned())
        }
        mark::PictureRead::TooFewColors { found: n } => found(
            None,
            format!(
                "no mark: {n} color(s) can carry one, fewer than a code needs (cropped, or its colors reduced)"
            ),
        ),
    }
}

/// What the pack `bytes` (`name`) holds: its layout's mark, and the
/// picture's in it.
fn pack_marks(
    name: &str,
    bytes: &[u8],
    picture: Option<&str>,
    own: Option<&Image>,
) -> Vec<FoundMark> {
    let raw = match Raw::read(bytes) {
        Ok(raw) => raw,
        Err(e) => {
            return vec![FoundMark {
                mark: MarkKind::Pack,
                file: name.to_owned(),
                code: None,
                said: format!("a pack that doesn't read: {e}"),
            }];
        }
    };
    let (code, said) = match raw.mark() {
        PackMark::None => (None, "no mark: its files in path order, nothing between them".to_owned()),
        PackMark::Code { code, order_agrees: true } => (
            Some(code),
            "the bytes between its files hold a code, its check right, and its files are in the order it gives".to_owned(),
        ),
        PackMark::Code { code, order_agrees: false } => (
            Some(code),
            "the bytes between its files hold a code, its check right; its files are not in the order it gives".to_owned(),
        ),
        PackMark::Broken => (None, "no mark: its layout holds no code (changed after it was laid out)".to_owned()),
    };
    let mut marks = vec![FoundMark {
        mark: MarkKind::Pack,
        file: name.to_owned(),
        code: code.map(|code| code.to_string()),
        said,
    }];
    let pictures: Vec<String> = match picture {
        Some(picture) => vec![picture.to_owned()],
        None => raw
            .paths()
            .filter(|path| {
                Path::new(path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            })
            .map(str::to_owned)
            .collect(),
    };
    for path in pictures {
        if let Some(png) = raw.file(&path) {
            marks.push(picture_mark(&format!("{name}: {path}"), &png, own));
        }
    }
    marks
}

/// What the program `bytes` (`name`) holds in its fields, if it has any.
fn program_marks(name: &str, bytes: &[u8]) -> Vec<FoundMark> {
    code::read_fields(bytes)
        .into_iter()
        .map(|(at, read)| {
            let (code, said) = match read {
                FieldRead::Code(code) => (
                    Some(code),
                    format!("its field at byte {at} holds a code, twice, its check right"),
                ),
                FieldRead::Empty => (
                    None,
                    format!("no mark: its field at byte {at} holds no code (a public build's)"),
                ),
                FieldRead::Broken => (None, format!("no mark: its field at byte {at} was changed")),
            };
            FoundMark {
                mark: MarkKind::Program,
                file: name.to_owned(),
                code: code.map(|code| code.to_string()),
                said,
            }
        })
        .collect()
}

/// `export --detect-recipient <file>` on `project`: every mark in the file,
/// the codes they name and whom the ledger gave them.
pub fn detect(project: &Project, file: &str) -> Result<RecipientFound, ExportError> {
    let path = project.root.join(file);
    let files = files_of(&path).map_err(ExportError::Setup)?;
    let picture = picture_of(project);
    let own = picture.and_then(|picture| {
        let assets = project.config.assets.as_deref()?;
        let bytes = fs::read(project.root.join(assets).join(picture)).ok()?;
        pixels(&bytes).ok()
    });
    let mut marks = Vec::new();
    for (name, bytes) in &files {
        if bytes.starts_with(&pack::MAGIC) {
            marks.extend(pack_marks(name, bytes, picture, own.as_ref()));
        } else if bytes.starts_with(PNG_SIGNATURE) {
            marks.push(picture_mark(name, bytes, own.as_ref()));
        } else {
            marks.extend(program_marks(name, bytes));
        }
    }
    let mut codes: Vec<String> = marks.iter().filter_map(|mark| mark.code.clone()).collect();
    codes.sort();
    codes.dedup();
    let ledger_path = ledger::path_beside(&kit_of(project));
    let ledger = ledger::load(&ledger_path).ok().flatten();
    let signed = ledger
        .as_ref()
        .and_then(|ledger| ledger::check(&ledger_path, ledger));
    let recipients = codes
        .iter()
        .filter_map(|code| {
            ledger
                .as_ref()?
                .entries
                .iter()
                .find(|entry| &entry.code == code)
                .cloned()
        })
        .map(Into::into)
        .collect();
    let verdict = verdict(&marks, &codes);
    Ok(RecipientFound {
        file: file.replace('\\', "/"),
        marks,
        codes,
        recipients,
        ledger: ledger
            .is_some()
            .then(|| shown_path(&project.root, &ledger_path)),
        signed,
        verdict,
        note: CIRCUMSTANTIAL.to_owned(),
    })
}

/// The verdict on `marks`, whose distinct codes are `codes`.
fn verdict(marks: &[FoundMark], codes: &[String]) -> String {
    let naming = marks.iter().filter(|mark| mark.code.is_some()).count();
    match codes {
        [] if marks.is_empty() => {
            "no mark found: the file holds no program field, pack or picture".to_owned()
        }
        [] => "no mark found".to_owned(),
        [code] => format!(
            "{naming} of {} mark(s) name the code {code}: circumstantial evidence, never proof",
            marks.len()
        ),
        _ => format!(
            "the marks name {} different codes ({}): a copy changed, or copies mixed; \
             circumstantial evidence, never proof",
            codes.len(),
            codes.join(", ")
        ),
    }
}

#[cfg(test)]
mod tests;
