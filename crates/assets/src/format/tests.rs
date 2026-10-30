//! The registry's tests: registering and replacing, the lookup order, the
//! limits, finding an asset by id, and paths. Its formats are made up:
//! words read from text.
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::bundle::Layer;

/// The tests' kind.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Word(String);

impl AssetKind for Word {
    const NAME: &'static str = "word";
}

/// A second kind, whose format claims an extension a word format claims.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Number(u32);

impl AssetKind for Number {
    const NAME: &'static str = "number";
}

/// A word format: it claims `extensions`, sniffs files that start with
/// `magic` (if it has one), reads at most `max` bytes, and reads a file as
/// the word `<id>:<text>`, unless the text holds `no-<id>`. It counts its
/// decodings.
struct Probe {
    id: &'static str,
    extensions: &'static [&'static str],
    magic: Option<&'static str>,
    max: usize,
    calls: Arc<AtomicUsize>,
}

impl Probe {
    fn new(id: &'static str, extensions: &'static [&'static str]) -> Self {
        Self {
            id,
            extensions,
            magic: None,
            max: DEFAULT_MAX_BYTES,
            calls: Arc::default(),
        }
    }

    fn sniffing(self, magic: &'static str) -> Self {
        Self {
            magic: Some(magic),
            ..self
        }
    }

    fn max(self, max: usize) -> Self {
        Self { max, ..self }
    }
}

impl Format for Probe {
    type Asset = Word;

    fn id(&self) -> &'static str {
        self.id
    }

    fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        self.magic
            .is_some_and(|magic| bytes.starts_with(magic.as_bytes()))
    }

    fn max_bytes(&self) -> usize {
        self.max
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Word, Vec<ContentError>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let text = text(file, bytes)?;
        if text.contains(&format!("no-{}", self.id)) {
            let message = format!("{} can't read it", self.id);
            return Err(vec![ContentError::new(file.shown(), message)]);
        }
        Ok(Word(format!("{}:{text}", self.id)))
    }
}

/// A number format that keeps every default of [`Format`]: it sniffs
/// nothing and reads up to [`DEFAULT_MAX_BYTES`].
struct Digits;

impl Format for Digits {
    type Asset = Number;

    fn id(&self) -> &'static str {
        "digits"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["up"]
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Number, Vec<ContentError>> {
        let text = text(file, bytes)?;
        text.trim()
            .parse()
            .map(Number)
            .map_err(|_| vec![ContentError::new(file.shown(), "not a number")])
    }
}

/// `upper` (`.up`, sniffs `UP`), `either` (`.alt`, sniffs `UP` too) and
/// `lower` (`.low`), in that order.
fn words() -> Formats {
    let mut formats = Formats::new();
    let probes = [
        Probe::new("upper", &["up"]).sniffing("UP"),
        Probe::new("either", &["alt"]).sniffing("UP"),
        Probe::new("lower", &["low"]),
    ];
    for probe in probes {
        assert_eq!(formats.register(probe), Ok(()));
    }
    formats
}

/// What `formats` reads `bytes` at `path` as, or the errors as text.
fn read(formats: &Formats, path: &str, bytes: &str) -> Result<String, Vec<String>> {
    formats
        .decode::<Word>(&AssetPath::new(path), bytes.as_bytes(), &DecodeCx::new())
        .map(|Word(word)| word)
        .map_err(|errors| errors.iter().map(ToString::to_string).collect())
}

