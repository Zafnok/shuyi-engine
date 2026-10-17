use proptest::prelude::*;

use super::*;
use crate::storage::{MAX_KEY_LEN, MemoryStorage};

fn numbered(n: u32) -> String {
    format!("Player {n}")
}

/// A sample game's keys from before it had profiles: its settings, keys,
/// a list of what the player has seen and two save slots.
const SETTINGS_KEY: &str = "settings";
const SEEN_KEY: &str = "tips_seen";

fn keys_from_before_profiles() -> Vec<String> {
    [SETTINGS_KEY, "keybindings", SEEN_KEY, "slot_01", "slot_30"]
        .map(str::to_owned)
        .to_vec()
}

/// [`load`] with the sample game's keys from before profiles.
fn load(storage: &mut dyn Storage, numbered: impl Fn(u32) -> String) -> (Profiles, Vec<String>) {
    super::load(storage, &keys_from_before_profiles(), numbered)
}

fn two() -> Profiles {
    let mut p = Profiles::first("Player 1");
    p.add("Mei").unwrap();
    p
}

#[test]
fn the_list_round_trips() {
    let mut p = two();
    let third = p.add("Odo").unwrap();
    p.set_current(third).unwrap();
    p.remove(ProfileId(2)).unwrap();
    assert_eq!(Profiles::from_ron(&p.to_ron()), Ok(p));
}

#[test]
fn a_profile_key_is_a_valid_storage_key() {
    assert_eq!(ProfileId::FIRST.key("settings"), "p1-settings");
    assert_eq!(ProfileId(8).key("slot_30"), "p8-slot_30");
    let longest = ProfileId(u32::MAX);
    for key in keys_from_before_profiles() {
        assert!(is_valid_key(&longest.key(&key)), "{key}");
        assert_eq!(ProfileId::of_key(&longest.key(&key)), Some(longest));
    }
}

#[test]
fn a_key_fits_every_profile_only_if_its_largest_key_is_valid() {
    // `p4294967295-` is 12 bytes: 52 are left of the 64.
    assert!(fits_every_profile("settings"));
    assert!(fits_every_profile(&"k".repeat(MAX_KEY_LEN - 12)));
    assert!(!fits_every_profile(&"k".repeat(MAX_KEY_LEN - 11)));
    assert!(!fits_every_profile("Upper"));
    assert!(!fits_every_profile(""));
}

#[test]
fn only_a_profile_key_names_a_profile() {
    assert_eq!(ProfileId::of_key("p12-tips_seen"), Some(ProfileId(12)));
    for key in [
        "profiles",
        "title_backdrop",
        "p-x",
        "p1-",
        "p01-x",
        "px-y",
        "settings",
    ] {
        assert_eq!(ProfileId::of_key(key), None, "{key}");
    }
}

#[test]
fn names_are_trimmed_unique_and_short() {
    let mut p = two();
    assert_eq!(p.add("  "), Err(ProfileError::NameEmpty));
    assert_eq!(p.add("mei"), Err(ProfileError::NameTaken));
    assert_eq!(p.add("Thirteen char"), Err(ProfileError::NameTooLong));
    let id = p.add(" Twelve chars ").unwrap();
    assert_eq!(p.get(id).unwrap().name, "Twelve chars");
    // Wide letters count one each.
    assert!(p.add("淑怡淑怡淑怡淑怡淑怡淑怡").is_ok());
}

#[test]
fn a_rename_keeps_its_own_name_free() {
    let mut p = two();
    p.rename(ProfileId(2), "MEI").unwrap();
    assert_eq!(p.get(ProfileId(2)).unwrap().name, "MEI");
    assert_eq!(
        p.rename(ProfileId(2), "player 1"),
        Err(ProfileError::NameTaken)
    );
    assert_eq!(
        p.rename(ProfileId(9), "Odo"),
        Err(ProfileError::NoSuchProfile(ProfileId(9)))
    );
}

#[test]
fn there_are_at_most_eight() {
    let mut p = Profiles::first("Player 1");
    for n in 2..=MAX_PROFILES {
        p.add(&numbered(u32::try_from(n).unwrap())).unwrap();
    }
    assert_eq!(p.add("One more"), Err(ProfileError::TooMany));
}

#[test]
fn the_last_one_cant_be_deleted() {
    let mut p = two();
    p.set_current(ProfileId(2)).unwrap();
    p.remove(ProfileId(2)).unwrap();
    assert_eq!(p.current(), ProfileId::FIRST);
    assert_eq!(p.remove(ProfileId::FIRST), Err(ProfileError::LastOne));
}

#[test]
fn a_deleted_profiles_number_is_never_given_again() {
    let mut p = two();
    p.remove(ProfileId(2)).unwrap();
    assert_eq!(p.add("Odo"), Ok(ProfileId(3)));
    let saved = Profiles::from_ron(&p.to_ron()).unwrap();
    let mut again = saved.clone();
    assert_eq!(again.add("Piet"), Ok(ProfileId(4)));
}

