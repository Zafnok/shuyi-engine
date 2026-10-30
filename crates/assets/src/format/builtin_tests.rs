//! The built-in formats' tests (ADR-0007): each has its id, extensions,
//! magic bytes and limit; each reads what the parser it wraps reads, with
//! the files it names from the bundle; and no bytes make one panic (§5),
//! from random bytes to its own fixtures with a few characters changed.
use alloc::borrow::Cow;
use std::sync::OnceLock;

use proptest::prelude::*;
use serde::{Deserialize, Serialize};

use super::*;
use crate::audio::{
    self, AudioManifest, AudioManifestRon, MAX_SOUND_BYTES, MusicTrack, OggVorbis, OggVorbisTrack,
    Sound, Wav, format_problem,
};
use crate::bitmap::{self, MAGIC, OneBitInfo};
use crate::bundle::Layer;
use crate::cinematic::{self, CinematicRon, ShotCheck};
use crate::credits::{self, CreditsRon, Places};
use crate::dialogue::tests::{Cue, HeroTokens, LIMITS};
use crate::dialogue::{
    CharacterId, DialogueFile, DialogueLimits, DialogueTable, Dlg, NoDirective, NoTokens,
    SceneRules, Tokens, parse_dlg, scripts_from_parsed, scripts_from_sources,
};
use crate::font::{FontAtlasDef, FontAtlasRon};
use crate::grid_map::{self, MapFormat, MapText, NoFeature};
use crate::image::{
    Image, ImageId, ImageInfo, ImageTable, MAX_PNG_BYTES, MAX_SIDE, PNG_SIGNATURE, Png, PngInfo,
    png_header,
};
use crate::keymap::{KeymapDef, KeymapRon, sample};
use crate::lang::{self, Lang, LangCode, LangFile, LangPack, LangPackRon, PackRules, SourceText};
use crate::lighting::{self, LightingRon};
use crate::names::{self, Names, NamesRon};
use crate::onebit::{PatternMapDef, PatternMapRon};
use crate::palette::{PaletteDef, PaletteRon};
use crate::ron::{Ron, parse_ron};

/// The errors of `result`, each with its file and place.
fn shown<T>(result: Result<T, Vec<ContentError>>) -> Vec<String> {
    result
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// What `format` reads from `bytes` at `path`, with no bundle.
fn alone<F: Format>(format: &F, path: &str, bytes: &[u8]) -> Result<F::Asset, Vec<ContentError>> {
    format.decode(&AssetPath::new(path), bytes, &DecodeCx::new())
}

/// A bundle of `files`, shown under `base`, kept for the rest of the run.
pub(crate) fn bundle(files: Vec<(&'static str, Vec<u8>)>) -> &'static Bundle {
    let mut table: Vec<(&'static str, &'static [u8])> = files
        .into_iter()
        .map(|(path, bytes)| (path, &*Box::leak(bytes.into_boxed_slice())))
        .collect();
    table.sort_unstable_by_key(|&(path, _)| path);
    let layer = Layer::table("base", Box::leak(table.into_boxed_slice()));
    Box::leak(Box::new(Bundle::new(vec![layer])))
}

/// What `format` reads from the file at `path` of `bundle`.
fn bundled<F: Format>(
    format: &F,
    bundle: &Bundle,
    path: &str,
) -> Result<F::Asset, Vec<ContentError>> {
    let bytes = bundle.bytes(path).unwrap_or_default();
    format.decode(
        &AssetPath::in_bundle(bundle, path),
        bytes,
        &DecodeCx::in_bundle(bundle),
    )
}

/// Checks `format`'s id, extensions and limit.
fn named<F: Format>(format: &F, id: &str, extensions: &[&str], max_bytes: usize) {
    assert_eq!(format.id(), id);
    assert_eq!(format.extensions(), extensions, "{id}");
    assert_eq!(format.max_bytes(), max_bytes, "{id}");
}

/// The start of a WAV file: a `LIST` chunk (odd-sized, so padded), then
/// `fmt `.
pub(crate) fn wav(channels: u16, rate: u32, encoding: u16) -> Vec<u8> {
    let mut w = b"RIFF\0\0\0\0WAVELIST\x05\0\0\0abcde\0fmt \x10\0\0\0".to_vec();
    w.extend_from_slice(&encoding.to_le_bytes());
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&[0; 8]);
    w
}

/// The first page of an OGG Vorbis file (two lacing values).
pub(crate) fn ogg(channels: u8, rate: u32) -> Vec<u8> {
    let mut o = b"OggS".to_vec();
    o.extend_from_slice(&[0; 22]);
    o.extend_from_slice(&[2, 30, 0]);
    o.extend_from_slice(b"\x01vorbis\0\0\0\0");
    o.push(channels);
    o.extend_from_slice(&rate.to_le_bytes());
    o
}

/// An Ogg page after the first: `granule` in its header and `body` as its
/// one segment.
fn ogg_page(granule: u64, body: &[u8]) -> Vec<u8> {
    let mut p = b"OggS\0\x04".to_vec();
    p.extend_from_slice(&granule.to_le_bytes());
    p.extend_from_slice(&[0; 12]);
    p.extend_from_slice(&[1, u8::try_from(body.len()).unwrap_or(u8::MAX)]);
    p.extend_from_slice(body);
    p
}

/// A whole OGG Vorbis file's framing: [`ogg`]'s first page, filled out to
/// the 30 bytes it declares, then a last page at `samples`.
pub(crate) fn ogg_file(rate: u32, samples: u64) -> Vec<u8> {
    let mut o = ogg(2, rate);
    o.resize(29 + 30, 0);
    o.extend_from_slice(&ogg_page(samples, &[0; 40]));
    o
}

/// A 2 × 1 RGBA PNG of `pixels`.
pub(crate) fn png_file(pixels: [u8; 8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&pixels).unwrap();
    }
    out
}

/// The tests' terrains: `plain` and `water`.
pub(crate) fn terrain(id: &str) -> Option<u8> {
    let at = ["plain", "water"].iter().position(|&t| t == id)?;
    u8::try_from(at).ok()
}

/// The tests' maps' format.
pub(crate) const MAP_FORMAT: MapFormat<'static> = MapFormat {
    looks: &["outdoor", "indoor"],
    max_size: 8,
};

/// The `.map` format of the tests' terrains.
pub(crate) fn map_text() -> MapText<u8> {
    MapText::new(MAP_FORMAT, terrain)
}

/// The language packs of a game with no tokens and the tests' limits.
pub(crate) fn lang_pack() -> LangPackRon {
    LangPackRon::new(LIMITS, &NoTokens)
}

/// The same game's rules, as [`lang::from_files`] takes them.
fn rules(variants: &'static [&'static str]) -> PackRules<'static> {
    PackRules {
        data_problem: |_, _| None,
        tokens: &NoTokens,
        limits: LIMITS,
        variants,
    }
}

/// The tests' shots.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum Shot {
    Logo,
    Card(char),
}

/// Every shot is fine, and kept as the letter of its card.
struct AnyShot;

impl ShotCheck<Shot> for AnyShot {
    type Shot = Option<char>;

