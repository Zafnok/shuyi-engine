//! Files the player dropped on the window or the canvas (ticket 3092): the
//! system's [`SystemEvent::FilesDropped`], from what macroquad kept of
//! miniquad's `files_dropped_event`.
//!
//! miniquad 0.4 reads a drop on Windows, Linux (X11 and Wayland), macOS
//! and the web, where its loader reads each file the page was handed.
//! macroquad keeps each file's path and bytes until the end of the frame
//! (`get_dropped_files`), so the backend reads them in every frame, a web
//! frame that doesn't run the game too ([`Held`](crate::pacing::Held)), and
//! `App` hands them to the game in the next that does. `App` refuses a
//! drop over the limits ([`shuyi_platform::dropped::accept`]) and logs why.

use macroquad::input::get_dropped_files;
use shuyi_platform::{DroppedFile, SystemEvent};

/// One file as macroquad gives it: its path (on the web, its name alone)
/// and its bytes, either missing when the system couldn't say.
pub(crate) type Raw = (Option<String>, Option<Vec<u8>>);

/// The drop of the files macroquad has kept this frame, if any.
pub(crate) fn poll() -> Option<SystemEvent> {
    let raw = get_dropped_files()
        .into_iter()
        .map(|file| {
            let path = file.path.map(|p| p.to_string_lossy().into_owned());
            (path, file.bytes)
        })
        .collect();
    event(raw)
}

/// The drop of `raw`'s files, each by its name, leaving out one with no
/// path or no bytes; none when no file is left.
pub(crate) fn event(raw: Vec<Raw>) -> Option<SystemEvent> {
    let files: Vec<DroppedFile> = raw
        .into_iter()
        .filter_map(|(path, bytes)| Some(DroppedFile::new(&path?, bytes?)))
        .collect();
    (!files.is_empty()).then_some(SystemEvent::FilesDropped(files))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drop_is_each_file_by_its_name_with_its_bytes() {
        let raw = vec![
            (Some("C:\\games\\doom1.wad".to_owned()), Some(vec![1, 2])),
            (Some("/home/me/level.txt".to_owned()), Some(b"hi".to_vec())),
        ];
        let Some(SystemEvent::FilesDropped(files)) = event(raw) else {
            panic!("no drop");
        };
        let names: Vec<&str> = files.iter().map(DroppedFile::name).collect();
        assert_eq!(names, ["doom1.wad", "level.txt"]);
        assert_eq!(files[0].bytes(), [1, 2]);
    }

    #[test]
    fn a_file_the_system_couldnt_read_is_left_out() {
        let raw = vec![
            (None, Some(vec![1])),
            (Some("a.txt".to_owned()), None),
            (Some("b.txt".to_owned()), Some(Vec::new())),
        ];
        let Some(SystemEvent::FilesDropped(files)) = event(raw) else {
            panic!("no drop");
        };
        assert_eq!(files, [DroppedFile::new("b.txt", Vec::new())]);
        assert_eq!(event(Vec::new()), None);
        assert_eq!(event(vec![(None, None)]), None);
    }
}
