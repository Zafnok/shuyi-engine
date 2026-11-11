use super::*;
use crate::storage::MemoryStorage;
use proptest::prelude::*;
use shuyi_assets::format::Formats;
use shuyi_core::made::{ContentHash, Part};
use shuyi_core::state_hash;

/// The fixture: two entries, frozen for the golden table.
const FIXTURE: &str = include_str!("../../../tests/fixtures/bundle_v1.ron");

/// A toy save format: a slot of gold, of version 3.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Slot {
    version: u32,
    gold: u32,
}

impl SaveFormat for Slot {
    const NAME: &'static str = "Slot";
    const VERSION: u32 = 3;
}

/// The fixture's stamp.
fn fixture_made() -> Made {
    let mut made = Made::new("demo 1.0", ContentHash(0x1a2b_3c4d_5e6f_7a8b));
    made.engine = "0.1.0".to_owned();
    made
}

fn storage_of(entries: &[(&str, &str)]) -> MemoryStorage {
    let mut storage = MemoryStorage::new();
    for (key, value) in entries {
        storage.write(key, value).unwrap();
    }
    storage
}

/// Every key and value of `storage`, in key order.
fn contents(storage: &dyn Storage) -> Vec<(String, String)> {
    let mut keys = storage.list().unwrap();
    keys.sort();
    keys.into_iter()
        .map(|key| {
            let value = storage.read(&key).unwrap().unwrap();
            (key, value)
        })
        .collect()
}

fn accept_all(_: &str, _: &str) -> Verdict {
    Verdict::Ready
}

/// The check of slots under `save-slot-…`.
fn slots() -> FormatCheck<Slot> {
    FormatCheck::<Slot>::new(|key| key.starts_with("save-slot-"))
}

fn plan(text: &str, storage: &dyn Storage) -> ImportPlan {
    plan_import(text, storage, &accept_all, &fixture_made()).unwrap()
}

#[test]
fn the_fixture_round_trips_byte_for_byte() {
    let storage = storage_of(&[
        ("settings", "(volume:80)"),
        ("save-slot-1", "(version:3,gold:10)"),
    ]);
    let text = write_bundle(&storage, &fixture_made(), |_| true).unwrap();
    assert_eq!(text, FIXTURE);
    let bundle = read_bundle(FIXTURE).unwrap();
    assert_eq!(bundle.version, SAVE_BUNDLE_VERSION);
    assert_eq!(bundle.made, fixture_made());
    assert_eq!(bundle.to_text(), FIXTURE);
    assert_eq!(SaveBundleRon.encode(&bundle), FIXTURE.as_bytes());
}

#[test]
fn the_same_storage_gives_the_same_text_whatever_its_order() {
    let made = fixture_made();
    let one = storage_of(&[("b", "2"), ("a", "1"), ("c", "3")]);
    let other = storage_of(&[("c", "3"), ("a", "1"), ("b", "2")]);
    let text = write_bundle(&one, &made, |_| true).unwrap();
    assert_eq!(text, write_bundle(&one, &made, |_| true).unwrap());
    assert_eq!(text, write_bundle(&other, &made, |_| true).unwrap());
}

#[test]
fn a_bundle_holds_the_keys_kept_and_never_the_bundles_own() {
    let storage = storage_of(&[
        ("save-slot-1", "one"),
        ("save-slot-2", "two"),
        ("settings", "s"),
        ("_bundle-journal", "[]"),
        ("_bundle-backup-0", "old"),
    ]);
    let text = write_bundle(&storage, &fixture_made(), |key| key != "settings").unwrap();
    let keys: Vec<String> = read_bundle(&text).unwrap().entries.into_keys().collect();
    assert_eq!(keys, ["save-slot-1", "save-slot-2"]);
    let none = write_bundle(&storage, &fixture_made(), |_| false).unwrap();
    assert!(read_bundle(&none).unwrap().entries.is_empty());
}