    fn check(&mut self, _n: usize, shot: &Shot, _problems: &mut Vec<String>) -> Option<char> {
        match shot {
            Shot::Logo => None,
            Shot::Card(letter) => Some(*letter),
        }
    }
}

/// The tests' data table, read by [`Ron`].
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Table {
    rows: Vec<u8>,
}

impl AssetKind for Table {
    const NAME: &'static str = "table";
}

/// The tests' dialogue rules: a ghost can't speak.
struct Rules;

impl SceneRules<Cue> for Rules {
    fn limits(&self) -> &DialogueLimits {
        &LIMITS
    }

    fn tokens(&self) -> &dyn Tokens {
        &HeroTokens
    }

    fn can_speak(&self, character: &CharacterId) -> bool {
        character.0 != "ghost"
    }
}

pub(crate) const PALETTE: &str = "{\"black\": \"#000000\", \"grass\": \"#00ff00\"}";
pub(crate) const PATTERN_MAP: &str = "(roles: {\"black\": (fg: Black), \"grass\": (fg: Inverted(Dither25), bg: HLines)}, distinct: [[\"black\", \"grass\"]], text: [\"black\"])";
pub(crate) const LIGHTS: &str = "{\"noon\": (grey: 0.0, tint: (1.0, 1.0, 1.0)), \"dusk\": (grey: 0.35, tint: (0.62, 0.56, 0.46))}";
pub(crate) const NAMES: &str = "{\"hero\": \"Ana\", \"town\": \"Osk\"}";
pub(crate) const ATLAS: &str = "(cell_w: 8, cell_h: 16, columns: 16, glyphs: {'a': 0, 'b': 1})";
pub(crate) const CREDITS: &str = r#"(credits: [
    (id: "font", group: Fonts, title: "A Font", author: "Ann",
     source: "https://example.org/font", license: "OFL-1.1"),
])"#;
const BAD_CREDITS: &str = r#"(credits: [
    (id: "song", group: Music, title: "Song", author: "Ann",
     source: "https://example.org/song", license: "CC0-1.0"),
    (id: "art", group: Art, title: "Art", author: "Bob",
     source: "https://example.org/art", license: "Custom (Shop)", private: ["/abs"]),
])"#;
pub(crate) const MANIFEST: &str = r#"(
    music_fade_ms: 500,
    sounds: [
        (id: "tick", files: ["sfx/tick.wav"], volume: 100, credit: Own),
        (id: "gone", files: ["sfx/gone.wav"], volume: 100, credit: Own),
        (id: "trim", files: ["sfx/trim.wav"], volume: 100, credit: Own),
    ],
    music: [(id: "title", file: "title.ogg", volume: 80, length_ms: 2000, credit: Own)],
    pools: [],
    credits: [],
)"#;
pub(crate) const SCRIPT: &str = "\
@scene gate
@caption The gate, dusk
@left mentor neutral
@weather rain
mentor: Is the road clear?
> Rain falls.
@choice
* firm: We wait.
  mentor: Agreed.
* soft: We go.
@endchoice
@if mentor
mentor: Still here.
@else
> Gone.
@endif
@end
";
pub(crate) const MAP: &str = "(name: \"Field\", legend: {'.': \"plain\", '~': \"water\"}, look: (tiles: \"indoor\"))\n---\n.~\n~.\n";
pub(crate) const FILM: &str = "(id: \"title\", length: Seconds(10.0), shots: [(at: 0.0, shot: Logo), (at: 6.0, shot: Card('A'))])";
pub(crate) const ENGLISH_UI: &str = r#"{"a.one": "One", "a.two": "Two {count}"}"#;
pub(crate) const PACK_INFO: &str = r#"(name: "Test", made_by: Machine)"#;
pub(crate) const PACK_UI: &str = r#"[(key: "a.one", source: "One", text: "Un")]"#;
pub(crate) const TABLE: &str = "(rows: [1, 2])";

/// The files the fuzzed files name: an atlas's picture, a manifest's
/// sound, a pack's English and screen text.
pub(crate) fn fixtures() -> &'static Bundle {
    static FIXTURES: OnceLock<&'static Bundle> = OnceLock::new();
    FIXTURES.get_or_init(|| {
        bundle(vec![
            ("fonts/atlas.png", png_header(128, 16)),
            ("audio/sfx/tick.wav", wav(1, audio::SAMPLE_RATE, 1)),
            ("lang/en/ui.ron", ENGLISH_UI.into()),
            ("lang/xx/ui.ron", PACK_UI.into()),
        ])
    })
}

#[test]
fn every_built_in_has_its_id_extensions_and_limit() {
    named(&PngInfo, "png", &["png"], MAX_PNG_BYTES);
    named(&Png, "png", &["png"], MAX_PNG_BYTES);
    named(&OneBitInfo, "one-bit", &[], bitmap::MAX_BYTES);
    named(&Wav, "wav", &["wav"], MAX_SOUND_BYTES);
    named(&OggVorbis, "ogg-vorbis", &["ogg"], MAX_SOUND_BYTES);
    named(&OggVorbisTrack, "ogg-vorbis", &["ogg"], MAX_SOUND_BYTES);
    named(
        &AudioManifestRon,
        "audio-manifest",
        &["ron"],
        TEXT_MAX_BYTES,
    );
    named(&CreditsRon::default(), "credits", &["ron"], TEXT_MAX_BYTES);
    named(
        &FontAtlasRon::default(),
        "font-atlas",
        &["ron"],
        TEXT_MAX_BYTES,
    );
    named(&PaletteRon::default(), "palette", &["ron"], TEXT_MAX_BYTES);
    named(&PatternMapRon, "pattern-map", &["ron"], TEXT_MAX_BYTES);
    named(&LightingRon::default(), "lights", &["ron"], TEXT_MAX_BYTES);
    named(&NamesRon, "names", &["ron"], TEXT_MAX_BYTES);
    named(&Dlg::<NoDirective>::new(), "dlg", &["dlg"], TEXT_MAX_BYTES);
    named(&map_text(), "map", &["map"], TEXT_MAX_BYTES);
    named(
        &KeymapRon::<sample::Action>::new(),
        "keymap",
        &["ron"],
        TEXT_MAX_BYTES,
    );
    named(
        &CinematicRon::<Shot>::new(),
        "cinematic",
        &["ron"],
        TEXT_MAX_BYTES,
    );
    named(&lang_pack(), "lang-pack", &["ron"], TEXT_MAX_BYTES);
    named(&Ron::<Table>::default(), "ron", &["ron"], TEXT_MAX_BYTES);
    let custom = Ron::<Table>::named("table", &["tbl", "table"]);
    named(&custom, "table", &["tbl", "table"], TEXT_MAX_BYTES);
}

