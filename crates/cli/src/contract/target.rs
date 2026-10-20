//! `check --target <id>`: whether the game fits a target (ADR-0008 §1),
//! by the target's profile and the game's setting for it in `shuyi.ron`
//! ([`TargetSetting`]). Two checks for now:
//!
//! - **Fits.** The frame times its scale fits the display inside its safe
//!   area.
//! - **Text.** The smallest text (the font's cell height × the scale) is
//!   at least the profile's `min_text_px` (an error) and its
//!   `good_text_px` (a warning).
//!
//! The engine's profiles are the files of `targets/` at the repository's
//! root, embedded; a game's own (`shuyi.ron`'s `target_profiles`) add to
//! them, or replace one by its id.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use include_dir::{Dir, include_dir};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::target::{Size, TargetProfile, TargetRon};
use shuyi_assets::{AssetPath, ContentError, DecodeCx, Format};

use crate::project::{Config, FILE, Scale, TargetSetting};

/// The engine's target profiles, embedded.
static TARGETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../targets");

/// What `--target all` asks for: every target in the game's `shuyi.ron`.
pub const ALL: &str = "all";

/// How bad a target problem is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    /// The game doesn't fit the target: the check fails.
    Error,
    /// It fits, short of what the target recommends.
    Warning,
}

/// Which check found a target problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TargetCheck {
    /// The frame at its scale fits the display.
    Fits,
    /// The smallest text is tall enough.
    Text,
}

/// A problem of the game on one target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetProblem {
    /// An error or a warning.
    pub level: Level,
    /// The check that found it.
    pub check: TargetCheck,
    /// The game's setting it is about, in `shuyi.ron`
    /// (`targets."playdate".frame`).
    pub setting: String,
    /// What is wrong, with the numbers.
    pub message: String,
}

/// What `check --target` found on one target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetReport {
    /// The target's id.
    pub target: String,
    /// The profile's file: the engine's (`targets/playdate.ron`) or the
    /// game's.
    pub profile: String,
    /// Every problem, errors and warnings, display by display.
    pub problems: Vec<TargetProblem>,
}

impl TargetReport {
    /// Whether it has an error.
    pub fn failed(&self) -> bool {
        self.problems.iter().any(|p| p.level == Level::Error)
    }
}

/// A target profile and the file it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Its file, as messages show it.
    pub file: String,
    /// What it says.
    pub profile: TargetProfile,
}

/// The file `name` with `bytes`, shown as `shown`, read as a profile.
fn decode(shown: &str, bytes: &[u8]) -> Result<TargetProfile, Vec<ContentError>> {
    let file = AssetPath::new(shown);
    TargetRon.decode(&file, bytes, &DecodeCx::new())
}

/// The engine's profiles, by id.
pub fn engine_profiles() -> Result<BTreeMap<String, Profile>, Vec<ContentError>> {
    let files = TARGETS.files().map(|f| {
        let shown = format!("targets/{}", f.path().display());
        (shown, f.contents().to_vec())
    });
    read_all(files)
}

/// Whether the file shown as `shown` is a profile's: a `.ron` file.
fn is_profile(shown: &str) -> bool {
    AssetPath::new(shown)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ron"))
}

