//! One-time tips: which have been shown ([`TipsSeen`]), kept in
//! [`Storage`] under [`TIPS_SEEN_KEY`] (the player profile's), and
//! forgetting them all ([`reset_tips`]). When a tip shows, and where, is
//! the game's; a tip's `{Action}` placeholders are filled like any text
//! ([`Ctx::text_with`](crate::Ctx::text_with)).

use crate::prelude::*;
use alloc::collections::BTreeSet;

use crate::storage::{Storage, StorageError};

/// [`Storage`] key under which the ids of the tips already shown are saved,
/// one per line.
pub const TIPS_SEEN_KEY: &str = "tips_seen";

/// The tips already shown to this profile.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TipsSeen {
    ids: BTreeSet<String>,
}

impl TipsSeen {
    /// The ids saved in `storage`; none if nothing is saved or it can't be
    /// read (the tips then show again).
    pub fn load(storage: &dyn Storage) -> Self {
        let saved = storage.read(TIPS_SEEN_KEY).ok().flatten();
        let ids = saved
            .iter()
            .flat_map(|s| s.lines())
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect();
        Self { ids }
    }

    /// Whether the tip `id` was shown.
    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    /// Records that the tip `id` was shown and saves the list. The tip
    /// counts as seen even if saving fails (it then shows again next
    /// launch).
    pub fn mark(&mut self, id: &str, storage: &mut dyn Storage) {
        if self.ids.insert(id.to_owned()) {
            let text: Vec<&str> = self.ids.iter().map(String::as_str).collect();
            storage.write(TIPS_SEEN_KEY, &text.join("\n")).ok();
        }
    }
}

/// Forgets every tip seen, so they all show again (an options screen's
/// "reset tips").
pub fn reset_tips(storage: &mut dyn Storage) -> Result<(), StorageError> {
    storage.delete(TIPS_SEEN_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;

    #[test]
    fn seen_tips_are_saved_and_loaded() {
        let mut storage = MemoryStorage::new();
        let mut seen = TipsSeen::load(&storage);
        assert!(!seen.contains("a"));
        seen.mark("a", &mut storage);
        seen.mark("b", &mut storage);
        seen.mark("a", &mut storage);
        assert_eq!(
            storage.read(TIPS_SEEN_KEY).unwrap().as_deref(),
            Some("a\nb")
        );
        let again = TipsSeen::load(&storage);
        assert!(again.contains("a") && again.contains("b") && !again.contains("c"));
        assert_eq!(again, seen);
        // Blank lines and spaces around an id don't count.
        storage.write(TIPS_SEEN_KEY, " c \n\n  \nd").unwrap();
        let read = TipsSeen::load(&storage);
        assert!(read.contains("c") && read.contains("d"));
        assert_eq!(read.ids.len(), 2);
    }

    #[test]
    fn resetting_forgets_every_tip() {
        let mut storage = MemoryStorage::new();
        TipsSeen::default().mark("a", &mut storage);
        let mut seen = TipsSeen::load(&storage);
        seen.mark("b", &mut storage);
        reset_tips(&mut storage).unwrap();
        assert_eq!(TipsSeen::load(&storage), TipsSeen::default());
        // Nothing to reset is fine.
        reset_tips(&mut storage).unwrap();
    }
}
