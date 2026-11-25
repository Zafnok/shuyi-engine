//! `cargo xtask check-merges`: a merge commit brings no change of its own
//! (ticket 2065).
//!
//! A merge of `main` into a branch should change only what resolving its two
//! parents needs. `git show --remerge-diff` diffs the merge against git's own
//! automatic remerge of its parents, conflict markers and all, so what it
//! shows is what the merge's author typed:
//!
//! - in a file git couldn't merge by content (a `remerge CONFLICT` line and
//!   conflict markers), a change that touches a conflict region resolves it;
//!   any other change of that file is the merge's own;
//! - in a file of another conflict (modify/delete, rename), with no markers,
//!   the whole file is the resolution;
//! - any other file it shows is a change of the merge's own: the error.
//!
//! Why: #279's local `.codex/config.toml` came back through a merge of `main`
//! into #285's branch, in neither parent, and reached `main` unnamed; #277 hid
//! an 18-line fix inside a merge commit.

mod mend;

pub use mend::mend;

use std::path::Path;
use std::process::Command;

/// The merge commits in `base..head` and, for each, every change it makes
/// beyond resolving its parents. Empty means clean.
pub fn run(repo_root: &Path, base: &str, head: &str) -> Vec<String> {
    let range = format!("{base}..{head}");
    let merges = match git(repo_root, &["rev-list", "--merges", &range]) {
        Ok(listing) => listing,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for merge in merges.lines().filter(|line| !line.is_empty()) {
        errors.extend(check_merge(repo_root, merge));
    }
    errors
}

/// The errors of one merge commit.
pub(super) fn check_merge(repo_root: &Path, merge: &str) -> Vec<String> {
    let short = &merge[..merge.len().min(10)];
    let parents = match git(repo_root, &["rev-list", "--parents", "-n", "1", merge]) {
        Ok(line) => line.split_whitespace().count().saturating_sub(1),
        Err(e) => return vec![e],
    };
    if parents != 2 {
        return vec![format!(
            "merge {short}: {parents} parents; only a two-parent merge can be checked \
             (`git show --remerge-diff`), so merge one branch at a time"
        )];
    }
    let diff = match git(
        repo_root,
        &[
            "-c",
            "core.quotePath=false",
            "show",
            "--remerge-diff",
            "--format=",
            "--no-color",
            "--no-ext-diff",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            merge,
        ],
    ) {
        Ok(diff) => diff,
        Err(e) => return vec![e],
    };
    own_changes(&diff)
        .into_iter()
        .map(|change| {
            format!(
                "merge {short}: {change}; a merge changes only what resolving its \
                 parents needs, so commit it on its own (ticket 2065); before it's pushed, \
                 `cargo xtask mend-merges` splits it in two with the same files (ticket 3532)"
            )
        })
        .collect()
}

/// Runs git in `repo_root` and returns its standard output.
pub(super) fn git(repo_root: &Path, args: &[&str]) -> Result<String, String> {
    match Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
    {
        Ok(output) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(output) => Err(format!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(e) => Err(format!("git doesn't run: {e}")),
    }
}

/// One file of a remerge diff.
#[derive(Default)]
struct FileDiff {
    path: String,
    /// It has a `remerge CONFLICT` line: git couldn't merge it alone.
    conflict: bool,
    /// The lines of its hunks, headers included.
    hunks: Vec<String>,
}

/// What a remerge diff (`git show --remerge-diff`) changes beyond resolving
/// conflicts: one line per file, or per change inside a conflicted file.
pub fn own_changes(diff: &str) -> Vec<String> {
    files(diff).iter().flat_map(file_changes).collect()
}

/// `diff` split into its files.
fn files(diff: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut in_hunks = false;
    for line in diff.lines() {
        if let Some(names) = line.strip_prefix("diff --git ") {
            files.push(FileDiff {
                path: path_of(names),
                ..FileDiff::default()
            });
            in_hunks = false;
            continue;
        }
        let Some(file) = files.last_mut() else {
            continue;
        };
        if line.starts_with("@@") {
            in_hunks = true;
        }
        if in_hunks {
            file.hunks.push(line.to_owned());
        } else if line.starts_with("remerge CONFLICT") {
            file.conflict = true;
        }
    }
    files
}

/// The path of `a/<old> b/<new>`: the new one.
fn path_of(names: &str) -> String {
    names
        .rfind(" b/")
        .map_or(names, |at| &names[at + 3..])
        .to_owned()
}

/// Whether `line` (a hunk's line without its `-`, `+` or space) is a
/// conflict marker.
fn is_marker(line: &str) -> bool {
    ["<<<<<<< ", "=======", "||||||| ", ">>>>>>> "]
        .iter()
        .any(|marker| line.starts_with(marker) || line == marker.trim_end())
}

/// The changes of one file that resolve no conflict.
fn file_changes(file: &FileDiff) -> Vec<String> {
    if !file.conflict {
        return vec![format!(
            "{} is changed, and git merged it without a conflict",
            file.path
        )];
    }
    let has_markers = file
        .hunks
        .iter()
        .any(|line| line.strip_prefix(['-', ' ']).is_some_and(is_marker));
    if !has_markers {
        // A conflict of whole files (modify/delete, rename): the file is the
        // resolution.
        return Vec::new();
    }
    stray_blocks(file)
}

/// In a file with conflict markers, each run of changed lines that neither
/// touches a marker nor lies inside a conflict region, named by its first
/// line in the merge.
fn stray_blocks(file: &FileDiff) -> Vec<String> {
    let mut errors = Vec::new();
    let mut inside = false;
    let mut new_line = 0usize;
    // The block being read: its first line in the merge, whether it started
    // inside a region, and whether it has a marker.
    let mut block: Option<(usize, bool, bool)> = None;
    let close = |block: &mut Option<(usize, bool, bool)>, errors: &mut Vec<String>| {
        if let Some((line, started_inside, marker)) = block.take()
            && !started_inside
            && !marker
        {
            errors.push(format!(
                "{} is changed at line {line} outside its conflicts",
                file.path
            ));
        }
    };
    for line in &file.hunks {
        if line.starts_with("@@") {
            close(&mut block, &mut errors);
            new_line = hunk_start(line);
            continue;
        }
        let (kind, text) = line.split_at(line.len().min(1));
        match kind {
            "-" | "+" => {
                let entry = block.get_or_insert((new_line.max(1), inside, false));
                if kind == "-" && is_marker(text) {
                    entry.2 = true;
                    if text.starts_with("<<<<<<<") {
                        inside = true;
                    } else if text.starts_with(">>>>>>>") {
                        inside = false;
                    }
                }
                if kind == "+" {
                    new_line += 1;
                }
            }
            " " => {
                close(&mut block, &mut errors);
                new_line += 1;
            }
            // `\ No newline at end of file`.
            _ => {}
        }
    }
    close(&mut block, &mut errors);
    errors
}

/// The merge's first line of a hunk: `c` of `@@ -a,b +c,d @@`.
fn hunk_start(header: &str) -> usize {
    header
        .split_whitespace()
        .find_map(|word| word.strip_prefix('+'))
        .and_then(|range| range.split(',').next())
        .and_then(|start| start.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A remerge diff as git 2.55 prints it: a resolved content conflict with
    /// one more change below it, a modify/delete resolved by the deletion and
    /// a file in neither parent.
    const DIFF: &str = "\
diff --git a/f b/f
remerge CONFLICT (content): Merge conflict in f
index 156a6df..28420b0 100644
--- a/f
+++ b/f
@@ -1,9 +1,5 @@
 1
-<<<<<<< f82dc71 (main)
-2main
-=======
-2side
->>>>>>> 4f684dc (rmg)
+2both
 3
 4
 5
@@ -12,5 +8,5 @@
 8
 9
 10
-11
+11x
 12
diff --git a/g b/g
deleted file mode 100644
remerge CONFLICT (modify/delete): g deleted in 4f684dc (rmg) and modified in f82dc71 (main).  Version f82dc71 (main) of g left in tree.
index 975fbec..0000000
--- a/g
+++ /dev/null
@@ -1 +0,0 @@
-y
diff --git a/h b/h
new file mode 100644
index 0000000..3e75765
--- /dev/null
+++ b/h
@@ -0,0 +1 @@
+new
";

    #[test]
    fn a_clean_merge_has_no_changes_of_its_own() {
        assert_eq!(own_changes(""), Vec::<String>::new());
    }

    #[test]
    fn only_the_stray_change_and_the_new_file_are_named() {
        let changes = own_changes(DIFF);
        assert_eq!(
            changes,
            [
                "f is changed at line 11 outside its conflicts",
                "h is changed, and git merged it without a conflict",
            ]
        );
    }

    #[test]
    fn a_change_inside_a_conflict_region_resolves_it() {
        // Ours kept, with its second line edited: the edit has no marker of
        // its own, but it is between them.
        let diff = "\
diff --git a/f b/f
remerge CONFLICT (content): Merge conflict in f
@@ -1,12 +1,8 @@
 0
-<<<<<<< ours
 a
 b
 c
 d
-b-edited
+b-changed
 e
 f
 g
-=======
-theirs
->>>>>>> theirs
 1
";
        assert_eq!(own_changes(diff), Vec::<String>::new());
    }

    #[test]
    fn diff3_markers_count_as_markers() {
        assert!(is_marker("||||||| base"));
        assert!(is_marker("======="));
        assert!(!is_marker("==== a heading"));
    }

    #[test]
    fn the_new_path_of_a_rename_is_named() {
        assert_eq!(path_of("a/old name.rs b/new name.rs"), "new name.rs");
    }

    /// A new repository in the temporary folder, `name`d, with one commit
    /// on `main`: a file of twelve numbered lines.
    pub(super) fn repository(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("xtask-merges-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temporary folder");
        for args in [
            &["init", "-q", "-b", "main"][..],
            &["config", "user.name", "test"],
            &["config", "user.email", "test@example.com"],
            &["config", "core.autocrlf", "false"],
            &["config", "commit.gpgsign", "false"],
        ] {
            git(&dir, args).expect("git init");
        }
        let lines = numbered(0, "");
        commit(&dir, "f", &lines, "base");
        dir
    }

    /// Twelve numbered lines, line `changed` reading `text` instead.
    pub(super) fn numbered(changed: u32, text: &str) -> String {
        (1..=12)
            .map(|n| {
                if n == changed {
                    text.to_owned()
                } else {
                    n.to_string()
                }
            })
            .fold(String::new(), |all, line| all + &line + "\n")
    }

    /// Writes `text` to `file` in `dir` and commits it.
    pub(super) fn commit(dir: &Path, file: &str, text: &str, message: &str) {
        std::fs::write(dir.join(file), text).expect("write");
        git(dir, &["add", "."]).expect("add");
        git(dir, &["commit", "-q", "-m", message]).expect("commit");
    }

    /// `side` changes line 2 of `f`; `main` changes line 12. Back on `side`,
    /// ready to merge `main` into it.
    pub(super) fn diverged(name: &str) -> std::path::PathBuf {
        let dir = repository(name);
        git(&dir, &["switch", "-q", "-c", "side"]).expect("switch");
        let side = numbered(2, "2side");
        commit(&dir, "f", &side, "side");
        git(&dir, &["switch", "-q", "main"]).expect("switch");
        let main = numbered(12, "12main");
        commit(&dir, "f", &main, "main");
        git(&dir, &["switch", "-q", "side"]).expect("switch");
        dir
    }

    #[test]
    fn a_clean_merge_in_a_repository_passes() {
        let dir = diverged("clean");
        git(&dir, &["merge", "-q", "--no-edit", "main"]).expect("merge");
        assert_eq!(run(&dir, "main", "HEAD"), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_merge_that_adds_a_file_fails_naming_it() {
        let dir = diverged("adds");
        git(&dir, &["merge", "-q", "--no-commit", "main"]).expect("merge");
        std::fs::write(dir.join("config.toml"), "local = true\n").expect("write");
        git(&dir, &["add", "config.toml"]).expect("add");
        git(&dir, &["commit", "-q", "--no-edit"]).expect("commit");
        let errors = run(&dir, "main", "HEAD");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("config.toml is changed, and git merged it without a conflict"),
            "{}",
            errors[0]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_range_is_an_error() {
        let dir = repository("range");
        let errors = run(&dir, "no-such-branch", "HEAD");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("rev-list"), "{}", errors[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hunk_header_gives_the_merges_first_line() {
        assert_eq!(hunk_start("@@ -12,5 +8,5 @@ fn x()"), 8);
        assert_eq!(hunk_start("@@ -0,0 +1 @@"), 1);
    }
}
