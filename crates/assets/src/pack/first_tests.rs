//! Folders laid first (ADR-0128 §3): the order [`Bake::first`] lays, each
//! folder's span in the index and its check, the recipient's layout within
//! the spans, and [`Bundle::arrived`] over a source whose bytes arrive in
//! time.

use std::collections::BTreeMap;

use proptest::prelude::*;

use super::tests::{
    STAMP, assembled, code, every_kind, files_of, formats, pack_bundle, pack_files, pack_formats,
    raw_pack, sources, test_info,
};
use super::*;
use crate::bundle::Bundle;

/// The pack of `files` (path → raw bytes), the folders of `first` laid
/// first.
fn first_pack(first: &[&str], files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let files: BTreeMap<String, Content> = files
        .iter()
        .map(|(path, bytes)| (path.clone(), Content::Raw(bytes.clone())))
        .collect();
    write_first(&test_info(), first, &files)
}

/// Files at `paths`, each holding its path.
fn named(paths: &[&str]) -> Vec<(String, Vec<u8>)> {
    paths
        .iter()
        .map(|path| ((*path).to_owned(), path.as_bytes().to_vec()))
        .collect()
}

/// The paths of pack `bytes` in the order its bytes lie.
fn laid(bytes: &[u8]) -> Vec<String> {
    let index = Index::read(bytes, STAMP).unwrap_or_else(|e| panic!("{e}"));
    let mut paths: Vec<(u64, String)> = index
        .paths()
        .map(|path| (index.file_range(path).unwrap().start, path.to_owned()))
        .collect();
    paths.sort();
    paths.into_iter().map(|(_, path)| path).collect()
}

/// The spans of pack `bytes`: each folder and its range.
fn spans(bytes: &[u8]) -> Vec<(String, core::ops::Range<u64>)> {
    let index = Index::read(bytes, STAMP).unwrap_or_else(|e| panic!("{e}"));
    index
        .spans()
        .map(|(dir, range)| (dir.to_owned(), range))
        .collect()
}

/// From the first byte of the files of pack `bytes` at `paths` to the last.
fn hull(bytes: &[u8], paths: &[&str]) -> core::ops::Range<u64> {
    let index = Index::read(bytes, STAMP).unwrap();
    let ranges: Vec<_> = paths.iter().map(|p| index.file_range(p).unwrap()).collect();
    let start = ranges.iter().map(|r| r.start).min().unwrap();
    let end = ranges.iter().map(|r| r.end).max().unwrap();
    start..end
}

/// Whether each span of pack `bytes` holds every file under its folder.
fn spans_hold_their_folders(bytes: &[u8]) -> bool {
    let index = Index::read(bytes, STAMP).unwrap();
    index.spans().all(|(dir, span)| {
        index.paths().filter(|path| under(path, dir)).all(|path| {
            let file = index.file_range(path).unwrap();
            span.start <= file.start && file.end <= span.end
        })
    })
}

const PATHS: &[&str] = &[
    "a.txt",
    "part1/map.ron",
    "part1/inner/deep.ron",
    "part2/map.ron",
    "title/logo.txt",
    "title/menu.ron",
];

#[test]
fn a_folder_named_first_is_laid_at_the_front_in_the_order_named() {
    let files = named(PATHS);
    let pack = first_pack(&["title/", "part2"], &files);
    assert_eq!(
        laid(&pack),
        [
            "title/logo.txt",
            "title/menu.ron",
            "part2/map.ron",
            "a.txt",
            "part1/inner/deep.ron",
            "part1/map.ron",
        ]
    );
    assert_eq!(
        spans(&pack),
        [
            (
                "title".to_owned(),
                hull(&pack, &["title/logo.txt", "title/menu.ron"])
            ),
            ("part2".to_owned(), hull(&pack, &["part2/map.ron"])),
        ]
    );
    // The game reads the same files.
    assert_eq!(files_of(&pack), files_of(&raw_pack(&test_info(), &files)));
    // With no folder named, it is the pack `write` lays.
    assert_eq!(first_pack(&[], &files), raw_pack(&test_info(), &files));
    assert_eq!(spans(&first_pack(&[], &files)), []);
}

