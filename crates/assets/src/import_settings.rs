//! Import settings (ADR-0057): how one file, or every file of a folder, is
//! read or baked. A file's own settings are beside it, its full name kept
//! (`images/hero.png.meta`); a folder's defaults are in its
//! `_folder.meta`, for every file in it and below it. A settings file is
//! RON, a struct of sections, each read by the step it is named after:
//! `(onebit: (dither: bayer), indexed: (group: "ui"))`.
//!
//! A section is a type that implements [`ImportSettings`], registered in a
//! [`Formats`](crate::Formats) at start-up
//! ([`register_import_settings`](crate::Formats::register_import_settings)).
//! A format reads its section with
//! [`DecodeCx::import_settings`](crate::DecodeCx::import_settings): from
//! the nearest file of the chain that has it ([`settings_chain`]), whole,
//! or its `Default` when none does.
//! [`check_import_settings`](crate::Formats::check_import_settings) checks
//! every settings file of a bundle against the registered sections.
//!
//! No format may claim `meta`, so the walkers over a folder never read a
//! settings file as an asset.
use crate::prelude::*;

use core::fmt;

use ::ron::error::SpannedError;
use ::ron::value::RawValue;
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, MapAccess, Visitor};

use crate::bundle::Bundle;
use crate::error::{ContentError, codes};
use crate::format::join;

/// A section of a settings file: a struct the step named [`SECTION`]
/// reads (`onebit`, `indexed`), never named after a file format.
///
/// A section is a struct with `#[serde(default, deny_unknown_fields)]`:
/// every field may be left out, and a typo is an error.
/// [`register_import_settings`](crate::Formats::register_import_settings)
/// refuses one without them. Its fields hold integers, bools, enums and
/// strings, never floats (ADR-0057 §9).
///
/// ```
/// use serde::Deserialize;
/// use shuyi_assets::import_settings::ImportSettings;
///
/// /// How a picture is flipped.
/// #[derive(Debug, Default, PartialEq, Eq, Deserialize)]
/// #[serde(default, deny_unknown_fields)]
/// struct Mirror {
///     /// Whether it is flipped left to right.
///     mirror: bool,
/// }
///
/// impl ImportSettings for Mirror {
///     const SECTION: &'static str = "mirror";
/// }
///
/// let mut formats = shuyi_assets::Formats::standard();
/// assert!(formats.register_import_settings::<Mirror>().is_ok());
/// ```
///
/// [`SECTION`]: Self::SECTION
pub trait ImportSettings: DeserializeOwned + Default + 'static {
    /// The section's name in a settings file: the step that reads it.
    const SECTION: &'static str;
}

/// The extension of a settings file, which no format may claim.
pub const SETTINGS_EXTENSION: &str = "meta";

/// The name of a folder's settings file: the defaults of every file in
/// the folder and below it.
pub const FOLDER_SETTINGS: &str = "_folder.meta";

/// Whether the file at `path` is a settings file: its extension is
/// [`SETTINGS_EXTENSION`] (in any case).
pub fn is_settings_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty() && extension.eq_ignore_ascii_case(SETTINGS_EXTENSION)
    })
}

/// The path of the settings file of the file at `file`: `images/hero.png`
/// has `images/hero.png.meta`.
pub fn settings_of(file: &str) -> String {
    format!("{file}.{SETTINGS_EXTENSION}")
}

/// The settings files the file at `file` reads a section from, nearest
/// first: its own ([`settings_of`]), then the [`FOLDER_SETTINGS`] of its
/// folder and of each folder above it, up to the root.
pub fn settings_chain(file: &str) -> Vec<String> {
    // Each folder is the path up to one of its slashes, the nearest last.
    let folders = file.match_indices('/').rev().map(|(at, _)| &file[..at]);
    let mut chain = vec![settings_of(file)];
    chain.extend(folders.map(|dir| join(dir, FOLDER_SETTINGS)));
    chain.push(FOLDER_SETTINGS.to_owned());
    chain
}

/// A place in a settings file: a 1-based line and column (in characters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Place {
    line: u32,
    column: u32,
}

impl Place {
    /// The place of byte `offset` of `source`.
    fn of(source: &str, offset: usize) -> Self {
        let before = source.get(..offset).unwrap_or(source);
        let line = 1 + before.matches('\n').count();
        let column = 1 + before.rsplit('\n').next().unwrap_or("").chars().count();
        Self {
            line: to_u32(line),
            column: to_u32(column),
        }
    }
}

/// One section of a settings file: its name, its text (from right after
/// the colon, with the whitespace before the value) and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Section<'a> {
    /// Its name.
    pub(crate) name: String,
    /// Its value's text.
    text: &'a str,
    /// Where the text starts: RON counts an error in it from here.
    start: Place,
    /// Where its name is (where its value starts if the name isn't found
    /// before the colon): where errors about the whole section point.
    at: Place,
}

impl Section<'_> {
    /// An error about the whole section, of the file shown as `shown`.
    pub(crate) fn error(&self, shown: &str, message: impl Into<String>) -> ContentError {
        ContentError::new(shown, message).at(self.at.line, Some(self.at.column))
    }

    /// RON's error `e` inside the section's text, moved to its place in the
    /// file shown as `shown`.
    fn moved(&self, shown: &str, e: &SpannedError) -> ContentError {
        // `span.end` is the offending token, as `ron::parse_ron` reports it.
        let pos = e.span.end;
        let line = to_u32(pos.line.saturating_sub(1));
        let column = to_u32(pos.col);
        let (line, column) = if line == 0 {
            (
                self.start.line,
                self.start.column.saturating_add(column.saturating_sub(1)),
            )
        } else {
            (self.start.line.saturating_add(line), column)
        };
        ContentError::new(shown, e.code.to_string())
            .at(line, Some(column))
            .with_code(codes::PARSE)
    }
}

