//! Hello, Grid baked into a pack (ticket 2204): `cargo shuyi bake` makes
//! one, the game loads from it what it loads from its files, and `cargo
//! shuyi check --baked` proves the two equal. Its first frame's files are
//! laid first, and its load waits for them where the pack streams (3318).

use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use hello_grid::tools::HelloTools;
use hello_grid::{Hello, PACK_STAMP};
use shuyi::assets::pack::{InMemory, Index, Pack, PackError, Source, SourceError};
use shuyi::platform::Progress;
use shuyi::prelude::{Bundle, Layer};
use shuyi_cli::prelude::{BakeInput, CheckInput, Config, GameTools, Project, bake, check};

/// The game's pack, as `cargo shuyi bake` makes it, kept for the run as
/// a game that embeds it keeps it.
fn baked() -> &'static [u8] {
    let game = HelloTools.bake(None).unwrap_or_else(|e| panic!("{e}"));
    let baked = game
        .bake
        .bake(&game.formats, &game.bundle, None)
        .unwrap_or_else(|e| panic!("{e:?}"));
    baked.bytes.leak()
}

#[test]
fn the_game_loads_from_its_pack_what_it_loads_from_its_files() {
    let bytes = baked();
    let from_files = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    let from_pack = Hello::from_pack(bytes).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(format!("{from_pack:?}"), format!("{from_files:?}"));
    // Its text went in baked, its pictures as they are.
    let pack = Pack::open(bytes, PACK_STAMP).unwrap();
    let kinds = |path: &str| {
        let file = pack.files().iter().find(|f| f.path == path).unwrap();
        file.kinds.clone()
    };
    assert_eq!(kinds("palette.ron"), ["palette"]);
    assert_eq!(kinds("stage.map"), ["map"]);
    let found = kinds("tiles.png");
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        pack.get("tiles.png"),
        hello_grid::file("tiles.png").as_deref()
    );
}

/// A mod's files, laid over the game's.
static MOD: &[(&str, &[u8])] = &[("stage.map", b"a mod's map"), ("new.txt", b"new")];

#[test]
fn the_pack_has_the_fingerprint_of_the_files_it_was_baked_from() {
    let loose = Bundle::new(vec![hello_grid::layer()]);
    let pack = Pack::open(baked(), PACK_STAMP).unwrap();
    assert_eq!(pack.info().content, loose.fingerprint());
    let layer = pack.into_layer("pack");
    let packed = Bundle::new(vec![layer]);
    assert_eq!(packed.fingerprint(), loose.fingerprint());
    assert_eq!(hello_grid::made().content, packed.fingerprint());
    // A loose layer over the pack counts as it does over the files.
    let over = |under: Layer| Bundle::new(vec![Layer::table("mod", MOD), under]).fingerprint();
    assert_eq!(over(layer), over(hello_grid::layer()));
    assert_ne!(over(layer), loose.fingerprint());
}

#[test]
fn a_pack_of_another_game_or_version_is_refused() {
    let mut bytes = baked().to_vec();
    let error = Hello::from_pack(b"not a pack").unwrap_err();
    assert_eq!(error[0].file, "pack");
    assert_eq!(error[0].message, PackError::NotAPack.to_string());
    bytes[8] ^= 1;
    let error = Hello::from_pack(bytes.leak()).unwrap_err();
    assert!(error[0].message.contains("bake it again"), "{error:?}");
}

#[test]
fn check_baked_passes_on_the_game() {
    let mut input = CheckInput::default();
    input.baked = true;
    let answer = check(&HelloTools, &input);
    assert!(answer.ok, "{answer:?}");
}

