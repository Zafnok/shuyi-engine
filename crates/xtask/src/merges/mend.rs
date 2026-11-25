//! `cargo xtask mend-merges`: a merge that `check-merges` refuses is split
//! in two before it's pushed (ticket 3532).
//!
//! Only the branch's commits that no remote-tracking branch has are made
//! again (`git rev-list HEAD --not --remotes`), so the push that follows is
//! a plain one. Each refused merge becomes:
//!
//! 1. the same merge of the same parents, its conflicts resolved to the
//!    branch's side (`git merge-tree --write-tree -X ours`), which changes
//!    nothing outside a conflict, so `check-merges` passes it;
//! 2. a commit on top holding the rest: the original merge's own tree.
//!
//! Every commit after it is made again on the new ones, with its own tree,
//! author and message. The branch's last tree doesn't change, so the
//! working copy and the index stay as they are.
//!
//! Why: ten of the eleven PRs closed unmerged from 2026-11-17 to 2026-11-24
//! were replaced by a new PR only because a merge of `main` carried an edit
//! (an ADR renumbered, a lint fix), which CI's `check-merges` saw only after
//! the push, when the history could change only by a force-push.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use super::{check_merge, git};

/// Splits every refused merge among the branch's unpushed commits. Returns
/// what it did, one line each, or why it changed nothing.
pub fn mend(repo_root: &Path) -> Result<Vec<String>, String> {
    let branch = git(repo_root, &["symbolic-ref", "-q", "HEAD"])
        .map_err(|_| "HEAD isn't on a branch: switch to the branch to mend".to_owned())?;
    let branch = branch.trim().to_owned();
    for state in ["MERGE_HEAD", "REBASE_HEAD", "CHERRY_PICK_HEAD"] {
        if git(repo_root, &["rev-parse", "-q", "--verify", state]).is_ok() {
            return Err(format!(
                "{state} exists: a merge, rebase or cherry-pick is under way; finish or abort it first"
            ));
        }
    }
    let head = rev(repo_root, "HEAD")?;
    let listing = git(
        repo_root,
        &[
            "rev-list",
            "--reverse",
            "--topo-order",
            "HEAD",
            "--not",
            "--remotes",
        ],
    )?;
    let unpushed: Vec<&str> = listing.lines().filter(|line| !line.is_empty()).collect();
    refuse_pushed(repo_root, &unpushed)?;
    let refused: Vec<&str> = unpushed
        .iter()
        .copied()
        .filter(|commit| parents(repo_root, commit).is_ok_and(|p| p.len() == 2))
        .filter(|merge| !check_merge(repo_root, merge).is_empty())
        .collect();
    if refused.is_empty() {
        return Ok(vec!["no merge to mend".to_owned()]);
    }

    let mut made: HashMap<String, String> = HashMap::new();
    let mut report = Vec::new();
    let mut new_merges = Vec::new();
    for &commit in &unpushed {
        let old_parents = parents(repo_root, commit)?;
        let new_parents: Vec<String> = old_parents
            .iter()
            .map(|p| made.get(p).cloned().unwrap_or_else(|| p.clone()))
            .collect();
        let tree = rev(repo_root, &format!("{commit}^{{tree}}"))?;
        let info = Info::of(repo_root, commit)?;
        let short = &commit[..commit.len().min(10)];
        if refused.contains(&commit) {
            let remerged = git(
                repo_root,
                &[
                    "merge-tree",
                    "--write-tree",
                    "-X",
                    "ours",
                    &new_parents[0],
                    &new_parents[1],
                ],
            )
            .map_err(|_| {
                format!(
                    "merge {short}: git can't remerge its parents by content alone (a rename or \
                     delete conflict), so split it by hand: `git reset --hard {short}^1`, merge \
                     again, commit only the resolution, then commit the rest on its own"
                )
            })?;
            let clean = remerged.lines().next().unwrap_or_default().trim().to_owned();
            let merge = commit_tree(repo_root, &clean, &new_parents, &info, &info.message)?;
            new_merges.push(merge.clone());
            let split_message = format!(
                "Split out of merge {short}: what it changed beyond resolving its parents\n\n\
                 {}\n\n(`cargo xtask mend-merges`, ticket 3532)\n",
                info.message.lines().next().unwrap_or_default()
            );
            let split = commit_tree(repo_root, &tree, &[merge.clone()], &info, &split_message)?;
            report.push(format!(
                "merge {short}: split into the merge {} and {}, which holds its own changes",
                &merge[..10],
                &split[..10]
            ));
            made.insert(commit.to_owned(), split);
        } else if new_parents != old_parents {
            let again = commit_tree(repo_root, &tree, &new_parents, &info, &info.message)?;
            made.insert(commit.to_owned(), again);
        }
    }

    let new_head = made.get(&head).cloned().unwrap_or_else(|| head.clone());
    if rev(repo_root, &format!("{new_head}^{{tree}}"))? != rev(repo_root, "HEAD^{tree}")? {
        return Err("the mended branch's last tree differs from HEAD's; nothing changed".to_owned());
    }
    for merge in &new_merges {
        let errors = check_merge(repo_root, merge);
        if !errors.is_empty() {
            return Err(format!("the remade merge is still refused, so nothing changed: {}", errors.join("; ")));
        }
    }
    git(
        repo_root,
        &["update-ref", "-m", "mend-merges (ticket 3532)", &branch, &new_head, &head],
    )?;
    report.push(format!(
        "{} is now {}, with the same files; push it as usual",
        branch.trim_start_matches("refs/heads/"),
        &new_head[..10]
    ));
    Ok(report)
}

