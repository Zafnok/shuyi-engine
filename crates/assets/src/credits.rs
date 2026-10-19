//! The credits: every third-party work a game uses, for its credits
//! screen. Music and sounds are credited in the audio manifest
//! ([`crate::audio`]); everything else (fonts, bought art, vendored
//! software) in a credits file. [`merge`] makes the two one list, so
//! nothing is typed twice.
//!
//! A bought work names the files of the game's private folder it covers
//! (the bought art, kept out of the public repository). It is shown only
//! in a build that has files there, and [`uncredited`] finds the bought
//! files no credit covers (game ADR-0051).
use crate::prelude::*;

use alloc::collections::BTreeSet;

use serde::Deserialize;

use crate::audio::{self, AudioManifest};
use crate::error::{ContentError, line_of};
use crate::ron::parse_ron;

/// Licenses an entry of the credits file may have, unless it is a bought
/// work ([`FileCredit::private`]): game ADR-0013's code and font lists.
pub const LICENSES: [&str; 15] = [
    "MIT",
    "MIT-0",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "0BSD",
    "ISC",
    "Zlib",
    "BSL-1.0",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "Unlicense",
    "OFL-1.1",
];

/// The kinds of work the credits screen groups by, in the order it shows
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub enum CreditGroup {
    /// Music tracks (from the audio manifest's music cues).
    Music,
    /// Sound effects (from the audio manifest's sound cues).
    SoundEffects,
    /// Pictures: portraits, sprites, tiles.
    Art,
    /// Fonts.
    Fonts,
    /// Vendored code and data tables.
    Software,
}

impl CreditGroup {
    /// Every group, in display order.
    pub const ALL: [Self; 5] = [
        Self::Music,
        Self::SoundEffects,
        Self::Art,
        Self::Fonts,
        Self::Software,
    ];

    /// Whether the group's credits come from the audio manifest, so the
    /// credits file may not use it.
    pub fn is_audio(self) -> bool {
        matches!(self, Self::Music | Self::SoundEffects)
    }
}

/// One entry of the credits file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCredit {
    /// Stable id, unique in the file.
    pub id: String,
    /// What kind of work it is; not an audio group.
    pub group: CreditGroup,
    /// The work's title.
    pub title: String,
    /// Its author, as they want to be credited.
    pub author: String,
    /// The page it came from: the link in its `THIRD_PARTY_ASSETS.md` row.
    pub source: String,
    /// One of [`LICENSES`]; for a bought work, `Custom (<seller>)`.
    pub license: String,
    /// Kept off the credits screen: plumbing the player never sees.
    #[serde(default)]
    pub hidden: bool,
    /// For a bought work: the files of the private folder it covers, each
    /// a path there or the start of one (`units/`). Empty for any other
    /// work.
    #[serde(default)]
    pub private: Vec<String>,
}

impl FileCredit {
    /// Whether the credit covers the bought file at `path` (relative to
    /// the private folder).
    pub fn covers(&self, path: &str) -> bool {
        self.private.iter().any(|p| path.starts_with(p.as_str()))
    }
}

/// The note in the root of the private folder: ours, so it needs no
/// credit.
const PRIVATE_README: &str = "README.md";

/// Whether `license` is how a bought work's is written: `Custom (<seller>)`.
fn is_custom(license: &str) -> bool {
    let seller = license
        .strip_prefix("Custom (")
        .and_then(|rest| rest.strip_suffix(')'));
    seller.is_some_and(|s| !s.trim().is_empty())
}

/// The bought files among `paths` (relative to the private folder) that
/// no credit of `file` covers. Each needs one before it ships
/// (game ADR-0051).
pub fn uncredited<'a>(file: &CreditsFile, paths: &[&'a str]) -> Vec<&'a str> {
    let covered = |path: &str| file.credits.iter().any(|c| c.covers(path));
    let needs_one = paths.iter().filter(|p| **p != PRIVATE_README);
    needs_one.copied().filter(|p| !covered(p)).collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    credits: Vec<FileCredit>,
}

/// The validated credits file, in file order, hidden entries included.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreditsFile {
    /// The entries.
    pub credits: Vec<FileCredit>,
}

