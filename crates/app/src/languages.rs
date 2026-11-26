//! The system's preferred languages: what a game picks its
//! first language from ([`PlatformFacts::languages`]). On native,
//! `sys-locale` reads them (the `system-languages` feature; without it
//! there are none); on the web, the browser's `navigator.languages`,
//! through `crates/cli/web/languages.js`. Each is as the system writes it
//! (`en-US`, `pt_BR`): `PlatformFacts::with_languages` reads them as tags.
//! `system()` gives them, most preferred first; none where the system
//! can't say.
//!
//! [`PlatformFacts::languages`]: shuyi_ui::PlatformFacts::languages

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::read as system;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::read as system;