#[test]
fn a_file_under_two_named_folders_goes_with_the_first() {
    let files = named(PATHS);
    // The inner folder first: the outer one's span holds both groups.
    let inner_first = first_pack(&["part1/inner", "part1"], &files);
    assert_eq!(
        laid(&inner_first)[..2],
        ["part1/inner/deep.ron", "part1/map.ron"]
    );
    let both = hull(&inner_first, &["part1/inner/deep.ron", "part1/map.ron"]);
    assert_eq!(spans(&inner_first)[1], ("part1".to_owned(), both));
    // The outer one first: the inner one's files went with it, and its span
    // is where they lie, inside the outer one's.
    let outer_first = first_pack(&["part1", "part1/inner/"], &files);
    assert_eq!(
        laid(&outer_first)[..2],
        ["part1/inner/deep.ron", "part1/map.ron"]
    );
    let deep = hull(&outer_first, &["part1/inner/deep.ron"]);
    assert_eq!(spans(&outer_first)[1], ("part1/inner".to_owned(), deep));
    // A folder named twice is laid where first named, and has one span.
    let twice = first_pack(&["title", "part2", "title/"], &files);
    assert_eq!(twice, first_pack(&["title", "part2"], &files));
    // A folder named that holds no file has an empty span, and changes
    // nothing of the layout.
    let empty = first_pack(&["none/", "title"], &files);
    assert_eq!(laid(&empty), laid(&first_pack(&["title"], &files)));
    let (dir, span) = &spans(&empty)[0];
    assert_eq!((dir.as_str(), span.is_empty()), ("none", true));
    // The whole pack is a folder too.
    let all = first_pack(&[""], &files);
    assert_eq!(laid(&all), laid(&raw_pack(&test_info(), &files)));
    assert_eq!(spans(&all)[0].1, hull(&all, PATHS));
}

