//! The game's identity in its desktop packages: `shuyi.ron`'s
//! `app` part ([`AppPart`], ADR-0059) as each system reads it.
//!
//! - Windows: the program's icon and version resource, `app.rc` ([`rc`])
//!   and `app.ico` ([`ico`]), compiled by binutils' `windres` and linked into
//!   the program;
//! - macOS: the `.app`'s `Info.plist` ([`info_plist`]) and
//!   `Resources/AppIcon.icns` ([`icns`]);
//! - Linux: `share/applications/<id>.desktop` ([`desktop_entry`]) and the
//!   icons at their hicolor paths ([`HICOLOR_SIDES`]).
//!
//! Every size of the icon is the picture `app.icon_sizes` draws for it, else
//! downscaled from the one icon ([`icons`], ADR-0061). A project without an
//! `app` part exports as it did before, and the report says so ([`NONE`]).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::app_info;
use crate::codec;
use crate::project::{AppPart, FILE};

/// The warning of an export of a project without an `app` part.
pub const NONE: &str = "no identity: shuyi.ron has no `app`";

/// The sides of the Windows program's icon, in pixels.
pub const ICO_SIDES: [u32; 6] = [16, 24, 32, 48, 64, 256];

/// The sides of the Linux package's icons, in pixels: each at
/// `share/icons/hicolor/<n>x<n>/apps/<id>.png`.
pub const HICOLOR_SIDES: [u32; 7] = [16, 32, 48, 64, 128, 256, 512];

/// The entries of a macOS `.icns`: each type and its side in pixels.
pub const ICNS_ENTRIES: [(&[u8; 4], u32); 8] = [
    (b"ic11", 32),
    (b"ic12", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic13", 256),
    (b"ic09", 512),
    (b"ic14", 512),
    (b"ic10", 1024),
];

/// Who the game is, as its packages say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The name players see.
    pub name: String,
    /// Its reverse-DNS id.
    pub id: String,
    /// Its version: the part's, else the game's package's.
    pub version: String,
    /// The number that only grows from one build handed out to the next.
    pub build: u32,
    /// Who publishes it, or empty.
    pub publisher: String,
    /// Its icon's file, if it has one.
    pub icon: Option<PathBuf>,
    /// The sizes drawn by hand: each side and its file.
    pub icon_sizes: BTreeMap<u32, PathBuf>,
}

impl Identity {
    /// The identity of `part`, in the project at `root`, whose version is
    /// `version` (`ExportCx::version`: the part's, else the package's).
    pub fn of(part: &AppPart, root: &Path, version: &str) -> Self {
        Self {
            name: part.name.clone(),
            id: part.id.clone(),
            version: version.to_owned(),
            build: part.build,
            publisher: part.publisher.clone(),
            icon: part.icon.as_ref().map(|icon| root.join(icon)),
            icon_sizes: part
                .icon_sizes
                .iter()
                .map(|(&side, path)| (side, root.join(path)))
                .collect(),
        }
    }

    /// The version's major, minor and patch numbers (semver), as
    /// `app_info::check_version` parses them.
    fn semver(&self) -> Result<semver::Version, String> {
        semver::Version::parse(&self.version)
            .map_err(|e| format!("`app.version` \"{}\" isn't semver: {e}", self.version))
    }

    /// The four numbers of a Windows version resource: major, minor and
    /// patch, then the build. Each is at most 65535: a bigger one is an error
    /// naming `app.version` or `app.build`.
    pub fn numbers(&self) -> Result<[u16; 4], String> {
        let version = self.semver()?;
        let small = |value: u64, field: &str| {
            u16::try_from(value).map_err(|_| {
                format!("`{field}` holds {value}, and a Windows version number is at most 65535")
            })
        };
        Ok([
            small(version.major, "app.version")?,
            small(version.minor, "app.version")?,
            small(version.patch, "app.version")?,
            small(u64::from(self.build), "app.build")?,
        ])
    }

    /// `major.minor.patch`: macOS's `CFBundleShortVersionString`.
    pub fn short_version(&self) -> Result<String, String> {
        let version = self.semver()?;
        Ok(format!(
            "{}.{}.{}",
            version.major, version.minor, version.patch
        ))
    }

