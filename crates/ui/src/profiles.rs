//! Player profiles: a profile holds a player's options, keys and saves,
//! whatever the game keeps for each player. Each of those keeps the
//! [`Storage`] key it would have without profiles, inside the profile: a
//! profile's key is its id's prefix and that key (`p1-settings`,
//! `p2-slot_03`, [`ProfileId::key`]). [`ProfileStore`] is the [`Storage`]
//! of the current profile, and the computer's own storage under it.
//!
//! The list of profiles and which one was used last stay with the computer,
//! under [`PROFILES_KEY`] ([`Profiles::to_ron`]). [`load`] reads it, makes
//! the first profile on a first launch, and moves a player's keys from
//! before a game had profiles into it ([`migrate`]).

use crate::prelude::*;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::storage::{Storage, StorageError, is_valid_key};

/// [`Storage`] key of the list of profiles, kept per computer.
pub const PROFILES_KEY: &str = "profiles";

/// The saved list's format version.
pub const PROFILES_VERSION: u32 = 1;

/// The most profiles there can be.
pub const MAX_PROFILES: usize = 8;

/// The longest profile name, in characters.
pub const MAX_NAME_CHARS: usize = 12;

/// A profile's number. Never reused: a deleted profile's number is never
/// given to a new one, so nothing it left behind can turn up in another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileId(u32);

impl ProfileId {
    /// The first profile's, made on a first launch.
    pub const FIRST: Self = Self(1);

    /// The number.
    pub fn get(self) -> u32 {
        self.0
    }

    /// What starts every [`Storage`] key of this profile: `p1-`.
    pub fn prefix(self) -> String {
        format!("p{}-", self.0)
    }

    /// The [`Storage`] key of this profile's `key`: `p1-settings`.
    pub fn key(self, key: &str) -> String {
        format!("{}{key}", self.prefix())
    }

    /// The profile a computer [`Storage`] key belongs to, if it is one
    /// ([`key`](Self::key)'s form).
    fn of_key(key: &str) -> Option<Self> {
        let (number, rest) = key.strip_prefix('p')?.split_once('-')?;
        let canonical = !number.starts_with('0') && number.bytes().all(|b| b.is_ascii_digit());
        let id = number
            .parse()
            .ok()
            .filter(|_| canonical && !rest.is_empty())?;
        Some(Self(id))
    }
}

/// One profile in the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Its number, which names its keys.
    pub id: ProfileId,
    /// What the player called it.
    pub name: String,
}

/// Why a change to the list of profiles was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The name is empty (or only spaces).
    NameEmpty,
    /// The name is over [`MAX_NAME_CHARS`].
    NameTooLong,
    /// Another profile has this name (ignoring case).
    NameTaken,
    /// There are [`MAX_PROFILES`] already.
    TooMany,
    /// The only profile left can't be deleted.
    LastOne,
    /// No profile has this id.
    NoSuchProfile(ProfileId),
    /// The list couldn't be saved, so nothing changed.
    Storage(StorageError),
}

// check-text: not player text (for the log; a profile screen words its own)
impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameEmpty => f.write_str("the name is empty"),
            Self::NameTooLong => write!(f, "the name is over {MAX_NAME_CHARS} characters"),
            Self::NameTaken => f.write_str("another profile has that name"),
            Self::TooMany => write!(f, "there are {MAX_PROFILES} profiles already"),
            Self::LastOne => f.write_str("the only profile can't be deleted"),
            Self::NoSuchProfile(id) => write!(f, "no profile {}", id.get()),
            Self::Storage(e) => write!(f, "profiles: {e}"),
        }
    }
}

impl core::error::Error for ProfileError {}

/// The list of profiles, in the order they were made, and the one in use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profiles {
    list: Vec<Profile>,
    current: ProfileId,
    /// The number the next profile made gets.
    next_id: u32,
}

/// The saved form of [`Profiles`].
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "Profiles")]
struct ProfilesFile {
    version: u32,
    profiles: Vec<Profile>,
    current: ProfileId,
    next_id: u32,
}