/// Fails if a merge `check-merges` refuses is on a remote already: then only
/// a force-push or a new branch can change it.
fn refuse_pushed(repo_root: &Path, unpushed: &[&str]) -> Result<(), String> {
    let Ok(listing) = git(repo_root, &["rev-list", "--merges", "origin/main..HEAD"]) else {
        return Ok(());
    };
    for merge in listing.lines().filter(|line| !line.is_empty()) {
        if !unpushed.contains(&merge) && !check_merge(repo_root, merge).is_empty() {
            return Err(format!(
                "merge {} is on a remote already, and its history can't change without a \
                 force-push: push the branch's work to a new branch instead",
                &merge[..merge.len().min(10)]
            ));
        }
    }
    Ok(())
}

/// A commit's parents.
fn parents(repo_root: &Path, commit: &str) -> Result<Vec<String>, String> {
    let line = git(repo_root, &["rev-list", "--parents", "-n", "1", commit])?;
    Ok(line.split_whitespace().skip(1).map(str::to_owned).collect())
}

/// The full id of `name`.
fn rev(repo_root: &Path, name: &str) -> Result<String, String> {
    Ok(git(repo_root, &["rev-parse", "--verify", name])?.trim().to_owned())
}

/// What a commit keeps when it is made again.
struct Info {
    author_name: String,
    author_email: String,
    author_date: String,
    message: String,
}

impl Info {
    fn of(repo_root: &Path, commit: &str) -> Result<Self, String> {
        let out = git(repo_root, &["log", "-1", "--format=%an%x00%ae%x00%aD%x00%B", commit])?;
        let mut parts = out.splitn(4, '\0');
        let mut next = || parts.next().unwrap_or_default().to_owned();
        Ok(Self {
            author_name: next(),
            author_email: next(),
            author_date: next(),
            message: next(),
        })
    }
}

