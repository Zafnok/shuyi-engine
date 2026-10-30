//! [`AppInfo`]: who a game is, the same on every target: the name players
//! see, a reverse-DNS id, its version and build, its publisher, and the
//! name its saves are kept under (ADR-0059).
//!
//! A game writes it once, in its `shuyi.ron`'s `app` part, and
//! `shuyi app-info` writes it as a Rust constant (`pub const APP:
//! AppInfo`) its binary compiles. A game that keeps its own builds it
//! here in a `const` item:
//!
//! ```
//! use shuyi_platform::AppInfo;
//!
//! const APP: AppInfo = AppInfo::new("Moon Garden", "com.example.moon-garden", "0.1.0")
//!     .with_storage("moon_garden")
//!     .with_publisher("Moon Garden's makers");
//!
//! assert_eq!((APP.storage, APP.build), ("moon_garden", 1));
//! ```

/// Who a game is. Build it with [`AppInfo::new`] and the `with_` setters:
/// a struct literal doesn't compile outside this crate, so a new field
/// never breaks a game (ADR-0039).
///
/// ```compile_fail
/// let app = shuyi_platform::AppInfo {
///     name: "A game",
///     id: "com.example.a-game",
///     version: "0.1.0",
///     publisher: "",
///     storage: "com.example.a-game",
///     build: 1,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AppInfo {
    /// The name players see: the window's title and the loading card's
    /// line.
    pub name: &'static str,
    /// Its reverse-DNS id, unique to the game (`com.example.moon-garden`).
    pub id: &'static str,
    /// Its version, `MAJOR.MINOR.PATCH` (semver).
    pub version: &'static str,
    /// Who publishes it; empty if no one is named.
    pub publisher: &'static str,
    /// The name its saves are kept under: the folder in the OS's data
    /// folder on native, the prefix of every `localStorage` key on the
    /// web. Never change it after a release, or every player's saves and
    /// settings are lost.
    pub storage: &'static str,
    /// A number that only grows from one build handed out to the next,
    /// where a target asks for one beside the version.
    pub build: u32,
}

impl AppInfo {
    /// The game called `name`, with the id `id`, at version `version`: its
    /// saves kept under its id, at build 1, with no publisher named.
    #[must_use]
    pub const fn new(name: &'static str, id: &'static str, version: &'static str) -> Self {
        Self {
            name,
            id,
            version,
            publisher: "",
            storage: id,
            build: 1,
        }
    }

    /// The game with its saves kept under `storage`
    /// ([`storage`](Self::storage)).
    #[must_use]
    pub const fn with_storage(self, storage: &'static str) -> Self {
        Self { storage, ..self }
    }

    /// The game at build `build` ([`build`](Self::build)).
    #[must_use]
    pub const fn with_build(self, build: u32) -> Self {
        Self { build, ..self }
    }

    /// The game published by `publisher`.
    #[must_use]
    pub const fn with_publisher(self, publisher: &'static str) -> Self {
        Self { publisher, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Built in a `const` item, as a game's generated file builds it.
    const SET: AppInfo = AppInfo::new("A game", "com.example.a-game", "1.2.3")
        .with_storage("a-game")
        .with_build(7)
        .with_publisher("Someone");

    #[test]
    fn a_new_app_keeps_its_saves_under_its_id_at_build_one() {
        let app = AppInfo::new("A game", "com.example.a-game", "0.1.0");
        assert_eq!(
            (app.name, app.id, app.version),
            ("A game", "com.example.a-game", "0.1.0")
        );
        assert_eq!(
            (app.storage, app.build, app.publisher),
            ("com.example.a-game", 1, "")
        );
    }

    #[test]
    fn each_setter_changes_its_field_alone() {
        assert_eq!(
            (SET.storage, SET.build, SET.publisher),
            ("a-game", 7, "Someone")
        );
        assert_eq!(
            (SET.name, SET.id, SET.version),
            ("A game", "com.example.a-game", "1.2.3")
        );
        let plain = AppInfo::new("A game", "com.example.a-game", "1.2.3");
        assert_eq!(
            plain
                .with_storage("a-game")
                .with_build(7)
                .with_publisher("Someone"),
            SET
        );
    }
}