impl Profiles {
    /// A list of one profile, [`ProfileId::FIRST`], called `name`, in use.
    pub fn first(name: &str) -> Self {
        Self {
            list: vec![Profile {
                id: ProfileId::FIRST,
                name: name.to_owned(),
            }],
            current: ProfileId::FIRST,
            next_id: ProfileId::FIRST.0 + 1,
        }
    }

    /// Every profile, in the order they were made.
    pub fn all(&self) -> &[Profile] {
        &self.list
    }

    /// The profile in use.
    pub fn current(&self) -> ProfileId {
        self.current
    }

    /// The profile `id`, if there is one.
    pub fn get(&self, id: ProfileId) -> Option<&Profile> {
        self.list.iter().find(|p| p.id == id)
    }

    /// Makes `id` the profile in use.
    pub fn set_current(&mut self, id: ProfileId) -> Result<(), ProfileError> {
        self.get(id).ok_or(ProfileError::NoSuchProfile(id))?;
        self.current = id;
        Ok(())
    }

    /// Adds a profile called `name` (spaces around it dropped) and returns
    /// its id. The profile in use stays the same.
    pub fn add(&mut self, name: &str) -> Result<ProfileId, ProfileError> {
        if self.list.len() >= MAX_PROFILES {
            return Err(ProfileError::TooMany);
        }
        let name = self.check_name(name, None)?;
        let id = ProfileId(self.next_id);
        self.next_id += 1;
        self.list.push(Profile { id, name });
        Ok(id)
    }

    /// Renames the profile `id` to `name` (spaces around it dropped).
    pub fn rename(&mut self, id: ProfileId, name: &str) -> Result<(), ProfileError> {
        self.get(id).ok_or(ProfileError::NoSuchProfile(id))?;
        let name = self.check_name(name, Some(id))?;
        if let Some(profile) = self.list.iter_mut().find(|p| p.id == id) {
            profile.name = name; // check-text: not a data name
        }
        Ok(())
    }

    /// Takes the profile `id` off the list (not its keys:
    /// [`delete_keys`]). If it was in use, the first profile left is.
    pub fn remove(&mut self, id: ProfileId) -> Result<(), ProfileError> {
        self.get(id).ok_or(ProfileError::NoSuchProfile(id))?;
        if self.list.len() == 1 {
            return Err(ProfileError::LastOne);
        }
        self.list.retain(|p| p.id != id);
        if self.current == id {
            self.current = self.list[0].id;
        }
        Ok(())
    }

    /// `name` without the spaces around it, if a profile (other than
    /// `except`) may be called that.
    fn check_name(&self, name: &str, except: Option<ProfileId>) -> Result<String, ProfileError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ProfileError::NameEmpty);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(ProfileError::NameTooLong);
        }
        let lower = name.to_lowercase();
        let taken = self
            .list
            .iter()
            // check-text: not a data name
            .any(|p| Some(p.id) != except && p.name.to_lowercase() == lower);
        if taken {
            return Err(ProfileError::NameTaken);
        }
        Ok(name.to_owned())
    }

    /// The saved form, as RON text.
    pub fn to_ron(&self) -> String {
        let file = ProfilesFile {
            version: PROFILES_VERSION,
            profiles: self.list.clone(),
            current: self.current,
            next_id: self.next_id,
        };
        let config = ron::ser::PrettyConfig::new().struct_names(true);
        // Numbers and strings always serialise.
        ron::ser::to_string_pretty(&file, config).unwrap_or_default()
    }

    /// Reads the saved form. Text that can't be read, of another version
    /// or with no profile in it gives `Err` with why. A `current` that
    /// isn't on the list is the first profile; a `next_id` that some
    /// profile already has is moved past them all.
    // check-text: not player text (warnings for the log)
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let file: ProfilesFile = ron::from_str(text).map_err(|e| format!("unreadable: {e}"))?;
        if file.version != PROFILES_VERSION {
            let version = file.version;
            return Err(format!("version {version} isn't {PROFILES_VERSION}"));
        }
        let first = file.profiles.first().ok_or("no profile in it")?.id;
        let current = if file.profiles.iter().any(|p| p.id == file.current) {
            file.current
        } else {
            first
        };
        let past_all = file.profiles.iter().map(|p| p.id.0 + 1).max().unwrap_or(1);
        Ok(Self {
            current,
            next_id: file.next_id.max(past_all),
            list: file.profiles,
        })
    }

    /// Saves the list under [`PROFILES_KEY`].
    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), StorageError> {
        storage.write(PROFILES_KEY, &self.to_ron())
    }
}

