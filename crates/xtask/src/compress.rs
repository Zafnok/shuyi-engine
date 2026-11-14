//! The NTFS compression of `target/` that `cargo xtask gates` makes sure of
//! on Windows, before its first gate (ticket 2075).
//!
//! The `post-checkout` hook (`scripts/git-hooks/post-checkout`) compresses a
//! new worktree's `target/`; this covers a worktree made before the hook, or
//! without hooks. Compression set on the folder is inherited by every file a
//! build writes there, and halves the disk a `target/` takes. A failure is a
//! warning: it never fails a gate.

use std::path::Path;

/// `FILE_ATTRIBUTE_COMPRESSED`, as `GetFileAttributes` reports it.
pub const FILE_ATTRIBUTE_COMPRESSED: u32 = 0x800;

/// Whether `attributes` (a file's Windows attributes) say it is compressed.
pub const fn is_compressed(attributes: u32) -> bool {
    attributes & FILE_ATTRIBUTE_COMPRESSED != 0
}

/// Whether the gates check `target/` at all: on Windows, and not on CI
/// (`ci` is the `CI` variable, which every CI service sets).
pub fn wanted(windows: bool, ci: Option<&std::ffi::OsStr>) -> bool {
    windows && ci.is_none()
}

/// Compresses `root`'s `target/` when it lacks the attribute, saying so on
/// stderr. Does nothing off Windows or on CI.
pub fn ensure(root: &Path) {
    if !wanted(cfg!(windows), std::env::var_os("CI").as_deref()) {
        return;
    }
    #[cfg(windows)]
    windows::ensure(&root.join("target"));
    #[cfg(not(windows))]
    let _ = root;
}

#[cfg(windows)]
mod windows {
    use std::os::windows::fs::MetadataExt;
    use std::path::Path;
    use std::process::{Command, Stdio};

    /// The folder's attributes, or `None` when it can't be read.
    pub fn attributes(dir: &Path) -> Option<u32> {
        std::fs::metadata(dir).ok().map(|m| m.file_attributes())
    }

    pub fn ensure(target: &Path) {
        if attributes(target).is_some_and(super::is_compressed) {
            return;
        }
        eprintln!(
            "== target/ is not compressed: compressing it with NTFS first \
             (once; minutes on a large folder)"
        );
        if let Err(e) = compress(target) {
            eprintln!("warning: could not compress target/ ({e}); the gates go on");
        }
    }

    /// `compact /c /s:<target> /i /q`: marks the folder and every subfolder,
    /// and compresses the files there.
    fn compress(target: &Path) -> Result<(), String> {
        std::fs::create_dir_all(target).map_err(|e| e.to_string())?;
        let mut subtree = std::ffi::OsString::from("/s:");
        subtree.push(target);
        let status = Command::new("compact.exe")
            .arg("/c")
            .arg(subtree)
            .args(["/i", "/q"])
            .stdout(Stdio::null())
            .status()
            .map_err(|e| format!("compact.exe did not start: {e}"))?;
        // `/i` goes on past a file in use (this program's own exe), and may
        // then exit non-zero; the folder's own attribute is what later
        // builds inherit, so it is the answer.
        if attributes(target).is_some_and(super::is_compressed) {
            Ok(())
        } else {
            Err(format!(
                "compact.exe ({status}) left the folder without the compressed attribute"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compressed_attribute_is_0x800() {
        assert!(is_compressed(0x800));
        assert!(is_compressed(0x810), "a compressed folder");
        assert!(!is_compressed(0x10), "a plain folder");
        assert!(!is_compressed(0));
    }

    #[test]
    fn the_check_runs_on_windows_only_and_never_on_ci() {
        assert!(wanted(true, None));
        assert!(!wanted(true, Some("true".as_ref())));
        assert!(!wanted(true, Some("".as_ref())), "set, even empty");
        assert!(!wanted(false, None));
    }

    /// A folder `compact` marks reads as compressed, and one it doesn't as
    /// not: the attribute is read off the disk, not assumed.
    #[cfg(windows)]
    #[test]
    fn a_folder_reads_its_compressed_attribute() {
        let dir = std::env::temp_dir().join(format!("xtask-compress-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let read = |d: &Path| windows::attributes(d).map(is_compressed);
        let compact = |switch: &str| {
            std::process::Command::new("compact.exe")
                .arg(switch)
                .arg(&dir)
                .stdout(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        };
        // A volume that isn't NTFS can't compress: then only the read holds.
        if compact("/u") && compact("/c") {
            assert_eq!(read(&dir), Some(true));
            assert!(compact("/u"));
            assert_eq!(read(&dir), Some(false));
        }
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(read(&dir), None);
    }
}
