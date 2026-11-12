//! The player's files on the web, which has no files of its own: an offered
//! file is a download, and an ask opens the browser's file picker, through
//! the `shuyi_player_files` miniquad plugin in
//! `crates/cli/web/player-files.js` (our own code, loaded by
//! `crates/cli/web/index.html`). The debug menu's "Save session" downloads
//! through it too ([`download`]).
//!
//! The picker answers later: the backend polls for the answer each frame
//! ([`PlayerFiles::poll`]) while an ask is open, and one ask at a time is
//! open (another is refused).

use sapp_jsutils::{JsObject, JsObjectWeak};
use shuyi_platform::player_files::MAX_CHOSEN_BYTES;
use shuyi_platform::{ChosenFile, Offered, SystemEvent};
use shuyi_ui::player_files::Folder;

/// The plugin's version, which the JS loader compares with the `version`
/// in `crates/cli/web/player-files.js` (a `shuyi-cli` test keeps them
/// equal). Bump both when the functions below change.
const PLUGIN_VERSION: u32 = 2;

// Declaring the functions `crates/cli/web/player-files.js` provides. Strings
// cross as `sapp_jsutils.js`'s objects, so calling them is safe.
#[allow(unsafe_code)]
unsafe extern "C" {
    /// Offers the player a file called `name` holding `text`, as a download:
    /// 1 once the browser was handed it, 0 if it threw.
    safe fn shuyi_offer_file(name: JsObjectWeak, text: JsObjectWeak) -> u32;
    /// Opens the browser's file picker for the extensions in `accept`
    /// (`.ron,.txt`; any when empty), refusing a file over `max_bytes`: 1
    /// once it opened, 0 when the browser won't (no recent press).
    safe fn shuyi_ask_for_file(accept: JsObjectWeak, max_bytes: u32) -> u32;
    /// The answer to the open ask: an object whose `kind` is 0 while the
    /// player chooses, 1 picked (`name`, `bytes`), 2 cancelled, 3 refused
    /// (`reason`).
    safe fn shuyi_file_answer() -> JsObject;
}

/// What the JS loader calls to check the plugin's version.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_player_files_crate_version() -> u32 {
    PLUGIN_VERSION
}

/// Why an ask is refused while another is open.
const ONE_AT_A_TIME: &str = "another file is being chosen";

/// Why an ask is refused when the browser won't open its picker.
const NO_PRESS: &str = "the browser opens its file picker only just after a press: \
                        ask in the frame of the press, or drop the file on the page";

/// The player's files on the web.
#[derive(Debug, Default)]
pub(crate) struct PlayerFiles {
    /// Whether an ask waits for the player's answer.
    asking: bool,
}

impl PlayerFiles {
    /// The player's files of the game whose storage is called `_name`: its
    /// storage is the page's, and has no folder.
    pub(crate) fn new(_at: &crate::storage::StorageAt) -> Self {
        Self::default()
    }

    /// Hands the browser `text` as a download called `name`.
    #[allow(clippy::unused_self)] // the desktop's reads its folder
    pub(crate) fn offer(&self, name: &str, text: &str) -> Offered {
        if download(name, text) {
            Offered::Downloaded
        } else {
            Offered::Failed(format!("the browser refused the download of {name}"))
        }
    }

    /// Opens the picker: the answer comes later ([`poll`](Self::poll)), or
    /// now when the picker can't open.
    pub(crate) fn ask(&mut self, accept: &[String]) -> Option<ChosenFile> {
        if self.asking {
            return Some(ChosenFile::Refused(ONE_AT_A_TIME.to_owned()));
        }
        let accept = JsObject::string(&accept.join(","));
        let max = u32::try_from(MAX_CHOSEN_BYTES).unwrap_or(u32::MAX);
        if shuyi_ask_for_file(accept.weak(), max) == 0 {
            return Some(ChosenFile::Refused(NO_PRESS.to_owned()));
        }
        self.asking = true;
        None
    }

    /// The answer to the open ask, once the player has given it.
    pub(crate) fn poll(&mut self) -> Option<SystemEvent> {
        if !self.asking {
            return None;
        }
        let answer = shuyi_file_answer();
        let read = |field: &str| {
            let mut text = String::new();
            answer.field(field).to_string(&mut text);
            text
        };
        let chosen = match answer.field_u32("kind") {
            0 => return None,
            1 => {
                let mut bytes = Vec::new();
                answer.field("bytes").to_byte_buffer(&mut bytes);
                ChosenFile::from_bytes(&read("name"), bytes)
            }
            2 => ChosenFile::Cancelled,
            _ => ChosenFile::Refused(read("reason")),
        };
        self.asking = false;
        Some(SystemEvent::FileChosen(chosen))
    }

    /// The web has no folders to open.
    #[allow(clippy::unused_self)] // as `offer`
    pub(crate) fn open(&self, _folder: Folder) -> Result<(), String> {
        Err("the web has no folders: the game keeps its data in the browser's site storage".into())
    }
}

/// Offers the player `text` as a file called `name`, a download: whether
/// the browser was handed it.
pub(crate) fn download(name: &str, text: &str) -> bool {
    let name = JsObject::string(name);
    let text = JsObject::string(text);
    shuyi_offer_file(name.weak(), text.weak()) == 1
}