#[test]
fn formats_are_kept_per_kind_in_the_order_they_were_registered() {
    let mut formats = words();
    assert_eq!(formats.ids::<Word>(), ["upper", "either", "lower"]);
    assert_eq!(formats.extensions::<Word>(), ["up", "alt", "low"]);
    // Another kind may claim an extension a word format claims.
    assert_eq!(formats.register(Digits), Ok(()));
    assert_eq!(formats.ids::<Number>(), ["digits"]);
    assert_eq!(formats.ids::<Word>(), ["upper", "either", "lower"]);
    // A kind with no format has none.
    let got = Formats::new().ids::<Word>();
    assert!(got.is_empty(), "{got:?}");
    let got = Formats::new().extensions::<Word>();
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_second_format_for_one_extension_is_refused_naming_both() {
    let mut formats = words();
    let error = formats.register(Probe::new("shouting", &["yell", "up"]));
    assert_eq!(
        error,
        Err(RegisterError::Extension {
            kind: "word",
            format: "shouting",
            earlier: "upper",
            extension: "up",
        })
    );
    assert_eq!(
        error.map_err(|e| e.to_string()),
        Err(
            "word format \"shouting\" claims the extension \"up\", which word format \"upper\" \
             claims already; replace \"upper\" to read .up files with \"shouting\""
                .to_owned()
        )
    );
    // Another extension, another earlier format.
    let error = formats.register(Probe::new("loud", &["low"]));
    assert!(matches!(
        error,
        Err(RegisterError::Extension {
            earlier: "lower",
            ..
        })
    ));
    // Nothing was added.
    assert_eq!(formats.ids::<Word>(), ["upper", "either", "lower"]);
}

#[test]
fn a_second_format_with_one_id_is_refused() {
    let mut formats = words();
    let error = formats.register(Probe::new("lower", &["other"]));
    assert_eq!(
        error,
        Err(RegisterError::Id {
            kind: "word",
            format: "lower",
        })
    );
    assert_eq!(
        error.map_err(|e| e.to_string()),
        Err(
            "word format \"lower\" is registered already; replace it to use the new one".to_owned()
        )
    );
    assert_eq!(formats.ids::<Word>(), ["upper", "either", "lower"]);
}

#[test]
fn an_extension_is_lowercase_without_a_dot_and_not_empty() {
    for bad in [".png", "PNG", "Png", "", "tar.gz"] {
        let mut formats = Formats::new();
        let extensions: &'static [&'static str] = Box::leak(Box::new([bad]));
        let error = formats.register(Probe::new("odd", extensions));
        assert_eq!(
            error,
            Err(RegisterError::BadExtension {
                kind: "word",
                format: "odd",
                extension: bad,
            }),
            "{bad:?}"
        );
        let replaced = formats.replace(Probe::new("odd", extensions));
        assert!(replaced.is_err(), "{bad:?}");
        // A refused format adds no kind.
        assert_eq!(format!("{formats:?}"), format!("{:?}", Formats::new()));
    }
    let error = Formats::new().register(Probe::new("odd", &["ok", ".x"]));
    assert_eq!(
        error.map_err(|e| e.to_string()),
        Err(
            "word format \"odd\" claims the extension \".x\"; an extension is lowercase, \
             without the dot, and not empty"
                .to_owned()
        )
    );
    // Digits and other characters are fine.
    assert_eq!(
        Formats::new().register(Probe::new("ok", &["1bit", "c++"])),
        Ok(())
    );
}

#[test]
fn a_format_replaces_those_that_share_its_id_or_an_extension_in_their_place() {
    let mut formats = words();
    // It shares `lower`'s id and `upper`'s extension: it takes `upper`'s
    // place, the first of them.
    let replaced = formats.replace(Probe::new("lower", &["up"]));
    assert_eq!(replaced, Ok(vec!["upper", "lower"]));
    assert_eq!(formats.ids::<Word>(), ["lower", "either"]);
    assert_eq!(formats.extensions::<Word>(), ["up", "alt"]);
    assert_eq!(read(&formats, "a.up", "hi"), Ok("lower:hi".to_owned()));
    // One that clashes with none goes last.
    let replaced = formats.replace(Probe::new("new", &["new"]));
    assert_eq!(replaced, Ok(Vec::new()));
    assert_eq!(formats.ids::<Word>(), ["lower", "either", "new"]);
    // Into a kind with no format yet.
    let mut numbers = Formats::new();
    assert_eq!(numbers.replace(Digits), Ok(Vec::new()));
    assert_eq!(numbers.ids::<Number>(), ["digits"]);
    // The last of several.
    let mut formats = words();
    assert_eq!(
        formats.replace(Probe::new("lower", &["lower"])),
        Ok(vec!["lower"])
    );
    assert_eq!(formats.ids::<Word>(), ["upper", "either", "lower"]);
    assert_eq!(read(&formats, "a.low", "hi").ok(), None);
    assert_eq!(read(&formats, "a.lower", "hi"), Ok("lower:hi".to_owned()));
}

#[test]
fn the_formats_that_sniff_a_file_are_tried_before_the_one_its_extension_names() {
    let formats = words();
    // `lower` claims `.low`, but `upper` knows the bytes.
    assert_eq!(
        read(&formats, "a.low", "UP hi"),
        Ok("upper:UP hi".to_owned())
    );
    // The next to sniff it, when the first can't read it.
    assert_eq!(
        read(&formats, "a.low", "UP no-upper"),
        Ok("either:UP no-upper".to_owned())
    );
    // Then the one that claims the extension.
    assert_eq!(
        read(&formats, "a.low", "UP no-upper no-either"),
        Ok("lower:UP no-upper no-either".to_owned())
    );
    // When none can, the first one's errors.
    assert_eq!(
        read(&formats, "a.low", "UP no-upper no-either no-lower"),
        Err(vec!["a.low: upper can't read it".to_owned()])
    );
    // With no sniff, the extension's, in any case.
    assert_eq!(read(&formats, "a.LOW", "hi"), Ok("lower:hi".to_owned()));
    assert_eq!(read(&formats, "a.alt", "hi"), Ok("either:hi".to_owned()));
    assert_eq!(
        read(&formats, "a.low", "no-lower"),
        Err(vec!["a.low: lower can't read it".to_owned()])
    );
}