/// One work as the credits screen shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditEntry {
    /// The heading it goes under.
    pub group: CreditGroup,
    /// The work's title.
    pub title: String,
    /// Its author.
    pub author: String,
    /// Its license (an SPDX id, e.g. `CC-BY-4.0`).
    pub license: String,
    /// The page it came from.
    pub source: String,
}

/// Every credit the screen shows: sorted by group ([`CreditGroup::ALL`]),
/// audio works by title within their group, the file's in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Credits {
    /// The entries, in display order.
    pub entries: Vec<CreditEntry>,
}

impl Credits {
    /// The entries under `group`, in display order.
    pub fn in_group(&self, group: CreditGroup) -> impl Iterator<Item = &CreditEntry> {
        self.entries.iter().filter(move |e| e.group == group)
    }
}

/// Where the game keeps what a credits file's errors point to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Places<'a> {
    /// The audio manifest, as errors show it (`base/audio/audio.ron`):
    /// where music and sound credits go.
    pub audio_manifest: &'a str,
    /// The private folder of bought files, as errors show it
    /// (`private`).
    pub private_root: &'a str,
}

/// Parses and validates credits `source`, attributing errors to `file`
/// and naming the game's other files by `places`. Reports every problem
/// found.
pub fn from_source(
    file: &str,
    source: &str,
    places: &Places<'_>,
) -> Result<CreditsFile, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    for c in &raw.credits {
        let mut err = |message: String| {
            let e = ContentError::new(file, format!("credit \"{}\": {message}", c.id));
            errors.push(match line_of(source, &format!("id: \"{}\"", c.id)) {
                Some(l) => e.at(l, None),
                None => e,
            });
        };
        if c.id.is_empty() {
            err("the id is empty".into());
        }
        if !ids.insert(c.id.as_str()) {
            err("the id is used twice".into());
        }
        if c.group.is_audio() {
            err(format!(
                "{:?} credits go in {}",
                c.group, places.audio_manifest
            ));
        }
        for (field, value) in [("title", &c.title), ("author", &c.author)] {
            if value.trim().is_empty() {
                err(format!("{field} is missing"));
            }
        }
        if !c.source.starts_with("https://") && !c.source.starts_with("http://") {
            err("source must be a web link (http:// or https://)".into());
        }
        if !c.private.is_empty() {
            if !is_custom(&c.license) {
                err(format!(
                    "license \"{}\": a bought work's is written \"Custom (<seller>)\"",
                    c.license
                ));
            }
        } else if !LICENSES.contains(&c.license.as_str()) {
            err(format!(
                "license \"{}\" is not allowed (ADR-0013: {})",
                c.license,
                LICENSES.join(", ")
            ));
        }
        for path in &c.private {
            if path.is_empty() || path.starts_with('/') {
                err(format!(
                    "private path \"{path}\" must be a path inside {}/",
                    places.private_root
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(CreditsFile {
            credits: raw.credits,
        })
    } else {
        Err(errors)
    }
}

/// The credits the screen shows: the third-party works `audio`'s music
/// cues name, then those its sound cues name (each once, by title; a work
/// used for both counts as music), then `file`'s entries that aren't
/// hidden. Our own work ([`audio::OWN`]) isn't listed, and a bought work
/// only when one of `bought` (the build's files from the private folder)
/// is its: a build without the art doesn't use it.
pub fn merge(file: &CreditsFile, audio: &AudioManifest, bought: &[&str]) -> Credits {
    let music = audio.music.values().map(|m| &m.credit);
    let sounds = audio.sounds.values().map(|s| &s.credit);
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for (group, refs) in [
        (CreditGroup::Music, music.collect::<Vec<_>>()),
        (CreditGroup::SoundEffects, sounds.collect()),
    ] {
        let from = entries.len();
        for id in refs.into_iter().flat_map(audio::CreditRef::ids) {
            let Some(c) = audio.credits.get(id) else {
                continue;
            };
            if c.license != audio::OWN && seen.insert(id) {
                entries.push(CreditEntry {
                    group,
                    title: c.title.clone(),
                    author: c.author.clone(),
                    license: c.license.clone(),
                    source: c.source.clone(),
                });
            }
        }
        entries[from..].sort_by_key(|e| e.title.to_lowercase());
    }
    let in_build = |c: &FileCredit| c.private.is_empty() || bought.iter().any(|p| c.covers(p));
    let shown = file.credits.iter().filter(|c| !c.hidden && in_build(c));
    entries.extend(shown.map(|c| CreditEntry {
        group: c.group,
        title: c.title.clone(),
        author: c.author.clone(),
        license: c.license.clone(),
        source: c.source.clone(),
    }));
    // Stable: keeps the orders above within each group.
    entries.sort_by_key(|e| e.group);
    Credits { entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Credit, CreditRef, MusicCue, SoundCue};

    const FILE: &str = r#"(credits: [
    (id: "font", group: Fonts, title: "A Font", author: "Ann",
     source: "https://example.org/font", license: "OFL-1.1"),
    (id: "loader", group: Software, title: "Loader", author: "Bob",
     source: "http://example.org/loader", license: "MIT", hidden: true),
    (id: "table", group: Software, title: "Table", author: "Cy",
     source: "https://example.org/table", license: "Zlib"),
])"#;

    const PLACES: Places<'static> = Places {
        audio_manifest: "base/audio/audio.ron",
        private_root: "private",
    };

    fn from_source(file: &str, source: &str) -> Result<CreditsFile, Vec<ContentError>> {
        super::from_source(file, source, &PLACES)
    }

    fn errors(source: &str) -> Vec<String> {
        from_source("c.ron", source)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn credit(title: &str, license: &str) -> Credit {
        Credit {
            title: title.into(),
            author: format!("{title}'s author"),
            source: format!("https://example.org/{title}"),
            license: license.into(),
            ..Credit::default()
        }
    }

    fn music(credit: CreditRef) -> MusicCue {
        MusicCue {
            file: "a.ogg".into(),
            volume: 100,
            looped: true,
            length_ms: 1000,
            credit,
        }
    }

    fn sound(credit: CreditRef) -> SoundCue {
        SoundCue {
            files: vec!["a.ogg".into()],
            volume: 100,
            credit,
        }
    }

    /// Two tracks, three sounds (one ours, one sharing a track's work).
    fn manifest() -> AudioManifest {
        let one = |id: &str| CreditRef::Credit(id.into());
        let mut m = AudioManifest::default();
        for (id, c) in [
            ("zeta", credit("Zeta", "CC0-1.0")),
            ("alpha", credit("alpha", "CC-BY-4.0")),
            ("step", credit("Step", "CC-BY-3.0")),
            ("clang", credit("Clang", "CC0-1.0")),
            ("ours", credit("", audio::OWN)),
            ("unused", credit("Unused", "CC0-1.0")),
        ] {
            m.credits.insert(id.into(), c);
        }
        m.music.insert("a".into(), music(one("zeta")));
        m.music.insert("b".into(), music(one("alpha")));
        m.music.insert("c".into(), music(one("zeta")));
        m.music.insert("d".into(), music(CreditRef::Own));
        m.music.insert("e".into(), music(one("ours")));
        m.sounds.insert(
            "s".into(),
            sound(CreditRef::Credits(vec!["step".into(), "clang".into()])),
        );
        m.sounds.insert("t".into(), sound(one("zeta")));
        m.sounds.insert("u".into(), sound(CreditRef::Own));
        m.sounds.insert("v".into(), sound(one("missing")));
        m
    }

    fn shown(credits: &Credits) -> Vec<(CreditGroup, &str)> {
        credits
            .entries
            .iter()
            .map(|e| (e.group, e.title.as_str()))
            .collect()
    }

    #[test]
    fn a_good_file_loads_in_file_order() {
        let file = from_source("c.ron", FILE).unwrap_or_default();
        let ids: Vec<&str> = file.credits.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["font", "loader", "table"]);
        let hidden: Vec<bool> = file.credits.iter().map(|c| c.hidden).collect();
        assert_eq!(hidden, [false, true, false]);
        assert_eq!(file.credits[0].group, CreditGroup::Fonts);
        assert_eq!(file.credits[0].license, "OFL-1.1");
        assert_eq!(
            from_source("c.ron", "(credits: [])"),
            Ok(CreditsFile::default())
        );
    }

    #[test]
    fn bad_entries_are_reported_with_their_line() {
        let dup = FILE.replace(r#"id: "table""#, r#"id: "font""#);
        assert_eq!(
            errors(&dup),
            ["c.ron:2: credit \"font\": the id is used twice"]
        );
        let empty = FILE.replace(r#"id: "table""#, r#"id: """#);
        assert_eq!(errors(&empty), ["c.ron:6: credit \"\": the id is empty"]);
        let bare = FILE
            .replace(
                r#"title: "Table", author: "Cy""#,
                r#"title: " ", author: """#,
            )
            .replace("https://example.org/table", "example.org/table");
        assert_eq!(
            errors(&bare),
            [
                "c.ron:6: credit \"table\": title is missing",
                "c.ron:6: credit \"table\": author is missing",
                "c.ron:6: credit \"table\": source must be a web link (http:// or https://)",
            ]
        );
        let gpl = FILE.replace("\"Zlib\"", "\"GPL-3.0\"");
        assert_eq!(
            errors(&gpl),
            [format!(
                "c.ron:6: credit \"table\": license \"GPL-3.0\" is not allowed (ADR-0013: {})",
                LICENSES.join(", ")
            )]
        );
    }

    #[test]
    fn audio_groups_belong_to_the_audio_manifest() {
        assert_eq!(
            errors(&FILE.replace("group: Fonts", "group: Music")),
            ["c.ron:2: credit \"font\": Music credits go in base/audio/audio.ron"]
        );
        assert_eq!(
            errors(&FILE.replace("group: Fonts", "group: SoundEffects")),
            ["c.ron:2: credit \"font\": SoundEffects credits go in base/audio/audio.ron"]
        );
        let audio: Vec<bool> = CreditGroup::ALL.iter().map(|g| g.is_audio()).collect();
        assert_eq!(audio, [true, true, false, false, false]);
    }

    #[test]
    fn syntax_and_unknown_fields_are_refused() {
        assert_eq!(errors("(credits: [").len(), 1);
        assert_eq!(errors(&FILE.replace("hidden: true", "hide: true")).len(), 1);
        assert_eq!(
            errors(&FILE.replace("group: Fonts", "group: Pics")).len(),
            1
        );
    }

    #[test]
    fn merging_groups_sorts_and_drops_what_is_not_shown() {
        use CreditGroup::{Fonts, Music, Software, SoundEffects};
        let file = from_source("c.ron", FILE).unwrap_or_default();
        let credits = merge(&file, &manifest(), &[]);
        // Music by title whatever its case, each work once; a work used by
        // a track and a sound is music; ours, the unused credit, the
        // unknown id and the hidden entry aren't shown.
        assert_eq!(
            shown(&credits),
            [
                (Music, "alpha"),
                (Music, "Zeta"),
                (SoundEffects, "Clang"),
                (SoundEffects, "Step"),
                (Fonts, "A Font"),
                (Software, "Table"),
            ]
        );
        let first = &credits.entries[0];
        assert_eq!(first.author, "alpha's author");
        assert_eq!(first.license, "CC-BY-4.0");
        assert_eq!(first.source, "https://example.org/alpha");
        let font = credits.in_group(Fonts).collect::<Vec<_>>();
        assert_eq!(font.len(), 1);
        assert_eq!(
            (font[0].author.as_str(), font[0].license.as_str()),
            ("Ann", "OFL-1.1")
        );
        assert_eq!(font[0].source, "https://example.org/font");
        assert_eq!(credits.in_group(Music).count(), 2);
    }

    #[test]
    fn file_entries_keep_their_order_and_sort_under_their_group() {
        use CreditGroup::{Fonts, Software};
        // Software before Fonts in the file: Fonts still shows first, and
        // two entries of one group stay in file order.
        let source = r#"(credits: [
            (id: "z", group: Software, title: "Zed", author: "a", source: "https://z", license: "MIT"),
            (id: "b", group: Software, title: "Bee", author: "a", source: "https://b", license: "MIT"),
            (id: "f", group: Fonts, title: "Font", author: "a", source: "https://f", license: "MIT"),
        ])"#;
        let file = from_source("c.ron", source).unwrap_or_default();
        let credits = merge(&file, &AudioManifest::default(), &[]);
        assert_eq!(
            shown(&credits),
            [(Fonts, "Font"), (Software, "Zed"), (Software, "Bee")]
        );
    }

    /// [`FILE`] with a bought pack between the font and the loader.
    fn with_bought() -> String {
        let pack = r#"    (id: "pack", group: Art, title: "Pack", author: "Dee",
     source: "https://example.org/pack", license: "Custom (Dee)",
     private: ["units/", "tilesets/pack.ron"]),
    (id: "loader""#;
        FILE.replace(r#"    (id: "loader""#, pack)
    }

    #[test]
    fn a_bought_work_names_its_private_files_and_its_sellers_license() {
        let file = from_source("c.ron", &with_bought()).unwrap_or_default();
        assert_eq!(file.credits[1].private, ["units/", "tilesets/pack.ron"]);
        assert_eq!(file.credits[1].group, CreditGroup::Art);
        assert!(file.credits[0].private.is_empty());
        // An open licence isn't a bought work's, and a custom one is only
        // a bought work's.
        for bad in [
            "MIT",
            "Custom",
            "Custom ()",
            "Custom ( )",
            "Custom (Dee",
            "(Dee)",
        ] {
            assert_eq!(
                errors(&with_bought().replace("Custom (Dee)", bad)),
                [format!(
                    "c.ron:4: credit \"pack\": license \"{bad}\": a bought work's is written \
                     \"Custom (<seller>)\""
                )]
            );
        }
        let custom = FILE.replace("\"Zlib\"", "\"Custom (Cy)\"");
        assert_eq!(errors(&custom).len(), 1);
        assert!(errors(&custom)[0].contains("is not allowed (ADR-0013: "));
        for bad in ["", "/units/"] {
            assert_eq!(
                errors(&with_bought().replace("\"units/\"", &format!("\"{bad}\""))),
                [format!(
                    "c.ron:4: credit \"pack\": private path \"{bad}\" must be a path inside \
                     private/"
                )]
            );
        }
    }

    #[test]
    fn a_credit_covers_the_files_under_its_private_paths() {
        let file = from_source("c.ron", &with_bought()).unwrap_or_default();
        let pack = &file.credits[1];
        assert!(pack.covers("units/a.png"));
        assert!(pack.covers("tilesets/pack.ron"));
        assert!(!pack.covers("tilesets/other.ron"));
        assert!(!pack.covers("portraits/units/a.png"));
        assert!(!file.credits[0].covers("units/a.png"));
        let paths = [
            "README.md",
            "portraits/README.md",
            "portraits/a.png",
            "tilesets/pack.ron",
            "units/a.png",
        ];
        assert_eq!(
            uncredited(&file, &paths),
            ["portraits/README.md", "portraits/a.png"]
        );
        assert_eq!(
            uncredited(&CreditsFile::default(), &paths),
            paths[1..].to_vec()
        );
        assert!(uncredited(&file, &[]).is_empty());
    }

    #[test]
    fn a_bought_work_is_shown_only_in_a_build_that_has_its_files() {
        use CreditGroup::{Art, Fonts, Software};
        let file = from_source("c.ron", &with_bought()).unwrap_or_default();
        let none = AudioManifest::default();
        let without = [(Fonts, "A Font"), (Software, "Table")];
        assert_eq!(shown(&merge(&file, &none, &[])), without);
        assert_eq!(shown(&merge(&file, &none, &["portraits/a.png"])), without);
        let with = merge(&file, &none, &["portraits/a.png", "units/a.png"]);
        assert_eq!(
            shown(&with),
            [(Art, "Pack"), (Fonts, "A Font"), (Software, "Table")]
        );
        assert_eq!(with.entries[0].license, "Custom (Dee)");
        // Hidden stays hidden, with its files or not.
        let hidden = with_bought().replace("private: [", "hidden: true, private: [");
        let file = from_source("c.ron", &hidden).unwrap_or_default();
        assert_eq!(shown(&merge(&file, &none, &["units/a.png"])), without);
    }
}
