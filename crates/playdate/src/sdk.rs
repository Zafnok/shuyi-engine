//! [`Sdk`]: the calls of Panic's SDK the backend is written against, small
//! enough that a fake stands in for them in the tests (`crate::fake`) and
//! `crate::device` puts them on the real SDK through `shim/shim.c`.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use shuyi_ui::frame::onebit::Frame;
use shuyi_ui::perf::HeapCounts;
use shuyi_ui::storage::StorageError;

/// The file of the game's baked pack in the `.pdx`: `shuyi export playdate`
/// places it, and `pdc` copies a file it doesn't know as it is.
pub const PACK: &str = "game.pack";

/// What the SDK tells a game between updates (`PDSystemEvent`, by number;
/// `shim/shim.c` checks them against the header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdkEvent {
    /// The device was locked (`kEventLock`).
    Lock,
    /// The device was unlocked (`kEventUnlock`).
    Unlock,
    /// The system menu opened (`kEventPause`).
    Pause,
    /// The system menu closed (`kEventResume`).
    Resume,
    /// The system is closing the game (`kEventTerminate`).
    Terminate,
    /// The battery is nearly empty (`kEventLowPower`).
    LowPower,
}

impl SdkEvent {
    /// The event of the SDK's number `event`, if the backend hears it.
    pub const fn from_raw(event: i32) -> Option<Self> {
        match event {
            2 => Some(Self::Lock),
            3 => Some(Self::Unlock),
            4 => Some(Self::Pause),
            5 => Some(Self::Resume),
            6 => Some(Self::Terminate),
            9 => Some(Self::LowPower),
            _ => None,
        }
    }
}

