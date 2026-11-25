//! The system's languages on Windows, macOS and Linux: `sys-locale`'s
//! list, with the `system-languages` feature; none without it.

/// The system's preferred languages, most preferred first.
#[cfg(feature = "system-languages")]
pub(crate) fn read() -> Vec<String> {
    sys_locale::get_locales().collect()
}

/// None: the `system-languages` feature is off, so a game starts in its
/// source language until the player picks one.
#[cfg(not(feature = "system-languages"))]
pub(crate) fn read() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn each_language_read_is_text() {
        // What the machine running the test prefers: only that each is
        // text, as a system writes it.
        for language in super::read() {
            assert!(!language.is_empty() && language.is_ascii(), "{language:?}");
        }
    }
}
