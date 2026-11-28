//! Picture sets: a file of named pictures for one id (a person, an item,
//! a card), each name an image: a villager's `neutral`, `happy` and
//! `upset` faces, a card's `front` and `back`. The game says which names
//! every set must have and the largest size a picture may be
//! ([`PictureRules`]); [`name_problem`] answers whether an id has a
//! picture of a name, as a dialogue check asks of an expression.
//!
//! A file is a [`PictureSetDef`], read by the format [`PictureSetRon`]
//! and checked alone ([`PictureSetDef::from_source`]): its `id` is its
//! file's stem, and it has every required name. Each path is relative to
//! the file's folder:
//!
//! ```ron
//! // portraits/ana.ron
//! (
//!     id: "ana",
//!     pictures: {
//!         "neutral": "ana/neutral.png",
//!         "happy": "ana/smile.png",
//!         "upset": "ana/frown.png",
//!     },
//! )
//! ```
//!
//! Once the image table is known, [`PictureSetDef::resolve`] makes the
//! [`PictureSet`] a game draws from, or reports each picture not in the
//! table or bigger than the rules allow, at its line.
//!
//! A dialogue check's [`SceneRules::expression_problem`] can answer from
//! the sets, a character with no set showing only a standard list:
//!
//! ```
//! use std::collections::BTreeMap;
//!
//! use shuyi_assets::dialogue::{CharacterId, DialogueLimits, NoTokens, SceneRules, Tokens};
//! use shuyi_assets::image::{ImageInfo, ImageTable};
//! use shuyi_assets::picture_set::{NamedPicture, PictureSet, name_problem};
//!
//! /// What a character with no picture set may show.
//! const STANDARD: &[&str] = &["neutral"];
//!
//! struct Rules {
//!     limits: DialogueLimits,
//!     sets: BTreeMap<String, PictureSet>,
//! }
//!
//! impl SceneRules<shuyi_assets::dialogue::NoDirective> for Rules {
//!     fn limits(&self) -> &DialogueLimits {
//!         &self.limits
//!     }
//!
//!     fn tokens(&self) -> &dyn Tokens {
//!         &NoTokens
//!     }
//!
//!     fn expression_problem(&self, character: &CharacterId, expression: &str) -> Option<String> {
//!         let sets: Vec<&PictureSet> = self.sets.get(&character.0).into_iter().collect();
//!         name_problem(&sets, expression, STANDARD).map(|problem| problem.to_string())
//!     }
//! }
//!
//! let images = ImageTable::new([("ana/neutral.png", ImageInfo::new(64, 64))].into());
//! let neutral = images.id("ana/neutral.png").expect("in the table");
//! let ana = PictureSet::new(
//!     "ana",
//!     vec![NamedPicture::new("neutral", neutral, ImageInfo::new(64, 64))],
//! );
//! let limits = DialogueLimits {
//!     box_width: 30,
//!     box_lines: 3,
//!     max_text: 90,
//!     max_option: 20,
//!     max_options: 4,
//!     min_options: 2,
//!     max_reaction_texts: 3,
//! };
//! let rules = Rules { limits, sets: [("ana".to_owned(), ana)].into() };
//! let ana = CharacterId("ana".into());
//! assert_eq!(rules.expression_problem(&ana, "neutral"), None);
//! assert_eq!(
//!     rules.expression_problem(&ana, "happy").as_deref(),
//!     Some("\"ana\" has no picture of that name; it has: neutral"),
//! );
//! let bo = CharacterId("bo".into());
//! assert_eq!(rules.expression_problem(&bo, "neutral"), None);
//! assert!(rules.expression_problem(&bo, "happy").is_some());
//! ```
//!
//! [`SceneRules::expression_problem`]: crate::dialogue::SceneRules::expression_problem
use crate::prelude::*;

use alloc::collections::BTreeMap;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::bundle::Bundle;
use crate::error::{ContentError, codes};
use crate::format::{AssetKind, AssetPath, DecodeCx, TextFormat, join};
use crate::image::{ImageId, ImageInfo, ImageTable};
use crate::ron::parse_ron;

/// What every picture set of a game must be: the names each has, and the
/// largest a picture may be. Build it with [`PictureRules::new`]
/// (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PictureRules<'a> {
    /// The names every set must have, in the order a [`PictureSet`] lists
    /// them first.
    pub required: &'a [&'a str],
    /// The largest a picture may be, in pixels, width × height.
    pub max_px: (u32, u32),
}