#[test]
fn the_binary_formats_know_their_magic_bytes() {
    let rate = audio::SAMPLE_RATE;
    let png = png_header(1, 1);
    let bits = bitmap::encode((1, 1), (1, 1), &[&[0]], None).unwrap_or_default();
    let (wav, ogg) = (wav(1, rate, 1), ogg(1, rate));
    // Which of PNG, one-bit, WAV and OGG knows the bytes.
    let cases: [(&[u8], [bool; 4]); 9] = [
        (&png, [true, false, false, false]),
        (PNG_SIGNATURE, [true, false, false, false]),
        (&PNG_SIGNATURE[..7], [false; 4]),
        (&bits, [false, true, false, false]),
        (&wav, [false, false, true, false]),
        (b"RIFF\0\0\0\0WAVX", [false; 4]),
        (b"RIFX\0\0\0\0WAVE", [false; 4]),
        (&ogg, [false, false, false, true]),
        (b"Ogg", [false; 4]),
    ];
    for (bytes, [is_png, is_bits, is_wav, is_ogg]) in cases {
        let sniffed = [
            PngInfo.sniff(bytes),
            Png.sniff(bytes),
            OneBitInfo.sniff(bytes),
            Wav.sniff(bytes),
            OggVorbis.sniff(bytes),
            OggVorbisTrack.sniff(bytes),
        ];
        let expected = [is_png, is_png, is_bits, is_wav, is_ogg, is_ogg];
        assert_eq!(sniffed, expected, "{bytes:?}");
    }
    assert!(!Wav.sniff(b"RIFF\0\0\0\0WAV"));
    assert!(!PngInfo.sniff(b""));
}

#[test]
fn png_and_one_bit_read_an_images_size_as_the_image_table_does() {
    let ok = png_header(16, 24);
    let size = |width, height| ImageInfo { width, height };
    assert_eq!(alone(&PngInfo, "a.png", &ok), Ok(size(16, 24)));
    assert_eq!(
        shown(alone(&PngInfo, "a.png", b"not a png")),
        ["a.png: not a valid PNG file"]
    );
    // 4 × 2 pixels, made from a 2 × 1 PNG.
    let bits = bitmap::encode((4, 2), (2, 1), &[&[0, 0]], None).unwrap_or_default();
    assert_eq!(alone(&OneBitInfo, "a.png", &bits), Ok(size(2, 1)));
    let mut broken = bits.clone();
    broken.pop();
    assert_eq!(
        shown(alone(&OneBitInfo, "a.png", &broken)),
        ["a.png: a one-bit picture's size doesn't match its header"]
    );
    // The table, file by file, as `from_files` reads it.
    let (wide, flat) = (png_header(MAX_SIDE + 1, 8), png_header(8, 0));
    let files = [
        ("images/ok.png", &ok[..]),
        ("images/bits.png", &bits[..]),
        ("images/broken.png", &broken[..]),
        ("images/text.png", &b"not an image"[..]),
        ("images/wide.png", &wide[..]),
        ("images/flat.png", &flat[..]),
    ];
    let (bundle, formats) = (bundle(Vec::new()), Formats::standard());
    let with = ImageTable::from_files_with(bundle, &formats, files);
    assert_eq!(with, ImageTable::from_files(bundle, files));
    assert_eq!(shown(with).len(), 4);
    let good = [files[0], files[1]];
    let with = ImageTable::from_files_with(bundle, &formats, good);
    assert_eq!(with, ImageTable::from_files(bundle, good));
    assert_eq!(with.map(|table| table.images.len()), Ok(2));
}

#[test]
fn the_image_table_with_the_standard_formats_reads_the_files_load_reads() {
    let bits = bitmap::encode((4, 2), (2, 1), &[&[0, 0]], None).unwrap_or_default();
    let files = bundle(vec![
        ("data/a.ron", b"()".to_vec()),
        ("fonts/atlas.png", png_header(128, 64)),
        ("images/card.png", png_header(16, 16)),
        // A touched-up picture beside its PNG is no image of its own.
        ("images/card.1bit", bits.clone()),
        // A one-bit picture in a PNG's place is one.
        ("images/small.png", bits),
        ("units/Big.PNG", png_header(32, 48)),
    ]);
    let formats = Formats::standard();
    let skip = ["fonts/atlas.png"];
    let table = ImageTable::load_with(files, &formats, &skip);
    assert_eq!(table, ImageTable::load(files, &skip));
    let paths: Vec<_> = table.unwrap_or_default().ids().map(ImageId::path).collect();
    assert_eq!(
        paths,
        ["images/card.png", "images/small.png", "units/Big.PNG"]
    );
    assert_eq!(
        ImageTable::load_with(files, &formats, &[]),
        ImageTable::load(files, &[])
    );
    let broken = bundle(vec![("images/bad.png", b"nope".to_vec())]);
    let table = ImageTable::load_with(broken, &formats, &[]);
    assert_eq!(table, ImageTable::load(broken, &[]));
    assert_eq!(shown(table), ["base/images/bad.png: not a valid PNG file"]);
    // The registry's limits hold.
    let mut small = Formats::standard();
    small.limits = Limits { max_bytes: 8 };
    assert_eq!(shown(ImageTable::load_with(files, &small, &skip)).len(), 3);
}

#[test]
fn png_reads_an_images_pixels_once_its_size_is_allowed() {
    let pixels = [255, 0, 0, 255, 0, 0, 255, 128];
    let image = alone(&Png, "a.png", &png_file(pixels));
    let expected = Image {
        width: 2,
        height: 1,
        rgba: pixels.to_vec(),
    };
    assert_eq!(
        image.as_ref().map(Image::info),
        Ok(ImageInfo {
            width: 2,
            height: 1
        })
    );
    assert_eq!(image, Ok(expected));
    // Too big: refused before its pixels are decoded.
    assert_eq!(
        shown(alone(&Png, "a.png", &png_header(MAX_SIDE + 1, 1))),
        ["a.png: image is 4097×1 px; neither side may be over 4096"]
    );
    assert_eq!(
        shown(alone(&Png, "a.png", b"nope")),
        ["a.png: not a valid PNG file"]
    );
    // A header with no picture after it.
    let errors = shown(alone(&Png, "a.png", &png_header(2, 1)));
    assert_eq!(errors.len(), 1);
    assert!(errors[0].starts_with("a.png: "), "{errors:?}");
    // By id, through the registry.
    let files = bundle(vec![("images/a.png", png_file(pixels))]);
    let rgba = Formats::standard().load::<Image>(files, "images/a");
    assert_eq!(rgba.map(|image| image.rgba), Ok(pixels.to_vec()));
}