#[test]
fn a_format_that_sniffs_a_file_and_claims_its_extension_is_tried_once() {
    let upper = Probe::new("upper", &["up"]).sniffing("UP");
    let calls = Arc::clone(&upper.calls);
    let mut formats = Formats::new();
    assert_eq!(formats.register(upper), Ok(()));
    let read = read(&formats, "a.up", "UP no-upper");
    assert_eq!(read, Err(vec!["a.up: upper can't read it".to_owned()]));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn a_file_no_format_sniffs_or_claims_is_refused_naming_the_kinds_formats() {
    let formats = words();
    assert_eq!(
        read(&formats, "notes/a.txt", "hi"),
        Err(vec![
            "notes/a.txt: no word format knows its bytes or claims its extension \"txt\"; the \
             word formats are upper, either, lower"
                .to_owned()
        ])
    );
    assert_eq!(
        read(&formats, "README", "hi"),
        Err(vec![
            "README: no word format knows its bytes, and it has no extension; the word formats \
             are upper, either, lower"
                .to_owned()
        ])
    );
    assert_eq!(
        read(&Formats::new(), "a.low", "hi"),
        Err(vec![
            "a.low: no word format knows its bytes or claims its extension \"low\"; no word \
             format is registered"
                .to_owned()
        ])
    );
}

#[test]
fn a_format_keeps_its_defaults_unless_it_says() {
    assert!(!Digits.sniff(b""));
    assert!(!Digits.sniff(b"12"));
    assert_eq!(Digits.max_bytes(), DEFAULT_MAX_BYTES);
    let mut formats = Formats::new();
    assert_eq!(formats.register(Digits), Ok(()));
    let decode = |path: &str, bytes: &[u8]| {
        formats.decode::<Number>(&AssetPath::new(path), bytes, &DecodeCx::new())
    };
    assert_eq!(decode("a.up", b" 12 "), Ok(Number(12)));
    // Not sniffed: only its extension's files.
    assert!(decode("a.txt", b"12").is_err());
}

#[test]
fn a_file_over_a_formats_limit_or_the_contexts_is_refused_before_it_is_read() {
    let lower = Probe::new("lower", &["low"]).max(4);
    let calls = Arc::clone(&lower.calls);
    let mut formats = Formats::new();
    assert_eq!(formats.register(lower), Ok(()));
    assert_eq!(read(&formats, "a.low", "four"), Ok("lower:four".to_owned()));
    assert_eq!(
        read(&formats, "a.low", "fives"),
        Err(vec![
            "a.low: the file is 5 bytes; the \"lower\" word format reads at most 4".to_owned()
        ])
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    // The context's limit, when lower.
    let cx = DecodeCx::new().with_limits(Limits { max_bytes: 3 });
    let file = AssetPath::new("a.low");
    let errors = formats.decode::<Word>(&file, b"four", &cx).err();
    let shown: Vec<String> = errors
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        shown,
        ["a.low: the file is 4 bytes; the \"lower\" word format reads at most 3"]
    );
    assert_eq!(
        formats.decode::<Word>(&file, b"one", &cx),
        Ok(Word("lower:one".to_owned()))
    );
    // A sniffing format over its limit lets the next one try.
    let mut formats = Formats::new();
    assert_eq!(
        formats.register(Probe::new("small", &["s"]).sniffing("UP").max(2)),
        Ok(())
    );
    assert_eq!(
        formats.register(Probe::new("big", &["b"]).sniffing("UP")),
        Ok(())
    );
    assert_eq!(read(&formats, "a.s", "UP hi"), Ok("big:UP hi".to_owned()));
}

#[test]
fn decode_with_reads_with_one_format_within_its_limit() {
    let lower = Probe::new("lower", &["low"]).max(3);
    let file = AssetPath::new("x/a.txt");
    assert_eq!(
        decode_with(&lower, &file, b"abc", &DecodeCx::new()),
        Ok(Word("lower:abc".to_owned()))
    );
    let errors = decode_with(&lower, &file, b"abcd", &DecodeCx::new()).err();
    assert_eq!(
        errors.unwrap_or_default()[0].message,
        "the file is 4 bytes; the \"lower\" word format reads at most 3"
    );
    let cx = DecodeCx::new().with_limits(Limits { max_bytes: 2 });
    assert!(decode_with(&lower, &file, b"abc", &cx).is_err());
    assert!(decode_with(&lower, &file, b"ab", &cx).is_ok());
}

/// A bundle of words, shown under `base`.
fn bundle() -> Bundle {
    static FILES: &[(&str, &[u8])] = &[
        ("notes/d.txt", b"text"),
        ("words/a.low", b"hi"),
        ("words/b.low", b"lo"),
        ("words/b.up", b"UP x"),
        ("words/c", b"UP plain"),
        ("words/deep/f.alt", b"deep"),
    ];
    Bundle::new(vec![Layer::table("base", FILES)])
}

#[test]
fn an_asset_is_found_by_its_path_or_by_its_path_without_an_extension() {
    let (formats, bundle) = (words(), bundle());
    let find = |id: &str| formats.find::<Word>(&bundle, id).map_err(|e| e.to_string());
    assert_eq!(find("words/a.low"), Ok("words/a.low".to_owned()));
    assert_eq!(find("words/a"), Ok("words/a.low".to_owned()));
    assert_eq!(find("words/deep/f"), Ok("words/deep/f.alt".to_owned()));
    // A file named without an extension is found as named.
    assert_eq!(find("words/c"), Ok("words/c".to_owned()));
    assert_eq!(
        find("words/b"),
        Err(
            "base/words/b: 2 files could be this word: base/words/b.up, base/words/b.low; \
             name the one meant with its extension"
                .to_owned()
        )
    );
    assert_eq!(find("words/b.up"), Ok("words/b.up".to_owned()));
    assert_eq!(
        find("words/z"),
        Err(
            "base/words/z: file not found in asset bundle, nor words/z.up nor words/z.alt nor \
             words/z.low"
                .to_owned()
        )
    );
    // A kind with no format looks for the path alone.
    let numbers = formats
        .find::<Number>(&bundle, "words/z")
        .map_err(|e| e.to_string());
    assert_eq!(
        numbers,
        Err("base/words/z: file not found in asset bundle".to_owned())
    );
}

#[test]
fn each_problem_of_the_registry_has_its_code() {
    let (mut formats, bundle) = (words(), bundle());
    let code = |id: &str, formats: &Formats| {
        formats
            .load::<Word>(&bundle, id)
            .err()
            .and_then(|errors| errors[0].code)
    };
    assert_eq!(code("words/z", &formats), Some(codes::MISSING_FILE));
    assert_eq!(code("words/b", &formats), Some(codes::AMBIGUOUS_FILE));
    assert_eq!(code("notes/d.txt", &formats), Some(codes::NO_FORMAT));
    formats.limits = Limits { max_bytes: 1 };
    assert_eq!(code("words/a", &formats), Some(codes::TOO_BIG));
    let not_text = text(&AssetPath::new("a.ron"), &[0xff]).err();
    assert_eq!(not_text.unwrap_or_default()[0].code, Some(codes::NOT_TEXT));
    let from_bundle = ContentError::from(crate::bundle::NotText {
        path: "b.ron".to_owned(),
    });
    assert_eq!(from_bundle.code, Some(codes::NOT_TEXT));
}

#[test]
fn an_asset_is_loaded_by_id_from_the_bundle_with_the_limits_of_the_registry() {
    let (mut formats, bundle) = (words(), bundle());
    let load = |formats: &Formats, id: &str| {
        formats
            .load::<Word>(&bundle, id)
            .map(|Word(word)| word)
            .map_err(|errors| errors.iter().map(ToString::to_string).collect::<Vec<_>>())
    };
    assert_eq!(load(&formats, "words/a"), Ok("lower:hi".to_owned()));
    assert_eq!(load(&formats, "words/c"), Ok("upper:UP plain".to_owned()));
    assert_eq!(load(&formats, "words/b.up"), Ok("upper:UP x".to_owned()));
    // Errors name the file as the bundle shows it.
    assert_eq!(
        load(&formats, "notes/d.txt"),
        Err(vec![
            "base/notes/d.txt: no word format knows its bytes or claims its extension \"txt\"; \
             the word formats are upper, either, lower"
                .to_owned()
        ])
    );
    assert_eq!(load(&formats, "words/z").map_err(|e| e.len()), Err(1));
    formats.limits = Limits { max_bytes: 1 };
    assert_eq!(
        load(&formats, "words/a"),
        Err(vec![
            "base/words/a.low: the file is 2 bytes; the \"lower\" word format reads at most 1"
                .to_owned()
        ])
    );
}

/// Files two of which no word format reads.
static BROKEN: &[(&str, &[u8])] = &[
    ("a.low", b"no-lower"),
    ("b.low", b"fine"),
    ("c.alt", b"no-either"),
];

#[test]
fn every_asset_of_a_kind_under_a_directory_loads_together() {
    let (formats, bundle) = (words(), bundle());
    let words: Vec<(&str, String)> = formats
        .load_under::<Word>(&bundle, "words")
        .unwrap_or_default()
        .into_iter()
        .map(|(path, Word(word))| (path, word))
        .collect();
    // `words/c` has no extension a word format claims.
    assert_eq!(
        words,
        [
            ("words/a.low", "lower:hi".to_owned()),
            ("words/b.low", "lower:lo".to_owned()),
            ("words/b.up", "upper:UP x".to_owned()),
            ("words/deep/f.alt", "either:deep".to_owned()),
        ]
    );
    assert_eq!(
        formats.load_under::<Word>(&bundle, "notes"),
        Ok(BTreeMap::new())
    );
    // Every file's errors.
    let broken = Bundle::new(vec![Layer::table("base", BROKEN)]);
    let errors = formats
        .load_under::<Word>(&broken, "")
        .err()
        .unwrap_or_default();
    let shown: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(
        shown,
        [
            "base/a.low: lower can't read it",
            "base/c.alt: either can't read it"
        ]
    );
}

#[test]
fn each_asset_of_a_kind_directly_in_a_folder_loads_on_its_own() {
    /// Two word files (one broken), one of an extension no word format
    /// claims, and one in a subfolder.
    static FILES: &[(&str, &[u8])] = &[
        ("crops/deep/d.low", b"deep"),
        ("crops/notes.txt", b"text"),
        ("crops/pumpkin.low", b"fine"),
        ("crops/turnip.low", b"no-lower"),
    ];
    let (formats, bundle) = (words(), Bundle::new(vec![Layer::table("base", FILES)]));
    let each: Vec<(&str, Result<String, Vec<String>>)> = formats
        .load_each::<Word>(&bundle, "crops")
        .into_iter()
        .map(|(path, read)| {
            let read = read
                .map(|Word(word)| word)
                .map_err(|errors| errors.iter().map(ToString::to_string).collect());
            (path, read)
        })
        .collect();
    assert_eq!(
        each,
        [
            ("crops/pumpkin.low", Ok("lower:fine".to_owned())),
            (
                "crops/turnip.low",
                Err(vec![
                    "base/crops/turnip.low: lower can't read it".to_owned()
                ])
            ),
        ]
    );
    // An empty or missing folder gives an empty map.
    let seen = formats.load_each::<Word>(&bundle, "seasons");
    assert!(seen.is_empty(), "{seen:?}");
    let seen = formats.load_each::<Word>(&bundle, "crops/deep/d.low");
    assert!(seen.is_empty(), "{seen:?}");
    let seen = formats.load_each::<Word>(&Bundle::new(Vec::new()), "");
    assert!(seen.is_empty(), "{seen:?}");
}

#[test]
fn a_registry_claims_the_extensions_of_its_kinds_formats_in_any_case() {
    let formats = words();
    assert!(formats.claims::<Word>("a.low"));
    assert!(formats.claims::<Word>("x/a.UP"));
    assert!(!formats.claims::<Word>("a.txt"));
    assert!(!formats.claims::<Word>("low"));
    assert!(!formats.claims::<Word>("x/.low"));
    assert!(!formats.claims::<Number>("a.up"));
}

#[test]
fn a_registry_shows_its_kinds_formats_sections_and_limits() {
    let mut formats = words();
    assert_eq!(formats.register(Digits), Ok(()));
    assert_eq!(formats.register_import_settings::<Mirror>(), Ok(()));
    formats.limits = Limits { max_bytes: 9 };
    assert_eq!(
        format!("{formats:?}"),
        "Formats { kinds: [(\"word\", [\"upper\", \"either\", \"lower\"]), (\"number\", \
         [\"digits\"])], sections: [\"mirror\"], limits: Limits { max_bytes: 9 } }"
    );
}

#[test]
fn the_standard_formats_are_the_engines_that_need_nothing_of_a_game() {
    use crate::audio::{AudioManifest, MusicTrack, Sound};
    use crate::credits::CreditsFile;
    use crate::dialogue::DialogueFile;
    use crate::font::FontAtlasDef;
    use crate::image::{Image, ImageInfo};
    use crate::lighting::Lighting;
    use crate::names::Names;
    use crate::onebit::PatternMapDef;
    use crate::palette::PaletteDef;
    use crate::target::TargetProfile;

    let formats = Formats::standard();
    assert_eq!(formats.ids::<ImageInfo>(), ["png", "one-bit"]);
    assert_eq!(formats.extensions::<ImageInfo>(), ["png"]);
    assert_eq!(formats.ids::<Image>(), ["png"]);
    assert_eq!(formats.ids::<Sound>(), ["wav", "ogg-vorbis"]);
    assert_eq!(formats.extensions::<Sound>(), ["wav", "ogg"]);
    assert_eq!(formats.ids::<MusicTrack>(), ["ogg-vorbis"]);
    assert_eq!(formats.ids::<AudioManifest>(), ["audio-manifest"]);
    assert_eq!(formats.ids::<CreditsFile>(), ["credits"]);
    assert_eq!(formats.ids::<FontAtlasDef>(), ["font-atlas"]);
    assert_eq!(formats.ids::<PaletteDef>(), ["palette"]);
    assert_eq!(formats.ids::<PatternMapDef>(), ["pattern-map"]);
    assert_eq!(formats.ids::<Lighting>(), ["lights"]);
    assert_eq!(formats.ids::<Names>(), ["names"]);
    assert_eq!(formats.ids::<DialogueFile>(), ["dlg"]);
    assert_eq!(formats.ids::<TargetProfile>(), ["target-ron"]);
    assert_eq!(formats.limits, Limits::NONE);
    // Every kind: nothing else.
    let kinds = format!("{formats:?}").matches("\", [\"").count();
    assert_eq!(kinds, 13);
    // Registered again, the first is refused.
    let mut twice = Formats::standard();
    assert_eq!(
        twice.register_standard(),
        Err(RegisterError::Id {
            kind: "image",
            format: "png",
        })
    );
    // On an empty registry, the same.
    let mut empty = Formats::new();
    assert_eq!(empty.register_standard(), Ok(()));
    assert_eq!(format!("{empty:?}"), format!("{formats:?}"));
}

#[test]
fn a_path_has_a_name_a_directory_an_extension_and_a_stem() {
    let path = AssetPath::new("images/a.png");
    assert_eq!(
        (path.path(), path.shown()),
        ("images/a.png", "images/a.png")
    );
    assert_eq!((path.name(), path.dir()), ("a.png", "images"));
    assert_eq!((path.extension(), path.stem()), (Some("png"), "a"));
    assert_eq!(path.sibling("b.ron"), "images/b.ron");
    assert_eq!(path.with_extension("ron"), "images/a.ron");
    let top = AssetPath::new("top");
    assert_eq!((top.name(), top.dir(), top.extension()), ("top", "", None));
    assert_eq!((top.stem(), top.sibling("x")), ("top", "x".to_owned()));
    assert_eq!(top.with_extension("png"), "top.png");
    for (path, extension, stem) in [
        ("a.PNG", Some("PNG"), "a"),
        ("a.tar.gz", Some("gz"), "a.tar"),
        (".hidden", None, ".hidden"),
        ("x/.png", None, ".png"),
        ("dir.d/file", None, "file"),
        ("a.", Some(""), "a"),
    ] {
        let path = AssetPath::new(path);
        assert_eq!(
            (path.extension(), path.stem()),
            (extension, stem),
            "{path:?}"
        );
    }
    assert_eq!(AssetPath::new("dir.d/file").dir(), "dir.d");
    let shown = AssetPath::new("a.png").shown_as("base/a.png");
    assert_eq!((shown.path(), shown.shown()), ("a.png", "base/a.png"));
    let bundled = AssetPath::in_bundle(&bundle(), "words/a.low");
    assert_eq!(
        bundled,
        AssetPath::new("words/a.low").shown_as("base/words/a.low")
    );
    assert_eq!(join("", "a"), "a");
    assert_eq!(join("d/e", "a"), "d/e/a");
}

#[test]
fn a_context_reads_the_files_a_file_names_from_its_bundle() {
    static FILES: &[(&str, &[u8])] = &[("a/b.txt", b"b"), ("a/c.bin", &[0xff]), ("a/d/e", b"e")];
    let trimmed = Layer::table("base", FILES).leaving_out(&["a/gone.txt"]);
    let bundle = Bundle::new(vec![trimmed]);
    let cx = DecodeCx::in_bundle(&bundle);
    assert_eq!(cx.limits, Limits::NONE);
    assert!(cx.has("a/b.txt") && !cx.has("a/x"));
    assert_eq!(cx.bytes("a/c.bin"), Some(&[0xff][..]));
    assert_eq!(cx.text("a/b.txt"), Some("b"));
    assert_eq!((cx.text("a/c.bin"), cx.bytes("a/x")), (None, None));
    assert_eq!(cx.shown("a/x"), "base/a/x");
    assert!(cx.left_out("a/gone.txt") && !cx.left_out("a/b.txt"));
    assert_eq!(cx.files_under("a"), ["a/b.txt", "a/c.bin", "a/d/e"]);
    // Without a bundle, every file named is missing.
    let alone = DecodeCx::new();
    assert!(alone.bundle.is_none() && !alone.has("a/b.txt"));
    assert_eq!(
        (alone.bytes("a/b.txt"), alone.text("a/b.txt")),
        (None, None)
    );
    assert_eq!(alone.shown("a/b.txt"), "a/b.txt");
    assert!(!alone.left_out("a/gone.txt"));
    let seen = alone.files_under("a");
    assert!(seen.is_empty(), "{seen:?}");
    assert_eq!(alone.limits, Limits::NONE);
    let default = DecodeCx::default();
    assert!(default.bundle.is_none() && default.limits == Limits::default());
    let limited = cx.with_limits(Limits { max_bytes: 7 });
    assert_eq!(limited.limits.max_bytes, 7);
    assert!(limited.has("a/b.txt"));
    assert_eq!(Limits::default(), Limits::NONE);
    assert_eq!(Limits::NONE.max_bytes, usize::MAX);
}

#[test]
fn a_text_format_refuses_bytes_that_arent_utf8() {
    let file = AssetPath::new("a.ron").shown_as("base/a.ron");
    assert_eq!(text(&file, b"(a: 1)"), Ok("(a: 1)"));
    let errors = text(&file, &[b'(', 0xff]).err().unwrap_or_default();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].to_string(), "base/a.ron: file is not valid UTF-8");
}

