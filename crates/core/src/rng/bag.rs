//! A shuffle bag: each item once, in a random order, before any repeats.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use super::RandomSource;

/// A shuffle bag (a "7-bag"), kept in the game's state: every `len` draws
/// hold each item once. When the bag is empty, [`draw`](Self::draw) puts
/// every item back, [`shuffle`](RandomSource::shuffle)s them (`len - 1`
/// draws), then deals from the end; the other draws take nothing from the
/// RNG.
///
/// Its fields keep an invariant (each index left is below the item count
/// and appears once), so they are private, and a saved bag is checked as
/// it loads.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "SavedBag<T>")]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct Bag<T> {
    items: Vec<T>,
    left: Vec<usize>,
}

impl<T> Bag<T> {
    /// A bag of `items`, empty until its first draw fills it.
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            left: Vec::new(),
        }
    }

    /// The next item; `None` only for a bag with no items, with no draw.
    pub fn draw(&mut self, rng: &mut impl RandomSource) -> Option<&T> {
        if self.items.is_empty() {
            return None;
        }
        if self.left.is_empty() {
            self.left.extend(0..self.items.len());
            rng.shuffle(&mut self.left);
        }
        let index = self.left.pop()?;
        self.items.get(index)
    }

    /// How many items are left before the bag refills.
    pub fn left(&self) -> usize {
        self.left.len()
    }

    /// Every item, in the order the bag was made with.
    pub fn items(&self) -> &[T] {
        &self.items
    }
}

/// A [`Bag`] as saved, before its indices are checked.
#[derive(Deserialize)]
#[serde(rename = "Bag")]
struct SavedBag<T> {
    items: Vec<T>,
    left: Vec<usize>,
}

impl<T> TryFrom<SavedBag<T>> for Bag<T> {
    type Error = String;

    fn try_from(saved: SavedBag<T>) -> Result<Self, String> {
        let mut seen = alloc::vec![false; saved.items.len()];
        for &index in &saved.left {
            match seen.get_mut(index) {
                None => {
                    return Err(format!(
                        "the bag's index {index} is not below its {} items",
                        saved.items.len()
                    ));
                }
                Some(true) => return Err(format!("the bag's index {index} is left twice")),
                Some(slot) => *slot = true,
            }
        }
        Ok(Self {
            items: saved.items,
            left: saved.left,
        })
    }
}