#[test]
fn wav_and_ogg_read_a_sounds_header_as_the_manifest_check_does() {
    let rate = audio::SAMPLE_RATE;
    let cases = [
        ("a.wav", wav(2, rate, 1)),
        ("a.wav", wav(1, rate, 3)),
        ("a.wav", wav(3, rate, 1)),
        ("a.wav", wav(1, 22_050, 1)),
        ("a.wav", wav(1, rate, 2)),
        ("a.wav", b"RIFF".to_vec()),
        ("a.ogg", ogg(2, rate)),
        ("a.ogg", ogg(0, rate)),
        ("a.ogg", ogg(1, 48_000)),
        ("a.ogg", b"OggS".to_vec()),
    ];
    for (name, bytes) in cases {
        let format: &dyn Format<Asset = Sound> = if name == "a.wav" { &Wav } else { &OggVorbis };
        let read = format.decode(&AssetPath::new(name), &bytes, &DecodeCx::new());
        let problem = read.err().map(|errors| {
            assert_eq!(errors.len(), 1);
            errors[0].message.clone()
        });
        assert_eq!(problem, format_problem(name, &bytes), "{name} {bytes:?}");
    }
    let sound = |channels| Sound {
        channels,
        sample_rate: rate,
    };
    assert_eq!(alone(&Wav, "a.wav", &wav(1, rate, 3)), Ok(sound(1)));
    assert_eq!(alone(&OggVorbis, "a.ogg", &ogg(2, rate)), Ok(sound(2)));
    assert_eq!(
        shown(alone(&Wav, "sfx/a.wav", b"RIFF")),
        ["sfx/a.wav: not a PCM WAV file"]
    );
    assert_eq!(
        shown(alone(&OggVorbis, "a.ogg", &ogg(1, 48_000))),
        ["a.ogg: 48000 Hz; must be 44100 Hz"]
    );
    // Through the registry, which keeps each format to its limit.
    let standard = Formats::standard();
    let read = |name: &str, bytes: &[u8]| {
        standard.decode::<Sound>(&AssetPath::new(name), bytes, &DecodeCx::new())
    };
    assert_eq!(read("a.wav", &wav(1, rate, 1)), Ok(sound(1)));
    assert_eq!(read("a.ogg", &ogg(2, rate)), Ok(sound(2)));
}

#[test]
fn a_music_track_is_a_whole_ogg_vorbis_file_whose_last_page_gives_its_length() {
    let rate = audio::SAMPLE_RATE;
    assert_eq!(
        alone(&OggVorbisTrack, "a.ogg", &ogg_file(rate, 88_200)),
        Ok(MusicTrack {
            channels: 2,
            sample_rate: rate,
            length_ms: 2000,
        })
    );
    assert_eq!(
        shown(alone(&OggVorbisTrack, "a.ogg", &ogg(2, rate))),
        ["a.ogg: its length can't be read (no last Ogg page)"]
    );
    assert_eq!(
        shown(alone(&OggVorbisTrack, "a.ogg", &ogg(3, rate))),
        ["a.ogg: 3 channels; must be mono or stereo"]
    );
    assert_eq!(
        shown(alone(&OggVorbisTrack, "a.ogg", b"OggS")),
        ["a.ogg: not an OGG Vorbis file"]
    );
    // Through the registry, which keeps the format to its limit.
    let file = AssetPath::new("music/a.ogg");
    let track =
        Formats::standard().decode::<MusicTrack>(&file, &ogg_file(rate, 44_100), &DecodeCx::new());
    assert_eq!(track.map(|t| t.length_ms), Ok(1000));
}

/// The manifest in `source`, read as [`audio::from_source`] reads it with
/// the sound files of `bundle` beside it.
fn manifest(bundle: &Bundle, source: &str) -> Result<AudioManifest, Vec<ContentError>> {
    let path = |name: &str| format!("audio/{name}");
    let files = audio::Files {
        sound: &|name| bundle.bytes(&path(name)).map(Cow::Borrowed),
        sound_shown: &|name| bundle.display_path(&path(name)),
        sound_left_out: &|name| bundle.left_out(&path(name)),
        music_dir: "music",
        music: None,
    };
    audio::from_source(&bundle.display_path("audio/audio.ron"), source, &files)
}

#[test]
fn the_audio_manifest_reads_its_sounds_beside_it_and_not_the_music() {
    let tick = wav(1, audio::SAMPLE_RATE, 1);
    let layer = |source: &str| {
        let files: Vec<(&'static str, &'static [u8])> = vec![
            ("audio/audio.ron", Box::leak(source.as_bytes().into())),
            (
                "audio/sfx/tick.wav",
                Box::leak(tick.clone().into_boxed_slice()),
            ),
        ];
        let table = Box::leak(files.into_boxed_slice());
        let trimmed = Layer::table("base", table).leaving_out(&["audio/sfx/trim.wav"]);
        &*Box::leak(Box::new(Bundle::new(vec![trimmed])))
    };
    let all = layer(MANIFEST);
    let read = Formats::standard().load::<AudioManifest>(all, "audio/audio");
    assert_eq!(read, manifest(all, MANIFEST));
    // `tick` is there, `trim` is left out and `title` isn't read.
    assert_eq!(
        shown(read),
        ["base/audio/audio.ron:5: sound \"gone\": file base/audio/sfx/gone.wav not found"]
    );
    let source = MANIFEST.replace("sfx/gone.wav", "sfx/tick.wav");
    let found = layer(&source);
    let read = bundled(&AudioManifestRon, found, "audio/audio.ron");
    assert_eq!(read, manifest(found, &source));
    assert_eq!(read.map(|m| m.sounds.len()), Ok(3));
}

#[test]
fn credits_name_the_games_other_files_as_the_format_is_told() {
    let places = |audio_manifest, private_root| Places {
        audio_manifest,
        private_root,
    };
    let default = CreditsRon::default();
    for source in [CREDITS, BAD_CREDITS] {
        let read = alone(&default, "credits.ron", source.as_bytes());
        let parsed = credits::from_source(
            "credits.ron",
            source,
            &places("the audio manifest", "private"),
        );
        assert_eq!(read, parsed);
    }
    assert!(alone(&default, "credits.ron", CREDITS.as_bytes()).is_ok());
    let told = CreditsRon::new("base/audio/audio.ron", "bought");
    let read = alone(&told, "credits.ron", BAD_CREDITS.as_bytes());
    let parsed = credits::from_source(
        "credits.ron",
        BAD_CREDITS,
        &places("base/audio/audio.ron", "bought"),
    );
    assert_eq!(read, parsed);
    assert_eq!(
        shown(read),
        [
            "credits.ron:2: credit \"song\": Music credits go in base/audio/audio.ron",
            "credits.ron:4: credit \"art\": private path \"/abs\" must be a path inside bought/",
        ]
    );
}

#[test]
fn a_font_atlas_is_read_with_the_png_beside_it() {
    let files = bundle(vec![
        ("fonts/atlas.ron", ATLAS.into()),
        ("fonts/atlas.png", png_header(128, 16)),
    ]);
    let parsed = |required| {
        FontAtlasDef::from_source(
            "base/fonts/atlas.ron",
            ATLAS,
            "base/fonts/atlas.png",
            files.bytes("fonts/atlas.png"),
            required,
        )
    };
    let read = Formats::standard().load::<FontAtlasDef>(files, "fonts/atlas");
    assert_eq!(read, parsed(""));
    assert!(read.is_ok());
    let needs_c = FontAtlasRon::requiring("abc");
    let read = bundled(&needs_c, files, "fonts/atlas.ron");
    assert_eq!(read, parsed("abc"));
    assert_eq!(
        shown(read),
        ["base/fonts/atlas.ron: missing required glyphs: 'c' (U+0063)"]
    );
    let without = bundle(vec![("fonts/atlas.ron", ATLAS.into())]);
    assert_eq!(
        shown(bundled(
            &FontAtlasRon::default(),
            without,
            "fonts/atlas.ron"
        )),
        ["base/fonts/atlas.png: file not found in asset bundle"]
    );
}

