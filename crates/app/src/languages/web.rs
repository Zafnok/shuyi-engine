//! The system's languages on the web: the browser's `navigator.languages`
//! (else `navigator.language`), through the `shuyi_languages` miniquad
//! plugin in `crates/cli/web/languages.js` (our own code, loaded by
//! `crates/cli/web/index.html`). A tag is ASCII, so it crosses one
//! character code at a time and needs no string helper.

/// The plugin's version, which the JS loader compares with the `version`
/// in `crates/cli/web/languages.js` (a `shuyi-cli` test keeps them equal).
/// Bump both when the functions below change.
const PLUGIN_VERSION: u32 = 1;

/// The longest tag read; a longer one is cut there, and then isn't a tag.
const MAX_TAG: u32 = 64;

// Declaring the functions `crates/cli/web/languages.js` provides, and
// exporting the version the loader asks for (game ADR-0034). They take and
// return plain numbers, and the JS answers 0 past the end, so calling them
// is safe.
#[allow(unsafe_code)]
unsafe extern "C" {
    /// Reads the browser's languages; returns how many. `shuyi_lang_char`
    /// indexes into that list until the next call.
    safe fn shuyi_lang_count() -> u32;
    /// The character code at `j` of language `i`; 0 past its end.
    safe fn shuyi_lang_char(i: u32, j: u32) -> u32;
}

/// What the JS loader calls to check the plugin's version.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_languages_crate_version() -> u32 {
    PLUGIN_VERSION
}

/// The browser's languages, most preferred first.
pub(crate) fn read() -> Vec<String> {
    (0..shuyi_lang_count())
        .map(|i| {
            (0..MAX_TAG)
                .map(|j| shuyi_lang_char(i, j))
                .take_while(|&code| code != 0)
                .filter_map(char::from_u32)
                .collect()
        })
        .collect()
}