#[test]
fn a_saved_list_is_repaired_or_refused() {
    let text = "Profiles(version: 1, profiles: [(id: 3, name: \"A\")], current: 7, next_id: 2)";
    let p = Profiles::from_ron(text).unwrap();
    assert_eq!(p.current(), ProfileId(3));
    assert_eq!(p.clone().add("B"), Ok(ProfileId(4)));
    for bad in [
        "nonsense",
        "Profiles(version: 2, profiles: [(id: 1, name: \"A\")], current: 1, next_id: 2)",
        "Profiles(version: 1, profiles: [], current: 1, next_id: 2)",
    ] {
        assert!(Profiles::from_ron(bad).is_err(), "{bad}");
    }
}

#[test]
fn a_first_launch_makes_player_1_and_saves_it() {
    let mut storage = MemoryStorage::new();
    let (p, warnings) = load(&mut storage, numbered);
    assert_eq!(p, Profiles::first("Player 1"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(storage.list().unwrap(), [PROFILES_KEY]);
    assert_eq!(load(&mut storage, numbered).0, p);
}

/// A storage from before profiles, with every key a player had.
fn old_storage() -> MemoryStorage {
    let mut storage = MemoryStorage::new();
    for key in keys_from_before_profiles() {
        storage.write(&key, &format!("old {key}")).unwrap();
    }
    storage.write("title_backdrop", "backdrop.png").unwrap();
    storage
}

#[test]
fn the_keys_from_before_profiles_move_into_player_1() {
    let mut storage = old_storage();
    let (p, warnings) = load(&mut storage, numbered);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(p.current(), ProfileId::FIRST);
    let profile = ProfileStore::new(Box::new(storage), ProfileId::FIRST);
    for key in keys_from_before_profiles() {
        assert_eq!(profile.read(&key), Ok(Some(format!("old {key}"))));
        assert_eq!(profile.computer().read(&key), Ok(None), "{key} is moved");
    }
    let computer = profile.computer();
    assert_eq!(
        computer.read("title_backdrop"),
        Ok(Some("backdrop.png".into()))
    );
}

#[test]
fn the_move_runs_once() {
    let mut storage = old_storage();
    load(&mut storage, numbered);
    // An old key turning up later is not moved over the profile's.
    storage.write(SETTINGS_KEY, "stray").unwrap();
    load(&mut storage, numbered);
    assert_eq!(storage.read(SETTINGS_KEY), Ok(Some("stray".into())));
    let first = ProfileId::FIRST.key(SETTINGS_KEY);
    assert_eq!(
        storage.read(&first),
        Ok(Some(format!("old {SETTINGS_KEY}")))
    );
}

/// A [`MemoryStorage`] whose writes fail once `writes` have succeeded,
/// and whose deletes fail while `deletes_fail`.
#[derive(Debug)]
struct Stopping {
    inner: MemoryStorage,
    writes: usize,
    deletes_fail: bool,
}

impl Storage for Stopping {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        self.inner.read(key)
    }
    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        if self.writes == 0 {
            return Err(StorageError::Backend("stopped".into()));
        }
        self.writes -= 1;
        self.inner.write(key, value)
    }
    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        if self.deletes_fail {
            return Err(StorageError::Backend("stopped".into()));
        }
        self.inner.delete(key)
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.inner.list()
    }
}

#[test]
fn a_move_stopped_part_way_loses_nothing_and_is_tried_again() {
    let old = keys_from_before_profiles();
    for writes in 0..=old.len() {
        let mut storage = Stopping {
            inner: old_storage(),
            writes,
            deletes_fail: true,
        };
        let (_, warnings) = load(&mut storage, numbered);
        assert!(!warnings.is_empty(), "stopped after {writes} writes");
        // The next launch, with nothing failing.
        let mut storage = storage.inner;
        let (_, warnings) = load(&mut storage, numbered);
        assert!(warnings.is_empty(), "{warnings:?}");
        for key in &old {
            let moved = storage.read(&ProfileId::FIRST.key(key));
            assert_eq!(
                moved,
                Ok(Some(format!("old {key}"))),
                "{key} after {writes}"
            );
            assert_eq!(storage.read(key), Ok(None));
        }
    }
}

#[test]
fn a_profile_written_to_before_the_move_finished_keeps_its_own() {
    let mut storage = old_storage();
    // A launch whose move stopped part-way, then saved the settings.
    let first = ProfileId::FIRST.key(SETTINGS_KEY);
    storage.write(&first, "changed since").unwrap();
    load(&mut storage, numbered);
    assert_eq!(storage.read(&first), Ok(Some("changed since".into())));
    assert_eq!(storage.read(SETTINGS_KEY), Ok(None));
}