#[test]
fn a_palette_holds_the_colours_the_game_requires() {
    let bytes = PALETTE.as_bytes();
    let read = alone(&PaletteRon::default(), "p.ron", bytes);
    assert_eq!(read, PaletteDef::from_source("p.ron", PALETTE, &[]));
    assert!(read.is_ok());
    let needs = PaletteRon::requiring(["black", "white"]);
    let read = alone(&needs, "p.ron", bytes);
    assert_eq!(
        read,
        PaletteDef::from_source("p.ron", PALETTE, &["black", "white"])
    );
    assert_eq!(shown(read), ["p.ron: missing required colour \"white\""]);
    let black = PaletteRon::requiring(vec![String::from("black")]);
    assert!(alone(&black, "p.ron", bytes).is_ok());
}

#[test]
fn a_pattern_map_is_read_as_from_source_reads_it() {
    let read = alone(&PatternMapRon, "p@onebit.ron", PATTERN_MAP.as_bytes());
    assert_eq!(
        read,
        PatternMapDef::from_source("p@onebit.ron", PATTERN_MAP)
    );
    assert!(read.is_ok());
    // Beside the palette, standard: its kind is a pattern map.
    let files = bundle(vec![
        ("data/palette.ron", PALETTE.into()),
        ("data/palette@onebit.ron", PATTERN_MAP.into()),
    ]);
    let map = Formats::standard().load::<PatternMapDef>(files, "data/palette@onebit");
    assert_eq!(map, read);
}

#[test]
fn lights_have_the_light_of_a_map_that_names_none() {
    let read = alone(&LightingRon::default(), "l.ron", LIGHTS.as_bytes());
    assert_eq!(read, lighting::from_source("l.ron", LIGHTS, "noon"));
    assert!(read.is_ok());
    let dusk = r#"{"dusk": (grey: 0.35, tint: (0.62, 0.56, 0.46))}"#;
    let read = alone(&LightingRon::default(), "l.ron", dusk.as_bytes());
    assert_eq!(read, lighting::from_source("l.ron", dusk, "noon"));
    assert!(read.is_err());
    let read = alone(&LightingRon::new("dusk"), "l.ron", dusk.as_bytes());
    assert_eq!(read, lighting::from_source("l.ron", dusk, "dusk"));
    assert!(read.is_ok());
}

#[test]
fn names_are_read_as_the_names_table_is() {
    let read = alone(&NamesRon, "names.ron", NAMES.as_bytes());
    assert_eq!(read, names::from_source("names.ron", NAMES));
    assert_eq!(read.map(|n| n.names.len()), Ok(2));
    let bad = "{\"Hero\": \"Ana\"}";
    let read = alone(&NamesRon, "names.ron", bad.as_bytes());
    assert_eq!(read, names::from_source("names.ron", bad));
    assert!(read.is_err());
}

#[test]
fn dlg_reads_a_files_scenes_as_the_parser_does() {
    let format = Dlg::<Cue>::new();
    let (scenes, errors) = parse_dlg::<Cue>("story/gate.dlg", SCRIPT);
    assert_eq!(errors, []);
    let read = alone(&format, "story/gate.dlg", SCRIPT.as_bytes());
    assert_eq!(read, Ok(DialogueFile { scenes }));
    // Its errors are the parser's.
    let broken = "@scene gate\nmentor: Hi.\n@weather\n";
    let (_, errors) = parse_dlg::<Cue>("story/gate.dlg", broken);
    assert_eq!(errors.len(), 2);
    assert_eq!(
        alone(&format, "story/gate.dlg", broken.as_bytes()),
        Err(errors)
    );
}

#[test]
fn scenes_read_by_dlg_are_checked_as_scripts_from_sources_checks_them() {
    let ghost = "@scene gate\n@left ghost neutral\nghost: Boo.\n@end\n";
    let onward = "@scene road\n@left mentor neutral\nmentor: On we go.\n@end\n";
    let read = |files: &[(&str, &str)]| -> Vec<DialogueFile<Cue>> {
        let read = files
            .iter()
            .map(|(file, source)| alone(&Dlg::new(), file, source.as_bytes()));
        read.collect::<Result<_, _>>().unwrap_or_default()
    };
    let bad = [("a.dlg", SCRIPT), ("b.dlg", ghost)];
    let checked = scripts_from_parsed(read(&bad), &Rules);
    assert_eq!(checked, scripts_from_sources(&bad, &Rules));
    // `gate` is in both files, and `ghost` can't be placed or speak.
    assert_eq!(
        shown(checked),
        [
            "b.dlg:1: duplicate scene id \"gate\"; first used at a.dlg:1",
            "b.dlg:2: unknown character \"ghost\"",
            "b.dlg:3: unknown character \"ghost\"",
        ]
    );
    let good = [("a.dlg", SCRIPT), ("c.dlg", onward)];
    let checked = scripts_from_parsed(read(&good), &Rules);
    assert_eq!(checked, scripts_from_sources(&good, &Rules));
    assert_eq!(checked.map(|s| s.table.scenes.len()), Ok(2));
}

#[test]
fn map_reads_a_grid_map_as_the_parser_does() {
    let read = alone(&map_text(), "maps/field.map", MAP.as_bytes());
    let parsed = grid_map::parse::<u8, NoFeature>("maps/field.map", MAP, &MAP_FORMAT, terrain);
    assert_eq!(read, parsed);
    assert_eq!(
        read.map(|map| (map.name, map.look.tiles)),
        Ok(("Field".into(), "indoor".into()))
    );
    let lava = MAP.replace("water", "lava");
    let read = alone(&map_text(), "maps/field.map", lava.as_bytes());
    let parsed = grid_map::parse::<u8, NoFeature>("maps/field.map", &lava, &MAP_FORMAT, terrain);
    assert_eq!(read, parsed);
    assert_eq!(
        shown(read),
        ["maps/field.map:1:40: legend '~': unknown terrain \"lava\""]
    );
}

#[test]
fn keymap_reads_a_keymap_file_as_from_source_does() {
    let format = KeymapRon::<sample::Action>::default();
    let read = alone(&format, "sample.ron", sample::SOURCE.as_bytes());
    assert_eq!(read, Ok(sample::keymap()));
    let broken = sample::SOURCE.replacen("repeat: (", "repeats: (", 1);
    let read = alone(&format, "sample.ron", broken.as_bytes());
    assert_eq!(
        read,
        KeymapDef::<sample::Action>::from_source("sample.ron", &broken)
    );
    assert!(read.is_err());
}

#[test]
fn cinematic_reads_a_file_as_written_and_check_takes_its_stem() {
    let format = CinematicRon::<Shot>::default();
    let read = alone(&format, "films/title.ron", FILM.as_bytes());
    let parsed = cinematic::parse::<Shot, ()>("films/title.ron", FILM).map_err(|e| vec![e]);
    assert_eq!(format!("{read:?}"), format!("{parsed:?}"));
    let file = AssetPath::new("films/title.ron");
    let checked = read.map(|raw| {
        let audio = AudioManifest::default();
        cinematic::check(file.shown(), file.stem(), &raw, &audio, AnyShot)
    });
    let kept = |def: cinematic::CinematicDef<Option<char>>| def.shots.into_iter().map(|s| s.shot);
    let kept = checked.map(|def| def.map(|def| kept(def).collect::<Vec<_>>()));
    assert_eq!(kept, Ok(Ok(vec![None, Some('A')])));
    let broken = FILM.replace("shots", "shot");
    let read = alone(&format, "films/title.ron", broken.as_bytes());
    let parsed = cinematic::parse::<Shot, ()>("films/title.ron", &broken);
    assert_eq!(read.err(), parsed.err().map(|e| vec![e]));
}

