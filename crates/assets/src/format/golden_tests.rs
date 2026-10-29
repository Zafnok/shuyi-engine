//! The formats' golden table (ticket 2116, ADR-0007 §5): what each format
//! decodes from a fixture, as its `state_hash`, pinned in
//! `tests/golden/formats.ron` and checked on x64, ARM64 and wasm32. One row
//! per format, `<kind>: <format id>`, and a guard that every standard
//! format of a covered kind has one. The fixtures are `builtin_tests.rs`'s,
//! frozen for the table.
use alloc::collections::BTreeSet;

use serde::Serialize;
use shuyi_core::state_hash;

use super::builtin_tests::{
    ATLAS, CREDITS, FILM, LIGHTS, MANIFEST, MAP, NAMES, PALETTE, PATTERN_MAP, SCRIPT, Shot, TABLE,
    Table, bundle, lang_pack, map_text, ogg, ogg_file, pack_files, png_file, wav,
};
use super::*;
use crate::audio::{
    self, AudioManifest, AudioManifestRon, MusicTrack, OggVorbis, OggVorbisTrack, Sound, Wav,
};
use crate::bitmap::OneBitInfo;
use crate::cinematic::{CinematicFile, CinematicRon};
use crate::credits::{CreditsFile, CreditsRon};
use crate::dialogue::tests::Cue;
use crate::dialogue::{DialogueFile, Dlg, NoDirective};
use crate::font::{FontAtlasDef, FontAtlasRon};
use crate::golden::Golden;
use crate::grid_map::{GridMap, NoFeature};
use crate::image::{Image, ImageInfo, Png, PngInfo, png_header};
use crate::keymap::{KeymapDef, KeymapRon, sample};
use crate::lang::LangPack;
use crate::lighting::{Lighting, LightingRon};
use crate::names::{Names, NamesRon};
use crate::onebit::{PatternMapDef, PatternMapRon};
use crate::palette::{PaletteDef, PaletteRon};
use crate::ron::Ron;
use crate::target::{TargetProfile, TargetRon};

/// The table and the names of the rows checked so far, for the guard.
struct Rows {
    golden: Golden,
    names: BTreeSet<String>,
}

impl Rows {
    /// Checks the row of `format`, `<kind>: <format id>`: the `state_hash`
    /// of what it decodes from `bytes` at `path`, beside `files` (no
    /// bundle if `None`). The fixture must decode.
    fn format<F: Format>(&mut self, format: &F, path: &str, bytes: &[u8], files: Option<&Bundle>)
    where
        F::Asset: Serialize,
    {
        let name = format!("{}: {}", <F::Asset as AssetKind>::NAME, format.id());
        let (file, cx) = match files {
            Some(files) => (
                AssetPath::in_bundle(files, path),
                DecodeCx::in_bundle(files),
            ),
            None => (AssetPath::new(path), DecodeCx::new()),
        };
        let asset = format
            .decode(&file, bytes, &cx)
            .unwrap_or_else(|e| panic!("{name}: its fixture doesn't decode: {e:?}"));
        self.row(name, &asset);
    }

    /// Checks the row `name`: the `state_hash` of `value`.
    fn row(&mut self, name: String, value: &impl Serialize) {
        let hash = state_hash(value).unwrap_or_else(|e| panic!("{name}: {e}"));
        self.names.insert(name.clone());
        self.golden.check(name, hash);
    }
}

/// The rows `formats` needs and `rows` lacks: for each kind the table
/// covers, a row `<kind>: <id>` for every id `formats` lists. So a new
/// standard format of a covered kind fails until its row is added.
fn missing_rows(formats: &Formats, rows: &BTreeSet<String>) -> Vec<String> {
    fn kind<K: AssetKind>(formats: &Formats, rows: &BTreeSet<String>, missing: &mut Vec<String>) {
        for id in formats.ids::<K>() {
            let row = format!("{}: {id}", K::NAME);
            if !rows.contains(&row) {
                missing.push(row);
            }
        }
    }
    let mut missing = Vec::new();
    kind::<ImageInfo>(formats, rows, &mut missing);
    kind::<Image>(formats, rows, &mut missing);
    kind::<Sound>(formats, rows, &mut missing);
    kind::<MusicTrack>(formats, rows, &mut missing);
    kind::<AudioManifest>(formats, rows, &mut missing);
    kind::<CreditsFile>(formats, rows, &mut missing);
    kind::<FontAtlasDef>(formats, rows, &mut missing);
    kind::<PaletteDef>(formats, rows, &mut missing);
    kind::<PatternMapDef>(formats, rows, &mut missing);
    kind::<Lighting>(formats, rows, &mut missing);
    kind::<Names>(formats, rows, &mut missing);
    kind::<DialogueFile<NoDirective>>(formats, rows, &mut missing);
    kind::<DialogueFile<Cue>>(formats, rows, &mut missing);
    kind::<TargetProfile>(formats, rows, &mut missing);
    kind::<GridMap<u8, NoFeature>>(formats, rows, &mut missing);
    kind::<KeymapDef<sample::Action>>(formats, rows, &mut missing);
    kind::<CinematicFile<Shot>>(formats, rows, &mut missing);
    kind::<LangPack>(formats, rows, &mut missing);
    kind::<Table>(formats, rows, &mut missing);
    missing
}