/// What the test's decode watch heard, on one thread.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Heard {
    /// `before`, with the kind, and the token it gave.
    Before(&'static str, u32),
    /// `after`, with the token and the decode.
    After(u32, &'static str, String, usize, bool),
}

std::thread_local! {
    /// What this thread's decodes told the watch, inside [`heard`].
    static HEARD: std::cell::RefCell<Option<Vec<Heard>>> =
        const { std::cell::RefCell::new(None) };
}

/// The test's `before`: a token that counts the decodes on this thread.
fn heard_before(kind: &'static str) -> u32 {
    HEARD.with_borrow_mut(|heard| {
        let Some(heard) = heard else {
            return 0;
        };
        let token = u32::try_from(heard.len()).unwrap() + 100;
        heard.push(Heard::Before(kind, token));
        token
    })
}

/// The test's `after`.
fn heard_after(token: u32, decoded: &Decoded<'_>) {
    HEARD.with_borrow_mut(|heard| {
        if let Some(heard) = heard {
            let path = decoded.path.to_owned();
            heard.push(Heard::After(
                token,
                decoded.kind,
                path,
                decoded.bytes,
                decoded.ok,
            ));
        }
    });
}

/// What this thread's decodes in `f` told the watch.
fn heard(f: impl FnOnce()) -> Vec<Heard> {
    HEARD.set(Some(Vec::new()));
    f();
    HEARD.take().unwrap_or_default()
}

/// The only test that sets the decode watch: it is process-wide.
#[test]
fn the_decode_watch_hears_each_decode_before_and_after() {
    // A table's files are sorted by path.
    static FILES: &[(&str, &[u8])] = &[
        ("numbers/n.up", b"12"),
        ("words/a.low", b"hi"),
        ("words/broken.low", b"no-lower"),
    ];
    assert!(watch_decodes(DecodeWatch::new(heard_before, heard_after)).is_ok());
    let refused = DecodeWatch::new(|_| 0, |_, _| {});
    assert!(watch_decodes(refused).is_err());
    let mut formats = words();
    assert_eq!(formats.register(Digits), Ok(()));
    let bundle = Bundle::new(vec![Layer::table("base", FILES)]);
    let seen = heard(|| {
        assert!(formats.load::<Word>(&bundle, "words/a").is_ok());
        assert!(formats.load::<Word>(&bundle, "words/broken").is_err());
        assert_eq!(formats.load::<Number>(&bundle, "numbers/n"), Ok(Number(12)));
        // Not found: nothing decoded, nothing heard.
        assert!(formats.load::<Word>(&bundle, "words/none").is_err());
    });
    let after =
        |token, kind, path: &str, bytes, ok| Heard::After(token, kind, path.to_owned(), bytes, ok);
    assert_eq!(
        seen,
        [
            Heard::Before("word", 100),
            after(100, "word", "words/a.low", 2, true),
            Heard::Before("word", 102),
            after(102, "word", "words/broken.low", 8, false),
            Heard::Before("number", 104),
            after(104, "number", "numbers/n.up", 2, true),
        ]
    );
}

#[test]
fn a_registry_is_send_and_sync() {
    fn shared<T: Send + Sync>() {}
    shared::<Formats>();
    shared::<DecodeWatch>();
}

/// The tests' section of import settings.
#[derive(Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Mirror {
    mirror: bool,
}