#[test]
fn a_plan_gives_each_entry_its_verdict_and_marks_what_it_replaces() {
    let bundle = SaveBundle::new(
        fixture_made(),
        [
            ("save-slot-1", "(version:3,gold:10)"),
            ("save-slot-2", "Slot(version:2,gold:\"old\")"),
            ("save-slot-3", "(version:3,gold:\"ten\")"),
            ("save-slot-4", "not a save"),
            ("trophies", "[1,2]"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect(),
    );
    let storage = storage_of(&[("save-slot-1", "(version:3,gold:1)"), ("other", "x")]);
    let plan = plan_import(&bundle.to_text(), &storage, &slots(), &fixture_made()).unwrap();
    let verdicts: Vec<(&str, Verdict, bool)> = plan
        .entries
        .iter()
        .map(|e| (e.key.as_str(), e.verdict, e.replaces))
        .collect();
    assert_eq!(
        verdicts,
        [
            ("save-slot-1", Verdict::Ready, true),
            (
                "save-slot-2",
                Verdict::Incompatible { found: 2, wants: 3 },
                false
            ),
            ("save-slot-3", Verdict::Unreadable, false),
            ("save-slot-4", Verdict::Unreadable, false),
            ("trophies", Verdict::Unknown, false),
        ]
    );
    assert_eq!(plan.mismatch, None);
    assert_eq!(plan.made, fixture_made());
    let slot: Slot = ron::from_str(&plan.entries[0].text).unwrap();
    assert_eq!((slot.version, slot.gold), (3, 10));
    let entry = plan.entry("save-slot-1").unwrap();
    assert_eq!(entry.text, "(version:3,gold:10)");
    assert!(plan.entry("other").is_none());
    let ready: Vec<&str> = plan.ready().map(|e| e.key.as_str()).collect();
    assert_eq!(ready, ["save-slot-1"]);
    // Planning wrote nothing.
    assert_eq!(
        contents(&storage),
        [
            ("other".to_owned(), "x".to_owned()),
            ("save-slot-1".to_owned(), "(version:3,gold:1)".to_owned())
        ]
    );
}

#[test]
fn a_bundle_another_build_wrote_reports_the_mismatch_and_still_plans() {
    let mut other = fixture_made();
    other.engine = "0.2.0".to_owned();
    other.game = "demo 2.0".to_owned();
    let text = write_bundle(
        &storage_of(&[("save-slot-1", "(version:3,gold:10)")]),
        &other,
        |_| true,
    )
    .unwrap();
    let plan = plan_import(&text, &MemoryStorage::new(), &slots(), &fixture_made()).unwrap();
    let mismatch = plan.mismatch.unwrap();
    let parts: Vec<Part> = mismatch.parts.iter().map(|d| d.part).collect();
    assert_eq!(parts, [Part::Engine, Part::Game]);
    assert_eq!(plan.made, other);
    assert_eq!(plan.entries[0].verdict, Verdict::Ready);
}

#[test]
fn checks_give_the_first_verdict_that_knows_the_key() {
    let settings = |key: &str, _: &str| {
        if key == "settings" {
            Verdict::Ready
        } else {
            Verdict::Unknown
        }
    };
    let checks = Checks::new()
        .with(slots())
        .with(settings)
        .with(|key: &str, _: &str| {
            if key.starts_with("save-slot-") || key == "settings" {
                Verdict::Unreadable
            } else {
                Verdict::Unknown
            }
        });
    assert_eq!(
        checks.check("save-slot-1", "(version:3,gold:1)"),
        Verdict::Ready
    );
    assert_eq!(
        checks.check("save-slot-1", "(version:3)"),
        Verdict::Unreadable
    );
    assert_eq!(checks.check("settings", "anything"), Verdict::Ready);
    assert_eq!(checks.check("trophies", "[]"), Verdict::Unknown);
    assert_eq!(Checks::new().check("settings", "x"), Verdict::Unknown);
    assert_eq!(format!("{checks:?}"), "Checks { checks: 3 }");
    let shown = format!("{:?}", slots());
    assert!(
        shown.starts_with("FormatCheck { format: ") && shown.contains("Slot"),
        "{shown}"
    );
}

#[test]
fn what_isnt_a_bundle_this_build_reads_is_refused() {
    let storage = MemoryStorage::new();
    let refused = |text: &str| plan_import(text, &storage, &accept_all, &fixture_made());
    let Err(BundleError::NotABundle { line, column, .. }) =
        refused("SaveBundle(\n  version: 1,\n  oops")
    else {
        panic!("syntax");
    };
    assert_eq!((line, column), (3, 7));
    assert!(matches!(
        refused("[1, 2]"),
        Err(BundleError::NotABundle { .. })
    ));
    assert!(matches!(
        refused("Other(version: 1)"),
        Err(BundleError::NotABundle { .. })
    ));
    // The version reads, the rest doesn't.
    assert!(matches!(
        refused("SaveBundle(version: 1, entries: {})"),
        Err(BundleError::NotABundle { .. })
    ));
    let newer = FIXTURE.replace("version: 1", "version: 2");
    assert_eq!(
        refused(&newer),
        Err(BundleError::Version { found: 2, reads: 1 })
    );
    // A bundle of another version is told by its version alone, whatever
    // the rest holds.
    assert_eq!(
        refused("SaveBundle(version: 7, pages: [1])"),
        Err(BundleError::Version { found: 7, reads: 1 })
    );
    for bad in ["Has Space", "_bundle-journal", ""] {
        let text = FIXTURE.replace("\"settings\"", &format!("{bad:?}"));
        assert_eq!(refused(&text), Err(BundleError::BadKey(bad.to_owned())));
    }
}

/// A storage that lists nothing: fails, or holds nothing.
#[derive(Debug)]
struct Unlistable;

impl Storage for Unlistable {
    fn read(&self, _key: &str) -> Result<Option<String>, StorageError> {
        Ok(None)
    }
    fn write(&mut self, _key: &str, _value: &str) -> Result<(), StorageError> {
        Ok(())
    }
    fn delete(&mut self, _key: &str) -> Result<(), StorageError> {
        Ok(())
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        Err(StorageError::Backend("no list".to_owned()))
    }
}

#[test]
fn a_storage_that_fails_fails_the_plan_and_the_bundle() {
    let failed = plan_import(FIXTURE, &Unlistable, &accept_all, &fixture_made());
    assert_eq!(
        failed,
        Err(BundleError::Storage(StorageError::Backend(
            "no list".to_owned()
        )))
    );
    assert!(write_bundle(&Unlistable, &fixture_made(), |_| true).is_err());
}

#[test]
fn errors_say_what_is_wrong() {
    let shown = |e: BundleError| e.to_string();
    assert_eq!(
        shown(BundleError::NotABundle {
            line: 3,
            column: 7,
            message: "Expected comma".to_owned()
        }),
        "not a save bundle: 3:7: Expected comma"
    );
    assert_eq!(
        shown(BundleError::Version { found: 2, reads: 1 }),
        "a save bundle of version 2; this build reads version 1"
    );
    assert_eq!(
        shown(BundleError::BadKey("A B".to_owned())),
        "a save bundle's entry has the bad key \"A B\""
    );
    assert_eq!(
        shown(StorageError::Backend("disk full".to_owned()).into()),
        "storage error: disk full"
    );
}

#[test]
fn apply_writes_only_the_ready_entries_taken() {
    let text = write_bundle(
        &storage_of(&[
            ("save-slot-1", "(version:3,gold:10)"),
            ("save-slot-2", "(version:3,gold:20)"),
            ("save-slot-3", "(version:1,gold:30)"),
            ("trophies", "[1]"),
        ]),
        &fixture_made(),
        |_| true,
    )
    .unwrap();
    let mut storage = storage_of(&[("save-slot-1", "(version:3,gold:1)"), ("other", "x")]);
    let plan = plan_import(&text, &storage, &slots(), &fixture_made()).unwrap();
    let applied = apply(&plan, &mut storage, |e| e.key != "save-slot-2").unwrap();
    assert_eq!(applied.written, ["save-slot-1"]);
    assert_eq!(applied.replaced, ["save-slot-1"]);
    assert_eq!(
        contents(&storage),
        [
            ("other".to_owned(), "x".to_owned()),
            ("save-slot-1".to_owned(), "(version:3,gold:10)".to_owned())
        ]
    );
    let applied = apply(&plan, &mut storage, |_| true).unwrap();
    assert_eq!(applied.written, ["save-slot-1", "save-slot-2"]);
    assert_eq!(applied.replaced, ["save-slot-1"]);
    assert_eq!(
        apply(&plan, &mut storage, |_| false).unwrap(),
        Applied::default()
    );
    assert_eq!(
        storage.read("save-slot-2").unwrap().as_deref(),
        Some("(version:3,gold:20)")
    );
    assert_eq!(storage.read("save-slot-3").unwrap(), None);
    assert_eq!(storage.read("trophies").unwrap(), None);
}

/// A memory storage whose `fail_at`th write (from 1) fails, once.
#[derive(Debug)]
struct FailingWrite {
    inner: MemoryStorage,
    writes: u32,
    fail_at: u32,
}

impl Storage for FailingWrite {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        self.inner.read(key)
    }
    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.writes += 1;
        if self.writes == self.fail_at {
            return Err(StorageError::Backend("the third write".to_owned()));
        }
        self.inner.write(key, value)
    }
    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        self.inner.delete(key)
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.inner.list()
    }
}