    /// The `.app` folder's name: the name, with `/` and `:` as `-`.
    pub fn app_folder(&self) -> String {
        format!("{}.app", self.name.replace(['/', ':'], "-"))
    }
}

/// The icon of `identity` as a PNG at each of `sizes` no larger than its
/// source, in that order: the picture `app.icon_sizes` draws for a side, else
/// downscaled from the icon (ADR-0061). Empty without an icon. A bad file is
/// an error naming its path.
pub fn icons(identity: &Identity, sizes: &[u32]) -> Result<Vec<(u32, Vec<u8>)>, String> {
    let Some(file) = &identity.icon else {
        return Ok(Vec::new());
    };
    let icon = app_info::icon_file(file, &file.display().to_string())?;
    let mut out = Vec::new();
    for &side in sizes.iter().filter(|&&side| side <= icon.side) {
        let pixels = match identity.icon_sizes.get(&side) {
            Some(drawn) => app_info::sized_file(drawn, &drawn.display().to_string(), (side, side))?,
            None => app_info::downscale(icon.side, icon.side, &icon.rgba, side),
        };
        out.push((side, codec::encode_png(side, side, &pixels)?));
    }
    Ok(out)
}

/// A side as an `.ico` entry writes it: 0 for 256.
fn ico_side(side: u32) -> u8 {
    u8::try_from(side).unwrap_or(0)
}

/// A length as a 32-bit field: a picture is never 4 GiB.
fn u32_of(length: usize) -> u32 {
    u32::try_from(length).unwrap_or(u32::MAX)
}

/// The `.ico` of `pictures`, each a side and its PNG: the 6-byte header (0,
/// type 1, the count), one 16-byte entry per picture (width and height, 0
/// for 256; 0 colours; 0; 1 plane; 32 bits; the PNG's length; its offset),
/// then the pictures in the same order.
pub fn ico(pictures: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(pictures.len()).unwrap_or(u16::MAX);
    let mut out = Vec::new();
    for word in [0, 1, count] {
        out.extend(word.to_le_bytes());
    }
    let mut offset = 6 + 16 * u32::from(count);
    for (side, png) in pictures {
        out.extend([ico_side(*side), ico_side(*side), 0, 0]);
        out.extend(1_u16.to_le_bytes());
        out.extend(32_u16.to_le_bytes());
        out.extend(u32_of(png.len()).to_le_bytes());
        out.extend(offset.to_le_bytes());
        offset += u32_of(png.len());
    }
    for (_, png) in pictures {
        out.extend(png);
    }
    out
}

/// `text` as a resource script's string: in quotes, a `"` as `""` and a `\`
/// as `\\`.
fn rc_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\"\""))
}

/// The resource script of `identity`'s program `bin`, `app.rc`: UTF-8, its
/// icon `app.ico` when `icon`, and its version resource, with the four
/// `numbers`.
pub fn rc(identity: &Identity, bin: &str, numbers: [u16; 4], icon: bool) -> String {
    let [a, b, c, d] = numbers;
    let mut out = String::from("#pragma code_page(65001)\n");
    if icon {
        out.push_str("1 ICON \"app.ico\"\n");
    }
    let _ = write!(
        out,
        "1 VERSIONINFO\n\
         FILEVERSION {a},{b},{c},{d}\n\
         PRODUCTVERSION {a},{b},{c},{d}\n\
         FILEOS 0x40004\n\
         FILETYPE 0x1\n\
         BEGIN\n\
         \x20 BLOCK \"StringFileInfo\"\n\
         \x20 BEGIN\n\
         \x20   BLOCK \"040904B0\"\n\
         \x20   BEGIN\n"
    );
    let program = format!("{bin}.exe");
    let mut values = Vec::new();
    if !identity.publisher.is_empty() {
        values.push(("CompanyName", identity.publisher.as_str()));
    }
    values.extend([
        ("FileDescription", identity.name.as_str()),
        ("FileVersion", identity.version.as_str()),
        ("InternalName", bin),
        ("OriginalFilename", program.as_str()),
        ("ProductName", identity.name.as_str()),
        ("ProductVersion", identity.version.as_str()),
    ]);
    for (key, value) in values {
        let _ = writeln!(out, "      VALUE \"{key}\", {}", rc_string(value));
    }
    out.push_str(
        "    END\n  END\n  BLOCK \"VarFileInfo\"\n  BEGIN\n    VALUE \"Translation\", 0x409, \
         1200\n  END\nEND\n",
    );
    out
}