/// The languages of the files under `lang/` of `bundle`, as
/// [`lang::from_files`] reads them.
fn languages(
    bundle: &Bundle,
    data: Option<SourceText>,
    rules: &PackRules<'_>,
) -> Result<Lang, Vec<ContentError>> {
    let paths = bundle.files_under("lang");
    let files: Vec<LangFile<'_>> = paths
        .iter()
        .map(|path| LangFile {
            path: path.strip_prefix("lang/").unwrap_or(path),
            source: bundle.file(path),
        })
        .collect();
    let display = |path: &str| bundle.display_path(&format!("lang/{path}"));
    lang::from_files(&files, data, display, rules)
}

/// The code `code`.
fn code(code: &str) -> LangCode {
    LangCode::new(code).unwrap_or_default()
}

/// English's screen text, the pack's description and its screen text.
pub(crate) fn pack_files() -> [(&'static str, Vec<u8>); 3] {
    [
        ("lang/en/ui.ron", ENGLISH_UI.into()),
        ("lang/xx/lang.ron", PACK_INFO.into()),
        ("lang/xx/ui.ron", PACK_UI.into()),
    ]
}

#[test]
fn a_language_pack_is_read_from_its_lang_ron_as_the_languages_read_it() {
    let files = bundle(pack_files().into());
    let mut formats = Formats::new();
    assert_eq!(formats.register(lang_pack()), Ok(()));
    let pack = formats.load::<LangPack>(files, "lang/xx/lang");
    let lang = languages(files, None, &rules(&[])).unwrap_or_default();
    assert_eq!(pack.as_ref().ok(), lang.pack(&code("xx")));
    let text = pack.ok().and_then(|p| Some(p.entry("a.one")?.text.clone()));
    assert_eq!(text.as_deref(), Some("Un"));
    // A pack at the bundle's root, beside English.
    let root = bundle(vec![
        ("en/ui.ron", ENGLISH_UI.into()),
        ("xx/lang.ron", PACK_INFO.into()),
        ("xx/ui.ron", PACK_UI.into()),
    ]);
    assert!(bundled(&lang_pack(), root, "xx/lang.ron").is_ok());
}

#[test]
fn a_settings_file_in_a_language_pack_is_no_error() {
    let [english, info, ui] = pack_files();
    let settings = |path: &'static str| (path, b"()".to_vec());
    let files = vec![
        english,
        info,
        ui,
        settings("lang/xx/_folder.meta"),
        settings("lang/xx/ui.ron.meta"),
        settings("lang/xx/dialogue/_folder.meta"),
    ];
    let read = bundled(&lang_pack(), bundle(files), "lang/xx/lang.ron");
    assert!(read.is_ok(), "{read:?}");
}

#[test]
fn a_language_packs_errors_name_its_files() {
    let pack = |files: Vec<(&'static str, Vec<u8>)>, path: &str| {
        shown(bundled(&lang_pack(), bundle(files), path))
    };
    let [english, info, ui] = pack_files();
    let info_path = "lang/xx/lang.ron";
    assert_eq!(
        pack(vec![english.clone(), info.clone()], info_path),
        ["base/lang/xx/ui.ron: file not found in asset bundle"]
    );
    let notes = ("lang/xx/notes.txt", b"x".to_vec());
    let deep = ("lang/xx/dialogue/a/b.ron", b"[]".to_vec());
    let all = vec![english.clone(), info.clone(), ui.clone(), notes, deep];
    assert_eq!(
        pack(all, info_path),
        [
            "base/lang/xx/dialogue/a/b.ron: not a file a language has",
            "base/lang/xx/notes.txt: not a file a language has",
        ]
    );
    let bad_code = ("lang/x1/lang.ron", PACK_INFO.into());
    assert_eq!(
        pack(vec![english.clone(), bad_code], "lang/x1/lang.ron"),
        ["base/lang/x1: a language's directory must be named with 2 to 8 lowercase letters"]
    );
    let in_english = ("lang/en/lang.ron", PACK_INFO.into());
    let files = vec![english.clone(), info.clone(), ui.clone(), in_english];
    for path in ["lang/en/lang.ron", "lang/xx/ui.ron"] {
        assert_eq!(
            pack(files.clone(), path),
            [format!(
                "base/{path}: not a file a language has: a pack is read from its lang.ron"
            )]
        );
    }
    assert_eq!(
        pack(vec![info.clone(), ui.clone()], info_path),
        ["base/lang/en/ui.ron: file not found in asset bundle"]
    );
    let bad_english = ("lang/en/ui.ron", br#"{"A": "x"}"#.to_vec());
    assert_eq!(
        pack(vec![bad_english, info.clone(), ui.clone()], info_path),
        ["base/lang/en/ui.ron: \"A\": a key must be lowercase letters, digits, `_` and `.`"]
    );
    // The pack's own errors, as the languages report them.
    let bad_ui = (
        "lang/xx/ui.ron",
        br#"[(key: "a.zero", source: "Zero", text: "Zero")]"#.to_vec(),
    );
    let files = bundle(vec![english, info, bad_ui]);
    let read = bundled(&lang_pack(), files, info_path);
    assert_eq!(
        read.as_ref().err(),
        languages(files, None, &rules(&[])).as_ref().err()
    );
    assert_eq!(
        shown(read),
        ["base/lang/xx/ui.ron: \"a.zero\": no such key in English"]
    );
}

/// The English of a game's data: an item's name, and one scene's line,
/// whose key is the second.
fn data_text() -> (SourceText, String) {
    let script = "@scene gate\n@left guide neutral\nguide: Open it.\n@end\n";
    let (scenes, _) = parse_dlg::<NoDirective>("story/gate.dlg", script);
    let table = DialogueTable {
        scenes: scenes
            .iter()
            .map(|p| (p.scene.id.clone(), p.scene.clone()))
            .collect(),
        files: [("gate".to_owned(), "story/gate.dlg".to_owned())].into(),
    };
    let data = [("items.rope.name".to_owned(), "Rope".to_owned())];
    let mut source = SourceText::new(data.into(), Names::default());
    source.add_dialogue(&table);
    let line = source.lines.iter().find(|(_, l)| l.text == "Open it.");
    let key = line.map(|(key, _)| key.clone()).unwrap_or_default();
    (source, key)
}

#[test]
fn a_language_packs_data_and_dialogue_are_read_against_the_games_data_text() {
    let (source, key) = data_text();
    let data = (
        "lang/xx/data.ron",
        br#"[(key: "items.rope.name", source: "Rope", text: "Corde")]"#.to_vec(),
    );
    let dialogue = |fields: &str| {
        let entry = format!("[(key: {key:?}, source: \"Open it.\", {fields})]");
        ("lang/xx/dialogue/gate.ron", entry.into_bytes())
    };
    let mut files: Vec<_> = pack_files().into();
    files.extend([data.clone(), dialogue("text: \"Ouvre.\"")]);
    let files = bundle(files);
    let info_path = "lang/xx/lang.ron";
    let pack = bundled(&lang_pack().with_data(source.clone()), files, info_path);
    let lang = languages(files, Some(source.clone()), &rules(&[])).unwrap_or_default();
    assert_eq!(pack.as_ref().ok(), lang.pack(&code("xx")));
    let pack = pack.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        pack.entry("items.rope.name").map(|e| e.text.as_str()),
        Some("Corde")
    );
    assert!(pack.line(&key).is_some());
    // Without the game's data text, they are left out.
    let bare = bundled(&lang_pack(), files, info_path).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(bare.entry("items.rope.name"), None);
    assert!(bare.line(&key).is_none());
    // The game's rules for data text.
    let strict = lang_pack()
        .data_problem(|key, text| {
            (key.starts_with("items.") && text.len() > 3).then(|| "long".into())
        })
        .with_data(source.clone());
    assert_eq!(
        shown(bundled(&strict, files, info_path)),
        ["base/lang/xx/data.ron: \"items.rope.name\": long"]
    );
    // A data file that isn't text.
    let mut broken: Vec<_> = pack_files().into();
    broken.push(("lang/xx/data.ron", vec![0xff]));
    let broken = bundle(broken);
    assert_eq!(
        shown(bundled(&lang_pack(), broken, info_path)),
        ["base/lang/xx/data.ron: file not found in asset bundle"]
    );
    // The variants a line may have.
    let mut varied: Vec<_> = pack_files().into();
    varied.extend([data, dialogue("text_a: \"Ouvre.\", text_b: \"Ouvrez.\"")]);
    let varied = bundle(varied);
    let plain = lang_pack().with_data(source.clone());
    assert_eq!(shown(bundled(&plain, varied, info_path)).len(), 1);
    let both = lang_pack().variants(&["a", "b"]).with_data(source.clone());
    let pack = bundled(&both, varied, info_path);
    let lang = languages(varied, Some(source), &rules(&["a", "b"])).unwrap_or_default();
    assert_eq!(pack.as_ref().ok(), lang.pack(&code("xx")));
    assert!(pack.is_ok());
}

#[test]
fn ron_reads_any_kind_serde_reads() {
    let read = alone(&Ron::<Table>::new(), "t.ron", TABLE.as_bytes());
    assert_eq!(read, Ok(Table { rows: vec![1, 2] }));
    let bad = "(rows: [1, x])";
    let read = alone(&Ron::<Table>::new(), "t.ron", bad.as_bytes());
    assert_eq!(read, parse_ron::<Table>("t.ron", bad).map_err(|e| vec![e]));
    assert!(read.is_err());
    // A game's own id and extension.
    let mut formats = Formats::new();
    assert_eq!(
        formats.register(Ron::<Table>::named("table", &["tbl"])),
        Ok(())
    );
    let files = bundle(vec![("data/t.tbl", b"(rows: [])".to_vec())]);
    assert_eq!(
        formats.load::<Table>(files, "data/t"),
        Ok(Table { rows: Vec::new() })
    );
}

#[test]
fn every_text_format_refuses_bytes_that_arent_utf8() {
    fn refuses<F: Format>(format: &F) {
        let read = alone(format, "x/a.txt", &[b'(', 0xff]);
        assert_eq!(
            shown(read),
            ["x/a.txt: file is not valid UTF-8"],
            "{}",
            format.id()
        );
    }
    refuses(&AudioManifestRon);
    refuses(&CreditsRon::default());
    refuses(&FontAtlasRon::default());
    refuses(&PaletteRon::default());
    refuses(&LightingRon::default());
    refuses(&PatternMapRon);
    refuses(&NamesRon);
    refuses(&Dlg::<NoDirective>::new());
    refuses(&map_text());
    refuses(&KeymapRon::<sample::Action>::new());
    refuses(&CinematicRon::<Shot>::new());
    refuses(&lang_pack());
    refuses(&Ron::<Table>::new());
}

#[test]
fn the_formats_of_a_games_types_show_what_they_are() {
    assert_eq!(format!("{:?}", Dlg::<Cue>::default()), "Dlg");
    assert_eq!(
        format!("{:?}", KeymapRon::<sample::Action>::new()),
        "KeymapRon"
    );
    assert_eq!(format!("{:?}", CinematicRon::<Shot>::new()), "CinematicRon");
    assert_eq!(
        format!("{:?}", Ron::<Table>::named("table", &["tbl"])),
        "Ron { id: \"table\", extensions: [\"tbl\"] }"
    );
    assert_eq!(
        format!("{:?}", map_text()),
        "MapText { format: MapFormat { looks: [\"outdoor\", \"indoor\"], max_size: 8 }, .. }"
    );
    assert_eq!(
        format!("{:?}", lang_pack().variants(&["a"])),
        format!("LangPackRon {{ limits: {LIMITS:?}, variants: [\"a\"], data: false, .. }}")
    );
    let with_data = lang_pack().with_data(SourceText::default());
    assert!(format!("{with_data:?}").contains("data: true"));
}

/// Up to three edits of `source`, each a character taken out, put in or
/// changed, from those that shape the engine's text formats.
pub(crate) fn edited(source: &'static str) -> impl Strategy<Value = String> {
    const CHARS: &[char] = &[
        '(', ')', '{', '}', '[', ']', ':', ',', '"', '\'', '\n', ' ', '-', '#', '@', '*', '>', '.',
        '/', 'a', 'z', '0', '9', '~', 'é', '\0',
    ];
    let edit = (
        any::<prop::sample::Index>(),
        0..3_u8,
        prop::sample::select(CHARS),
    );
    proptest::collection::vec(edit, 1..4).prop_map(move |edits| {
        let mut text: Vec<char> = source.chars().collect();
        for (at, how, c) in edits {
            let i = at.index(text.len() + 1);
            match (how, i < text.len()) {
                (0, true) => {
                    text.remove(i);
                }
                (1, true) => text[i] = c,
                _ => text.insert(i, c),
            }
        }
        text.into_iter().collect()
    })
}

/// Random bytes.
fn bytes() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(any::<u8>(), 0..300)
}