#[test]
fn a_storage_that_fails_a_write_keeps_every_key_as_it_was() {
    let text = write_bundle(
        &storage_of(&[("a", "new a"), ("b", "new b"), ("c", "new c")]),
        &fixture_made(),
        |_| true,
    )
    .unwrap();
    let before = storage_of(&[("a", "old a"), ("b", "old b"), ("other", "x")]);
    let plan = plan(&text, &before);
    // Two backups, the journal, three values: six writes. Each fails in
    // turn, the third (the journal) and the fourth (the first value) among
    // them.
    for fail_at in 1..=6 {
        let mut storage = FailingWrite {
            inner: before.clone(),
            writes: 0,
            fail_at,
        };
        let failed = apply(&plan, &mut storage, |_| true);
        assert!(failed.is_err(), "write {fail_at}");
        assert_eq!(contents(&storage), contents(&before), "write {fail_at}");
    }
    // Past the six, it works.
    let mut storage = FailingWrite {
        inner: before.clone(),
        writes: 0,
        fail_at: 7,
    };
    apply(&plan, &mut storage, |_| true).unwrap();
    assert_eq!(
        contents(&storage),
        [
            ("a".to_owned(), "new a".to_owned()),
            ("b".to_owned(), "new b".to_owned()),
            ("c".to_owned(), "new c".to_owned()),
            ("other".to_owned(), "x".to_owned())
        ]
    );
}

