//! Shuyi Engine's default crates as one dynamic library (ticket 2009): the
//! `shuyi` crate's `dynamic` feature links it, and rustc then takes
//! `shuyi-core` to `shuyi-app`, macroquad among them, from it, so a
//! rebuild of the game links only the game.

pub use shuyi_app;
pub use shuyi_assets;
pub use shuyi_core;
pub use shuyi_platform;
pub use shuyi_ui;
