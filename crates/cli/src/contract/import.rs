//! `import <file> [--kind K] [--print]`: a file read through the format
//! registry (ADR-0019) as every kind whose formats know it, with a short
//! summary of what was read, the asset itself with `--print`, or every
//! error with its file and line.
//!
//! The kinds tried are [`Importers`]: the engine's standard ones
//! ([`Importers::standard`]) and those the game adds with its own formats
//! ([`GameTools::importers`](crate::GameTools::importers)). The importers
//! of other tools' files (Tiled, `LDtk`, Aseprite) are formats of the
//! registry too, when tickets 1123 and 1124 add them; `import` needs no
//! change for them.

use std::fmt::{Debug, Write as _};
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::audio::{AudioManifest, MusicTrack, Sound};
use shuyi_assets::bundle::{Bundle, Layer};
use shuyi_assets::credits::CreditsFile;
use shuyi_assets::dialogue::{DialogueFile, NoDirective};
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::format::{AssetKind, AssetPath, DecodeCx, Formats};
use shuyi_assets::image::ImageInfo;
use shuyi_assets::lighting::Lighting;
use shuyi_assets::names::Names;
use shuyi_assets::palette::PaletteDef;

use super::{Answer, Op, Problem};

/// The most files of the file's folder read for the files it names.
pub const MAX_FOLDER_FILES: usize = 1_000;

/// The most bytes of the file's folder read for the files it names.
pub const MAX_FOLDER_BYTES: usize = 64 << 20;

/// What `import` reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportInput {
    /// The file, from the project's root (or the working folder outside a
    /// project).
    pub file: String,
    /// Only this kind (`palette`, `image`, `map`…); every kind if none.
    #[serde(default)]
    pub kind: Option<String>,
    /// Whether to give the asset itself, not only its summary.
    #[serde(default)]
    pub print: bool,
}

/// The file read as one kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Imported {
    /// The kind (`palette`).
    pub kind: String,
    /// What was read, in a line (`palette: 21 colors`).
    pub summary: String,
    /// With `print`, the asset: in our text format if the kind has a
    /// writer, else as read, in Rust's debug form.
    pub text: Option<String>,
}

/// What `import` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportOutput {
    /// The file.
    pub file: String,
    /// Every kind it reads as.
    pub read_as: Vec<Imported>,
    /// When it reads as none: the errors of the kinds that claim it.
    pub errors: Vec<Problem>,
}

/// What reading a file as one kind gave: its summary, the asset as text
/// if asked for, and whether that text is our own format.
type Read = (String, Option<String>, bool);

/// A kind `import` tries, over the registry's formats of it.
trait ImportKind: Send + Sync {
    fn name(&self) -> &'static str;

    /// Whether a format of the kind claims `path`'s extension.
    fn claims(&self, formats: &Formats, path: &str) -> bool;

    fn read(
        &self,
        formats: &Formats,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
        print: bool,
    ) -> Result<Read, Vec<ContentError>>;
}

/// Kind `K`, its summary, and its writer in our text format if it has
/// one.
struct KindOf<K> {
    summary: fn(&K) -> String,
    text: Option<fn(&K) -> String>,
}

impl<K: AssetKind + Debug> ImportKind for KindOf<K> {
    fn name(&self) -> &'static str {
        K::NAME
    }

    fn claims(&self, formats: &Formats, path: &str) -> bool {
        formats.claims::<K>(path)
    }

    fn read(
        &self,
        formats: &Formats,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
        print: bool,
    ) -> Result<Read, Vec<ContentError>> {
        let asset = formats.decode::<K>(file, bytes, cx)?;
        let (text, ours) = match (print, self.text) {
            (false, _) => (None, true),
            (true, Some(write)) => (Some(write(&asset)), true),
            (true, None) => (Some(format!("{asset:#?}")), false),
        };
        Ok(((self.summary)(&asset), text, ours))
    }
}

/// The kinds `import` tries and the formats it reads them with.
pub struct Importers {
    /// The registry: the engine's standard formats, and the game's.
    pub formats: Formats,
    kinds: Vec<Box<dyn ImportKind>>,
}

impl Debug for Importers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Importers")
            .field("formats", &self.formats)
            .field("kinds", &self.kinds())
            .finish()
    }
}