/// Checks every row of `formats.ron` but the guard's, and returns them.
fn check_rows() -> Rows {
    let mut rows = Rows {
        golden: crate::golden!("tests/golden/formats.ron"),
        names: BTreeSet::new(),
    };
    let rate = audio::SAMPLE_RATE;
    let picture = png_file([255, 0, 0, 255, 0, 0, 255, 127]);
    rows.format(&PngInfo, "a.png", &picture, None);
    rows.format(&Png, "a.png", &picture, None);
    rows.format(&OneBitInfo, "a.png", &crate::bitmap::tests::sample(), None);
    rows.format(&Wav, "a.wav", &wav(1, rate, 1), None);
    rows.format(&OggVorbis, "a.ogg", &ogg(2, rate), None);
    rows.format(&OggVorbisTrack, "a.ogg", &ogg_file(rate, 88_200), None);
    let sounds = bundle(vec![
        ("audio/audio.ron", MANIFEST.into()),
        ("audio/sfx/gone.wav", wav(1, rate, 1)),
        ("audio/sfx/tick.wav", wav(1, rate, 1)),
        ("audio/sfx/trim.wav", wav(1, rate, 1)),
    ]);
    rows.format(
        &AudioManifestRon,
        "audio/audio.ron",
        MANIFEST.as_bytes(),
        Some(sounds),
    );
    rows.format(
        &CreditsRon::default(),
        "credits.ron",
        CREDITS.as_bytes(),
        None,
    );
    let font = bundle(vec![
        ("fonts/atlas.ron", ATLAS.into()),
        ("fonts/atlas.png", png_header(128, 16)),
    ]);
    rows.format(
        &FontAtlasRon::default(),
        "fonts/atlas.ron",
        ATLAS.as_bytes(),
        Some(font),
    );
    rows.format(&PaletteRon::default(), "p.ron", PALETTE.as_bytes(), None);
    rows.format(&PatternMapRon, "p@onebit.ron", PATTERN_MAP.as_bytes(), None);
    rows.format(&LightingRon::default(), "l.ron", LIGHTS.as_bytes(), None);
    rows.format(&NamesRon, "names.ron", NAMES.as_bytes(), None);
    rows.format(
        &Dlg::<Cue>::new(),
        "story/gate.dlg",
        SCRIPT.as_bytes(),
        None,
    );
    let profile = crate::target::tests::profile("small", "");
    rows.format(&TargetRon, "targets/small.ron", profile.as_bytes(), None);
    rows.format(&map_text(), "maps/field.map", MAP.as_bytes(), None);
    let keymap = KeymapRon::<sample::Action>::new();
    rows.format(&keymap, "keymap.ron", sample::SOURCE.as_bytes(), None);
    rows.format(
        &CinematicRon::<Shot>::new(),
        "films/title.ron",
        FILM.as_bytes(),
        None,
    );
    let pack = bundle(pack_files().into());
    let info = pack.bytes("lang/xx/lang.ron").unwrap_or_default();
    rows.format(&lang_pack(), "lang/xx/lang.ron", info, Some(pack));
    rows.format(&Ron::<Table>::new(), "t.ron", TABLE.as_bytes(), None);
    // The fixed bundle's fingerprint (ticket 2111), which a game's stamps
    // carry.
    let fingerprint = crate::bundle::fixed_fingerprint_bundle().fingerprint();
    rows.names.insert("fingerprint".to_owned());
    rows.golden.check("fingerprint", fingerprint);
    rows
}

#[test]
fn every_format_decodes_its_golden_value() {
    let rows = check_rows();
    let missing = missing_rows(&Formats::standard(), &rows.names);
    assert!(
        missing.is_empty(),
        "standard formats with no golden row: {missing:?}"
    );
    rows.golden.assert();
}

/// A toy format of a covered kind (palettes).
struct ToyPalette;

impl Format for ToyPalette {
    type Asset = PaletteDef;

    fn id(&self) -> &'static str {
        "toy"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["toy"]
    }

    fn decode(
        &self,
        _file: &AssetPath,
        _bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<PaletteDef, Vec<ContentError>> {
        Ok(PaletteDef::default())
    }
}

#[test]
fn the_golden_guard_fails_on_a_format_of_a_covered_kind_with_no_row() {
    let rows = check_rows();
    assert_eq!(
        missing_rows(&Formats::standard(), &rows.names),
        Vec::<String>::new()
    );
    let mut formats = Formats::standard();
    assert_eq!(formats.register(ToyPalette), Ok(()));
    assert_eq!(missing_rows(&formats, &rows.names), ["palette: toy"]);
}
