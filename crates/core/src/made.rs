//! What made a record or a save: the engine's version, the game's word
//! for its build, and a fingerprint of the content it ran on ([`Made`]).
//! A record played back by another build can play differently without a
//! bug in either; its stamp tells that apart from a desync.
//!
//! A game stamps its own save or record with a `made: Made` field, filled
//! by [`Made::new`] when it is written (the fingerprint is the shipped
//! bundle's, `shuyi_assets::bundle::Bundle::fingerprint`, computed once
//! and kept). After reading one back it calls [`Made::check`] with this
//! build's stamp: what to do with a [`Mismatch`] is the game's (play on,
//! warn, refuse). A field `#[serde(default)]` of `Option<Made>` keeps
//! older files without one readable.
//!
//! The engine's version ([`ENGINE_VERSION`]) moves only at a release. A
//! game pinned to a git revision of the engine that wants that revision
//! in its stamp puts it in the game's word (`"my-game 1.2.0 engine
//! 3f2a1c9"`).

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};

/// The engine's version: every `shuyi-*` crate has the workspace's.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A fingerprint of a game's content: the same bytes give the same one on
/// every platform. Shown, and written to a file, as sixteen lowercase hex
/// digits, as a `StateHash` is shown (a string, since
/// JSON can't hold every `u64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ContentHash(
    /// The hash.
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub u64,
);

impl fmt::Display for ContentHash {
    /// Sixteen lowercase hex digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Text that isn't a [`ContentHash`]: sixteen hex digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAHash(pub String);

impl fmt::Display for NotAHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\"{}\" is not a content hash (sixteen hex digits)",
            self.0
        )
    }
}

impl FromStr for ContentHash {
    type Err = NotAHash;

    /// Sixteen hex digits, as [`Display`](fmt::Display) writes them.
    fn from_str(text: &str) -> Result<Self, NotAHash> {
        let digits = text.len() == 16 && text.bytes().all(|b| b.is_ascii_hexdigit());
        digits
            .then(|| u64::from_str_radix(text, 16).ok())
            .flatten()
            .map(Self)
            .ok_or_else(|| NotAHash(text.to_string()))
    }
}

impl Serialize for ContentHash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Reads the hash from its text.
        struct Text;

        impl Visitor<'_> for Text {
            type Value = ContentHash;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("sixteen hex digits")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<ContentHash, E> {
                text.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Text)
    }
}

/// What made a record or a save: the engine, the game's build and the
/// content it ran on, as the module docs say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
// A stamp: built with `Made::new`, it may grow (ADR-0039).
#[non_exhaustive]
pub struct Made {
    /// The engine's version ([`ENGINE_VERSION`]).
    pub engine: String,
    /// The game's word for its build (its name and version, say), as a
    /// pack's stamp is.
    pub game: String,
    /// The fingerprint of the content it ran on.
    pub content: ContentHash,
}

impl Made {
    /// The stamp of this engine, the game's build `game` and its content
    /// `content`.
    pub fn new(game: impl Into<String>, content: ContentHash) -> Self {
        Self {
            engine: ENGINE_VERSION.to_string(),
            game: game.into(),
            content,
        }
    }

    /// Whether this stamp is `now`'s, this build's; otherwise each part
    /// that differs.
    pub fn check(&self, now: &Made) -> Result<(), Mismatch> {
        let parts: Vec<Differs> = [
            (Part::Engine, self.engine.clone(), now.engine.clone()),
            (Part::Game, self.game.clone(), now.game.clone()),
            (
                Part::Content,
                self.content.to_string(),
                now.content.to_string(),
            ),
        ]
        .into_iter()
        .filter(|(_, made, now)| made != now)
        .map(|(part, made, now)| Differs { part, made, now })
        .collect();
        if parts.is_empty() {
            Ok(())
        } else {
            Err(Mismatch { parts })
        }
    }
}

impl fmt::Display for Made {
    /// `engine 0.1.0, game "demo 1.0", content 1a2b3c4d5e6f7a8b`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "engine {}, game \"{}\", content {}",
            self.engine, self.game, self.content
        )
    }
}

/// A part of a [`Made`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The engine's version.
    Engine,
    /// The game's word for its build.
    Game,
    /// The content's fingerprint.
    Content,
}

impl Part {
    /// Its name: `engine`, `game` or `content`.
    pub fn name(self) -> &'static str {
        match self {
            Part::Engine => "engine",
            Part::Game => "game",
            Part::Content => "content",
        }
    }
}

/// One part that differs between two stamps.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Differs {
    /// Which part.
    pub part: Part,
    /// Its value in the stamp checked.
    pub made: String,
    /// Its value in this build's.
    pub now: String,
}

/// How a stamp differs from this build's ([`Made::check`]): each part
/// that differs, in the order engine, game, content. Shown as `made with
/// engine 0.1.0, content 1a2b3c4d5e6f7a8b; this build: engine 0.2.0,
/// content 3c4d5e6f7a8b9c0d`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Mismatch {
    /// The parts that differ: at least one.
    pub parts: Vec<Differs>,
}