/// The profiles of `files` (each a shown name and its bytes; only `.ron`
/// ones are read), by id, with every error of every file.
fn read_all(
    files: impl Iterator<Item = (String, Vec<u8>)>,
) -> Result<BTreeMap<String, Profile>, Vec<ContentError>> {
    let mut out = BTreeMap::new();
    let mut errors = Vec::new();
    for (shown, bytes) in files.filter(|(shown, _)| is_profile(shown)) {
        match decode(&shown, &bytes) {
            Ok(profile) => {
                out.insert(
                    profile.id.clone(),
                    Profile {
                        file: shown,
                        profile,
                    },
                );
            }
            Err(e) => errors.extend(e),
        }
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// The engine's profiles, with the game's of `config` (from `root`) added
/// or in place of the engine's of the same id.
pub fn profiles(
    config: &Config,
    root: &Path,
) -> Result<BTreeMap<String, Profile>, Vec<ContentError>> {
    let mut all = engine_profiles()?;
    let Some(dir) = &config.target_profiles else {
        return Ok(all);
    };
    let entries = fs::read_dir(root.join(dir))
        .map_err(|e| vec![ContentError::new(dir.as_str(), e.to_string())])?;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut errors = Vec::new();
    for entry in entries.flatten() {
        let shown = format!("{dir}/{}", entry.file_name().to_string_lossy());
        if !is_profile(&shown) {
            continue;
        }
        match fs::read(entry.path()) {
            Ok(bytes) => files.push((shown, bytes)),
            Err(e) => errors.push(ContentError::new(shown, e.to_string())),
        }
    }
    files.sort();
    match read_all(files.into_iter()) {
        Ok(game) if errors.is_empty() => {
            all.extend(game);
            Ok(all)
        }
        Ok(_) => Err(errors),
        Err(e) => {
            errors.extend(e);
            Err(errors)
        }
    }
}

/// The scale a frame is shown at, as a fraction: `num / den`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Factor {
    /// Its numerator.
    pub num: u32,
    /// Its denominator, never 0.
    pub den: u32,
}

impl Factor {
    /// The whole number `n`.
    pub const fn whole(n: u32) -> Self {
        Self { num: n, den: 1 }
    }

    /// `px` times this, rounded down.
    pub fn of(self, px: u16) -> u32 {
        u32::from(px) * self.num / self.den
    }

    /// The smaller of two.
    fn min(self, other: Self) -> Self {
        // a/b < c/d when a·d < c·b; the numbers are u16 products.
        if u64::from(self.num) * u64::from(other.den) <= u64::from(other.num) * u64::from(self.den)
        {
            self
        } else {
            other
        }
    }

    /// The factor as messages show it: `2`, or `1.60` rounded down.
    pub fn shown(self) -> String {
        if self.num.is_multiple_of(self.den) {
            return (self.num / self.den).to_string();
        }
        let hundredths = u64::from(self.num) * 100 / u64::from(self.den);
        format!("{}.{:02}", hundredths / 100, hundredths % 100)
    }
}

/// The scale `scale` gives frame `(width, height)` on `space` (the
/// display inside its safe area): for [`Scale::Integer`], the largest
/// whole number that fits, which is 0 when even 1 doesn't; for
/// [`Scale::Fit`], the largest factor that fits.
pub fn scale_on(scale: Scale, (width, height): (u16, u16), space: &Size) -> Factor {
    let across = Factor {
        num: u32::from(space.width),
        den: u32::from(width.max(1)),
    };
    let down = Factor {
        num: u32::from(space.height),
        den: u32::from(height.max(1)),
    };
    let fit = across.min(down);
    match scale {
        Scale::Integer => Factor::whole(fit.num / fit.den),
        Scale::Fit => fit,
    }
}

/// The game's `setting` for target `id` checked against `profile`: every
/// problem, display size by display size. A display of any size (a
/// window) shows the frame at scale 1.
pub fn check_target(
    id: &str,
    setting: &TargetSetting,
    profile: &TargetProfile,
) -> Vec<TargetProblem> {
    let display = &profile.display;
    let key = format!("targets.\"{id}\"");
    let mut out = Vec::new();
    let (width, height) = setting.frame;
    let fixed: Vec<Option<&Size>> = if display.sizes.is_empty() {
        vec![None]
    } else {
        display.sizes.iter().map(Some).collect()
    };
    for size in fixed {
        let factor = match size {
            None => Factor::whole(1),
            Some(size) => {
                let space = size.inside(display.safe_area);
                let factor = scale_on(setting.scale, setting.frame, &space);
                if factor.num == 0 {
                    let place = if space == *size {
                        format!("the display {}", size.name())
                    } else {
                        format!(
                            "the safe area {} × {} of the display {}",
                            space.width,
                            space.height,
                            size.name()
                        )
                    };
                    out.push(TargetProblem {
                        level: Level::Error,
                        check: TargetCheck::Fits,
                        setting: format!("{key}.frame"),
                        message: format!(
                            "frame {width} × {height} at scale 1 is larger than {place}"
                        ),
                    });
                    // The text is checked as the frame would show if it
                    // fitted.
                    Factor::whole(1)
                } else {
                    factor
                }
            }
        };
        let shown = factor.of(setting.text_px);
        let on = size.map_or_else(String::new, |size| format!(" on {}", size.name()));
        let text = format!(
            "text {} px at scale {} is {shown} px{on}",
            setting.text_px,
            factor.shown()
        );
        let problem = if shown < u32::from(display.min_text_px) {
            Some((
                Level::Error,
                format!(
                    "{text}, below the target's minimum of {} px",
                    display.min_text_px
                ),
            ))
        } else if shown < u32::from(display.good_text_px) {
            Some((
                Level::Warning,
                format!(
                    "{text}, below the {} px the target recommends",
                    display.good_text_px
                ),
            ))
        } else {
            None
        };
        if let Some((level, message)) = problem {
            out.push(TargetProblem {
                level,
                check: TargetCheck::Text,
                setting: format!("{key}.text_px"),
                message,
            });
        }
    }
    out
}

/// The targets `target` asks for in `config`: itself, or every one with
/// [`ALL`]; an error if `shuyi.ron` doesn't list it.
pub fn asked<'a>(
    config: &'a Config,
    target: &str,
) -> Result<Vec<(&'a str, &'a TargetSetting)>, String> {
    if target == ALL {
        if config.targets.is_empty() {
            return Err(format!("{FILE} lists no targets (`targets`)"));
        }
        return Ok(config
            .targets
            .iter()
            .map(|(id, s)| (id.as_str(), s))
            .collect());
    }
    config
        .targets
        .get_key_value(target)
        .map(|(id, s)| vec![(id.as_str(), s)])
        .ok_or_else(|| {
            let listed: Vec<&str> = config.targets.keys().map(String::as_str).collect();
            let listed = if listed.is_empty() {
                "none".to_owned()
            } else {
                listed.join(", ")
            };
            format!(
                "{FILE}'s `targets` has no \"{target}\" (it lists {listed}): a game lists the targets it ships on"
            )
        })
}

#[cfg(test)]
mod tests;
