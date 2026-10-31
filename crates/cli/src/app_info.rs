//! Who the game is: `shuyi.ron`'s `app` part ([`AppPart`], ADR-0059), its
//! rules ([`problems`], which `check --target` reports), and `app-info`,
//! which writes it as the Rust constant the game's binary compiles
//! ([`rust`], [`run`]) and the window's icons, downscaled from the game's
//! one icon ([`downscale`], ADR-0061).
//!
//! The rules:
//!
//! - `name`: not empty, with no control character;
//! - `id`: reverse-DNS, two or more parts between dots, each of ASCII
//!   letters, digits and `-`, starting with a letter; an id under
//!   `com.example.` is a warning, since two games with it share their
//!   saves;
//! - `version`, when given: semver, `MAJOR.MINOR.PATCH` then an optional
//!   `-pre-release` and `+build`;
//! - `build`: 1 or more;
//! - `storage`, or the id when it is unset: one folder name;
//! - `icon`, when given: a PNG file, square, [`ICON_SIDE`] pixels a side
//!   or more;
//! - `window_icons`: set when `icon` and `code` are, since the Rust reads
//!   the window's icons from there;
//! - the files `app-info` writes (`code` and the window's icons) are the
//!   ones on disk.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::codec;
use crate::contract::target::Level;
use crate::project::{AppPart, FILE};

/// What `app-info` and the rules call the part.
const PART: &str = "app";

/// The id's start that marks it as a placeholder.
pub const PLACEHOLDER: &str = "com.example.";

/// The smallest side of the game's icon, in pixels: every size a target
/// asks is downscaled from it, never scaled up.
pub const ICON_SIDE: u32 = 512;

/// The window's icons' sides, in pixels: `WindowIcon`'s small, medium and
/// big.
pub const WINDOW_ICON_SIDES: [u32; 3] = [16, 32, 64];

/// A problem of the `app` part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppProblem {
    /// An error or a warning.
    pub level: Level,
    /// The field it is about: `app.id`.
    pub setting: String,
    /// What is wrong.
    pub message: String,
}

impl AppProblem {
    fn new(level: Level, field: &str, message: impl Into<String>) -> Self {
        Self {
            level,
            setting: format!("{PART}.{field}"),
            message: message.into(),
        }
    }

    fn error(field: &str, message: impl Into<String>) -> Self {
        Self::new(Level::Error, field, message)
    }
}

/// Every problem of `part`, in the project at `root` whose tool is called
/// `command`: its fields and its icon by the rules, and each file
/// `app-info` writes that isn't the one on disk.
pub fn problems(part: &AppPart, root: &Path, command: &str) -> Vec<AppProblem> {
    let mut out = checked(part, root);
    for path in stale(part, root, command) {
        let field = if is_png(&path) {
            "window_icons"
        } else {
            "code"
        };
        out.push(AppProblem::error(
            field,
            format!("{path} is out of date: run `{command} app-info`"),
        ));
    }
    out
}

/// The problems of `part`'s fields, without reading a file.
fn fields(part: &AppPart) -> Vec<AppProblem> {
    let mut out = Vec::new();
    if let Err(e) = check_name(&part.name) {
        out.push(AppProblem::error("name", e));
    }
    match check_id(&part.id) {
        Err(e) => out.push(AppProblem::error("id", e)),
        Ok(()) if part.id.starts_with(PLACEHOLDER) => out.push(AppProblem::new(
            Level::Warning,
            "id",
            format!(
                "\"{}\" is a placeholder: two games with this id share their saves on one \
                 computer and on one web origin; set your own before you hand out a build, and \
                 before the first release, since the save folder follows it",
                part.id
            ),
        )),
        Ok(()) => {}
    }
    if let Some(version) = &part.version
        && let Err(e) = check_version(version)
    {
        out.push(AppProblem::error("version", e));
    }
    if part.build == 0 {
        out.push(AppProblem::error("build", "the build is 0: it starts at 1"));
    }
    let field = if part.storage.is_some() {
        "storage"
    } else {
        "id"
    };
    if let Err(e) = check_storage(part.storage()) {
        out.push(AppProblem::error(field, e));
    }
    if part.icon.is_some() && part.code.is_some() && part.window_icons.is_none() {
        out.push(AppProblem::error(
            "window_icons",
            "the icon and `code` are set, so `window_icons` must be too: the folder where \
             `app-info` writes the window's icons, which `code` reads",
        ));
    }
    out
}