impl<'a> PictureRules<'a> {
    /// Sets that must have every name of `required`, each picture at most
    /// `max_px` (width × height).
    #[must_use]
    pub const fn new(required: &'a [&'a str], max_px: (u32, u32)) -> Self {
        Self { required, max_px }
    }
}

/// A picture set as written: an id and each picture's image path,
/// relative to the file's folder, by name. Build it with
/// [`PictureSetDef::new`] (ADR-0039), or read it with [`PictureSetRon`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PictureSetDef {
    /// Whom the pictures show: the file's stem.
    pub id: String,
    /// Each picture's image path, by name.
    pub pictures: BTreeMap<String, String>,
    /// Each picture's line in its file, for [`resolve`](Self::resolve)'s
    /// errors.
    #[serde(skip)]
    lines: BTreeMap<String, u32>,
}

impl AssetKind for PictureSetDef {
    const NAME: &str = "picture set";
}

impl PictureSetDef {
    /// The set `id` of `pictures` (image path by name), with no lines.
    #[must_use]
    pub const fn new(id: String, pictures: BTreeMap<String, String>) -> Self {
        Self {
            id,
            pictures,
            lines: BTreeMap::new(),
        }
    }

    /// Parses `source`, the file shown as `file` whose stem is `stem`,
    /// and checks it against `rules`. Reports every problem: its RON, an
    /// `id` other than `stem`, each name of `rules.required` it lacks.
    pub fn from_source(
        file: &str,
        stem: &str,
        source: &str,
        rules: &PictureRules<'_>,
    ) -> Result<Self, Vec<ContentError>> {
        let mut def: Self = parse_ron(file, source).map_err(|e| vec![e])?;
        let mut errors = Vec::new();
        if def.id != stem {
            errors.push(ContentError::new(
                file,
                format!("id \"{}\" isn't the file's name \"{stem}\"", def.id),
            ));
        }
        for &name in rules.required {
            if !def.pictures.contains_key(name) {
                errors.push(ContentError::new(
                    file,
                    format!("missing required picture \"{name}\""),
                ));
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        // The pictures' names come after `pictures`, where the id can't be.
        let lines: Vec<&str> = source.lines().collect();
        let start = lines.iter().position(|l| l.contains("pictures"));
        let after = lines.get(start.unwrap_or(0)..).unwrap_or_default();
        def.lines = def
            .pictures
            .keys()
            .filter_map(|name| {
                let key = format!("\"{name}\"");
                let at = after.iter().position(|l| l.contains(&key))?;
                let line = u32::try_from(start.unwrap_or(0) + at + 1).ok()?;
                Some((name.clone(), line))
            })
            .collect();
        Ok(def)
    }

    /// The set a game draws from: each path, relative to `dir` (the
    /// file's folder), an image of `images` within `rules.max_px`, named
    /// as `bundle` shows it when one is given. Its pictures are the names
    /// of `rules.required` first, in their order, then the others by
    /// name. Reports every picture that isn't, at its line, as a problem
    /// of `file`.
    pub fn resolve(
        &self,
        file: &str,
        dir: &str,
        images: &ImageTable,
        bundle: Option<&Bundle>,
        rules: &PictureRules<'_>,
    ) -> Result<PictureSet, Vec<ContentError>> {
        let mut pictures = Vec::new();
        let mut errors = Vec::new();
        for (name, relative) in &self.pictures {
            let path = join(dir, relative);
            let shown = bundle.map_or_else(|| path.clone(), |b| b.display_path(&path));
            let found = images.id(&path).and_then(|id| Some((id, images.info(id)?)));
            let problem = match found {
                None => Some((
                    format!("{shown} is not an image in the assets"),
                    codes::MISSING_FILE,
                )),
                Some((_, size)) if size.width > rules.max_px.0 || size.height > rules.max_px.1 => {
                    let (w, h) = rules.max_px;
                    Some((
                        format!(
                            "{shown} is {}×{} px; a picture must fit {w}×{h} px",
                            size.width, size.height
                        ),
                        codes::OUT_OF_RANGE,
                    ))
                }
                Some((image, size)) => {
                    pictures.push(NamedPicture::new(name.clone(), image, size));
                    None
                }
            };
            if let Some((message, code)) = problem {
                let mut e = ContentError::new(file, format!("picture \"{name}\": {message}"))
                    .with_code(code);
                if let Some(&line) = self.lines.get(name) {
                    e = e.at(line, None);
                }
                errors.push(e);
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        // Stable: the others stay in name order.
        pictures.sort_by_key(|p| {
            let at = rules.required.iter().position(|r| *r == p.name);
            at.unwrap_or(rules.required.len())
        });
        Ok(PictureSet::new(self.id.clone(), pictures))
    }
}

/// A picture set file (`picture-set`, `.ron`), checked as
/// [`PictureSetDef::from_source`] says against the game's
/// [`PictureRules`], its stem the file's ([`AssetPath::stem`]). Not in
/// [`Formats::standard`](crate::format::Formats::standard): a game
/// registers it with its rules, and resolves what it reads
/// ([`PictureSetDef::resolve`]) once its images are known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureSetRon {
    rules: PictureRules<'static>,
}

impl PictureSetRon {
    /// Picture set files checked against `rules`.
    #[must_use]
    pub const fn new(rules: PictureRules<'static>) -> Self {
        Self { rules }
    }
}

impl TextFormat for PictureSetRon {
    type Asset = PictureSetDef;

    fn id(&self) -> &'static str {
        "picture-set"
    }

    fn read(
        &self,
        file: &AssetPath,
        source: &str,
        _cx: &DecodeCx<'_>,
    ) -> Result<PictureSetDef, Vec<ContentError>> {
        PictureSetDef::from_source(file.shown(), file.stem(), source, &self.rules)
    }
}

/// A picture set a game draws from, made by [`PictureSetDef::resolve`].
/// Build one with [`PictureSet::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PictureSet {
    /// Whom the pictures show.
    pub id: String,
    /// The pictures: the required names first, in the rules' order, then
    /// the others by name.
    pub pictures: Vec<NamedPicture>,
}

impl PictureSet {
    /// The set `id` of `pictures`, in their order.
    #[must_use]
    pub fn new(id: impl Into<String>, pictures: Vec<NamedPicture>) -> Self {
        Self {
            id: id.into(),
            pictures,
        }
    }