impl ImportSettings for Mirror {
    const SECTION: &'static str = "mirror";
}

/// A section with a field that has no default.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Strict {
    size: u8,
}

impl ImportSettings for Strict {
    const SECTION: &'static str = "strict";
}

/// A section that reads fields it doesn't have.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct Loose {}

impl ImportSettings for Loose {
    const SECTION: &'static str = "loose";
}

/// The same name as [`Mirror`].
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct OtherMirror {}

impl ImportSettings for OtherMirror {
    const SECTION: &'static str = "mirror";
}

#[test]
fn a_section_is_refused_twice_without_defaults_or_open_to_unknown_fields() {
    let mut formats = Formats::new();
    assert_eq!(formats.register_import_settings::<Mirror>(), Ok(()));
    let refused = [
        (
            formats.register_import_settings::<OtherMirror>(),
            RegisterError::SectionTwice { section: "mirror" },
            "is registered already",
        ),
        (
            formats.register_import_settings::<Strict>(),
            RegisterError::SectionWithoutDefault { section: "strict" },
            "#[serde(default)]",
        ),
        (
            formats.register_import_settings::<Loose>(),
            RegisterError::SectionAllowsUnknownFields { section: "loose" },
            "#[serde(deny_unknown_fields)]",
        ),
    ];
    for (got, want, says) in refused {
        assert_eq!(got.as_ref().err(), Some(&want));
        assert!(want.to_string().contains(says), "{want}");
    }
    assert_eq!(formats.import_settings_sections(), ["mirror"]);
    // A field without a default still reads when it is given.
    assert_eq!(
        ::ron::from_str::<Strict>("(size: 3)").map(|s| s.size).ok(),
        Some(3)
    );
    assert_eq!(
        Formats::standard().import_settings_sections(),
        Vec::<&str>::new()
    );
}