/// The SDK's calls. A clone is the same SDK: the storage keeps its own, as a
/// game's context outlives nothing the backend holds.
pub trait Sdk: Clone + fmt::Debug + 'static {
    /// A sound the SDK holds in memory (`AudioSample`).
    type Sample: Copy + PartialEq + fmt::Debug;
    /// A sample player (`SamplePlayer`).
    type SamplePlayer: Copy + fmt::Debug;
    /// A file player (`FilePlayer`).
    type FilePlayer: Copy + fmt::Debug;

    /// `getCurrentTimeMilliseconds`: milliseconds from a fixed start, which
    /// stands still while the device sleeps and wraps at `u32::MAX`.
    fn millis(&self) -> u32;
    /// The high-resolution timer in whole microseconds from its start
    /// (`getElapsedTime`), never going back; never reset by the backend.
    fn clock_us() -> u64;
    /// The debug build's heap counter, read: `None` in a release build.
    fn heap() -> Option<HeapCounts>;
    /// `getButtonState`: the bits held now, and those pressed since the
    /// last call.
    fn buttons(&self) -> (u32, u32);
    /// `getCrankChange`: degrees turned since the last call, clockwise
    /// positive.
    fn crank_change(&self) -> f32;
    /// `isCrankDocked`.
    fn crank_docked(&self) -> bool;
    /// `addMenuItem`: an item named `title` that gives `slot` back when
    /// chosen; false if the SDK refused it.
    fn add_menu_item(&self, title: &str, slot: u8) -> bool;
    /// `display->setRefreshRate`.
    fn set_refresh_rate(&self, fps: u16);
    /// Runs `draw` on the frame buffer (`getFrame`).
    fn with_frame(&self, draw: impl FnOnce(&mut Frame));
    /// `markUpdatedRows`: rows `first` to `last` of the frame buffer changed.
    fn mark_rows(&self, first: i32, last: i32);
    /// Clears the screen and draws `text`, centred and wrapped, in the
    /// system's font: black on white if `white`, else white on black.
    fn draw_text(&self, text: &str, white: bool);
    /// `logToConsole`: one line.
    fn log(&self, line: &str);
    /// Whether `path` in the game's data folder is a file.
    fn is_file(&self, path: &str) -> bool;
    /// The whole of file `path` in the game's data folder.
    fn read_data(&self, path: &str) -> Result<Vec<u8>, String>;
    /// The whole of file `path` in the game's `.pdx`.
    fn read_bundled(&self, path: &str) -> Result<Vec<u8>, String>;
    /// Replaces file `path` in the game's data folder with `bytes`, flushed.
    fn write_data(&self, path: &str, bytes: &[u8]) -> Result<(), String>;
    /// Moves file `from` to `to` in the game's data folder.
    fn rename_data(&self, from: &str, to: &str) -> Result<(), String>;
    /// Deletes file `path` in the game's data folder.
    fn remove_data(&self, path: &str) -> Result<(), String>;
    /// The names in `folder` of the game's data folder (`""`: the data
    /// folder itself); a folder's has a `/` after it.
    fn data_names(&self, folder: &str) -> Result<Vec<String>, String>;
    /// Makes `folder` in the game's data folder; an error if it is there.
    fn make_data_folder(&self, folder: &str) -> Result<(), String>;
    /// `sample->load`: the sound at `path` in the `.pdx`, without its
    /// extension; `None` if there is none.
    fn sample_load(&self, path: &str) -> Option<Self::Sample>;
    /// `sample->getLength`, in seconds.
    fn sample_length(&self, sample: Self::Sample) -> f32;
    /// `sampleplayer->newPlayer`.
    fn sample_player_new(&self) -> Option<Self::SamplePlayer>;
    /// Plays `sample` from its start on `player`, stopping what it played, at
    /// `volume`, once or looped, at `rate` (1 as recorded).
    fn sample_player_play(
        &self,
        player: Self::SamplePlayer,
        sample: Self::Sample,
        volume: f32,
        looped: bool,
        rate: f32,
    );
    /// `sampleplayer->setVolume`, the same on both sides.
    fn sample_player_set_volume(&self, player: Self::SamplePlayer, volume: f32);
    /// `sampleplayer->stop`.
    fn sample_player_stop(&self, player: Self::SamplePlayer);
    /// `sampleplayer->isPlaying`.
    fn sample_player_is_playing(&self, player: Self::SamplePlayer) -> bool;
    /// `sampleplayer->setPaused`.
    fn sample_player_set_paused(&self, player: Self::SamplePlayer, paused: bool);
    /// A new file player on the file at `path` in the `.pdx`, without its
    /// extension; `None` if there is no such file.
    fn file_open(&self, path: &str) -> Option<Self::FilePlayer>;
    /// Starts `player`, or goes on from where its pause left it, once or
    /// looped: false if it could not (`fileplayer->play`).
    fn file_play(&self, player: Self::FilePlayer, looped: bool) -> bool;
    /// `fileplayer->setVolume`, the same on both sides.
    fn file_set_volume(&self, player: Self::FilePlayer, volume: f32);
    /// `fileplayer->setRate`.
    fn file_set_rate(&self, player: Self::FilePlayer, rate: f32);
    /// `fileplayer->pause`.
    fn file_pause(&self, player: Self::FilePlayer);
    /// `fileplayer->stop`.
    fn file_stop(&self, player: Self::FilePlayer);
    /// `fileplayer->setOffset` to the file's start.
    fn file_rewind(&self, player: Self::FilePlayer);
    /// `fileplayer->isPlaying`.
    fn file_is_playing(&self, player: Self::FilePlayer) -> bool;
    /// `fileplayer->getLength`, in seconds.
    fn file_length(&self, player: Self::FilePlayer) -> f32;
    /// Stops `player` and frees it: it isn't used again.
    fn file_close(&self, player: Self::FilePlayer);
}

/// A file call's error, as storage reports it.
pub(crate) fn backend(path: &str, message: &str) -> StorageError {
    // check-text: not player text (for the log)
    StorageError::Backend(alloc::format!("{path}: {message}"))
}
