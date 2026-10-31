//! Where the game's voice clips are (game ADR-0046), and their manifest: the
//! shared player plays them ([`shuyi_platform::audio::player`]).

use std::path::Path;

use super::folder_beside;

/// Where the voice clips of the script's language `lang` are: `<lang>` in
/// the voice `folder` next to the executable when it's there (a shipped
/// build), else relative to the working directory (`cargo run` from the
/// repo root, or the web build, which fetches from beside the page).
pub(crate) fn platform_voice_dir(folder: &str, lang: &str) -> String {
    let voice = if cfg!(target_arch = "wasm32") {
        folder.to_owned()
    } else {
        let exe = std::env::current_exe().ok();
        folder_beside(folder, exe.as_deref(), Path::is_dir)
    };
    format!("{voice}/{lang}")
}

/// The text of the voice manifest `file` in `dir`, or `None` if there is
/// none (no voices: not a problem). `Err` if the file is there and can't
/// be read.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::unused_async)] // The web build fetches it.
pub async fn read_manifest(dir: &str, file: &str) -> Result<Option<String>, String> {
    let path = format!("{dir}/{file}");
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{path}: {e}")),
    }
}

/// The text of the voice manifest `file` in `dir`, fetched from beside the
/// page, or `None` if it can't be had: a build without voices has no such
/// file, and a failed fetch doesn't say why it failed.
#[cfg(target_arch = "wasm32")]
pub async fn read_manifest(dir: &str, file: &str) -> Result<Option<String>, String> {
    let path = format!("{dir}/{file}");
    Ok(macroquad::file::load_string(&path).await.ok())
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use super::*;

    /// Runs a future that never waits on anything external.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    /// Acceptance: with no voice folder the game starts and logs nothing
    /// about voices.
    #[test]
    fn without_a_voice_folder_there_is_no_manifest() {
        let nowhere = tempfile::tempdir().unwrap();
        let dir = format!("{}/voice/en", nowhere.path().display());
        assert_eq!(block_on(read_manifest(&dir, "voice.ron")), Ok(None));
    }

    #[test]
    fn a_manifest_that_cannot_be_read_is_an_error_naming_it() {
        // A folder where the file should be.
        let odd = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(odd.path().join("voice.ron")).unwrap();
        let dir = odd.path().display().to_string();
        let error = block_on(read_manifest(&dir, "voice.ron")).unwrap_err();
        let start = format!("{dir}/voice.ron: ");
        assert!(error.starts_with(&start), "{error}");
    }

    #[test]
    fn a_manifest_is_read_whole() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("voices.ron"), "(clips: [])").unwrap();
        let dir = dir.path().display().to_string();
        let text = block_on(read_manifest(&dir, "voices.ron"));
        assert_eq!(text, Ok(Some("(clips: [])".to_owned())));
    }

    #[test]
    fn the_voice_folder_is_the_scripts_language_under_the_voice_folder() {
        let dir = platform_voice_dir("voice", "en");
        assert!(dir.ends_with("voice/en"), "{dir}");
        assert!(!dir.contains('\\'), "{dir}");
    }
}
