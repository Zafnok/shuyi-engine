//! Moving a program aside: its twin found by length and time, the names
//! freed, the folder swept.

use std::time::Duration;

use super::*;

/// A fresh `debug/` folder, with `deps/`, under the system temp folder.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("shuyi-aside-{name}-{}", std::process::id()))
        .join("debug");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("deps")).unwrap();
    dir
}

/// A program at `debug/shuyi.exe` and its hard-linked twin in `deps/`.
fn built(dir: &Path) -> (PathBuf, PathBuf) {
    let twin = dir.join("deps/shuyi-0123abcd.exe");
    fs::write(&twin, b"program").unwrap();
    let exe = dir.join("shuyi.exe");
    fs::hard_link(&twin, &exe).unwrap();
    (exe, twin)
}

#[test]
fn a_program_and_its_twin_are_both_moved_and_a_copy_left_in_their_place() {
    let dir = scratch("both");
    let (exe, twin) = built(&dir);
    fs::write(dir.join("deps/shuyi-0123abcd.d"), b"program").unwrap();
    let modified = fs::metadata(&exe).unwrap().modified().unwrap();
    let moved = move_aside(&exe, 7).unwrap();
    let running = dir.join(RUNNING);
    assert_eq!(
        moved,
        [running.join("shuyi-7-0.exe"), running.join("shuyi-7-1.exe")]
    );
    // The same program is at both names, as cargo left it, so cargo sees
    // it fresh; and the copy there is still the program's twin.
    for path in [&exe, &twin] {
        assert_eq!(fs::read(path).unwrap(), b"program");
        assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), modified);
    }
    assert!(!dir.join(RUNNING).join("shuyi-7-0.copy").exists());
    // The copies are files of their own: a build replaces them, and the
    // moved program stays as it was.
    fs::write(&exe, b"fresh").unwrap();
    fs::write(&twin, b"fresh").unwrap();
    assert!(
        moved
            .iter()
            .all(|path| fs::read(path).unwrap() == b"program")
    );
    assert!(dir.join("deps/shuyi-0123abcd.d").exists());
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn an_examples_twin_beside_it_is_moved_too() {
    let dir = scratch("example").join("examples");
    fs::create_dir_all(&dir).unwrap();
    let twin = dir.join("tools-0123abcd.exe");
    fs::write(&twin, b"program").unwrap();
    let exe = dir.join("tools.exe");
    fs::hard_link(&twin, &exe).unwrap();
    let running = dir.join(RUNNING);
    assert_eq!(
        move_aside(&exe, 7).unwrap(),
        [running.join("tools-7-0.exe"), running.join("tools-7-1.exe")]
    );
    assert_eq!(twins(&exe, "tools"), [twin]);
    fs::remove_dir_all(dir.parent().unwrap().parent().unwrap()).unwrap();
}

#[test]
fn a_program_with_no_extension_keeps_none() {
    let dir = scratch("no-extension");
    let twin = dir.join("deps/shuyi-0123abcd");
    fs::write(&twin, b"program").unwrap();
    let exe = dir.join("shuyi");
    fs::hard_link(&twin, &exe).unwrap();
    let running = dir.join(RUNNING);
    assert_eq!(
        move_aside(&exe, 7).unwrap(),
        [running.join("shuyi-7-0"), running.join("shuyi-7-1")]
    );
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn a_deps_file_of_another_length_or_time_is_left() {
    let dir = scratch("other");
    let (exe, twin) = built(&dir);
    let longer = dir.join("deps/shuyi-longer.exe");
    fs::write(&longer, b"programs").unwrap();
    let older = dir.join("deps/shuyi-older.exe");
    fs::write(&older, b"program").unwrap();
    let modified = fs::metadata(&exe).unwrap().modified().unwrap();
    fs::File::options()
        .write(true)
        .open(&older)
        .unwrap()
        .set_modified(modified - Duration::from_secs(60))
        .unwrap();
    let other_program = dir.join("deps/cargo-shuyi-0123abcd.exe");
    fs::hard_link(&twin, &other_program).unwrap();
    assert_eq!(move_aside(&exe, 7).unwrap().len(), 2);
    assert!(longer.exists() && older.exists() && other_program.exists());
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn a_program_with_no_twin_is_never_moved() {
    let dir = scratch("installed");
    let exe = dir.join("shuyi.exe");
    fs::write(&exe, b"program").unwrap();
    fs::write(dir.join("deps/shuyi-0123abcd.exe"), b"an older build").unwrap();
    assert_eq!(move_aside(&exe, 7).unwrap(), Vec::<PathBuf>::new());
    assert!(exe.exists());
    assert!(!dir.join(RUNNING).exists());
    // No `deps/` at all, as in cargo install's bin folder.
    fs::remove_dir_all(dir.join("deps")).unwrap();
    assert_eq!(move_aside(&exe, 7).unwrap(), Vec::<PathBuf>::new());
    assert!(exe.exists());
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[test]
fn sweep_deletes_what_it_can_and_skips_what_it_cannot() {
    let dir = scratch("sweep").join(RUNNING);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("shuyi-1-0.exe"), b"done").unwrap();
    fs::create_dir(dir.join("a-folder")).unwrap();
    #[cfg(windows)]
    let held = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::write(dir.join("shuyi-2-0.exe"), b"running").unwrap();
        // Shared with no one, as a running program's file refuses deletion.
        fs::File::options()
            .read(true)
            .share_mode(0)
            .open(dir.join("shuyi-2-0.exe"))
            .unwrap()
    };
    sweep(&dir);
    assert!(!dir.join("shuyi-1-0.exe").exists());
    assert!(dir.join("a-folder").is_dir());
    #[cfg(windows)]
    {
        assert!(dir.join("shuyi-2-0.exe").exists());
        drop(held);
    }
    // A folder that isn't there is nothing to sweep.
    sweep(&dir.join("none"));
    fs::remove_dir_all(dir.parent().unwrap().parent().unwrap()).unwrap();
}

#[test]
fn two_moves_with_the_same_pid_dont_collide() {
    let dir = scratch("same-pid");
    let (exe, _) = built(&dir);
    let first = move_aside(&exe, 7).unwrap();
    // What the first move left can't be swept here (a folder stands in
    // for a program still running), so the second picks other names.
    fs::remove_file(&first[0]).unwrap();
    fs::create_dir(&first[0]).unwrap();
    // The copies left in place are twins, and moved in turn.
    let second = move_aside(&exe, 7).unwrap();
    let running = dir.join(RUNNING);
    assert_eq!(
        second,
        [running.join("shuyi-7-1.exe"), running.join("shuyi-7-2.exe")]
    );
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}