/// Whether `key` can be a key of any profile: whether the largest
/// profile's key for it ([`ProfileId::key`]) is a valid [`Storage`] key.
/// A key a game keeps per player must be.
pub fn fits_every_profile(key: &str) -> bool {
    is_valid_key(key) && is_valid_key(&ProfileId(u32::MAX).key(key))
}

/// The list of profiles in `storage`, and warnings for the platform to
/// log.
///
/// - Saved: that list.
/// - None saved (a first launch, or a player from before the game had
///   profiles): one profile called `numbered(1)`, with the player's keys
///   from before profiles (`old_keys`: those the game kept them under)
///   moved into it ([`migrate`]).
/// - Saved but unreadable: a profile for each that has keys in `storage`,
///   called `numbered(id)`, so no save is lost; with none, as if none was
///   saved.
/// - `storage` can't be read: one profile, for this launch only; nothing
///   is written.
pub fn load(
    storage: &mut dyn Storage,
    old_keys: &[String],
    numbered: impl Fn(u32) -> String,
) -> (Profiles, Vec<String>) {
    let mut warnings = Vec::new();
    let profiles = match storage.read(PROFILES_KEY) {
        Ok(Some(text)) => match Profiles::from_ron(&text) {
            Ok(profiles) => profiles,
            Err(why) => {
                // check-text: not player text (a warning for the log)
                warnings.push(format!("{PROFILES_KEY}: {why}, made again from the saves"));
                rebuild(storage, old_keys, &numbered, &mut warnings)
            }
        },
        Ok(None) => migrate(
            storage,
            old_keys,
            &numbered(ProfileId::FIRST.0),
            &mut warnings,
        ),
        Err(e) => {
            // check-text: not player text (a warning for the log)
            warnings.push(format!("{PROFILES_KEY}: can't be read: {e}"));
            Profiles::first(&numbered(ProfileId::FIRST.0))
        }
    };
    (profiles, warnings)
}

/// A list with a profile for each one that has keys in `storage`, saved;
/// [`migrate`] if none has.
fn rebuild(
    storage: &mut dyn Storage,
    old_keys: &[String],
    numbered: &impl Fn(u32) -> String,
    warnings: &mut Vec<String>,
) -> Profiles {
    let mut ids: Vec<ProfileId> = storage
        .list()
        .unwrap_or_default()
        .iter()
        .filter_map(|key| ProfileId::of_key(key))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let Some(&first) = ids.first() else {
        return migrate(storage, old_keys, &numbered(ProfileId::FIRST.0), warnings);
    };
    let profiles = Profiles {
        list: ids
            .iter()
            .map(|&id| Profile {
                id,
                name: numbered(id.0),
            })
            .collect(),
        current: first,
        next_id: ids.iter().map(|id| id.0 + 1).max().unwrap_or(1),
    };
    if let Err(e) = profiles.save(storage) {
        // check-text: not player text (a warning for the log)
        warnings.push(format!("{PROFILES_KEY}: can't be saved: {e}"));
    }
    profiles
}