/// The problems of `part`'s fields, and of its icon, read from `root`.
fn checked(part: &AppPart, root: &Path) -> Vec<AppProblem> {
    let mut out = fields(part);
    if let Err(e) = icon(part, root) {
        out.push(AppProblem::error("icon", e));
    }
    let launcher = [
        ("playdate_icon", &part.playdate_icon, (32, 32)),
        ("playdate_card", &part.playdate_card, (350, 155)),
    ];
    for (field, path, size) in launcher {
        if let Some(path) = path
            && let Err(e) = sized_png(root, path, size)
        {
            out.push(AppProblem::error(field, e));
        }
    }
    out
}

/// Whether `path` in the project at `root` is a PNG of `size` pixels, the
/// size the device's launcher asks.
fn sized_png(root: &Path, path: &str, size: (u32, u32)) -> Result<(), String> {
    let bytes = fs::read(root.join(path)).map_err(|e| format!("{path} can't be read: {e}"))?;
    let (width, height, _) =
        codec::decode_png(&bytes).map_err(|e| format!("{path} isn't a PNG file: {e}"))?;
    if (width, height) == size {
        Ok(())
    } else {
        Err(format!(
            "{path} is {width} × {height} pixels: it must be {} × {}",
            size.0, size.1
        ))
    }
}

/// The game's icon: `side` × `side` RGBA8 pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    /// Its width and height, in pixels.
    pub side: u32,
    /// Its pixels, row by row, four bytes each.
    pub rgba: Vec<u8>,
}

/// `part`'s icon, read from the project at `root` and checked: a PNG
/// file, square, [`ICON_SIDE`] pixels a side or more. `None` when the part
/// names none.
pub fn icon(part: &AppPart, root: &Path) -> Result<Option<Icon>, String> {
    let Some(path) = &part.icon else {
        return Ok(None);
    };
    let bytes = fs::read(root.join(path)).map_err(|e| format!("{path} can't be read: {e}"))?;
    let (width, height, rgba) =
        codec::decode_png(&bytes).map_err(|e| format!("{path} isn't a PNG file: {e}"))?;
    if width != height {
        return Err(format!(
            "{path} is {width} × {height} pixels: an icon is square"
        ));
    }
    if width < ICON_SIDE {
        return Err(format!(
            "{path} is {width} pixels a side: an icon is {ICON_SIDE} or more, so every size a \
             target asks is made from it by downscaling"
        ));
    }
    Ok(Some(Icon { side: width, rgba }))
}

/// `rgba`, a `width` × `height` picture of RGBA8 pixels, downscaled to
/// `size` × `size`, in integers, so the same picture gives the same bytes
/// everywhere (ADR-0061). Each pixel out is the mean of the pixels of its
/// square in, from `x * width / size` up to `(x + 1) * width / size` (and
/// the same down), in premultiplied alpha, so a transparent pixel adds
/// nothing to the colour beside it; each mean is rounded half up.
pub fn downscale(width: u32, height: u32, rgba: &[u8], size: u32) -> Vec<u8> {
    let bounds = |at: u32, whole: u32| {
        let from = u64::from(at) * u64::from(whole) / u64::from(size);
        let to = (u64::from(at) + 1) * u64::from(whole) / u64::from(size);
        (from, to.max(from + 1))
    };
    let mut out = Vec::with_capacity(4 * size as usize * size as usize);
    for y in 0..size {
        let (top, bottom) = bounds(y, height);
        for x in 0..size {
            let (left, right) = bounds(x, width);
            // The sum of alpha, and of each colour times its alpha.
            let (mut alpha, mut colour) = (0_u64, [0_u64; 3]);
            for row in top..bottom {
                for column in left..right {
                    let at = usize::try_from(4 * (row * u64::from(width) + column))
                        .unwrap_or(usize::MAX);
                    let Some(pixel) = rgba.get(at..at + 4) else {
                        continue;
                    };
                    let a = u64::from(pixel[3]);
                    alpha += a;
                    for (sum, &c) in colour.iter_mut().zip(pixel) {
                        *sum += u64::from(c) * a;
                    }
                }
            }
            let count = (bottom - top) * (right - left);
            for sum in colour {
                out.push(if alpha == 0 { 0 } else { half_up(sum, alpha) });
            }
            out.push(half_up(alpha, count));
        }
    }
    out
}

/// `sum / count`, rounded half up, as a byte: the mean of bytes is one.
fn half_up(sum: u64, count: u64) -> u8 {
    u8::try_from((2 * sum + count) / (2 * count)).unwrap_or(u8::MAX)
}

/// Whether `name` can be the name players see: not empty, with no
/// control character.
fn check_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("the name is empty".to_owned());
    }
    match name.chars().find(|c| c.is_control()) {
        Some(c) => Err(format!("the name may not hold {c:?}")),
        None => Ok(()),
    }
}