/// `text` with `&`, `<` and `>` escaped, for XML.
fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The `Info.plist` of a bundle whose program is `bin`: with an identity,
/// its id, name, versions and, when `icon`, `AppIcon`; without one, the
/// program and the package type alone.
pub fn info_plist(bin: &str, identity: Option<(&Identity, &str)>, icon: bool) -> String {
    let mut entries: Vec<(&str, String)> = Vec::new();
    if let Some((identity, short)) = identity {
        entries.extend([
            ("CFBundleIdentifier", xml(&identity.id)),
            ("CFBundleName", xml(&identity.name)),
            ("CFBundleDisplayName", xml(&identity.name)),
            ("CFBundleShortVersionString", short.to_owned()),
            ("CFBundleVersion", identity.build.to_string()),
        ]);
    }
    entries.push(("CFBundleExecutable", xml(bin)));
    if icon {
        entries.push(("CFBundleIconFile", "AppIcon".to_owned()));
    }
    entries.push(("CFBundlePackageType", "APPL".to_owned()));
    let mut body = String::new();
    for (key, value) in entries {
        let _ = write!(body, "\t<key>{key}</key>\n\t<string>{value}</string>\n");
    }
    if identity.is_some() {
        body.push_str(
            "\t<key>CFBundleInfoDictionaryVersion</key>\n\t<string>6.0</string>\n\
             \t<key>NSHighResolutionCapable</key>\n\t<true/>\n\
             \t<key>LSApplicationCategoryType</key>\n\t<string>public.app-category.games</string>\n",
        );
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n{body}</dict>\n</plist>\n"
    )
}

/// The `.icns` of `pictures`, each a side and its PNG: `icns` and the file's
/// length, then per entry of [`ICNS_ENTRIES`] whose side `pictures` has, its
/// type, its length with its 8-byte header and the PNG; all big-endian.
pub fn icns(pictures: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut entries = Vec::new();
    for (kind, side) in ICNS_ENTRIES {
        if let Some((_, png)) = pictures.iter().find(|(s, _)| *s == side) {
            entries.extend(kind);
            entries.extend(u32_of(8 + png.len()).to_be_bytes());
            entries.extend(png);
        }
    }
    let mut out = b"icns".to_vec();
    out.extend(u32_of(8 + entries.len()).to_be_bytes());
    out.extend(entries);
    out
}

/// The sides an `.icns` asks of an icon, each once.
pub fn icns_sides() -> Vec<u32> {
    let mut sides: Vec<u32> = ICNS_ENTRIES.iter().map(|(_, side)| *side).collect();
    sides.dedup();
    sides
}

/// `text` as a desktop entry's value: a `\` as `\\`.
fn entry_value(text: &str) -> String {
    text.replace('\\', "\\\\")
}

/// The desktop entry of `identity`'s program `bin`,
/// `share/applications/<id>.desktop`: its icon named by the id when `icon`.
pub fn desktop_entry(identity: &Identity, bin: &str, icon: bool) -> String {
    let mut out = format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec={}\n",
        entry_value(&identity.name),
        entry_value(bin)
    );
    if icon {
        let _ = writeln!(out, "Icon={}", identity.id);
    }
    out.push_str("Categories=Game;\nTerminal=false\n");
    out
}

/// The paths in a Linux package of `identity`'s desktop entry and icons,
/// from its folder, with their bytes.
pub fn linux_files(identity: &Identity, bin: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    let icons = icons(identity, &HICOLOR_SIDES)?;
    let id = &identity.id;
    let mut out = vec![(
        format!("share/applications/{id}.desktop"),
        desktop_entry(identity, bin, !icons.is_empty()).into_bytes(),
    )];
    for (side, png) in icons {
        out.push((
            format!("share/icons/hicolor/{side}x{side}/apps/{id}.png"),
            png,
        ));
    }
    Ok(out)
}

/// The error of an icon that can't be read for a package, naming the file.
pub fn icon_error(why: &str) -> String {
    format!("{FILE}'s `app.icon`: {why}")
}

#[cfg(test)]
mod tests;