/// `magic`, then random bytes.
fn after(magic: &'static [u8]) -> impl Strategy<Value = Vec<u8>> {
    bytes().prop_map(move |rest| [magic, &rest].concat())
}

/// Printable text.
const TEXT: &str = "[ -~\n]{0,300}";

/// A one-bit picture's flags, number of styles, size and source size,
/// whether its body is as long as they say, and a body if not.
type OneBitParts = (u8, u8, (u16, u16), (u16, u16), bool, Vec<u8>);

/// The start of a one-bit picture's header with these flags, styles and
/// sizes, then `body`; or, when `exact`, as many bytes as the header says.
fn one_bit((flags, styles, size, source, exact, body): OneBitParts) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend([bitmap::VERSION, flags, styles, 0]);
    for side in [size.0, size.1, source.0, source.1] {
        bytes.extend(side.to_le_bytes());
    }
    let planes = usize::from(styles) + usize::from(flags & 1);
    let wanted = bitmap::Bitmap::plane_bytes(size.0, size.1) * planes;
    if exact {
        bytes.resize(bytes.len() + wanted, 0x5a);
    } else {
        bytes.extend(body);
    }
    bytes
}

/// Decodes `bytes` at `path` with `format`, alone and beside
/// [`fixtures`]: neither may panic.
fn decodes<F: Format>(format: &F, path: &str, bytes: &[u8]) {
    let _ = alone(format, path, bytes);
    let cx = DecodeCx::in_bundle(fixtures());
    let _ = format.decode(&AssetPath::new(path), bytes, &cx);
}

