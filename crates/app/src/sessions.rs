//! The sessions a build keeps (ADR-0074): with debug tools on, a record of
//! every run (`shuyi_platform::session`), which the debug menu's "Save
//! session" hands over as `<unix ms of its start>.session.ron`, the record
//! so far, and recording goes on. A desktop build also writes it when the
//! game quits ([`disk`]: the newest ten stay). The web has no files and never
//! quits, so it saves only when asked, as a download ([`web`], through the
//! player's files' plugin).

#[cfg(not(target_arch = "wasm32"))]
mod disk;
#[cfg(target_arch = "wasm32")]
mod web;

use shuyi_platform::App;
use shuyi_ui::GameData;

/// Whether a build records its sessions: only with debug tools on, so a
/// shipped build never holds a player's input.
const fn records(debug_tools: bool) -> bool {
    debug_tools
}

/// The file of a session that started `start` ms after the Unix epoch.
fn file_name(start: u64) -> String {
    format!("{start}{SUFFIX}")
}

/// What ends a kept session's file name.
const SUFFIX: &str = ".session.ron";

/// A run being kept, or not.
#[derive(Debug)]
pub(crate) struct Kept {
    /// Where it goes, if it is kept.
    #[cfg(not(target_arch = "wasm32"))]
    dir: Option<std::path::PathBuf>,
    /// When it started.
    start: u64,
}

impl Kept {
    /// Turns `app`'s recording on with debug tools on, before its first
    /// frame, with `voice_manifest` the text the backend put into the game's
    /// setup. `at` is where the game's storage is: its data folder on a
    /// desktop.
    pub(crate) fn start<G: GameData>(
        app: &mut App<G>,
        at: &crate::storage::StorageAt,
        voice_manifest: Option<String>,
    ) -> Self {
        let on = records(shuyi_ui::DEBUG_TOOLS) && app.record();
        if let Some(record) = app.session_mut() {
            record.voice_manifest = voice_manifest;
        }
        #[cfg(not(target_arch = "wasm32"))]
        return Self {
            dir: on.then(|| disk::folder(at.dir())).flatten(),
            start: disk::unix_ms(),
        };
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (on, at);
            Self {
                start: web::unix_ms(),
            }
        }
    }

    /// Hands over `app`'s record so far: as a file on a desktop, one line
    /// saying where or why not; as a download on the web.
    pub(crate) fn save<G: GameData>(&self, app: &App<G>) {
        let Some(record) = app.session() else {
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(dir) = &self.dir {
            disk::save(dir, self.start, &record.to_ron());
        }
        #[cfg(target_arch = "wasm32")]
        web::download(&file_name(self.start), &record.to_ron());
    }

    /// [`save`](Self::save), at the game's quit: the web never quits.
    pub(crate) fn save_at_quit<G: GameData>(&self, app: &App<G>) {
        if !cfg!(target_arch = "wasm32") {
            self.save(app);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_build_with_debug_tools_records() {
        assert!(records(true));
        assert!(!records(false));
    }

    #[test]
    fn a_sessions_file_is_named_for_its_start() {
        assert_eq!(file_name(1_700), "1700.session.ron");
    }
}