/// `git commit-tree` of `tree` on `parents`, keeping the author.
fn commit_tree(
    repo_root: &Path,
    tree: &str,
    parents: &[String],
    info: &Info,
    message: &str,
) -> Result<String, String> {
    let mut command = Command::new("git");
    command.current_dir(repo_root).args(["commit-tree", tree]);
    for parent in parents {
        command.args(["-p", parent]);
    }
    let mut child = command
        .args(["-F", "-"])
        .env("GIT_AUTHOR_NAME", &info.author_name)
        .env("GIT_AUTHOR_EMAIL", &info.author_email)
        .env("GIT_AUTHOR_DATE", &info.author_date)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git doesn't run: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("no stdin for git commit-tree")?
        .write_all(message.as_bytes())
        .map_err(|e| format!("git commit-tree: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("git commit-tree: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(format!(
            "`git commit-tree` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{commit, diverged, numbered};
    use super::super::{git, run};
    use super::*;

    /// `side` merges `main` and, in the same commit, edits line 6 of `f`
    /// (outside any conflict) and adds `extra`; then one more commit.
    fn with_a_bad_merge(name: &str) -> std::path::PathBuf {
        let dir = diverged(name);
        git(&dir, &["merge", "-q", "--no-commit", "main"]).expect("merge");
        let mut lines = numbered(2, "2side").replace("12\n", "12main\n");
        lines = lines.replace("6\n", "6edited\n");
        std::fs::write(dir.join("f"), lines).expect("write");
        std::fs::write(dir.join("extra"), "one\n").expect("write");
        git(&dir, &["add", "."]).expect("add");
        git(&dir, &["commit", "-q", "--no-edit"]).expect("commit");
        commit(&dir, "later", "after\n", "a commit after the merge");
        dir
    }

    fn tree(dir: &Path, name: &str) -> String {
        rev(dir, &format!("{name}^{{tree}}")).expect("tree")
    }

    #[test]
    fn a_merge_with_edits_is_split_and_the_last_tree_kept() {
        let dir = with_a_bad_merge("mend-split");
        assert!(!run(&dir, "main", "HEAD").is_empty());
        let before = tree(&dir, "HEAD");
        let report = mend(&dir).expect("mend");
        assert_eq!(report.len(), 2, "{report:?}");
        assert_eq!(tree(&dir, "HEAD"), before);
        assert_eq!(run(&dir, "main", "HEAD"), Vec::<String>::new());
        let subjects = git(&dir, &["log", "--format=%s", "-3"]).expect("log");
        let subjects: Vec<&str> = subjects.lines().collect();
        assert_eq!(subjects[0], "a commit after the merge");
        assert!(subjects[1].starts_with("Split out of merge "), "{subjects:?}");
        assert!(subjects[2].starts_with("Merge branch 'main'"), "{subjects:?}");
        let status = git(&dir, &["status", "--porcelain"]).expect("status");
        assert_eq!(status, "");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_clean_history_is_left_alone() {
        let dir = diverged("mend-clean");
        git(&dir, &["merge", "-q", "--no-edit", "main"]).expect("merge");
        let head = rev(&dir, "HEAD").expect("head");
        assert_eq!(mend(&dir).expect("mend"), vec!["no merge to mend".to_owned()]);
        assert_eq!(rev(&dir, "HEAD").expect("head"), head);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pushed_bad_merge_is_refused_and_nothing_changes() {
        let dir = with_a_bad_merge("mend-pushed");
        git(&dir, &["update-ref", "refs/remotes/origin/main", "main"]).expect("ref");
        git(&dir, &["update-ref", "refs/remotes/origin/side", "HEAD^"]).expect("ref");
        let head = rev(&dir, "HEAD").expect("head");
        let error = mend(&dir).expect_err("refused");
        assert!(error.contains("on a remote already"), "{error}");
        assert_eq!(rev(&dir, "HEAD").expect("head"), head);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_delete_conflict_is_left_to_be_split_by_hand() {
        let dir = super::super::tests::repository("mend-delete");
        git(&dir, &["switch", "-q", "-c", "side"]).expect("switch");
        git(&dir, &["rm", "-q", "f"]).expect("rm");
        git(&dir, &["commit", "-q", "-m", "delete f"]).expect("commit");
        git(&dir, &["switch", "-q", "main"]).expect("switch");
        commit(&dir, "f", &numbered(3, "3main"), "main changes f");
        git(&dir, &["switch", "-q", "side"]).expect("switch");
        let _ = git(&dir, &["merge", "-q", "main"]);
        git(&dir, &["rm", "-q", "f"]).expect("resolve");
        std::fs::write(dir.join("extra"), "one\n").expect("write");
        git(&dir, &["add", "."]).expect("add");
        git(&dir, &["commit", "-q", "--no-edit"]).expect("commit");
        let head = rev(&dir, "HEAD").expect("head");
        let error = mend(&dir).expect_err("refused");
        assert!(error.contains("split it by hand"), "{error}");
        assert_eq!(rev(&dir, "HEAD").expect("head"), head);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