/// Whether `id` is a reverse-DNS id: two or more parts between dots, each
/// of ASCII letters, digits and `-`, starting with a letter. These are the
/// characters Apple allows in a bundle id, with a letter first as Android
/// asks.
pub fn check_id(id: &str) -> Result<(), String> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return Err(format!(
            "\"{id}\" isn't reverse-DNS: it needs two or more parts between dots, as \
             com.example.moon-garden"
        ));
    }
    for part in parts {
        if !part.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return Err(format!(
                "\"{id}\": each part between dots starts with a letter, and \"{part}\" doesn't"
            ));
        }
        if let Some(c) = part
            .chars()
            .find(|&c| !(c.is_ascii_alphanumeric() || c == '-'))
        {
            return Err(format!(
                "\"{id}\" may hold only ASCII letters, digits, `-` and dots, not {c:?}"
            ));
        }
    }
    Ok(())
}

/// Whether `version` is semver: `MAJOR.MINOR.PATCH` in digits, then an
/// optional `-pre-release` and `+build`.
pub fn check_version(version: &str) -> Result<(), String> {
    semver::Version::parse(version).map(|_| ()).map_err(|e| {
        format!("\"{version}\" isn't semver (MAJOR.MINOR.PATCH, as 1.2.0 or 1.2.0-beta.1): {e}")
    })
}

