//! The window as the player left it (ticket 3044): a record per computer of
//! its last windowed size and its mode, kept by the backend beside the
//! game's storage (never in a profile), read as the window opens and
//! written once it has settled. All of it is pure: the backend reads the
//! window and the clock, and hands them in.
//!
//! The record is the storage key [`WINDOW_KEY`], one RON value:
//! `(version: 1, width: 1024, height: 768, fullscreen: false)`. Its size is
//! in the units `Conf::window_width` takes on that system: device pixels on
//! Windows and Linux, points on macOS. It is the last windowed size, never
//! the fullscreen one. A record that doesn't parse, or of another version,
//! is ignored with one warning.

use core::time::Duration;

use macroquad::window::Conf;
use serde::{Deserialize, Serialize};
use shuyi_platform::{Frame, Millis, Show};

use crate::Window;

#[cfg(windows)]
pub(crate) mod win32;

/// The storage key of the record, beside the computer's `profiles`: a game
/// must not use it for its own data.
pub(crate) const WINDOW_KEY: &str = "window";

/// The record's version: one of another version is ignored.
pub(crate) const WINDOW_VERSION: u32 = 1;

/// The smallest size recorded or opened at: Windows reports a minimized
/// window as 1 × 1.
pub(crate) const MIN_WINDOW: (i32, i32) = (320, 200);

/// The largest side opened at, whatever the record says.
const MAX_SIDE: i32 = 16384;

/// How long the windowed size and the mode stay the same before they are
/// written. *Tunable*: a crash loses at most this much of a change.
pub(crate) const SAVE_AFTER: Duration = Duration::from_secs(1);

/// The window's size and mode, as recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Record {
    /// [`WINDOW_VERSION`] when written.
    version: u32,
    /// The last windowed width.
    width: i32,
    /// The last windowed height.
    height: i32,
    /// Whether the window filled the screen.
    fullscreen: bool,
}

impl Record {
    /// The record of the windowed size `size` and the mode `fullscreen`.
    pub(crate) const fn new(size: (i32, i32), fullscreen: bool) -> Self {
        Self {
            version: WINDOW_VERSION,
            width: size.0,
            height: size.1,
            fullscreen,
        }
    }

    /// The record's text, as written.
    pub(crate) fn to_text(self) -> String {
        ron::to_string(&self).unwrap_or_default()
    }

    /// The record `text` holds, or the warning saying why it is ignored.
    pub(crate) fn read(text: &str) -> Result<Self, String> {
        match ron::from_str::<Self>(text) {
            // check-text: not player text (a warning for the log)
            Err(e) => Err(format!(
                "the window's record doesn't parse ({e}), so it is ignored"
            )),
            Ok(record) if record.version != WINDOW_VERSION => Err(format!(
                // check-text: not player text (a warning for the log)
                "the window's record is of version {}, not {WINDOW_VERSION}, so it is ignored",
                record.version
            )),
            Ok(record) => Ok(record),
        }
    }

    /// The record's size, within the sizes the window opens at.
    fn clamped_size(self) -> (i32, i32) {
        (
            self.width.clamp(MIN_WINDOW.0, MAX_SIDE),
            self.height.clamp(MIN_WINDOW.1, MAX_SIDE),
        )
    }
}

/// The settings macroquad opens `window` with, as the record `record` (its
/// text, if there is one) left it, and the warning if it is ignored: at the
/// record's size and in its mode, else at `window`'s size, windowed.
pub(crate) fn conf_from(window: &Window, record: Option<&str>) -> (Conf, Option<String>) {
    let (read, warning) = match record.map(Record::read) {
        Some(Ok(read)) => (Some(read), None),
        Some(Err(warning)) => (None, Some(warning)),
        None => (None, None),
    };
    let ((width, height), fullscreen) = read.map_or(((window.width, window.height), false), |r| {
        (r.clamped_size(), r.fullscreen)
    });
    let conf = Conf {
        window_title: window.title.to_owned(),
        window_width: width,
        window_height: height,
        fullscreen,
        window_resizable: true,
        // Full-resolution framebuffer on scaled displays, so the console's
        // integer scale is in real pixels (see render.rs).
        high_dpi: true,
        icon: window.icon.and_then(|icon| crate::window_icon(&icon)),
        platform: macroquad::miniquad::conf::Platform {
            // A hint the driver may ignore; miniquad reads it only here.
            swap_interval: Some(i32::from(window.vsync)),
            ..Default::default()
        },
        ..Default::default()
    };
    (conf, warning)
}

