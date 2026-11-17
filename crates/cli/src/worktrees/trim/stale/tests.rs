//! The stale hashes of a `deps/` folder: the hash in a file's name, the
//! groups, and fsutil's answers.

use std::fs::{self, File, FileTimes};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::*;

const HASH_A: &str = "0123456789abcdef";
const HASH_B: &str = "fedcba9876543210";
const HASH_C: &str = "00112233aabbccdd";

#[test]
fn the_hash_is_the_first_dash_and_sixteen_hex_digits() {
    assert_eq!(
        split_hash("libfoo-0123456789abcdef.rlib"),
        Some(("libfoo", ".rlib"))
    );
    assert_eq!(
        split_hash("shuyi_core-0123456789ABCDEF"),
        Some(("shuyi_core", "")),
        "a test program on Linux: no kind"
    );
    assert_eq!(
        split_hash("libmy-crate-0123456789abcdef.rmeta"),
        Some(("libmy-crate", ".rmeta")),
        "a dash in the crate's name"
    );
    assert_eq!(
        split_hash("libfoo-0123456789abcdef.dll.a"),
        Some(("libfoo", ".dll.a"))
    );
    assert_eq!(split_hash("libfoo-0123456789abcde.rlib"), None, "15 digits");
    assert_eq!(
        split_hash("libfoo-0123456789abcdef0.rlib"),
        None,
        "17 digits"
    );
    assert_eq!(
        split_hash("libfoo-0123456789abcdeg.rlib"),
        None,
        "a non-hex digit"
    );
    assert_eq!(
        split_hash("libcafé-0123456789abcdef.rlib"),
        Some(("libcafé", ".rlib")),
        "a non-ASCII name"
    );
    assert_eq!(split_hash("é-é"), None);
    assert_eq!(split_hash("libfoo.rlib"), None);
    assert_eq!(split_hash("libfoo-"), None);
}

/// A folder of its own under the system's temporary folder, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-stale-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn days_ago(days: u64) -> SystemTime {
    SystemTime::now() - Duration::from_secs(days * 24 * 60 * 60)
}

/// A file `name` in `deps`, last accessed `days` ago.
fn file(deps: &Path, name: &str, days: u64) {
    fs::write(deps.join(name), name).unwrap();
    File::options()
        .write(true)
        .open(deps.join(name))
        .unwrap()
        .set_times(FileTimes::new().set_accessed(days_ago(days)))
        .unwrap();
}

/// The names of `deps`'s stale files with `cutoff` days ago.
fn stale_names(deps: &Path, cutoff: u64) -> Vec<String> {
    stale_files(deps, days_ago(cutoff))
        .iter()
        .map(|stale| {
            stale
                .path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn the_newest_accessed_hash_and_hashes_read_since_the_cutoff_are_kept() {
    let deps = scratch("groups");
    file(&deps, &format!("libfoo-{HASH_A}.rlib"), 20);
    file(&deps, &format!("libfoo-{HASH_B}.rlib"), 10);
    file(&deps, &format!("libfoo-{HASH_C}.rlib"), 2);
    // All three older than the cutoff: only the newest is kept.
    file(&deps, &format!("libbar-{HASH_A}.rlib"), 30);
    file(&deps, &format!("libbar-{HASH_B}.rlib"), 40);
    file(&deps, &format!("libbar-{HASH_C}.rlib"), 50);
    assert_eq!(
        stale_names(&deps, 5),
        [
            format!("libbar-{HASH_C}.rlib"),
            format!("libbar-{HASH_B}.rlib"),
            format!("libfoo-{HASH_A}.rlib"),
            format!("libfoo-{HASH_B}.rlib"),
        ]
    );
    assert_eq!(
        stale_names(&deps, 15),
        [
            format!("libbar-{HASH_C}.rlib"),
            format!("libbar-{HASH_B}.rlib"),
            format!("libfoo-{HASH_A}.rlib"),
        ],
        "foo's hash read 10 days ago is after a cutoff of 15"
    );
    let lengths: u64 = stale_files(&deps, days_ago(15))
        .iter()
        .map(|stale| stale.len)
        .sum();
    assert_eq!(lengths, 3 * 28, "each file holds its own name");
}

#[test]
fn a_crate_under_one_hash_is_never_stale_and_kinds_are_groups_of_their_own() {
    let deps = scratch("kinds");
    file(&deps, &format!("libone-{HASH_A}.rlib"), 90);
    file(&deps, &format!("libfoo-{HASH_A}.rlib"), 2);
    file(&deps, &format!("libfoo-{HASH_B}.rmeta"), 90);
    file(&deps, &format!("foo-{HASH_B}.d"), 90);
    file(&deps, "libfoo.rlib", 90);
    // A folder, and a file with no hash, are never looked at.
    fs::create_dir_all(deps.join(format!("libfoo-{HASH_C}.rlib"))).unwrap();
    assert_eq!(stale_names(&deps, 5), Vec::<String>::new());
    file(&deps, &format!("libfoo-{HASH_C}.rmeta"), 1);
    assert_eq!(stale_names(&deps, 5), [format!("libfoo-{HASH_B}.rmeta")]);
    assert_eq!(stale_files(&deps.join("missing"), days_ago(5)), []);
}

#[test]
fn fsutil_is_read_by_its_words_then_by_its_value() {
    let off =
        Err("last-access updates are off (fsutil behavior query disablelastaccess)".to_owned());
    assert_eq!(
        fsutil_answer(
            "DisableLastAccess = 2  (System Managed, Last Access Time Updates ENABLED)\r\n"
        ),
        Ok(())
    );
    assert_eq!(
        fsutil_answer("DisableLastAccess = 3  (System Managed, Last Access Time Updates DISABLED)"),
        off
    );
    assert_eq!(
        fsutil_answer("DisableLastAccess = 0  (User Managed, Last Access Time Updates DISABLED)"),
        off,
        "the words win over the value"
    );
    assert_eq!(fsutil_answer("DisableLastAccess = 0"), Ok(()));
    assert_eq!(fsutil_answer("DisableLastAccess = 2"), Ok(()));
    assert_eq!(fsutil_answer("DisableLastAccess = 1"), off);
    assert_eq!(fsutil_answer("DisableLastAccess = 3"), off);
    assert_eq!(fsutil_answer("DisableLastAccess = 0x80000003"), off);
    assert_eq!(fsutil_answer("DisableLastAccess = 0x80000002"), Ok(()));
    let error = fsutil_answer("Error:  Access is denied.").unwrap_err();
    assert!(error.contains("Access is denied"), "{error}");
    assert!(fsutil_answer("DisableLastAccess = what").is_err());
}