/// Whether `storage` can name a folder on every system: ASCII letters,
/// digits, `.`, `-`, `_` and spaces, with no `.` or space first or last.
pub fn check_storage(storage: &str) -> Result<(), String> {
    if storage.is_empty() {
        return Err("the storage's name is empty".to_owned());
    }
    if let Some(c) = storage
        .chars()
        .find(|&c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ')))
    {
        return Err(format!(
            "the storage's name \"{storage}\" names a folder: ASCII letters, digits, `.`, `-`, \
             `_` and spaces only, not {c:?}"
        ));
    }
    let ends = [storage.chars().next(), storage.chars().last()];
    if ends.iter().flatten().any(|&c| matches!(c, '.' | ' ')) {
        return Err(format!(
            "the storage's name \"{storage}\" names a folder: no `.` or space first or last"
        ));
    }
    Ok(())
}

/// `text` as a Rust string literal: in quotes, with only `"`, `\` and
/// control characters escaped.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            // Writing to a `String` can't fail.
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The Rust file `app-info` writes for `part`, with `command` as the
/// tool's name: one `pub const APP: AppInfo`, kept from `rustfmt`, so
/// `app-info --check` is its only judge.
pub fn rust(part: &AppPart, command: &str) -> String {
    let version = part
        .version
        .as_deref()
        .map_or_else(|| "env!(\"CARGO_PKG_VERSION\")".to_owned(), quoted);
    let mut app = format!(
        "AppInfo::new({}, {}, {version})",
        quoted(&part.name),
        quoted(&part.id)
    );
    if let Some(storage) = &part.storage {
        let _ = write!(app, "\n    .with_storage({})", quoted(storage));
    }
    if part.build != 1 {
        let _ = write!(app, "\n    .with_build({})", part.build);
    }
    if !part.publisher.is_empty() {
        let _ = write!(app, "\n    .with_publisher({})", quoted(&part.publisher));
    }
    let mut uses = "AppInfo";
    if let Some(paths) = window_icon_paths(part) {
        uses = "{AppInfo, WindowIcon}";
        let code = part.code.as_deref().unwrap_or_default();
        app.push_str("\n    .with_window_icon(Some(WindowIcon::new(");
        for (_, path) in paths {
            let _ = write!(
                app,
                "\n        include_bytes!({}),",
                quoted(&relative(code, &path))
            );
        }
        app.push_str("\n    )))");
    }
    format!(
        "// Generated by `{command} {COMMAND}` from {FILE}'s `{PART}`. Do not edit.\n\
         //! Who the game is, as `{FILE}`'s `{PART}` says: its name, id and version, and the\n\
         //! name its saves are kept under.\n\
         \n\
         use shuyi::platform::{uses};\n\
         \n\
         /// Who the game is: the window's title, the loading card and the storage read it.\n\
         // check-text: not player text, who the game is (`{FILE}`'s `{PART}`)\n\
         #[rustfmt::skip]\n\
         pub const APP: AppInfo = {app};\n"
    )
}

/// The command's name.
pub const COMMAND: &str = "app-info";

/// The window's icons `app-info` writes for `part`, each side and path from
/// the project's root (`<window_icons>/16.png`, …): `None` unless the part
/// has an icon and `window_icons`.
fn window_icon_paths(part: &AppPart) -> Option<[(u32, String); 3]> {
    part.icon.as_ref()?;
    let folder = part.window_icons.as_deref()?.trim_end_matches(['/', '\\']);
    Some(WINDOW_ICON_SIDES.map(|side| (side, format!("{folder}/{side}.png"))))
}

/// Whether `path`, a file `app-info` writes, is a picture: one of the
/// window's icons, not the Rust file.
fn is_png(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
}

/// `to`, a path from the project's root, as a path from the folder of
/// `from`, another, with `/` between its parts: what `include_bytes!` in
/// `from` names `to` by.
fn relative(from: &str, to: &str) -> String {
    let parts = |path: &str| -> Vec<String> {
        path.split(['/', '\\'])
            .filter(|part| !part.is_empty() && *part != ".")
            .map(str::to_owned)
            .collect()
    };
    let mut folder = parts(from);
    folder.pop();
    let to = parts(to);
    let shared = folder.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut out = vec!["..".to_owned(); folder.len() - shared];
    out.extend_from_slice(&to[shared..]);
    out.join("/")
}

/// The Rust file `app-info` writes for `part`, its path and its bytes.
fn rust_file(part: &AppPart, command: &str) -> Option<(String, Vec<u8>)> {
    let path = part.code.clone()?;
    Some((path, rust(part, command).into_bytes()))
}

/// The files `app-info` writes for `part` in the project at `root`: each
/// path from the root and its bytes, the Rust file and then the window's
/// icons, downscaled from the icon. Fails when the icon doesn't load.
pub fn files(part: &AppPart, root: &Path, command: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out: Vec<_> = rust_file(part, command).into_iter().collect();
    if let Some(paths) = window_icon_paths(part)
        && let Some(icon) = icon(part, root)?
    {
        for (side, path) in paths {
            let pixels = downscale(icon.side, icon.side, &icon.rgba, side);
            out.push((path, codec::encode_png(side, side, &pixels)?));
        }
    }
    Ok(out)
}

/// The files `app-info` would write for `part` that differ from those in
/// `root` (a missing file differs), by their paths. A text file's line
/// endings don't count. When the icon doesn't load, the window's icons
/// aren't compared: the icon's rule says why.
pub fn stale(part: &AppPart, root: &Path, command: &str) -> Vec<String> {
    let files = files(part, root, command)
        .unwrap_or_else(|_| rust_file(part, command).into_iter().collect());
    files
        .into_iter()
        .filter(|(path, bytes)| {
            fs::read(root.join(path)).map_or(true, |disk| {
                if is_png(path) {
                    return disk != *bytes;
                }
                let lf: Vec<u8> = disk.into_iter().filter(|&b| b != b'\r').collect();
                lf != *bytes
            })
        })
        .map(|(path, _)| path)
        .collect()
}

/// `app-info [--check]` on `part` in the project at `root`: writes its
/// files, or with `check` writes nothing and fails naming each file that
/// differs. A part with an error writes nothing and fails with them.
pub fn run(part: &AppPart, root: &Path, command: &str, check: bool) -> Result<String, String> {
    let errors: Vec<String> = checked(part, root)
        .into_iter()
        .filter(|p| p.level == Level::Error)
        .map(|p| format!("{}: {}", p.setting, p.message))
        .collect();
    if !errors.is_empty() {
        return Err(format!(
            "{FILE}'s `{PART}` has {} error(s):\n  {}",
            errors.len(),
            errors.join("\n  ")
        ));
    }
    let files = files(part, root, command)?;
    if files.is_empty() {
        return Ok(format!(
            "{COMMAND}: {FILE}'s `{PART}` names no `code`: no file to write"
        ));
    }
    if check {
        let stale = stale(part, root, command);
        if stale.is_empty() {
            return Ok(format!("{COMMAND}: up to date"));
        }
        return Err(format!(
            "{} file(s) out of date: {}: run `{command} {COMMAND}`",
            stale.len(),
            stale.join(", ")
        ));
    }
    let mut wrote = Vec::new();
    for (path, bytes) in files {
        let full = root.join(&path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        fs::write(&full, bytes).map_err(|e| format!("write {path}: {e}"))?;
        wrote.push(path);
    }
    Ok(format!("{COMMAND}: wrote {}", wrote.join(", ")))
}

/// `app-info`'s arguments: none, or `--check`. `Ok(true)` for `--check`.
pub fn parse_args(args: &[String]) -> Result<bool, String> {
    match args {
        [] => Ok(false),
        [flag] if flag == "--check" => Ok(true),
        _ => Err("no arguments, or --check".to_owned()),
    }
}

#[cfg(test)]
mod tests;