/// What the window opened as ([`crate::window_conf`]), for the backend:
/// its mode, its windowed size, the record it read and the warning if it
/// ignored one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Opened {
    /// Whether it opened filling the screen.
    pub fullscreen: bool,
    /// The windowed size it opened at, or comes back to.
    pub windowed: (i32, i32),
    /// The record read, if it was good: what is on disk.
    pub record: Option<Record>,
    /// Why a record was ignored.
    pub warning: Option<String>,
    /// Whether the record is kept: on native, outside a capture run.
    pub kept: bool,
}

impl Opened {
    /// What `conf` opens, from the record `record` (its text), and its
    /// warning.
    pub(crate) fn new(conf: &Conf, record: Option<&str>, warning: Option<String>) -> Self {
        Self {
            fullscreen: conf.fullscreen,
            windowed: (conf.window_width, conf.window_height),
            record: record.and_then(|text| Record::read(text).ok()),
            warning,
            kept: true,
        }
    }

    /// A window opened windowed at `conf`'s size, whose record isn't kept:
    /// on the web, in a capture run, and for the engine's own programs.
    pub(crate) fn unkept(conf: &Conf) -> Self {
        Self {
            kept: false,
            ..Self::new(conf, None, None)
        }
    }
}

/// Whether `size` is one to record: not under [`MIN_WINDOW`].
fn recordable(size: (i32, i32)) -> bool {
    size.0 >= MIN_WINDOW.0 && size.1 >= MIN_WINDOW.1
}

/// What decides when the record is written: the last windowed size, the
/// record last written, the state seen and since when, and whether the
/// record is kept at all.
#[derive(Debug, Clone)]
pub(crate) struct WindowMemory {
    windowed: (i32, i32),
    written: Option<Record>,
    held: Option<(Record, Millis)>,
    kept: bool,
}

impl WindowMemory {
    /// The memory of a window opened as `opened` says: one whose record
    /// isn't kept ([`Opened::unkept`]) never writes it.
    pub(crate) fn new(opened: &Opened) -> Self {
        Self {
            windowed: opened.windowed,
            written: opened.record,
            held: None,
            kept: opened.kept,
        }
    }

    /// The last windowed size seen: the one to come back to (on Windows).
    #[cfg(any(windows, test))]
    pub(crate) fn windowed(&self) -> (i32, i32) {
        self.windowed
    }

    /// Sees the window at `now`, `size` and filling the screen or not: the
    /// record to write, once the windowed size and the mode have held
    /// [`SAVE_AFTER`] and differ from the last written.
    pub(crate) fn see(
        &mut self,
        now: Millis,
        size: (i32, i32),
        fullscreen: bool,
    ) -> Option<Record> {
        if !fullscreen && recordable(size) {
            self.windowed = size;
        }
        let seen = Record::new(self.windowed, fullscreen);
        match self.held {
            Some((held, since)) if held == seen => {
                if now.since(since) >= SAVE_AFTER {
                    return self.write(seen);
                }
            }
            _ => self.held = Some((seen, now)),
        }
        None
    }

    /// The record to write as the game quits: the last state seen, if it
    /// differs from the last written.
    pub(crate) fn at_quit(&mut self) -> Option<Record> {
        let (seen, _) = self.held?;
        self.write(seen)
    }

    /// `record`, if it is kept and differs from the last written, which it
    /// becomes.
    fn write(&mut self, record: Record) -> Option<Record> {
        if !self.kept || self.written == Some(record) {
            return None;
        }
        self.written = Some(record);
        Some(record)
    }
}

/// Where the window was before it filled the screen, for Windows, where
/// miniquad leaves fullscreen at the screen's size at (0, 0).
#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Place(Option<(i32, i32)>);

