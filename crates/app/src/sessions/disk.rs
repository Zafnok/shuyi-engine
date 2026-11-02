//! Where a desktop build keeps its sessions: `sessions/` under the game's
//! data folder, each written through a temporary file and a rename, the
//! newest [`KEEP_SESSIONS`] staying. A record must start at the first frame
//! to play back, so the files kept are whole sessions.
//!
//! The folder is beside the game's saves, never in its `Storage`: a record
//! holds the store it was handed, and must not land in the next one's.
//! `FileStorage` reads only the `.ron` files at the top of the data folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use macroquad::prelude::{info, warn};

use super::{SUFFIX, file_name};
use crate::storage::data_dir;

/// How many sessions a build keeps: older ones are deleted.
const KEEP_SESSIONS: usize = 10;

/// The folder of the kept sessions, under the game's data folder.
const FOLDER: &str = "sessions";

/// The start time in a kept session's file name; `None` for any other file.
fn start_of(name: &str) -> Option<u64> {
    name.strip_suffix(SUFFIX)?.parse().ok()
}

/// Which of the files `names` to delete: every kept session but the newest
/// [`KEEP_SESSIONS`], by the start times in their names. Any other file
/// stays.
fn to_delete(names: &[String]) -> Vec<String> {
    let mut kept: Vec<(u64, &String)> = names
        .iter()
        .filter_map(|name| Some((start_of(name)?, name)))
        .collect();
    kept.sort_unstable_by_key(|&(start, _)| core::cmp::Reverse(start));
    let old = kept.into_iter().skip(KEEP_SESSIONS);
    old.map(|(_, name)| name.clone()).collect()
}

/// Writes `text` as the session that started at `start` in `dir`, in place
/// of an earlier write of it, and deletes what [`to_delete`] says.
fn write(dir: &Path, start: u64, text: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(file_name(start));
    let tmp = dir.join(format!("{start}.tmp"));
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &path)?;
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        names.extend(entry?.file_name().into_string());
    }
    for name in to_delete(&names) {
        fs::remove_file(dir.join(name))?;
    }
    Ok(path)
}

/// The Unix time now, in ms: the name of a session, never what plays it.
pub(super) fn unix_ms() -> u64 {
    let since = SystemTime::now().duration_since(UNIX_EPOCH);
    since.map_or(0, |time| {
        u64::try_from(time.as_millis()).unwrap_or(u64::MAX)
    })
}

/// The folder of the sessions of the game called `name`. With no data
/// folder, a warning, and none.
pub(super) fn folder(name: &str) -> Option<PathBuf> {
    let dir = data_dir(name).map(|dir| dir.join(FOLDER));
    if dir.is_none() {
        // check-text: not player text (a warning for the log)
        warn!("storage: no data folder: this session won't be kept");
    }
    dir
}

/// [`write`], with one line saying where or why not.
pub(super) fn save(dir: &Path, start: u64, text: &str) {
    match write(dir, start, text) {
        Ok(path) => {
            // check-text: not player text (a line for the log)
            info!("storage: session kept in {}", path.display());
        }
        Err(e) => {
            // check-text: not player text (a warning for the log)
            warn!("storage: session not kept: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use shuyi_ui::storage::Storage;

    use super::*;
    use crate::storage::FileStorage;

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort_unstable();
        names
    }

    #[test]
    fn a_kept_sessions_start_is_in_its_name() {
        assert_eq!(start_of("1700.session.ron"), Some(1_700));
        for other in ["save.ron", "x.session.ron", "1700.tmp"] {
            assert_eq!(start_of(other), None, "{other}");
        }
    }

    #[test]
    fn the_newest_ten_stay_by_their_names_start_times() {
        // 9 and 10 sort the other way round as text.
        let mut files: Vec<String> = (1..=12).map(file_name).collect();
        files.push("notes.txt".to_owned());
        let mut gone = to_delete(&files);
        gone.sort_unstable();
        assert_eq!(gone, [file_name(1), file_name(2)]);
        assert_eq!(to_delete(&files[..10]), [] as [String; 0]);
    }

    #[test]
    fn writes_replace_their_session_and_keep_the_newest_ten() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(FOLDER);
        for start in 1..=12 {
            let path = write(&dir, start, "first").unwrap();
            assert_eq!(path, dir.join(file_name(start)));
        }
        let mut expected: Vec<String> = (3..=12).map(file_name).collect();
        expected.sort_unstable();
        assert_eq!(names(&dir), expected);
        write(&dir, 12, "later").unwrap();
        assert_eq!(names(&dir), expected);
        let text = fs::read_to_string(dir.join(file_name(12))).unwrap();
        assert_eq!(text, "later");
    }

    #[test]
    fn a_kept_session_is_not_among_the_games_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let mut storage = FileStorage::in_dir(tmp.path().to_path_buf());
        storage.write("save", "1").unwrap();
        write(&tmp.path().join(FOLDER), 5, "a record").unwrap();
        assert_eq!(storage.list().unwrap(), ["save"]);
    }
}