/// A settings file's sections as RON gives them, each value's text
/// borrowed from the source.
struct RawSections<'a>(Vec<(String, &'a RawValue)>);

impl<'de> Deserialize<'de> for RawSections<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(SectionsVisitor)
    }
}

/// Reads a struct of sections, or `()` for none.
struct SectionsVisitor;

impl<'de> Visitor<'de> for SectionsVisitor {
    type Value = RawSections<'de>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a struct of sections, as `(onebit: (dither: bayer))`")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(RawSections(Vec::new()))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut sections = Vec::new();
        while let Some(name) = map.next_key::<String>()? {
            sections.push((name, map.next_value::<&'de RawValue>()?));
        }
        Ok(RawSections(sections))
    }
}

/// The sections of the settings file shown as `shown`, whose bytes are
/// `bytes`, read at most `max` bytes: each with its text and place. Every
/// problem of the file as a whole: too big, not UTF-8, bad RON, a section
/// given twice.
pub(crate) fn sections<'a>(
    shown: &str,
    bytes: &'a [u8],
    max: usize,
) -> Result<Vec<Section<'a>>, Vec<ContentError>> {
    if bytes.len() > max {
        let message = format!(
            "the file is {} bytes; a settings file is read up to {max}",
            bytes.len()
        );
        return Err(vec![
            ContentError::new(shown, message).with_code(codes::TOO_BIG),
        ]);
    }
    let source = core::str::from_utf8(bytes).map_err(|_| {
        vec![ContentError::new(shown, "file is not valid UTF-8").with_code(codes::NOT_TEXT)]
    })?;
    let raw: RawSections<'a> =
        ::ron::from_str(source).map_err(|e| vec![crate::ron::error(shown, &e)])?;
    let base = source.as_ptr().addr();
    let sections: Vec<Section<'a>> = raw
        .0
        .into_iter()
        .map(|(name, value)| {
            let text = value.get_ron();
            // The text is borrowed from `source`, so it starts inside it.
            let offset = text
                .as_ptr()
                .addr()
                .checked_sub(base)
                .filter(|&offset| offset <= source.len())
                .unwrap_or(0);
            let at = name_offset(source, offset, &name)
                .unwrap_or(offset + text.len() - text.trim_start().len());
            Section {
                name,
                text,
                start: Place::of(source, offset),
                at: Place::of(source, at),
            }
        })
        .collect();
    let twice: Vec<ContentError> = sections
        .iter()
        .enumerate()
        .filter(|&(i, section)| sections[..i].iter().any(|s| s.name == section.name))
        .map(|(_, section)| {
            let message = format!(
                "the section \"{}\" is given twice; a settings file gives each section once",
                section.name
            );
            section.error(shown, message).with_code(codes::DUPLICATE)
        })
        .collect();
    if twice.is_empty() {
        Ok(sections)
    } else {
        Err(twice)
    }
}

/// The offset in `source` of the section name `name`, which ends before
/// the colon that ends right before byte `value` (spaces between).
fn name_offset(source: &str, value: usize, name: &str) -> Option<usize> {
    let before = source.get(..value)?.strip_suffix(':')?.trim_end();
    before.ends_with(name).then(|| before.len() - name.len())
}

/// `section` of the settings file shown as `shown`, read as `S`.
pub(crate) fn read_section<S: ImportSettings>(
    shown: &str,
    section: &Section<'_>,
) -> Result<S, ContentError> {
    ::ron::from_str(section.text).map_err(|e| section.moved(shown, &e))
}

/// What a registry keeps of a section: whether a section's text reads as
/// its type, and the error if it doesn't.
pub(crate) type CheckSection = fn(shown: &str, section: &Section<'_>) -> Option<ContentError>;

/// The [`CheckSection`] of `S`.
pub(crate) fn check_section<S: ImportSettings>(
    shown: &str,
    section: &Section<'_>,
) -> Option<ContentError> {
    read_section::<S>(shown, section).err()
}

/// The error for the settings file at `path` (shown as `shown`) if it
/// belongs to no file: a `<name>.meta` whose file `bundle` lacks and
/// doesn't leave out on purpose, or whose file is itself a settings file.
pub(crate) fn orphan(bundle: &Bundle, path: &str, shown: &str) -> Option<ContentError> {
    if path.rsplit('/').next() == Some(FOLDER_SETTINGS) {
        return None;
    }
    let file = path.get(..path.len().checked_sub(SETTINGS_EXTENSION.len() + 1)?)?;
    let file_shown = bundle.display_path(file);
    if is_settings_file(file) {
        let message = format!(
            "these are settings of the settings file {file_shown}; settings belong to an asset, \
             and a folder's defaults go in {FOLDER_SETTINGS}"
        );
        return Some(ContentError::new(shown, message));
    }
    if bundle.has(file) || bundle.left_out(file) {
        return None;
    }
    let message = format!(
        "these are the settings of {file_shown}, which the bundle doesn't have; rename them \
         with the file, or remove them"
    );
    Some(ContentError::new(shown, message).with_code(codes::MISSING_FILE))
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