#[cfg(any(windows, test))]
impl Place {
    /// The window enters fullscreen: its place as miniquad reads it,
    /// `read`, which crosses as `u32` (a negative x, on a monitor left of
    /// the main one, round-trips).
    pub(crate) fn entering(&mut self, read: (u32, u32)) {
        self.0 = Some((read.0.cast_signed(), read.1.cast_signed()));
    }

    /// The window leaves fullscreen at its windowed size `windowed`, from a
    /// picture of `picture`: where it goes, as miniquad takes it. Its place
    /// before, or centred in the picture if it opened fullscreen.
    pub(crate) fn leaving(self, windowed: (i32, i32), picture: (i32, i32)) -> (u32, u32) {
        let (x, y) = self.0.unwrap_or((
            (picture.0 - windowed.0).max(0) / 2,
            (picture.1 - windowed.1).max(0) / 2,
        ));
        (x.cast_unsigned(), y.cast_unsigned())
    }
}

/// How long the mouse rests, neither moved nor clicked, before the cursor
/// hides (ticket 3314). *Tunable*.
pub(crate) const CURSOR_IDLE: Duration = Duration::from_secs(2);

/// How long after the window's mode changes a change of the mouse's
/// position is the window moving under it, not the mouse: Windows may send
/// a move as the window enters or leaves fullscreen. *Tunable*.
pub(crate) const MODE_SETTLE: Duration = Duration::from_millis(200);

/// The mouse, as [`IdleCursor`] sees it in one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mouse {
    /// Its position over the window, in whole pixels.
    pub at: (i32, i32),
    /// Whether one of its buttons was pressed this frame.
    pub pressed: bool,
}

/// What [`IdleCursor`] sees in one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CursorSeen {
    /// The frame's clock.
    pub now: Millis,
    /// The mouse.
    pub mouse: Mouse,
    /// Whether the window fills the screen.
    pub fullscreen: bool,
    /// Whether the window has the player's focus.
    pub focused: bool,
    /// Whether the cursor stays shown whatever the mouse does: `false`
    /// until a screen reads the mouse (1121).
    pub keep_shown: bool,
}

/// Whether the system's cursor shows over the window (ticket 3314): it
/// hides once the mouse has rested [`CURSOR_IDLE`], and at once as the
/// window fills the screen, and shows again when the mouse moves or a
/// button is pressed. It shows while the window lacks focus, so it is never
/// hidden over another program. It starts as the system's: shown.
#[derive(Debug, Clone, Default)]
pub(crate) struct IdleCursor {
    /// Whether it is hidden, as last reported.
    hidden: bool,
    /// Where the mouse rested last frame.
    mouse: Option<(i32, i32)>,
    /// When the mouse last moved or a button was pressed: `None` once the
    /// window entered fullscreen with neither since.
    active: Option<Millis>,
    /// The mode last seen, and when it changed: `None` before the first
    /// frame.
    mode: Option<(bool, Millis)>,
}

impl IdleCursor {
    /// Sees a frame: whether the cursor is to be shown, if that changed.
    pub(crate) fn see(&mut self, seen: CursorSeen) -> Option<bool> {
        let now = seen.now;
        match self.mode {
            Some((fullscreen, _)) if fullscreen == seen.fullscreen => {}
            first_or_changed => {
                if first_or_changed.is_none() {
                    self.active = Some(now);
                }
                if seen.fullscreen {
                    self.active = None;
                }
                self.mode = Some((seen.fullscreen, now));
            }
        }
        let settling = self
            .mode
            .is_some_and(|(_, since)| now.since(since) < MODE_SETTLE);
        let moved = self.mouse.is_some_and(|at| at != seen.mouse.at);
        self.mouse = Some(seen.mouse.at);
        if seen.mouse.pressed || (moved && !settling) {
            self.active = Some(now);
        }
        let idle = self
            .active
            .is_none_or(|since| now.since(since) >= CURSOR_IDLE);
        let hidden = idle && seen.focused && !seen.keep_shown;
        (hidden != self.hidden).then(|| {
            self.hidden = hidden;
            !hidden
        })
    }
}