/// `n` things, as `1 color` or `3 colors`.
fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

impl Importers {
    /// The kinds of no format yet, over `formats`.
    pub fn new(formats: Formats) -> Self {
        Self {
            formats,
            kinds: Vec::new(),
        }
    }

    /// The engine's standard kinds over its standard formats
    /// ([`Formats::standard`]).
    pub fn standard() -> Self {
        Self::new(Formats::standard())
            .kind::<ImageInfo>(|i| format!("{}×{} pixels", i.width, i.height))
            .kind::<Sound>(|s| format!("{} channel(s) at {} Hz", s.channels, s.sample_rate))
            .kind::<MusicTrack>(|m| {
                format!(
                    "{} channel(s) at {} Hz, {} ms long",
                    m.channels, m.sample_rate, m.length_ms
                )
            })
            .kind::<AudioManifest>(|a| {
                format!(
                    "{}, {}",
                    count(a.sounds.len(), "sound", "sounds"),
                    count(a.music.len(), "music track", "music tracks")
                )
            })
            .kind::<CreditsFile>(|c| count(c.credits.len(), "credit", "credits"))
            .kind::<FontAtlasDef>(|f| {
                format!(
                    "{} of {}×{} pixels",
                    count(f.glyphs.len(), "glyph", "glyphs"),
                    f.cell_w,
                    f.cell_h
                )
            })
            .kind::<PaletteDef>(|p| count(p.colors.len(), "color", "colors"))
            .kind::<Lighting>(|l| count(l.lights.len(), "light", "lights"))
            .kind::<Names>(|n| count(n.names.len(), "name", "names"))
            .kind::<DialogueFile<NoDirective>>(|d| {
                let ids: Vec<&str> = d.scenes.iter().map(|s| s.scene.id.as_str()).collect();
                format!(
                    "{}: {}",
                    count(ids.len(), "scene", "scenes"),
                    ids.join(", ")
                )
            })
    }

    /// These importers, trying kind `K` too, summed up by `summary`. Its
    /// formats are the registry's ([`formats`](Self::formats)).
    #[must_use]
    pub fn kind<K: AssetKind + Debug>(mut self, summary: fn(&K) -> String) -> Self {
        self.kinds.push(Box::new(KindOf::<K> {
            summary,
            text: None,
        }));
        self
    }

    /// [`kind`](Self::kind), with `text` writing the asset in our text
    /// format for `--print`.
    #[must_use]
    pub fn kind_with_text<K: AssetKind + Debug>(
        mut self,
        summary: fn(&K) -> String,
        text: fn(&K) -> String,
    ) -> Self {
        self.kinds.push(Box::new(KindOf::<K> {
            summary,
            text: Some(text),
        }));
        self
    }

    /// The kinds' names, in the order they are tried.
    pub fn kinds(&self) -> Vec<&'static str> {
        self.kinds.iter().map(|k| k.name()).collect()
    }
}

