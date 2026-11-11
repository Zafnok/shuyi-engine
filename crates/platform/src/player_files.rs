//! Files the player keeps: the answers a backend gives to a screen's
//! requests ([`shuyi_ui::player_files`]), which the game's system hook hears
//! as [`SystemEvent::FileOffered`](crate::SystemEvent::FileOffered) and
//! [`SystemEvent::FileChosen`](crate::SystemEvent::FileChosen).
//!
//! - [`Offered`]: where a file the game offered went: downloaded (the web),
//!   written at a path (a desktop, in [`EXPORTS_FOLDER`]), or why not.
//! - [`ChosenFile`]: the file the player picked, as text, or that they
//!   cancelled, or why it was refused. A backend makes one from the bytes it
//!   read with [`ChosenFile::from_bytes`], which keeps the limits: at most
//!   [`MAX_CHOSEN_BYTES`], and UTF-8.
//!
//! A chosen file's answer is the system's event, so a session record holds
//! its text and a replay gets the same (ADR-0072), as a dropped file's.

use crate::alloc_prelude::*;
use alloc::format;

use serde::{Deserialize, Serialize};

use crate::dropped::MAX_DROPPED_BYTES;

/// The most bytes a chosen file may have: a bigger one is refused
/// ([`ChosenFile::from_bytes`]). The same as a dropped file's.
pub const MAX_CHOSEN_BYTES: usize = MAX_DROPPED_BYTES;

/// The folder, in the game's storage folder, where a desktop writes the
/// files the game offers.
pub const EXPORTS_FOLDER: &str = "exports";

/// Where a file the game offered went
/// ([`SystemEvent::FileOffered`](crate::SystemEvent::FileOffered)).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Offered {
    /// The browser was handed it as a download: where it lands is the
    /// browser's to say (its downloads folder, unless the player chose).
    Downloaded,
    /// It was written at this path: on a desktop, in the exports folder.
    Written(String),
    /// It wasn't offered, and why.
    Failed(String),
}

/// The file the player picked, or why there is none
/// ([`SystemEvent::FileChosen`](crate::SystemEvent::FileChosen)).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChosenFile {
    /// The file the player picked: its name, never a path, and its text.
    #[non_exhaustive]
    Picked {
        /// Its name (`farm.ron`).
        name: String,
        /// Its text.
        text: String,
    },
    /// The player closed the picker without choosing.
    Cancelled,
    /// The file, or the picker, was refused, and why: not UTF-8, over
    /// [`MAX_CHOSEN_BYTES`], no picker on this platform, the browser
    /// wouldn't open one.
    Refused(String),
}

impl ChosenFile {
    /// The file `path` (its whole path, or only its name: only what follows
    /// the last `/` or `\` is kept) holding `text`.
    pub fn picked(path: &str, text: impl Into<String>) -> Self {
        Self::Picked {
            name: file_name(path).to_owned(),
            text: text.into(),
        }
    }

    /// The answer for the file `path` holding `bytes`, as a backend read
    /// them: [`Picked`](Self::Picked) when they are UTF-8 text of at most
    /// [`MAX_CHOSEN_BYTES`], else [`Refused`](Self::Refused) saying which.
    pub fn from_bytes(path: &str, bytes: Vec<u8>) -> Self {
        let name = file_name(path);
        if bytes.len() > MAX_CHOSEN_BYTES {
            return Self::Refused(format!(
                "{name} was refused: {} bytes, more than {MAX_CHOSEN_BYTES}",
                bytes.len()
            ));
        }
        match String::from_utf8(bytes) {
            Ok(text) => Self::picked(name, text),
            Err(_) => Self::Refused(format!("{name} was refused: it isn't UTF-8 text")),
        }
    }
}

impl core::fmt::Debug for ChosenFile {
    /// A picked file's name and its size, never its text (a save is long).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Picked { name, text } => f
                .debug_struct("Picked")
                .field("name", name)
                .field("bytes", &text.len())
                .finish(),
            Self::Cancelled => f.write_str("Cancelled"),
            Self::Refused(why) => f.debug_tuple("Refused").field(why).finish(),
        }
    }
}

/// What follows the last `/` or `\` of `path`.
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Why `name` can't be the name of a file the game offers, if it can't: a
/// name is one file's, with no folder in it, and not empty, `.` or `..`.
/// `App` answers [`Offered::Failed`] with it before a backend sees the name.
pub fn check_offered_name(name: &str) -> Result<(), String> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':', '\0'])
        || name.trim() != name;
    if bad {
        Err(format!(
            "{name:?} isn't a file's name: one name, with no folder in it"
        ))
    } else {
        Ok(())
    }
}