/// Makes the first profile, called `name`, and moves the player's keys from
/// before profiles (`old_keys`) into it, once.
///
/// Nothing is lost if it stops part-way: each key is copied (unless the
/// profile has it already), then the list is saved, and only then are the
/// old keys deleted. Until the list is saved, every launch tries again;
/// once it is, the move is done, and an old key that couldn't be deleted is
/// a copy nobody reads.
pub fn migrate(
    storage: &mut dyn Storage,
    old_keys: &[String],
    name: &str,
    warnings: &mut Vec<String>,
) -> Profiles {
    let profiles = Profiles::first(name);
    let id = ProfileId::FIRST;
    let mut moved = Vec::new();
    let mut failed = false;
    for key in old_keys {
        let copied = match storage.read(key) {
            Ok(None) => continue,
            Ok(Some(value)) => match storage.read(&id.key(key)) {
                Ok(Some(_)) => Ok(()),
                Ok(None) => storage.write(&id.key(key), &value),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        };
        match copied {
            Ok(()) => moved.push(key),
            Err(e) => {
                // check-text: not player text (a warning for the log)
                warnings.push(format!("{key}: can't be moved into the first profile: {e}"));
                failed = true;
            }
        }
    }
    if failed {
        // check-text: not player text (a warning for the log)
        warnings.push(format!(
            "{PROFILES_KEY}: not saved, so the move is tried again"
        ));
        return profiles;
    }
    if let Err(e) = profiles.save(storage) {
        // check-text: not player text (a warning for the log)
        warnings.push(format!("{PROFILES_KEY}: can't be saved: {e}"));
        return profiles;
    }
    for key in moved {
        if let Err(e) = storage.delete(key) {
            // check-text: not player text (a warning for the log)
            warnings.push(format!("{key}: moved, but can't be deleted: {e}"));
        }
    }
    profiles
}

/// Deletes every key of the profile `id` from the computer's `storage`.
/// Tries them all; the first error, if any.
pub fn delete_keys(storage: &mut dyn Storage, id: ProfileId) -> Result<(), StorageError> {
    let keys = storage.list()?;
    let mut result = Ok(());
    for key in keys.iter().filter(|k| ProfileId::of_key(k) == Some(id)) {
        let deleted = storage.delete(key);
        if result.is_ok() {
            result = deleted;
        }
    }
    result
}

/// The [`Storage`] of the profile in use: each key is read and written as
/// that profile's ([`ProfileId::key`]), and [`list`](Storage::list) lists
/// only its keys, without their prefix. What stays with the computer (the
/// list of profiles, whatever the game keeps per computer) goes through
/// [`computer`](Self::computer).
#[derive(Debug)]
pub struct ProfileStore {
    computer: Box<dyn Storage>,
    profile: ProfileId,
}

/// An empty [`MemoryStorage`](crate::storage::MemoryStorage), as the
/// first profile's.
impl Default for ProfileStore {
    fn default() -> Self {
        Self::new(
            Box::new(crate::storage::MemoryStorage::new()),
            ProfileId::FIRST,
        )
    }
}

impl ProfileStore {
    /// The storage of `profile` in `computer`.
    pub fn new(computer: Box<dyn Storage>, profile: ProfileId) -> Self {
        Self { computer, profile }
    }

    /// The profile whose keys this reads and writes.
    pub fn profile(&self) -> ProfileId {
        self.profile
    }

    /// Reads and writes `profile`'s keys from now on.
    pub(crate) fn set_profile(&mut self, profile: ProfileId) {
        self.profile = profile;
    }

    /// The computer's own storage, for what no profile holds.
    pub fn computer(&self) -> &dyn Storage {
        &*self.computer
    }

    /// [`computer`](Self::computer), to write.
    pub fn computer_mut(&mut self) -> &mut dyn Storage {
        &mut *self.computer
    }

    /// The computer's storage, for the next launch.
    pub fn into_computer(self) -> Box<dyn Storage> {
        self.computer
    }

    /// The computer's key for `key`, if both are valid.
    fn full_key(&self, key: &str) -> Result<String, StorageError> {
        let full = self.profile.key(key);
        if is_valid_key(key) && is_valid_key(&full) {
            Ok(full)
        } else {
            Err(StorageError::InvalidKey(key.to_owned()))
        }
    }
}

impl Storage for ProfileStore {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        self.computer.read(&self.full_key(key)?)
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        let full = self.full_key(key)?;
        self.computer.write(&full, value)
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        let full = self.full_key(key)?;
        self.computer.delete(&full)
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        let prefix = self.profile.prefix();
        let keys = self.computer.list()?;
        Ok(keys
            .iter()
            .filter_map(|key| key.strip_prefix(&prefix).map(str::to_owned))
            .collect())
    }
}

#[cfg(test)]
mod tests;