/// The mode `frame` asks for: a game frame's `fullscreen`. A card's and the
/// errors' frames leave the window as it is.
pub(crate) fn wanted_fullscreen(frame: &Frame<'_>) -> Option<bool> {
    match frame.show {
        Show::Game { .. } => Some(frame.fullscreen),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use shuyi_platform::headless::HeadlessPlatform;
    use shuyi_platform::{
        App, Audio, Card, DisplayInfo, Platform, Storage, SystemEvent, TextInput,
    };
    use shuyi_ui::toy;
    use shuyi_ui::{PlatformFacts, RawInputEvent};

    use super::*;

    const WINDOW: Window = Window::new("A game", 784, 784);

    /// What `conf_from` opens `WINDOW` at: its size and mode, and the
    /// warning.
    fn opens(record: Option<&str>) -> ((i32, i32), bool, Option<String>) {
        let (conf, warning) = conf_from(&WINDOW, record);
        (
            (conf.window_width, conf.window_height),
            conf.fullscreen,
            warning,
        )
    }

    #[test]
    fn with_no_record_the_window_opens_at_the_games_size_windowed() {
        assert_eq!(opens(None), ((784, 784), false, None));
        let (conf, _) = conf_from(&WINDOW, None);
        assert_eq!(conf.window_title, "A game");
        assert!(conf.window_resizable && conf.high_dpi);
    }

    #[test]
    fn a_record_opens_the_window_at_its_size_and_in_its_mode() {
        let record = "(version: 1, width: 1000, height: 700, fullscreen: true)";
        assert_eq!(opens(Some(record)), ((1000, 700), true, None));
    }

    #[test]
    fn a_records_size_is_clamped() {
        let small = "(version: 1, width: 100, height: 50, fullscreen: false)";
        assert_eq!(opens(Some(small)), ((320, 200), false, None));
        let big = "(version: 1, width: 20000, height: 20000, fullscreen: false)";
        assert_eq!(opens(Some(big)), ((16384, 16384), false, None));
    }

    #[test]
    fn a_record_that_does_not_parse_or_of_another_version_is_ignored_with_a_warning() {
        for text in [
            "not a record",
            "(version: 2, width: 1000, height: 700, fullscreen: true)",
        ] {
            let (size, fullscreen, warning) = opens(Some(text));
            assert_eq!((size, fullscreen), ((784, 784), false), "{text}");
            let warning = warning.expect("a warning");
            assert!(warning.contains("ignored"), "{warning}");
            assert_eq!(warning.lines().count(), 1, "{warning}");
        }
    }

    #[test]
    fn a_record_written_and_read_back_is_the_same() {
        for record in [Record::new((1000, 700), true), Record::new((-5, 0), false)] {
            assert_eq!(Record::read(&record.to_text()), Ok(record));
        }
    }

    #[test]
    fn what_opened_keeps_the_good_record_it_read() {
        let text = "(version: 1, width: 1000, height: 700, fullscreen: true)";
        let (conf, warning) = conf_from(&WINDOW, Some(text));
        let opened = Opened::new(&conf, Some(text), warning);
        assert_eq!(opened.record, Some(Record::new((1000, 700), true)));
        assert!(opened.fullscreen && opened.kept);
        assert_eq!(opened.windowed, (1000, 700));
        let (conf, warning) = conf_from(&WINDOW, Some("broken"));
        let opened = Opened::new(&conf, Some("broken"), warning);
        assert_eq!(opened.record, None);
        assert!(opened.warning.is_some());
        let unkept = Opened::unkept(&conf);
        assert!(!unkept.kept && !unkept.fullscreen && unkept.warning.is_none());
    }

    fn at(ms: u64) -> Millis {
        Millis(ms)
    }

    /// The memory of a window opened windowed at 784 × 784 with no record.
    fn memory() -> WindowMemory {
        let (conf, _) = conf_from(&WINDOW, None);
        WindowMemory::new(&Opened::new(&conf, None, None))
    }

    #[test]
    fn a_size_is_written_once_it_has_held_a_second() {
        let mut memory = memory();
        assert_eq!(memory.see(at(0), (1000, 700), false), None);
        assert_eq!(memory.see(at(999), (1000, 700), false), None);
        let written = Record::new((1000, 700), false);
        assert_eq!(memory.see(at(1000), (1000, 700), false), Some(written));
        assert_eq!(memory.see(at(1500), (1000, 700), false), None);
        assert_eq!(memory.see(at(5000), (1000, 700), false), None);
    }

    #[test]
    fn a_change_starts_the_second_again() {
        let mut memory = memory();
        memory.see(at(0), (1000, 700), false);
        memory.see(at(900), (1010, 700), false);
        assert_eq!(memory.see(at(1000), (1010, 700), false), None);
        let written = memory.see(at(1900), (1010, 700), false);
        assert_eq!(written, Some(Record::new((1010, 700), false)));
    }

    #[test]
    fn the_record_read_at_start_is_not_written_again() {
        let text = "(version: 1, width: 1000, height: 700, fullscreen: false)";
        let (conf, warning) = conf_from(&WINDOW, Some(text));
        let mut memory = WindowMemory::new(&Opened::new(&conf, Some(text), warning));
        memory.see(at(0), (1000, 700), false);
        assert_eq!(memory.see(at(2000), (1000, 700), false), None);
        assert_eq!(memory.at_quit(), None);
    }

    #[test]
    fn a_minimized_size_never_replaces_the_windowed_one() {
        let mut memory = memory();
        memory.see(at(0), (1, 1), false);
        assert_eq!(memory.windowed(), (784, 784));
        let written = memory.see(at(1000), (1, 1), false);
        assert_eq!(written, Some(Record::new((784, 784), false)));
    }

    #[test]
    fn while_fullscreen_the_recorded_size_stays_the_last_windowed_one() {
        let mut memory = memory();
        memory.see(at(0), (1000, 700), false);
        memory.see(at(100), (1920, 1080), true);
        assert_eq!(memory.windowed(), (1000, 700));
        let written = memory.see(at(1100), (1920, 1080), true);
        assert_eq!(written, Some(Record::new((1000, 700), true)));
    }

    #[test]
    fn a_record_that_is_not_kept_is_never_written() {
        let (conf, _) = conf_from(&WINDOW, None);
        let mut memory = WindowMemory::new(&Opened::unkept(&conf));
        memory.see(at(0), (1000, 700), false);
        assert_eq!(memory.see(at(1000), (1000, 700), false), None);
        assert_eq!(memory.at_quit(), None);
        assert_eq!(memory.windowed(), (1000, 700));
    }

    #[test]
    fn quitting_writes_the_record_if_it_differs_from_the_last_written() {
        let mut memory = memory();
        assert_eq!(memory.at_quit(), None, "nothing seen");
        memory.see(at(0), (1000, 700), false);
        assert_eq!(memory.at_quit(), Some(Record::new((1000, 700), false)));
        assert_eq!(memory.at_quit(), None);
        memory.see(at(10), (1000, 700), true);
        assert_eq!(
            memory.see(at(1010), (1000, 700), true),
            Some(Record::new((1000, 700), true))
        );
        assert_eq!(memory.at_quit(), None);
    }

    #[test]
    fn leaving_fullscreen_goes_back_to_the_place_read_on_entering() {
        let mut place = Place::default();
        place.entering(((-1200i32).cast_unsigned(), 40));
        assert_eq!(place, Place(Some((-1200, 40))));
        let (x, y) = place.leaving((1000, 700), (1920, 1080));
        assert_eq!((x.cast_signed(), y), (-1200, 40));
    }

    #[test]
    fn a_window_that_opened_fullscreen_is_centred_when_it_leaves() {
        assert_eq!(
            Place::default().leaving((1000, 700), (1920, 1080)),
            (460, 190)
        );
        assert_eq!(
            Place::default().leaving((3000, 700), (1920, 1080)),
            (0, 190)
        );
    }

    /// A frame at `ms` with the mouse at `mouse`, windowed and focused, no
    /// button pressed.
    fn cursor_at(ms: u64, mouse: (i32, i32)) -> CursorSeen {
        CursorSeen {
            now: at(ms),
            mouse: Mouse {
                at: mouse,
                pressed: false,
            },
            fullscreen: false,
            focused: true,
            keep_shown: false,
        }
    }

    /// `seen` with a mouse button pressed.
    fn pressed(seen: CursorSeen) -> CursorSeen {
        CursorSeen {
            mouse: Mouse {
                pressed: true,
                ..seen.mouse
            },
            ..seen
        }
    }

    /// The same frame in fullscreen.
    fn fullscreen_at(ms: u64, mouse: (i32, i32)) -> CursorSeen {
        CursorSeen {
            fullscreen: true,
            ..cursor_at(ms, mouse)
        }
    }

    /// Whether the cursor shows after each of `frames`, from a new one.
    fn shown_after(frames: &[CursorSeen]) -> Vec<bool> {
        let mut cursor = IdleCursor::default();
        let mut shown = true;
        frames
            .iter()
            .map(|seen| {
                shown = cursor.see(*seen).unwrap_or(shown);
                shown
            })
            .collect()
    }

    #[test]
    fn at_a_windowed_start_the_cursor_shows_until_the_mouse_has_rested_two_seconds() {
        assert_eq!(CURSOR_IDLE, Duration::from_millis(2000));
        let rest = (10, 20);
        let frames = [
            cursor_at(0, rest),
            cursor_at(1999, rest),
            cursor_at(2000, rest),
        ];
        assert_eq!(shown_after(&frames), [true, true, false]);
        let mut cursor = IdleCursor::default();
        assert_eq!(cursor.see(cursor_at(0, rest)), None, "it starts shown");
    }

    #[test]
    fn a_move_or_a_button_shows_the_cursor_again_and_starts_the_rest_over() {
        let moved = [
            cursor_at(0, (10, 20)),
            cursor_at(2000, (10, 20)),
            cursor_at(2500, (11, 20)),
            cursor_at(4499, (11, 20)),
            cursor_at(4500, (11, 20)),
        ];
        assert_eq!(shown_after(&moved), [true, false, true, true, false]);
        let pressed = [
            cursor_at(0, (10, 20)),
            cursor_at(2000, (10, 20)),
            pressed(cursor_at(2500, (10, 20))),
            cursor_at(4499, (10, 20)),
            cursor_at(4500, (10, 20)),
        ];
        assert_eq!(shown_after(&pressed), [true, false, true, true, false]);
    }

    #[test]
    fn entering_fullscreen_hides_the_cursor_at_once() {
        let mut cursor = IdleCursor::default();
        assert_eq!(cursor.see(cursor_at(0, (5, 5))), None);
        assert_eq!(cursor.see(cursor_at(500, (6, 5))), None);
        assert_eq!(cursor.see(fullscreen_at(600, (6, 5))), Some(false));
    }

    #[test]
    fn a_fullscreen_start_hides_the_cursor_on_the_first_frame() {
        let mut cursor = IdleCursor::default();
        assert_eq!(cursor.see(fullscreen_at(0, (0, 0))), Some(false));
        assert_eq!(cursor.see(fullscreen_at(16, (0, 0))), None);
    }

    #[test]
    fn a_move_within_the_settle_of_a_mode_change_is_the_window_not_the_mouse() {
        assert_eq!(MODE_SETTLE, Duration::from_millis(200));
        // Entering fullscreen at 1000: the window moves under the mouse.
        let entering = [
            cursor_at(0, (5, 5)),
            fullscreen_at(1000, (5, 5)),
            fullscreen_at(1199, (300, 200)),
            fullscreen_at(1300, (300, 200)),
        ];
        assert_eq!(shown_after(&entering), [true, false, false, false]);
        // The new place is the mouse's resting place: a move at 200 ms
        // from it shows the cursor.
        let after = [
            cursor_at(0, (5, 5)),
            fullscreen_at(1000, (5, 5)),
            fullscreen_at(1199, (300, 200)),
            fullscreen_at(1200, (301, 200)),
        ];
        assert_eq!(shown_after(&after), [true, false, false, true]);
    }

    #[test]
    fn leaving_fullscreen_does_not_show_the_cursor_by_itself() {
        let frames = [
            fullscreen_at(0, (300, 200)),
            cursor_at(1000, (300, 200)),
            cursor_at(1100, (40, 30)),
            cursor_at(1250, (40, 30)),
            cursor_at(1250, (41, 30)),
        ];
        assert_eq!(shown_after(&frames), [false, false, false, false, true]);
    }

    #[test]
    fn a_button_shows_the_cursor_even_as_the_mode_settles() {
        let frames = [fullscreen_at(0, (0, 0)), pressed(fullscreen_at(50, (0, 0)))];
        assert_eq!(shown_after(&frames), [false, true]);
    }

    #[test]
    fn the_cursor_shows_while_the_window_lacks_focus() {
        let unfocused = |ms| CursorSeen {
            focused: false,
            ..cursor_at(ms, (5, 5))
        };
        let frames = [
            cursor_at(0, (5, 5)),
            cursor_at(2000, (5, 5)),
            unfocused(2100),
            unfocused(9000),
            cursor_at(9100, (5, 5)),
        ];
        assert_eq!(shown_after(&frames), [true, false, true, true, false]);
        let fullscreen = [
            fullscreen_at(0, (5, 5)),
            CursorSeen {
                focused: false,
                ..fullscreen_at(100, (5, 5))
            },
        ];
        assert_eq!(shown_after(&fullscreen), [false, true]);
    }

    #[test]
    fn keep_shown_never_hides_the_cursor() {
        let kept = |seen: CursorSeen| CursorSeen {
            keep_shown: true,
            ..seen
        };
        let frames = [
            kept(cursor_at(0, (5, 5))),
            kept(cursor_at(5000, (5, 5))),
            kept(fullscreen_at(5100, (5, 5))),
            kept(fullscreen_at(9000, (5, 5))),
            fullscreen_at(9100, (5, 5)),
        ];
        assert_eq!(shown_after(&frames), [true, true, true, true, false]);
    }

    #[test]
    fn a_change_is_reported_only_when_the_answer_changes() {
        let mut cursor = IdleCursor::default();
        let reported: Vec<_> = [
            cursor_at(0, (5, 5)),
            cursor_at(1000, (6, 5)),
            cursor_at(3000, (6, 5)),
            cursor_at(4000, (6, 5)),
            cursor_at(4100, (7, 5)),
            cursor_at(4200, (8, 5)),
            fullscreen_at(4300, (8, 5)),
            fullscreen_at(4400, (8, 5)),
        ]
        .into_iter()
        .map(|seen| cursor.see(seen))
        .collect();
        assert_eq!(
            reported,
            [
                None,
                None,
                Some(false),
                None,
                Some(true),
                None,
                Some(false),
                None
            ]
        );
    }

    #[test]
    fn only_a_game_frame_sets_the_mode() {
        let card = Card::text("Loading");
        let mut frame = Frame::new(Show::Card(&card));
        frame.fullscreen = true;
        assert_eq!(wanted_fullscreen(&frame), None);
        let mut frame = Frame::new(Show::Errors("broken"));
        frame.fullscreen = true;
        assert_eq!(wanted_fullscreen(&frame), None);
        // A game frame comes from a running game alone.
        let mut app = App::running(toy::game());
        let mut platform = Wanting(HeadlessPlatform::new(), Vec::new());
        let _ = app.frame_after(&mut platform, 1.0 / 60.0);
        platform.0.push_system(SystemEvent::ToggleFullscreen);
        let _ = app.frame_after(&mut platform, 1.0 / 60.0);
        assert_eq!(platform.1, [Some(false), Some(true)]);
    }

    /// The headless backend, keeping what [`wanted_fullscreen`] gives for
    /// each frame presented.
    struct Wanting(HeadlessPlatform, Vec<Option<bool>>);

    impl Platform for Wanting {
        fn display(&self) -> DisplayInfo {
            self.0.display()
        }
        fn present(&mut self, frame: &Frame<'_>) {
            self.1.push(wanted_fullscreen(frame));
            self.0.present(frame);
        }
        fn poll_input(&mut self, out: &mut Vec<RawInputEvent>) {
            self.0.poll_input(out);
        }
        fn system_events(&mut self, out: &mut Vec<SystemEvent>) {
            self.0.system_events(out);
        }
        fn storage(&mut self) -> Box<dyn Storage> {
            self.0.storage()
        }
        fn audio(&mut self) -> &mut dyn Audio {
            self.0.audio()
        }
        fn text_input(&mut self) -> Option<&mut dyn TextInput> {
            self.0.text_input()
        }
        fn facts(&self) -> PlatformFacts {
            self.0.facts()
        }
    }
}