/// The name the `n`th file offered as `name` takes in a folder, counted
/// from 1, so an offer never writes over another: `name` itself, then
/// `farm (2).ron`, `farm (3).ron` … The number goes before the last
/// extension; a name with none (or only a leading dot, `.notes`) takes it
/// at its end.
pub fn numbered_name(name: &str, n: u32) -> String {
    if n <= 1 {
        return name.to_owned();
    }
    match name.rfind('.').filter(|&dot| dot > 0) {
        Some(dot) => format!("{} ({n}){}", &name[..dot], &name[dot..]),
        None => format!("{name} ({n})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn a_second_offer_of_a_name_is_numbered_before_its_extension() {
        assert_eq!(numbered_name("a.ron", 1), "a.ron");
        assert_eq!(numbered_name("a.ron", 2), "a (2).ron");
        assert_eq!(numbered_name("save.tar.gz", 3), "save.tar (3).gz");
        assert_eq!(numbered_name("notes", 2), "notes (2)");
        assert_eq!(numbered_name(".notes", 2), ".notes (2)");
        assert_eq!(numbered_name("été.txt", 10), "été (10).txt");
    }

    #[test]
    fn a_picked_file_keeps_its_name_and_never_its_path() {
        for path in ["farm.ron", "/home/me/farm.ron", "C:\\Users\\me\\farm.ron"] {
            let ChosenFile::Picked { name, text } = ChosenFile::picked(path, "x") else {
                panic!("not picked");
            };
            assert_eq!((name.as_str(), text.as_str()), ("farm.ron", "x"));
        }
    }

    #[test]
    fn utf8_text_within_the_limit_is_picked() {
        let picked = ChosenFile::from_bytes("a.ron", "été".as_bytes().to_vec());
        assert_eq!(picked, ChosenFile::picked("a.ron", "été"));
        let at_limit = ChosenFile::from_bytes("big.ron", vec![b'a'; MAX_CHOSEN_BYTES]);
        assert!(matches!(at_limit, ChosenFile::Picked { .. }));
    }

    #[test]
    fn a_file_not_utf8_or_too_big_is_refused_saying_which() {
        let binary = ChosenFile::from_bytes("/x/a.png", vec![0x89, b'P', b'N', b'G', 0xff]);
        assert_eq!(
            binary,
            ChosenFile::Refused("a.png was refused: it isn't UTF-8 text".into())
        );
        let big = ChosenFile::from_bytes("big.ron", vec![b'a'; MAX_CHOSEN_BYTES + 1]);
        let ChosenFile::Refused(why) = big else {
            panic!("{big:?}");
        };
        assert!(
            why.starts_with("big.ron was refused: 268435457 bytes"),
            "{why}"
        );
    }

    #[test]
    fn its_debug_says_the_size_not_the_text() {
        let shown = format!("{:?}", ChosenFile::picked("a.ron", "hello"));
        assert_eq!(shown, r#"Picked { name: "a.ron", bytes: 5 }"#);
        assert_eq!(format!("{:?}", ChosenFile::Cancelled), "Cancelled");
        let refused = format!("{:?}", ChosenFile::Refused("no".into()));
        assert_eq!(refused, r#"Refused("no")"#);
    }

    #[test]
    fn an_offered_name_is_one_files_name() {
        for good in ["a.ron", "farm (2).ron", "été.txt", ".hidden"] {
            assert_eq!(check_offered_name(good), Ok(()), "{good}");
        }
        for bad in [
            "",
            ".",
            "..",
            "a/b.ron",
            "..\\up.ron",
            "C:x",
            " a.ron",
            "a\0",
        ] {
            assert!(check_offered_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn answers_read_back_from_their_text() {
        let answers = [
            ChosenFile::picked("a.ron", "(x: 1)"),
            ChosenFile::Cancelled,
            ChosenFile::Refused("why".into()),
        ];
        for answer in answers {
            let text = ron::to_string(&answer).unwrap_or_else(|e| panic!("{e}"));
            let back: ChosenFile = ron::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(back, answer);
        }
        for offered in [
            Offered::Downloaded,
            Offered::Written("/data/exports/a.ron".into()),
            Offered::Failed("no".into()),
        ] {
            let text = ron::to_string(&offered).unwrap_or_else(|e| panic!("{e}"));
            let back: Offered = ron::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(back, offered);
        }
    }
}