#[test]
fn bake_writes_the_games_pack() {
    let dir = std::env::temp_dir().join(format!("hello-grid-bake-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let project = Project::new(dir.clone(), Config::default());
    let answer = bake(&HelloTools, &project, &BakeInput::default());
    assert!(answer.ok, "{answer:?}");
    let made = answer.result.unwrap().pack.unwrap();
    // The four sound effects are kept as their own bytes, not baked.
    assert_eq!(made.files, 15);
    assert_eq!(made.baked, 6);
    let bytes = fs::read(dir.join(&made.file)).unwrap();
    assert!(Hello::from_pack(bytes.leak()).is_ok());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_pack_lays_the_first_frames_files_first() {
    let bytes = baked();
    let index = Index::read(bytes, PACK_STAMP).unwrap();
    let spans: Vec<(String, core::ops::Range<u64>)> = index
        .spans()
        .map(|(dir, range)| (dir.to_owned(), range))
        .collect();
    let named: Vec<&str> = spans.iter().map(|(dir, _)| dir.as_str()).collect();
    assert_eq!(named, hello_grid::FIRST);
    // One after the other, and every other file after them all.
    for pair in spans.windows(2) {
        assert!(pair[0].1.end <= pair[1].1.start, "{spans:?}");
    }
    let first_end = first_end(&index);
    let in_first = |path: &str| {
        hello_grid::FIRST
            .iter()
            .any(|first| path == *first || path.starts_with(&format!("{first}/")))
    };
    for path in index.paths() {
        let range = index.file_range(path).unwrap();
        assert_eq!(range.end <= first_end, in_first(path), "{path}");
    }
}

/// Where the last of the first frame's files ends in the pack of `index`.
fn first_end(index: &Index) -> u64 {
    index
        .spans()
        .map(|(_, range)| range.end)
        .max()
        .unwrap_or_else(|| panic!("the pack lays the first frame's files first"))
}

/// The game's pack as the page streams it: its first `filled` bytes have
/// arrived, and once `failed` the rest never will.
struct Streaming {
    bytes: &'static [u8],
    filled: AtomicU64,
    failed: AtomicBool,
}

impl Source for Streaming {
    fn read(&self, at: u64, into: &mut [u8]) -> Result<(), SourceError> {
        if at + into.len() as u64 <= self.filled.load(Ordering::Relaxed) {
            return InMemory::new(self.bytes).read(at, into);
        }
        if self.failed.load(Ordering::Relaxed) {
            return Err(SourceError::Failed("the fetch broke".to_owned()));
        }
        Err(SourceError::NotYet)
    }

    fn arrived(&self, at: u64, len: u64) -> bool {
        at + len <= self.filled.load(Ordering::Relaxed) || self.failed.load(Ordering::Relaxed)
    }
}

/// How the game's load ends, or `None` once it has waited for `waits`
/// waits, its steps over `bundle`, and the count it last gave.
fn load_over(bundle: Bundle, waits: u32) -> (Option<String>, Option<(u32, u32)>) {
    let mut step = hello_grid::loading::in_steps(
        move || Ok((bundle, hello_grid::formats())),
        |data, _, _| {
            Err(match data {
                Ok(_) => "loaded".to_owned(),
                Err(errors) => errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            })
        },
    );
    let (mut waited, mut count) = (0, None);
    for _ in 0..100 {
        match step() {
            Progress::Wait => {
                waited += 1;
                if waited == waits {
                    return (None, count);
                }
            }
            Progress::Of { done, of } => count = Some((done, of)),
            Progress::Done(Err(end)) => return (Some(end), count),
            _ => panic!("the test's load ends with what it read"),
        }
    }
    panic!("the load neither ended nor waited");
}

/// The first `count` steps of the game's load over `bundle`: `of done/of`
/// for a count, `wait` for a wait.
fn first_steps(bundle: Bundle, count: usize) -> Vec<String> {
    let mut step = hello_grid::loading::in_steps(
        move || Ok((bundle, hello_grid::formats())),
        |_, _, _| Err("loaded".to_owned()),
    );
    (0..count)
        .map(|_| match step() {
            Progress::Wait => "wait".to_owned(),
            Progress::Of { done, of } => format!("of {done}/{of}"),
            _ => "done".to_owned(),
        })
        .collect()
}

#[test]
fn the_load_waits_for_the_first_frames_files_and_reads_them_once_they_arrive() {
    let bytes = baked();
    let index = Index::read(bytes, PACK_STAMP).unwrap();
    let first_end = first_end(&index);
    let of = u32::try_from(hello_grid::FIRST.len()).unwrap() + hello_grid::DataLoad::FILES;
    let streaming = |filled: u64| -> (&'static Streaming, Bundle) {
        let source: &'static Streaming = Box::leak(Box::new(Streaming {
            bytes,
            filled: AtomicU64::new(filled),
            failed: AtomicBool::new(false),
        }));
        let index = Index::read(bytes, PACK_STAMP).unwrap();
        (source, Bundle::new(vec![index.into_layer("files", source)]))
    };
    // Only the index has arrived: the load waits, a count and a wait a
    // frame, and reads nothing.
    let index_end = index.spans().next().unwrap().1.start;
    let (_, bundle) = streaming(index_end);
    assert_eq!(load_over(bundle, 3), (None, Some((0, of))));
    // Half the first frame's files: it counts those.
    let span_end = index.spans().nth(2).unwrap().1.end;
    let (_, bundle) = streaming(span_end);
    assert_eq!(load_over(bundle, 3), (None, Some((3, of))));
    // Each frame gives the count first, then waits: the frame that waits
    // shows it on the card.
    let (_, bundle) = streaming(span_end);
    let count = format!("of 3/{of}");
    assert_eq!(
        first_steps(bundle, 5),
        [
            format!("of 0/{of}"),
            count.clone(),
            "wait".to_owned(),
            count,
            "wait".to_owned()
        ]
    );
    // The first frame's files, and nothing after them: it loads.
    let (_, bundle) = streaming(first_end);
    assert_eq!(
        load_over(bundle, 1),
        (Some("loaded".to_owned()), Some((of, of)))
    );
    // The fetch broke half way through the first frame's files: what
    // waited reads the missing files and says so, and waits no more.
    let (source, bundle) = streaming(span_end);
    source.failed.store(true, Ordering::Relaxed);
    let (end, _) = load_over(bundle, 1);
    let end = end.expect("the load ends");
    assert!(end.contains("stage.map"), "{end}");
    assert!(!end.contains("en.ron"), "{end}");
}