#[test]
fn an_unreadable_list_is_made_again_from_the_profiles_keys() {
    let mut storage = MemoryStorage::new();
    storage.write(PROFILES_KEY, "nonsense").unwrap();
    storage.write(&ProfileId(2).key("slot_01"), "save").unwrap();
    storage.write(&ProfileId(5).key("settings"), "s").unwrap();
    let (p, warnings) = load(&mut storage, numbered);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    let names: Vec<&str> = p.all().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Player 2", "Player 5"]);
    assert_eq!(p.current(), ProfileId(2));
    assert_eq!(p.clone().add("New"), Ok(ProfileId(6)));
    assert_eq!(load(&mut storage, numbered), (p, Vec::new()));
}

#[test]
fn an_unreadable_list_with_no_profile_keys_moves_the_old_ones() {
    let mut storage = old_storage();
    storage.write(PROFILES_KEY, "nonsense").unwrap();
    let (p, _) = load(&mut storage, numbered);
    assert_eq!(p, Profiles::first("Player 1"));
    let first = ProfileId::FIRST.key(SEEN_KEY);
    assert_eq!(storage.read(&first), Ok(Some(format!("old {SEEN_KEY}"))));
}

#[test]
fn deleting_a_profiles_keys_spares_the_others() {
    let mut storage = MemoryStorage::new();
    for id in [ProfileId(1), ProfileId(2), ProfileId(12)] {
        storage.write(&id.key("settings"), "s").unwrap();
        storage.write(&id.key("slot_01"), "save").unwrap();
    }
    storage.write(PROFILES_KEY, "list").unwrap();
    delete_keys(&mut storage, ProfileId(1)).unwrap();
    let mut left = storage.list().unwrap();
    left.sort();
    assert_eq!(
        left,
        [
            "p12-settings",
            "p12-slot_01",
            "p2-settings",
            "p2-slot_01",
            "profiles"
        ]
    );
}

#[test]
fn deleting_keys_tries_them_all_and_says_it_failed() {
    let mut inner = MemoryStorage::new();
    inner.write("p1-a", "1").unwrap();
    let mut storage = Stopping {
        inner,
        writes: 0,
        deletes_fail: true,
    };
    assert!(delete_keys(&mut storage, ProfileId::FIRST).is_err());
}

#[test]
fn a_profile_store_reads_and_lists_only_its_profiles_keys() {
    let mut computer = MemoryStorage::new();
    computer.write("p1-settings", "one").unwrap();
    computer.write("p10-settings", "ten").unwrap();
    computer.write("title_backdrop", "b").unwrap();
    let mut store = ProfileStore::new(Box::new(computer), ProfileId::FIRST);
    assert_eq!(store.read("settings"), Ok(Some("one".into())));
    assert_eq!(store.list(), Ok(vec!["settings".to_owned()]));
    assert_eq!(store.read("title_backdrop"), Ok(None));
    store.set_profile(ProfileId(10));
    assert_eq!(store.profile(), ProfileId(10));
    assert_eq!(store.read("settings"), Ok(Some("ten".into())));
    store.write("slot_01", "save").unwrap();
    store.delete("settings").unwrap();
    let mut keys = store.into_computer().list().unwrap();
    keys.sort();
    assert_eq!(keys, ["p1-settings", "p10-slot_01", "title_backdrop"]);
}

#[test]
fn a_profile_store_refuses_a_key_too_long_with_its_prefix() {
    let mut store = ProfileStore::new(Box::new(MemoryStorage::new()), ProfileId::FIRST);
    let longest = "x".repeat(MAX_KEY_LEN);
    assert!(matches!(store.write(&longest, "v"), Err(StorageError::InvalidKey(k)) if k == longest));
    assert!(matches!(
        store.read("p1-UPPER"),
        Err(StorageError::InvalidKey(_))
    ));
    assert!(store.delete("a/b").is_err());
    assert!(store.write(&"x".repeat(MAX_KEY_LEN - 3), "v").is_ok());
}

#[test]
fn profile_errors_say_what_happened() {
    assert_eq!(
        ProfileError::NameTooLong.to_string(),
        "the name is over 12 characters"
    );
    assert_eq!(
        ProfileError::TooMany.to_string(),
        "there are 8 profiles already"
    );
    assert_eq!(
        ProfileError::NoSuchProfile(ProfileId(3)).to_string(),
        "no profile 3"
    );
    let storage = ProfileError::Storage(StorageError::Backend("full".into()));
    assert_eq!(storage.to_string(), "profiles: storage error: full");
    for e in [
        ProfileError::NameEmpty,
        ProfileError::NameTaken,
        ProfileError::LastOne,
    ] {
        assert!(!e.to_string().is_empty());
    }
}

proptest! {
    #[test]
    fn every_profile_key_is_valid_and_names_its_profile(
        id in 1u32..,
        key in "[a-z0-9_-]{1,52}",
    ) {
        let full = ProfileId(id).key(&key);
        prop_assert!(is_valid_key(&full));
        prop_assert_eq!(ProfileId::of_key(&full), Some(ProfileId(id)));
    }
}