proptest! {
    #[test]
    fn no_bytes_panic_an_image_format(
        random in bytes(),
        png in after(PNG_SIGNATURE),
        (width, height, rest) in (1..64_u32, 1..64_u32, bytes()),
        bits in after(&MAGIC),
        shaped in (0..3_u8, 0..4_u8, (0..9_u16, 0..9_u16), (0..5000_u16, 0..9_u16), any::<bool>(), bytes()).prop_map(one_bit),
    ) {
        let header = [png_header(width, height), rest].concat();
        let standard = Formats::standard();
        for bytes in [&random, &png, &header, &bits, &shaped] {
            decodes(&PngInfo, "a.png", bytes);
            decodes(&Png, "a.png", bytes);
            decodes(&OneBitInfo, "a.png", bytes);
            let _ = standard.decode::<ImageInfo>(&AssetPath::new("a.png"), bytes, &DecodeCx::new());
        }
    }

    #[test]
    fn no_bytes_panic_a_sound_format(
        random in bytes(),
        riff in after(b"RIFF\0\0\0\0WAVE"),
        fmt in (any::<u32>(), bytes()).prop_map(|(size, rest)| [&b"RIFF\0\0\0\0WAVEfmt "[..], &size.to_le_bytes(), &rest].concat()),
        oggs in after(b"OggS"),
        pages in (any::<u8>(), any::<u32>(), any::<u64>(), bytes()).prop_map(|(channels, rate, granule, body)| {
            let mut o = ogg(channels, rate);
            o.extend_from_slice(&body);
            o.extend_from_slice(&ogg_page(granule, &body));
            o
        }),
    ) {
        for bytes in [&random, &riff, &fmt, &oggs, &pages] {
            decodes(&Wav, "a.wav", bytes);
            decodes(&OggVorbis, "a.ogg", bytes);
            decodes(&OggVorbisTrack, "a.ogg", bytes);
        }
    }

    #[test]
    fn no_bytes_panic_a_ron_sidecar(
        random in bytes(),
        text in TEXT,
        palette in edited(PALETTE),
        lights in edited(LIGHTS),
        patterns in edited(PATTERN_MAP),
        names in edited(NAMES),
        atlas in edited(ATLAS),
        credits in edited(BAD_CREDITS),
        manifest in edited(MANIFEST),
        table in edited(TABLE),
    ) {
        let random_text = [random.as_slice(), text.as_bytes()];
        for bytes in random_text.iter().copied().chain([palette.as_bytes()]) {
            decodes(&PaletteRon::requiring(["black"]), "p.ron", bytes);
        }
        for bytes in random_text.iter().copied().chain([patterns.as_bytes()]) {
            decodes(&PatternMapRon, "p@onebit.ron", bytes);
        }
        for bytes in random_text.iter().copied().chain([lights.as_bytes()]) {
            decodes(&LightingRon::default(), "l.ron", bytes);
        }
        for bytes in random_text.iter().copied().chain([names.as_bytes()]) {
            decodes(&NamesRon, "names.ron", bytes);
        }
        for bytes in random_text.iter().copied().chain([atlas.as_bytes()]) {
            decodes(&FontAtlasRon::requiring("ab"), "fonts/atlas.ron", bytes);
        }
        for bytes in random_text.iter().copied().chain([credits.as_bytes()]) {
            decodes(&CreditsRon::default(), "credits.ron", bytes);
        }
        for bytes in random_text.iter().copied().chain([manifest.as_bytes()]) {
            decodes(&AudioManifestRon, "audio/audio.ron", bytes);
        }
        for bytes in random_text.iter().copied().chain([table.as_bytes()]) {
            decodes(&Ron::<Table>::new(), "t.ron", bytes);
        }
    }

    #[test]
    fn no_bytes_panic_a_story_format(
        random in bytes(),
        text in TEXT,
        script in edited(SCRIPT),
        lines in "((@(scene|end|left|right|caption|music|choice|endchoice|if|else|endif|weather|shake)( [a-z_]{0,6}){0,2}|[a-z]{1,5}(\\[[a-z]{1,4}\\])?: [ -~]{0,20}|> [ -~]{0,20}|\\* [a-z]{1,5}: [ -~]{0,10}|  [a-z]{1,5}: [a-z ]{0,10}|)\n){0,30}",
        film in edited(FILM),
        keymap in edited(sample::SOURCE),
    ) {
        let random_text = [random.as_slice(), text.as_bytes()];
        for bytes in random_text.iter().copied().chain([script.as_bytes(), lines.as_bytes()]) {
            decodes(&Dlg::<Cue>::new(), "story/a.dlg", bytes);
            decodes(&Dlg::<NoDirective>::new(), "story/a.dlg", bytes);
        }
        for bytes in random_text.iter().copied().chain([film.as_bytes()]) {
            decodes(&CinematicRon::<Shot>::new(), "films/title.ron", bytes);
        }
        for bytes in random_text.iter().copied().chain([keymap.as_bytes()]) {
            decodes(&KeymapRon::<sample::Action>::new(), "keymap.ron", bytes);
        }
    }

    #[test]
    fn no_bytes_panic_the_map_format(
        random in bytes(),
        text in TEXT,
        map in edited(MAP),
        shaped in "\\(name: \"[a-z]{0,4}\", legend: \\{('[.~T#]': \"(plain|water|[a-z]{0,3})\", ?){0,3}\\}(, look: \\(tiles: \"[a-z]{0,8}\"\\))?\\)\n---\n([.~T# ]{0,9}\n){0,9}",
    ) {
        for bytes in [random.as_slice(), text.as_bytes(), map.as_bytes(), shaped.as_bytes()] {
            decodes(&map_text(), "maps/a.map", bytes);
        }
    }

    #[test]
    fn no_bytes_panic_a_language_pack(
        random in bytes(),
        text in TEXT,
        info in edited(PACK_INFO),
        ui in edited(PACK_UI),
        english in edited(ENGLISH_UI),
    ) {
        for bytes in [random.as_slice(), text.as_bytes(), info.as_bytes()] {
            decodes(&lang_pack(), "lang/xx/lang.ron", bytes);
        }
        let files = bundle(vec![
            ("lang/en/ui.ron", english.into_bytes()),
            ("lang/xx/lang.ron", PACK_INFO.into()),
            ("lang/xx/ui.ron", ui.into_bytes()),
        ]);
        let _ = bundled(&lang_pack(), files, "lang/xx/lang.ron");
    }
}