#[test]
fn no_format_may_claim_meta() {
    let mut formats = Formats::new();
    let reserved = RegisterError::Reserved {
        kind: "word",
        format: "settings",
    };
    let probe = || Probe::new("settings", &["txt", "meta"]);
    assert_eq!(formats.register(probe()), Err(reserved.clone()));
    assert_eq!(formats.replace(probe()), Err(reserved.clone()));
    assert_eq!(formats.ids::<Word>(), Vec::<&str>::new());
    assert!(
        reserved
            .to_string()
            .contains("kept for import settings files")
    );
}

/// The settings of a bundle: good ones, and one of each problem.
static SETTINGS: &[(&str, &[u8])] = &[
    ("_folder.meta", b"(mirror: (mirror: true))"),
    ("gone.png.meta", b"()"),
    (
        "images/_folder.meta",
        b"(\n    mirror: (),\n    nope: (),\n)",
    ),
    ("images/_folder.meta.meta", b"()"),
    ("images/a.png", b""),
    ("images/a.png.meta", b"(\n    mirror: (flip: true),\n)"),
    ("images/b.png", b""),
    ("images/b.png.meta", b"(mirror: ("),
    ("left.png.meta", b"()"),
];

/// What `check_import_settings` finds in the bundle of `files`, as
/// `(file, line, code)`, with a [`Mirror`] section registered.
fn settings_problems(files: &'static [(&'static str, &'static [u8])]) -> Vec<ContentError> {
    let mut formats = Formats::new();
    assert_eq!(formats.register_import_settings::<Mirror>(), Ok(()));
    let bundle = Bundle::new(vec![Layer::table("base", files).leaving_out(&["left.png"])]);
    formats.check_import_settings(&bundle)
}