#[test]
fn recover_puts_back_what_an_interrupted_import_changed() {
    let mut storage = storage_of(&[
        ("a", "new a"),
        ("b", "new b"),
        ("other", "x"),
        ("_bundle-backup-0", "old a"),
        ("_bundle-backup-7", "stray"),
        (
            "_bundle-journal",
            "[(\"a\", Some(\"_bundle-backup-0\")), (\"b\", None), (\"c\", Some(\"_bundle-backup-1\"))]",
        ),
        ("c", "c"),
    ]);
    assert_eq!(recover(&mut storage), Ok(true));
    assert_eq!(
        contents(&storage),
        [
            ("a".to_owned(), "old a".to_owned()),
            // Its backup was lost: it stays as it is.
            ("c".to_owned(), "c".to_owned()),
            ("other".to_owned(), "x".to_owned())
        ]
    );
    assert_eq!(recover(&mut storage), Ok(false));
    // A journal that doesn't read is dropped.
    storage.write("_bundle-journal", "torn").unwrap();
    assert_eq!(recover(&mut storage), Ok(true));
    assert_eq!(storage.read("_bundle-journal").unwrap(), None);
    assert_eq!(storage.read("a").unwrap().as_deref(), Some("old a"));
    assert!(recover(&mut Unlistable).is_err());
}

#[test]
fn the_format_reads_a_bundle_through_the_registry_and_says_where_one_is_broken() {
    let mut formats = Formats::new();
    formats.register(SaveBundleRon).unwrap();
    let cx = DecodeCx::new();
    let read = |text: &str| {
        formats.decode::<SaveBundle>(&AssetPath::new("saves/backup.ron"), text.as_bytes(), &cx)
    };
    assert_eq!(read(FIXTURE), Ok(read_bundle(FIXTURE).unwrap()));
    let error = |text: &str| read(text).unwrap_err().remove(0);

    let broken = error(&FIXTURE.replace("version: 1,", "version: 1"));
    assert_eq!(broken.file, "saves/backup.ron");
    assert_eq!(broken.line, Some(3));
    assert_eq!(broken.code, Some(codes::PARSE));

    let newer = error(&FIXTURE.replace("version: 1", "version: 2"));
    assert_eq!(newer.line, Some(2));
    assert_eq!(newer.code, Some(codes::OUT_OF_RANGE));
    assert_eq!(
        newer.message,
        "a save bundle of version 2; this build reads version 1"
    );

    let bad_key = error(&FIXTURE.replace("\"settings\"", "\"Settings\""));
    assert_eq!(bad_key.line, Some(10));
    assert_eq!(bad_key.code, Some(codes::UNKNOWN_NAME));

    let not_text = read_bad_bytes(&formats);
    assert_eq!(not_text.code, Some(codes::NOT_TEXT));
    assert_eq!(SaveBundleRon.id(), "save-bundle");
    assert_eq!(SaveBundleRon.extensions(), ["ron"]);
    assert_eq!(SaveBundleRon.max_bytes(), TEXT_MAX_BYTES);
}

