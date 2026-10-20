//! Save versions (game ADR-0039): every save names the version of its format
//! that wrote it, and a build reads only its own version. A save of another
//! version is refused, never migrated. [`SaveHeader`] reads the version of
//! any save, even one whose other fields no longer parse, so the refusal
//! can say "another version" rather than "damaged". What a save holds, and
//! where it is kept, are the game's.

use core::fmt;
use core::marker::PhantomData;

use serde::Deserialize;
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, Visitor};

/// A game's save format: the type its saves are read into.
pub trait SaveFormat {
    /// The name the save's type is written under: the struct name a text
    /// format may put before the save's fields.
    const NAME: &'static str;
    /// The version this build reads and writes. Raise it whenever a saved
    /// type changes shape or meaning.
    const VERSION: u32;
}

/// Just the version of a save of format `F`: reads any such save,
/// whatever the rest of it holds, in a format that names its fields (RON,
/// JSON).
pub struct SaveHeader<F> {
    /// The version that wrote the save.
    pub version: u32,
    format: PhantomData<fn() -> F>,
}

impl<F: SaveFormat> SaveHeader<F> {
    /// Whether this build can read the save: it is of
    /// [`F::VERSION`](SaveFormat::VERSION).
    pub fn is_current(&self) -> bool {
        self.version == F::VERSION
    }
}

// By hand rather than derived, so that they ask nothing of `F`.
impl<F> Clone for SaveHeader<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Copy for SaveHeader<F> {}

impl<F> PartialEq for SaveHeader<F> {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl<F> Eq for SaveHeader<F> {}

impl<F> fmt::Debug for SaveHeader<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaveHeader")
            .field("version", &self.version)
            .finish()
    }
}

impl<'de, F: SaveFormat> Deserialize<'de> for SaveHeader<F> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let version = deserializer.deserialize_struct(F::NAME, &["version"], VersionOnly)?;
        Ok(SaveHeader {
            version,
            format: PhantomData,
        })
    }
}

/// A save's fields: its version, and the others, which are skipped.
#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum Field {
    Version,
    #[serde(other)]
    Other,
}

/// Reads a save's `version` and skips its other fields.
struct VersionOnly;

impl<'de> Visitor<'de> for VersionOnly {
    type Value = u32;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a save with a version")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<u32, A::Error> {
        let mut version = None;
        while let Some(field) = map.next_key::<Field>()? {
            match field {
                Field::Version if version.is_some() => {
                    return Err(de::Error::duplicate_field("version"));
                }
                Field::Version => version = Some(map.next_value()?),
                Field::Other => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        version.ok_or_else(|| de::Error::missing_field("version"))
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::{String, ToString};

    use serde::de::value::{Error, U32Deserializer};

    use super::*;

    struct Toy;

    impl SaveFormat for Toy {
        const NAME: &'static str = "ToySave";
        const VERSION: u32 = 3;
    }

    type Header = SaveHeader<Toy>;

    fn read(text: &str) -> Result<Header, String> {
        ron::from_str(text).map_err(|e| e.to_string())
    }

    #[test]
    fn the_header_reads_the_version_whatever_the_rest_holds() {
        let header = read("(version:3,world:(seed:9),units:[1,2])").unwrap();
        assert_eq!(header.version, 3);
        assert!(header.is_current());
        // Named, with the version last, from a build whose fields differ.
        let header = read("ToySave(units:[(a:1)],era:\"old\",version:2)").unwrap();
        assert_eq!(header.version, 2);
        assert!(!header.is_current());
        assert!(!read("(version:4)").unwrap().is_current());
    }

    #[test]
    fn what_is_not_a_save_of_the_format_is_refused() {
        assert!(read("OtherSave(version:3)").is_err());
        assert!(read("not a save").is_err());
        assert!(read("[3]").is_err());
        let missing = read("(units:[1])").unwrap_err();
        assert!(
            missing.contains("missing field named `version`"),
            "{missing}"
        );
        let twice = read("(version:3,version:4)").unwrap_err();
        assert!(twice.contains("duplicate field named `version`"), "{twice}");
    }

    #[test]
    fn headers_compare_and_show_their_version() {
        let header = read("(version:2)").unwrap();
        let copy = header;
        assert_eq!(header, copy);
        assert_ne!(header, read("(version:3)").unwrap());
        assert_eq!(format!("{header:?}"), "SaveHeader { version: 2 }");
    }

    #[test]
    fn the_error_says_a_save_was_expected() {
        let error = Header::deserialize(U32Deserializer::<Error>::new(3)).unwrap_err();
        assert!(
            error.to_string().contains("expected a save with a version"),
            "{error}"
        );
    }
}