impl Mismatch {
    /// Each part as `name value`, the value `made` or `now` picks.
    fn side(&self, f: &mut fmt::Formatter<'_>, value: fn(&Differs) -> &str) -> fmt::Result {
        for (i, differs) in self.parts.iter().enumerate() {
            let comma = if i == 0 { "" } else { ", " };
            let value = value(differs);
            match differs.part {
                Part::Game => write!(f, "{comma}game \"{value}\"")?,
                part => write!(f, "{comma}{} {value}", part.name())?,
            }
        }
        Ok(())
    }
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("made with ")?;
        self.side(f, |d| &d.made)?;
        f.write_str("; this build: ")?;
        self.side(f, |d| &d.now)
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    fn made(engine: &str, game: &str, content: u64) -> Made {
        Made {
            engine: engine.into(),
            game: game.into(),
            content: ContentHash(content),
        }
    }

    #[test]
    fn a_new_stamp_names_this_engine() {
        let stamp = Made::new("demo 1.0", ContentHash(7));
        assert_eq!(stamp, made(ENGINE_VERSION, "demo 1.0", 7));
        assert_eq!(ENGINE_VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(stamp.check(&stamp.clone()), Ok(()));
    }

    #[test]
    fn a_check_names_each_part_that_differs_with_both_values() {
        let now = made("0.2.0", "demo 1.0", 0x3c4d_5e6f_7a8b_9c0d);
        let engine = made("0.1.0", "demo 1.0", 0x3c4d_5e6f_7a8b_9c0d)
            .check(&now)
            .unwrap_err();
        assert_eq!(
            engine.parts,
            [Differs {
                part: Part::Engine,
                made: "0.1.0".into(),
                now: "0.2.0".into()
            }]
        );
        assert_eq!(
            engine.to_string(),
            "made with engine 0.1.0; this build: engine 0.2.0"
        );
        let game = made("0.2.0", "demo 0.9", 0x3c4d_5e6f_7a8b_9c0d)
            .check(&now)
            .unwrap_err();
        assert_eq!(
            game.to_string(),
            "made with game \"demo 0.9\"; this build: game \"demo 1.0\""
        );
        let content = made("0.2.0", "demo 1.0", 1).check(&now).unwrap_err();
        assert_eq!(content.parts[0].part, Part::Content);
        assert_eq!(
            content.to_string(),
            "made with content 0000000000000001; this build: content 3c4d5e6f7a8b9c0d"
        );
        let two = made("0.1.0", "demo 1.0", 0x1a2b_3c4d_5e6f_7a8b)
            .check(&now)
            .unwrap_err();
        assert_eq!(
            two.to_string(),
            "made with engine 0.1.0, content 1a2b3c4d5e6f7a8b; this build: engine 0.2.0, \
             content 3c4d5e6f7a8b9c0d"
        );
        let all = made("0.1.0", "x", 1).check(&now).unwrap_err();
        let parts: Vec<&str> = all.parts.iter().map(|d| d.part.name()).collect();
        assert_eq!(parts, ["engine", "game", "content"]);
    }

    #[test]
    fn a_content_hash_reads_back_from_its_sixteen_hex_digits() {
        let hash = ContentHash(0x00ab_cdef_0123_4567);
        assert_eq!(hash.to_string(), "00abcdef01234567");
        assert_eq!("00abcdef01234567".parse(), Ok(hash));
        assert_eq!("00ABCDEF01234567".parse(), Ok(hash));
        assert_eq!(format!("{}", ContentHash(u64::MAX)), "ffffffffffffffff");
        for bad in [
            "",
            "abc",
            "00abcdef012345678",
            "+0abcdef01234567",
            "00abcdef0123456g",
        ] {
            let error = bad.parse::<ContentHash>().unwrap_err();
            assert_eq!(error, NotAHash(bad.into()));
        }
        assert_eq!(
            NotAHash("x".into()).to_string(),
            "\"x\" is not a content hash (sixteen hex digits)"
        );
    }

    #[test]
    fn a_stamp_is_written_with_its_hash_as_text_and_read_back() {
        let stamp = made("0.1.0", "demo 1.0", 0x1a2b_3c4d_5e6f_7a8b);
        let text = ron::to_string(&stamp).unwrap();
        assert_eq!(
            text,
            "(engine:\"0.1.0\",game:\"demo 1.0\",content:\"1a2b3c4d5e6f7a8b\")"
        );
        assert_eq!(ron::from_str::<Made>(&text).unwrap(), stamp);
        assert_eq!(
            stamp.to_string(),
            "engine 0.1.0, game \"demo 1.0\", content 1a2b3c4d5e6f7a8b"
        );
        let bad = ron::from_str::<ContentHash>("\"1a2b\"").unwrap_err();
        assert!(bad.to_string().contains("not a content hash"), "{bad}");
        assert!(ron::from_str::<ContentHash>("12").is_err());
        // A value of another type says what a hash is.
        let number: Result<ContentHash, serde::de::value::Error> =
            ContentHash::deserialize(serde::de::IntoDeserializer::into_deserializer(12_u64));
        let error = number.unwrap_err().to_string();
        assert!(error.ends_with("expected sixteen hex digits"), "{error}");
    }
}