fn read_bad_bytes(formats: &Formats) -> ContentError {
    formats
        .decode::<SaveBundle>(&AssetPath::new("x.ron"), &[0xff, 0xfe], &DecodeCx::new())
        .unwrap_err()
        .remove(0)
}

#[test]
fn a_storage_error_from_a_plan_is_shown_as_a_content_error_without_a_line() {
    let error = content_error(
        "x.ron",
        FIXTURE,
        &BundleError::Storage(StorageError::Backend("gone".to_owned())),
    );
    assert_eq!(error.line, None);
    assert_eq!(error.message, "storage error: gone");
}

#[test]
fn golden_save_bundle_fixture() {
    let mut golden = shuyi_assets::golden!("tests/golden/bundle.ron");
    let bundle = SaveBundleRon
        .decode(
            &AssetPath::new("bundle_v1.ron"),
            FIXTURE.as_bytes(),
            &DecodeCx::new(),
        )
        .unwrap();
    golden.check("save bundle: save-bundle", state_hash(&bundle).unwrap());
    golden.assert();
}

/// Keys a game may use: never the bundle's own.
fn game_key() -> impl Strategy<Value = String> {
    "[a-z0-9][a-z0-9_-]{0,15}"
}

proptest! {
    #[test]
    fn a_bundle_plans_every_entry_ready_and_applies_to_the_keys_chosen(
        entries in prop::collection::btree_map(game_key(), "\\PC{0,40}", 0..8),
        held in prop::collection::btree_map(game_key(), "\\PC{0,10}", 0..4),
        cut in "[a-z0-9]",
    ) {
        let mut source = MemoryStorage::new();
        for (key, value) in &entries {
            source.write(key, value).unwrap();
        }
        let keep = |key: &str| key < cut.as_str();
        let text = write_bundle(&source, &fixture_made(), keep).unwrap();
        let mut target = MemoryStorage::new();
        for (key, value) in &held {
            target.write(key, value).unwrap();
        }
        let plan = plan_import(&text, &target, &accept_all, &fixture_made()).unwrap();
        prop_assert!(plan.entries.iter().all(|e| e.verdict == Verdict::Ready));
        prop_assert!(plan.entries.iter().all(|e| e.replaces == held.contains_key(&e.key)));
        apply(&plan, &mut target, |_| true).unwrap();
        let mut expected = held.clone();
        expected.extend(
            entries.iter().filter(|(key, _)| keep(key)).map(|(k, v)| (k.clone(), v.clone())),
        );
        let expected: Vec<(String, String)> = expected.into_iter().collect();
        prop_assert_eq!(contents(&target), expected);
    }

    #[test]
    fn a_plan_of_any_text_never_panics(text in "\\PC*") {
        let _ = plan_import(&text, &MemoryStorage::new(), &accept_all, &fixture_made());
    }

    #[test]
    fn a_plan_of_text_shaped_like_a_bundle_never_panics(
        version in "[0-9]{1,3}|-1|99999999999",
        made in "\\(engine: \"[0-9.]{0,6}\", game: \".{0,6}\", content: \"[0-9a-f]{14,18}\"\\)|\\(\\)",
        entries in "\\{(\"[a-zA-Z_ -]{0,8}\": \".{0,8}\",? ?){0,3}\\}|\\[\\]",
    ) {
        let text = format!("SaveBundle(version: {version}, made: {made}, entries: {entries})");
        let _ = plan_import(&text, &MemoryStorage::new(), &slots(), &fixture_made());
        let _ = SaveBundleRon.decode(&AssetPath::new("x.ron"), text.as_bytes(), &DecodeCx::new());
    }

    #[test]
    fn the_format_never_panics_on_any_bytes(bytes in prop::collection::vec(any::<u8>(), 0..200)) {
        let _ = SaveBundleRon.decode(&AssetPath::new("x.ron"), &bytes, &DecodeCx::new());
    }
}