    /// The picture called `name`.
    pub fn get(&self, name: &str) -> Option<&NamedPicture> {
        self.pictures.iter().find(|p| p.name == name)
    }
}

/// One picture of a set: its name and its image.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NamedPicture {
    /// Its name, e.g. `happy`.
    pub name: String,
    /// The image that shows it.
    pub image: ImageId,
    /// That image's size in pixels.
    pub size: ImageInfo,
}

impl NamedPicture {
    /// The picture `name`, image `image` of size `size`.
    #[must_use]
    pub fn new(name: impl Into<String>, image: ImageId, size: ImageInfo) -> Self {
        Self {
            name: name.into(),
            image,
            size,
        }
    }
}

/// Why a name isn't a picture an id may show ([`name_problem`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NameProblem<'a> {
    /// This set lacks the name.
    NotInSet(&'a PictureSet),
    /// The id has no set, and the name isn't one of these.
    NotAllowed(&'a [&'a str]),
}

impl fmt::Display for NameProblem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (start, names): (String, Vec<&str>) = match self {
            Self::NotInSet(set) => (
                format!("\"{}\" has no picture of that name; it has", set.id),
                set.pictures.iter().map(|p| p.name.as_str()).collect(),
            ),
            Self::NotAllowed(names) => (
                "with no picture set, the name must be one of".to_owned(),
                names.to_vec(),
            ),
        };
        write!(f, "{start}: {}", names.join(", "))
    }
}

/// What is wrong with an id whose picture sets are `sets` showing the
/// picture `name`, if anything: the first set that lacks it, or, with no
/// set, that `name` isn't one of `without_a_set`.
pub fn name_problem<'a>(
    sets: &[&'a PictureSet],
    name: &str,
    without_a_set: &'a [&'a str],
) -> Option<NameProblem<'a>> {
    if sets.is_empty() {
        return (!without_a_set.contains(&name)).then_some(NameProblem::NotAllowed(without_a_set));
    }
    sets.iter()
        .find(|set| set.get(name).is_none())
        .map(|&set| NameProblem::NotInSet(set))
}

#[cfg(test)]
mod tests;
