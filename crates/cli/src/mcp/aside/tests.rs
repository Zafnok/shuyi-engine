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
fn a_program_is_at_its_path_at_every_moment_of_a_move() {
    let dir = scratch("never-missing");
    let from = dir.join("shuyi.exe");
    fs::write(&from, b"program").unwrap();
    let done = std::sync::atomic::AtomicBool::new(false);
    let missing = std::thread::scope(|scope| {
        let watcher = scope.spawn(|| {
            let mut missing = 0_u32;
            while !done.load(std::sync::atomic::Ordering::Relaxed) {
                missing += u32::from(!from.exists());
            }
            missing
        });
        for n in 0..200 {
            put_aside(&from, &dir.join(format!("aside-{n}.exe"))).unwrap();
        }
        done.store(true, std::sync::atomic::Ordering::Relaxed);
        watcher.join().unwrap()
    });
    assert_eq!(missing, 0);
    assert_eq!(fs::read(&from).unwrap(), b"program");
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

#[cfg(windows)]
#[test]
fn a_running_program_is_put_aside_and_its_path_left_replaceable() {
    let dir = scratch("running");
    let from = dir.join("ping.exe");
    let system = std::env::var_os("SystemRoot").unwrap();
    fs::copy(Path::new(&system).join("System32/PING.EXE"), &from).unwrap();
    let mut running = std::process::Command::new(&from)
        .args(["-n", "60", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let to = dir.join("ping-aside.exe");
    let put = put_aside(&from, &to);
    // What the build does next: write a new program at the old path.
    // A scan of the new `.exe` can hold the path for a moment, as for a
    // build's linker; a refusal that lasts still fails below.
    let rewritten = retrying(TRIES, PAUSE, || fs::write(&from, b"fresh"));
    running.kill().unwrap();
    running.wait().unwrap();
    put.unwrap();
    rewritten.unwrap();
    assert!(to.exists());
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

/// `retrying` with no wait, counting its tries; the first `busy` answer
/// `code`, and the rest succeed.
fn tried(tries: u32, busy: u32, code: i32) -> (std::io::Result<()>, u32) {
    let mut count = 0;
    let answer = retrying(tries, Duration::ZERO, || {
        count += 1;
        if count <= busy {
            Err(std::io::Error::from_raw_os_error(code))
        } else {
            Ok(())
        }
    });
    (answer, count)
}

#[test]
fn a_busy_answer_that_clears_is_retried() {
    for code in BUSY {
        let (answer, count) = tried(5, 1, code);
        assert!(answer.is_ok());
        assert_eq!(count, 2);
    }
}

#[test]
fn a_busy_answer_that_lasts_is_returned_after_every_try() {
    for code in BUSY {
        let (answer, count) = tried(5, u32::MAX, code);
        assert_eq!(answer.unwrap_err().raw_os_error(), Some(code));
        assert_eq!(count, 5);
    }
}

#[test]
fn any_other_error_is_returned_at_once() {
    let (answer, count) = tried(5, u32::MAX, 2);
    assert_eq!(answer.unwrap_err().raw_os_error(), Some(2));
    assert_eq!(count, 1);
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