#[test]
fn a_bakes_folders_are_laid_first_and_the_check_holds() {
    let bake = every_kind().first("data/").first("lang");
    assert_eq!(bake.firsts(), ["data/", "lang"]);
    let baked = bake.bake(&formats(), sources(), Some("desk")).unwrap();
    let order = laid(&baked.bytes);
    let data = sources().files_under("data");
    let lang = sources().files_under("lang");
    assert_eq!(order[..data.len()], data);
    assert_eq!(order[data.len()..data.len() + lang.len()], lang);
    let named: Vec<String> = spans(&baked.bytes).into_iter().map(|(d, _)| d).collect();
    assert_eq!(named, ["data", "lang"]);
    let formats = pack_formats(&bake);
    let pack = Pack::open(&baked.bytes, STAMP).unwrap().into_owned();
    let errors = bake.check(&formats, sources(), &pack_bundle(pack));
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_bake_refuses_more_folders_than_a_pack_lays_first() {
    let mut bake = Bake::new(STAMP);
    for at in 0..=MAX_FIRST {
        bake = bake.first(&format!("f{at}"));
    }
    let errors = bake.bake(&formats(), sources(), None).unwrap_err();
    assert_eq!(
        errors[0].to_string(),
        format!("bake: the bake lays more than {MAX_FIRST} folders first")
    );
    // The same folder named again and again is one.
    let mut bake = Bake::new(STAMP);
    for _ in 0..=MAX_FIRST {
        bake = bake.first("data/");
    }
    assert!(bake.bake(&formats(), sources(), None).is_ok());
}

/// What reading the index of a pack of `files` (path, offset, length) with
/// `spans` (folder, offset, length) says.
fn read_with(files: &[(&str, u64, u64)], spans: &[(&str, u64, u64)]) -> Result<(), String> {
    let data_len = files.iter().map(|(_, at, len)| at + len).max().unwrap_or(0);
    let data = vec![0u8; usize::try_from(data_len).unwrap()];
    let entries = files
        .iter()
        .map(|&(path, offset, stored)| Entry {
            path: path.to_owned(),
            kinds: Vec::new(),
            codec: Codec::Stored,
            offset,
            stored,
            size: stored,
            sum: checksum(&vec![0; usize::try_from(stored).unwrap()]),
        })
        .collect();
    let spans = spans
        .iter()
        .map(|&(dir, offset, length)| Span {
            dir: dir.to_owned(),
            offset,
            length,
        })
        .collect();
    let listing = Listing {
        info: test_info(),
        entries,
        spans,
    };
    let bytes = assembled(&listing, &data);
    match Index::read(&bytes, STAMP) {
        Ok(_) => Pack::open(&bytes, STAMP)
            .map(drop)
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[test]
fn an_index_whose_spans_dont_hold_their_folders_is_refused() {
    let files = [
        ("a.txt", 0, 2),
        ("t/b", 2, 3),
        ("t/c/d", 5, 1),
        ("t0", 6, 1),
    ];
    assert_eq!(read_with(&files, &[("t", 2, 4)]), Ok(()));
    // A span may hold more than its folder, and bytes between its files.
    assert_eq!(read_with(&files, &[("t", 0, 7), ("t/c", 4, 2)]), Ok(()));
    assert_eq!(read_with(&files, &[("", 0, 7), ("none", 0, 0)]), Ok(()));
    let broken = |why: &str| Err(format!("the pack is broken: {why}"));
    // One byte short at either end.
    assert_eq!(
        read_with(&files, &[("t", 3, 3)]),
        broken("t/b lies outside its span of t/")
    );
    assert_eq!(
        read_with(&files, &[("t", 2, 3)]),
        broken("t/c/d lies outside its span of t/")
    );
    assert_eq!(
        read_with(&files, &[("", 1, 6)]),
        broken("a.txt lies outside its span of /")
    );
    // A file whose path starts as the folder's but isn't under it is
    // outside it, and a file under it after one that isn't is checked.
    let files = [("t-x", 0, 1), ("t.y", 1, 1), ("t/z", 2, 1)];
    assert_eq!(read_with(&files, &[("t", 2, 1)]), Ok(()));
    assert_eq!(
        read_with(&files, &[("t", 0, 2)]),
        broken("t/z lies outside its span of t/")
    );
    // Past the files' end, or past what a number holds.
    assert_eq!(
        read_with(&files, &[("t", 2, 2)]),
        broken("its span of t/ goes past its files")
    );
    assert_eq!(
        read_with(&files, &[("t", u64::MAX, 2)]),
        broken("its span of t/ goes past its files")
    );
    // A folder named wrong or twice.
    assert_eq!(
        read_with(&files, &[("t/", 2, 1)]),
        broken("its span of t// is named wrong or twice")
    );
    assert_eq!(
        read_with(&files, &[("t", 2, 1), ("t", 2, 1)]),
        broken("its span of t/ is named wrong or twice")
    );
    // Exactly the most spans reads, and one more doesn't.
    let most: Vec<String> = (0..=MAX_FIRST).map(|at| format!("f{at}")).collect();
    let spans = |count: usize| -> Vec<(&str, u64, u64)> {
        most[..count]
            .iter()
            .map(|dir| (dir.as_str(), 0, 0))
            .collect()
    };
    assert_eq!(read_with(&files, &spans(MAX_FIRST)), Ok(()));
    assert_eq!(
        read_with(&files, &spans(MAX_FIRST + 1)),
        broken(&format!("it lays more than {MAX_FIRST} folders first"))
    );
}

#[test]
fn a_recipients_layout_keeps_each_folder_in_its_span() {
    let files: Vec<(String, Vec<u8>)> = super::tests::twelve()
        .into_iter()
        .enumerate()
        .map(|(at, (_, bytes))| (format!("{}/f{at:02}", ["a", "b", "c"][at % 3]), bytes))
        .collect();
    let pack = first_pack(&["b", "a"], &files);
    let raw = Raw::read(&pack).unwrap();
    assert_eq!(raw.mark(), PackMark::None);
    let marked = raw.write_marked(code(0x1234_5678)).unwrap();
    assert_ne!(laid(&marked), laid(&pack));
    assert!(spans_hold_their_folders(&marked));
    let groups: Vec<char> = laid(&marked)
        .iter()
        .map(|path| path.chars().next().unwrap())
        .collect();
    assert_eq!(groups, "bbbbaaaacccc".chars().collect::<Vec<_>>());
    assert_eq!(files_of(&marked), files_of(&pack));
    let read = Raw::read(&marked).unwrap();
    assert_eq!(
        read.mark(),
        PackMark::Code {
            code: code(0x1234_5678),
            order_agrees: true
        }
    );
    // Laid out again with no code, it is the pack baked.
    assert_eq!(read.write(), pack);
}

#[test]
fn with_no_folder_named_a_codes_layout_is_the_one_pinned_before() {
    let files = super::tests::twelve();
    let pack = raw_pack(&test_info(), &files);
    let marked = Raw::read(&pack)
        .unwrap()
        .write_marked(code(0x1234_5678))
        .unwrap();
    let order: Vec<String> = [2, 5, 8, 1, 11, 6, 7, 4, 9, 10, 0, 3]
        .iter()
        .map(|&at: &usize| files[at].0.clone())
        .collect();
    assert_eq!(laid(&marked), order);
    // Groups take the stream group after group.
    assert_eq!(
        order_in(code(0x1234_5678), &[12, 0]),
        order_of(code(0x1234_5678), 12)
    );
    assert_eq!(order_in(code(1), &[0, 0]), Vec::<usize>::new());
    let split = order_in(code(0x1234_5678), &[3, 9]);
    assert!(split[..3].iter().all(|&at| at < 3), "{split:?}");
    assert!(split[3..].iter().all(|&at| at >= 3), "{split:?}");
}

/// A source over a pack's bytes that has every byte but those in `missing`.
struct Streaming {
    bytes: &'static [u8],
    missing: core::ops::Range<u64>,
}

impl Streaming {
    fn has(&self, at: u64, len: u64) -> bool {
        len == 0 || at + len <= self.missing.start || at >= self.missing.end
    }
}

impl Source for Streaming {
    fn read(&self, at: u64, into: &mut [u8]) -> Result<(), SourceError> {
        if !self.has(at, into.len() as u64) {
            return Err(SourceError::NotYet);
        }
        InMemory::new(self.bytes).read(at, into)
    }

    fn arrived(&self, at: u64, len: u64) -> bool {
        self.has(at, len)
    }
}

/// A bundle of pack `bytes` read file by file, the bytes in `missing` not
/// arrived yet.
fn streamed(bytes: &[u8], missing: core::ops::Range<u64>) -> Bundle {
    let bytes: &'static [u8] = bytes.to_vec().leak();
    let source: &'static Streaming = Box::leak(Box::new(Streaming { bytes, missing }));
    let index = Index::read(bytes, STAMP).unwrap();
    Bundle::new(vec![index.into_layer("pack", source)])
}

#[test]
fn a_folder_has_arrived_once_its_span_has() {
    let pack = first_pack(&["title", "part1/"], &named(PATHS));
    let span = |dir: &str| {
        spans(&pack)
            .into_iter()
            .find(|(named, _)| named == dir)
            .unwrap()
            .1
    };
    // The first part's span hasn't arrived: the title has.
    let bundle = streamed(&pack, span("part1"));
    assert!(bundle.arrived("title/"));
    assert!(bundle.arrived("title"));
    assert!(!bundle.arrived("part1"));
    assert!(!bundle.arrived("part1/inner"));
    assert!(bundle.arrived("part2"));
    assert!(bundle.arrived("nowhere"));
    assert!(!bundle.arrived(""));
    assert_eq!(
        bundle.file("title/logo.txt").as_deref(),
        Some("title/logo.txt")
    );
    assert_eq!(bundle.file("part1/map.ron"), None);
    // Nothing past the title has arrived: a folder not named first waits for
    // its own files.
    let title_end = span("title").end;
    let bundle = streamed(&pack, title_end..u64::MAX);
    assert!(bundle.arrived("title"));
    assert!(!bundle.arrived("part1"));
    assert!(!bundle.arrived("part2"));
    // A file's path is no folder: nothing is under it, and nothing waits.
    assert!(bundle.arrived("a.txt"));
    // Every byte has.
    let bundle = streamed(&pack, 0..0);
    for dir in ["", "title", "part1", "part1/inner", "part2"] {
        assert!(bundle.arrived(dir), "{dir}");
    }
}

#[test]
fn a_bundle_with_no_layer_over_a_source_that_streams_has_every_folder() {
    let pack = first_pack(&["title"], &named(PATHS));
    let whole = Pack::open(&pack, STAMP).unwrap().into_owned();
    let bundle = pack_bundle(whole);
    let in_memory: &'static InMemory = Box::leak(Box::new(InMemory::new(pack.clone().leak())));
    let by_file = Bundle::new(vec![
        Index::read(&pack, STAMP)
            .unwrap()
            .into_layer("pack", in_memory),
    ]);
    for dir in ["", "title", "title/", "part1", "part1/inner", "nowhere"] {
        assert!(bundle.arrived(dir), "{dir}");
        assert!(sources().arrived(dir), "{dir}");
        assert!(by_file.arrived(dir), "{dir}");
    }
}

/// Folders the paths of `files` are under, and some they aren't.
fn some_folders(files: &[(String, Vec<u8>)], picks: &[prop::sample::Index]) -> Vec<String> {
    let mut dirs: Vec<String> = vec![String::new(), "zz".to_owned()];
    for (path, _) in files {
        let mut at = 0;
        while let Some(slash) = path[at..].find('/') {
            at += slash;
            dirs.push(path[..at].to_owned());
            at += 1;
        }
    }
    picks
        .iter()
        .map(|pick| {
            let dir = pick.get(&dirs).clone();
            if pick.index(2) == 0 { dir + "/" } else { dir }
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn folders_laid_first_change_no_file_read_and_any_code_keeps_them_in_their_spans(
        files in pack_files(),
        picks in proptest::collection::vec(any::<prop::sample::Index>(), 0..5),
        bits in 1u32..,
    ) {
        let first = some_folders(&files, &picks);
        let first: Vec<&str> = first.iter().map(String::as_str).collect();
        let pack = first_pack(&first, &files);
        prop_assert_eq!(files_of(&pack), files_of(&raw_pack(&test_info(), &files)));
        prop_assert!(spans_hold_their_folders(&pack));
        let raw = Raw::read(&pack).unwrap();
        prop_assert_eq!(raw.write(), pack.clone());
        if let Some(marked) = raw.write_marked(code(bits)) {
            prop_assert_eq!(files_of(&marked), files_of(&pack));
            prop_assert!(spans_hold_their_folders(&marked));
            prop_assert_eq!(spans(&marked).len(), spans(&pack).len());
            prop_assert_eq!(
                Raw::read(&marked).unwrap().mark(),
                PackMark::Code { code: code(bits), order_agrees: true }
            );
        }
    }
}
