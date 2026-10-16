//! Who a game's characters are, by id: who speaks in a dialogue scene,
//! who a piece on the board is.

use alloc::string::String;

use serde::{Deserialize, Serialize};

/// String id of a named character, e.g. `"guard_captain"`. Serialised as
/// the bare string, so files name a character as `"guard_captain"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CharacterId(pub String);