#[test]
fn every_settings_file_of_a_bundle_is_checked() {
    let problems = settings_problems(SETTINGS);
    let found: Vec<(&str, Option<u32>, Option<crate::error::Code>)> = problems
        .iter()
        .map(|e| (e.file.as_str(), e.line, e.code))
        .collect();
    assert_eq!(
        found,
        [
            ("base/gone.png.meta", None, Some(codes::MISSING_FILE)),
            (
                "base/images/_folder.meta",
                Some(3),
                Some(codes::UNKNOWN_NAME)
            ),
            ("base/images/_folder.meta.meta", None, None),
            ("base/images/a.png.meta", Some(2), Some(codes::PARSE)),
            ("base/images/b.png.meta", Some(1), Some(codes::PARSE)),
        ]
    );
    let says = |at: usize, words: &str| problems.get(at).is_some_and(|e| e.message.contains(words));
    assert!(says(0, "base/gone.png, which the bundle doesn't have"));
    assert!(says(
        1,
        "unknown section \"nope\"; the sections registered are mirror"
    ));
    assert!(says(
        2,
        "settings of the settings file base/images/_folder.meta"
    ));
    assert!(says(3, "flip"));
}

#[test]
fn a_bundle_of_good_settings_has_no_problem() {
    static GOOD: &[(&str, &[u8])] = &[
        ("_folder.meta", b"// all\n(mirror: (mirror: true))"),
        ("images/a.png", b""),
        ("images/a.png.meta", b"()"),
    ];
    assert_eq!(settings_problems(GOOD), []);
    let unknown = Formats::new().check_import_settings(&Bundle::new(vec![Layer::table("b", GOOD)]));
    assert!(
        unknown
            .first()
            .is_some_and(|e| e.message.ends_with("no section is registered"))
    );
}