/// A bundle of the files in `dir` and below it, for the files a file
/// names (a font atlas's picture, a manifest's sounds), or why there is
/// none: more than `max_files` files or `max_bytes` bytes. Its files are
/// leaked: a tool reads them once per run. (Ticket 2207 brings folders on
/// disk to the bundle itself.)
fn folder_bundle(dir: &Path, max_files: usize, max_bytes: usize) -> Result<Bundle, String> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut pending = vec![(dir.to_path_buf(), String::new())];
    let mut bytes = 0;
    while let Some((path, prefix)) = pending.pop() {
        let entries = fs::read_dir(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let inner = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                pending.push((entry.path(), inner));
            } else if kind.is_file() {
                let data = fs::read(entry.path()).map_err(|e| format!("{inner}: {e}"))?;
                bytes += data.len();
                files.push((inner, data));
            }
            if files.len() > max_files || bytes > max_bytes {
                return Err(format!(
                    "its folder holds more than {max_files} files or {max_bytes} bytes"
                ));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let table: Vec<(&'static str, &'static [u8])> = files
        .into_iter()
        .map(|(path, data)| {
            let path: &'static str = Box::leak(path.into_boxed_str());
            let data: &'static [u8] = Box::leak(data.into_boxed_slice());
            (path, data)
        })
        .collect();
    let table: &'static [(&'static str, &'static [u8])] = Box::leak(table.into_boxed_slice());
    Ok(Bundle::new(vec![Layer::table("", table)]))
}

/// `import`: `input`'s file, under `root`, read by `importers`.
pub fn import(importers: &Importers, root: &Path, input: &ImportInput) -> Answer<ImportOutput> {
    let path = root.join(&input.file);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => return Answer::failed(Op::Import, format!("{}: {e}", input.file)),
    };
    let kinds: Vec<&dyn ImportKind> = importers
        .kinds
        .iter()
        .map(|k| &**k)
        .filter(|k| input.kind.as_deref().is_none_or(|kind| kind == k.name()))
        .collect();
    if kinds.is_empty() {
        let known = importers.kinds().join(", ");
        return Answer::failed(
            Op::Import,
            format!(
                "no kind `{}`: the kinds are {known}",
                input.kind.as_deref().unwrap_or_default()
            ),
        );
    }
    let mut limits = Vec::new();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = path
        .parent()
        .map(|dir| folder_bundle(dir, MAX_FOLDER_FILES, MAX_FOLDER_BYTES));
    let bundle = match folder {
        Some(Ok(bundle)) => Some(bundle),
        Some(Err(why)) => {
            limits.push(format!("the files it names were not read: {why}"));
            None
        }
        None => None,
    };
    let cx = bundle
        .as_ref()
        .map_or_else(DecodeCx::new, DecodeCx::in_bundle);
    let file = AssetPath::new(name.clone()).shown_as(input.file.clone());
    let mut read_as = Vec::new();
    let mut errors = Vec::new();
    let mut claimed = false;
    for kind in kinds {
        let claims = kind.claims(&importers.formats, &name);
        claimed |= claims;
        match kind.read(&importers.formats, &file, &bytes, &cx, input.print) {
            Ok((summary, text, ours)) => {
                if !ours {
                    limits.push(format!(
                        "no writer of our text format for the {} kind yet: it is printed as read, \
                         in Rust's debug form",
                        kind.name()
                    ));
                }
                read_as.push(Imported {
                    kind: kind.name().to_owned(),
                    summary: format!("{}: {summary}", kind.name()),
                    text,
                });
            }
            Err(e) if claims => errors.extend(e.iter().map(Problem::from)),
            Err(_) => {}
        }
    }
    if !read_as.is_empty() {
        errors.clear();
        if read_as.len() > 1 {
            limits.push(format!(
                "it reads as {} kinds: name the one meant with --kind",
                read_as.len()
            ));
        }
    } else if !claimed {
        errors.push(Problem {
            file: input.file.clone(),
            line: None,
            column: None,
            message: format!(
                "no format of these kinds knows its bytes or claims its extension: {}",
                importers.kinds().join(", ")
            ),
        });
        limits.push(
            "the importers of Tiled, LDtk and Aseprite files come with tickets 1123 and 1124"
                .to_owned(),
        );
    }
    let ok = !read_as.is_empty();
    let output = ImportOutput {
        file: input.file.clone(),
        read_as,
        errors,
    };
    Answer::new(Op::Import, ok, output, limits)
}

/// Reads `import`'s arguments (not `--json`).
pub fn parse_args(args: &[String]) -> Result<ImportInput, String> {
    let mut input = ImportInput::default();
    let mut file = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--print" => input.print = true,
            "--kind" => {
                input.kind = Some(iter.next().ok_or("--kind needs a value")?.clone());
            }
            flag if flag.starts_with("--") => return Err(format!("unknown option: {flag}")),
            path if file.is_none() => file = Some(path.to_owned()),
            extra => return Err(format!("one <file> only, not also {extra}")),
        }
    }
    input.file = file.ok_or("missing <file>")?;
    Ok(input)
}

/// What `import` found, for people.
pub fn text(output: &ImportOutput) -> String {
    let mut out = String::new();
    for read in &output.read_as {
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "import: {} is {}", output.file, read.summary);
        if let Some(text) = &read.text {
            out.push_str(text);
            if !text.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    if !output.errors.is_empty() {
        let _ = writeln!(out, "import: {} error(s)", output.errors.len());
        for error in &output.errors {
            let _ = writeln!(out, "  {error}");
        }
    }
    out
}

#[cfg(test)]
mod tests;