/// Data tables with settings files beside them and in their folder.
static TABLES: &[(&str, &[u8])] = &[
    ("data/_folder.meta", b"()"),
    ("data/a.ron", b"(rows: [1])"),
    ("data/a.ron.meta", b"()"),
    ("data/deep/b.ron", b"(rows: [])"),
    ("data/deep/b.ron.meta", b"()"),
    ("images/_folder.meta", b"()"),
];

#[test]
fn the_walkers_over_a_folder_skip_settings_files() {
    use crate::format::builtin_tests::{Table, bundle as owned, png_file};
    use crate::image::ImageTable;
    let mut formats = Formats::standard();
    assert_eq!(formats.register(crate::ron::Ron::<Table>::new()), Ok(()));
    let tables = Bundle::new(vec![Layer::table("base", TABLES)]);
    let under = formats.load_under::<Table>(&tables, "data");
    assert_eq!(
        under.map(|tables| tables.into_keys().collect::<Vec<_>>()),
        Ok(vec!["data/a.ron", "data/deep/b.ron"])
    );
    let each = formats.load_each::<Table>(&tables, "data");
    assert_eq!(each.keys().copied().collect::<Vec<_>>(), ["data/a.ron"]);
    assert!(each.values().all(Result::is_ok));
    let images = owned(vec![
        ("images/_folder.meta", b"()".to_vec()),
        ("images/a.png", png_file([0; 8])),
        ("images/a.png.meta", b"()".to_vec()),
    ]);
    let only_the_png = |table: Result<ImageTable, Vec<ContentError>>| {
        table.is_ok_and(|table| table.ids().count() == 1 && table.id("images/a.png").is_some())
    };
    assert!(only_the_png(ImageTable::load(images, &[])));
    assert!(only_the_png(ImageTable::load_with(images, &formats, &[])));
}
