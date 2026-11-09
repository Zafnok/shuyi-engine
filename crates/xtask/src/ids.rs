//! `cargo xtask next-id`: the next ticket id or ADR number that neither
//! `main` nor an open PR has taken, and the check that no two ADRs share a
//! number (ticket 2052).
//!
//! Parallel sessions each took "the next number" from their own checkout,
//! so two of them took the same one. `next-id` reads `origin/main`'s tree,
//! this checkout's files and every open PR's files (`gh pr list`), and
//! prints one past the highest it finds. A gap is never filled: a number
//! once given may still be linked from somewhere. Without `gh`, it warns
//! that it read `main` and this checkout only.
//!
//! `check-docs` runs [`duplicate_adrs`]: two files of `docs/adr/` with one
//! number, or the index listing a number twice, fail. CI runs it on the
//! merge ref, so the second PR to take a number fails before it merges.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// The folders whose file names hold ticket ids.
const TICKET_DIRS: &[&str] = &["tickets/open", "tickets/done"];
/// The folder of the ADRs, and its index.
const ADR_DIR: &str = "docs/adr";
const ADR_INDEX: &str = "docs/adr/README.md";

/// What `next-id` is asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A ticket id in a block of `tickets/README.md`: its first two digits.
    Ticket(String),
    Adr,
}

impl Kind {
    /// `ticket <block>` or `adr`, or `None` on anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        match args {
            [kind] if kind == "adr" => Some(Self::Adr),
            [kind, block]
                if kind == "ticket"
                    && block.len() == 2
                    && block.bytes().all(|b| b.is_ascii_digit()) =>
            {
                Some(Self::Ticket(block.clone()))
            }
            _ => None,
        }
    }
}

/// The next number for `kind`, and the warnings when a source could not be
/// read.
pub fn run(root: &Path, kind: &Kind) -> Result<(String, Vec<String>), Vec<String>> {
    let (paths, warnings) = taken_paths(root);
    let next = match kind {
        Kind::Ticket(block) => next_ticket_id(block, &paths),
        Kind::Adr => next_adr_number(&paths),
    };
    next.map(|next| (next, warnings)).map_err(|e| vec![e])
}

/// The next ticket id in `block` (two digits) after every one among
/// `paths` (repository paths; only those in [`TICKET_DIRS`] count), or the
/// block's first, `NN01`, when it has none.
pub fn next_ticket_id(block: &str, paths: &[String]) -> Result<String, String> {
    let highest = paths
        .iter()
        .filter_map(|path| file_in(path, TICKET_DIRS))
        .filter_map(leading_number)
        .filter(|id| format!("{id:04}").starts_with(block))
        .max();
    let next = highest.map_or_else(|| format!("{block}01"), |id| format!("{:04}", id + 1));
    if next.starts_with(block) {
        Ok(next)
    } else {
        Err(format!(
            "block {block}xx is full; pick another block (tickets/README.md)"
        ))
    }
}

/// The next ADR number after every one among `paths`.
pub fn next_adr_number(paths: &[String]) -> Result<String, String> {
    let highest = paths
        .iter()
        .filter_map(|path| file_in(path, &[ADR_DIR]))
        .filter_map(leading_number)
        .max()
        .unwrap_or(0);
    if highest >= 9999 {
        return Err("no ADR number is left after 9999".to_string());
    }
    Ok(format!("{:04}", highest + 1))
}

/// The file name of `path` when its folder is one of `dirs`.
fn file_in<'a>(path: &'a str, dirs: &[&str]) -> Option<&'a str> {
    let (dir, name) = path.rsplit_once('/')?;
    dirs.contains(&dir).then_some(name)
}

/// The number of a file named `NNNN-<slug>.md`.
fn leading_number(name: &str) -> Option<u32> {
    let digits = name.get(..4)?;
    let rest = name.get(4..)?;
    let markdown = Path::new(rest)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
    (digits.bytes().all(|b| b.is_ascii_digit()) && rest.starts_with('-') && markdown)
        .then(|| digits.parse().ok())
        .flatten()
}

/// Every taken path: `origin/main`'s, this checkout's and the open PRs'.
fn taken_paths(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut paths = Vec::new();
    let mut warnings = Vec::new();
    match main_paths(root) {
        Ok(found) => paths.extend(found),
        Err(e) => warnings.push(e),
    }
    paths.extend(local_paths(root));
    match open_pr_paths(root) {
        Ok(found) => paths.extend(found),
        Err(e) => warnings.push(format!(
            "{e}; this reads `origin/main` and this checkout only, so a number an open PR \
             has taken may be printed"
        )),
    }
    (paths, warnings)
}

fn main_paths(root: &Path) -> Result<Vec<String>, String> {
    let mut args = vec!["ls-tree", "--name-only", "origin/main", "--"];
    let dirs: Vec<String> = TICKET_DIRS
        .iter()
        .chain(&[ADR_DIR])
        .map(|dir| format!("{dir}/"))
        .collect();
    args.extend(dirs.iter().map(String::as_str));
    let output = Command::new("git")
        .args(&args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("`git` did not run ({e})"))?;
    if !output.status.success() {
        return Err(format!(
            "`git ls-tree origin/main` failed ({}); run `git fetch origin` first",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

fn local_paths(root: &Path) -> Vec<String> {
    TICKET_DIRS
        .iter()
        .chain(&[ADR_DIR])
        .flat_map(|dir| {
            fs::read_dir(root.join(dir))
                .into_iter()
                .flatten()
                .flatten()
                .map(move |entry| format!("{dir}/{}", entry.file_name().to_string_lossy()))
        })
        .collect()
}

fn open_pr_paths(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("gh")
        .args([
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            "1000",
            "--json",
            "number,files",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| format!("`gh` is not available ({e})"))?;
    if !output.status.success() {
        return Err(format!(
            "`gh pr list` failed ({})",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    pr_paths(&String::from_utf8_lossy(&output.stdout))
}

/// The paths of every PR's files in `gh pr list --json number,files`'s
/// answer.
fn pr_paths(json: &str) -> Result<Vec<String>, String> {
    let prs: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("`gh pr list` answered no JSON ({e})"))?;
    Ok(prs
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|pr| pr.get("files")?.as_array())
        .flatten()
        .filter_map(|file| file.get("path")?.as_str())
        .map(str::to_string)
        .collect())
}

/// An error for each ADR number two files of `docs/adr/` share, and for
/// each the index lists twice.
pub fn duplicate_adrs(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root.join(ADR_DIR)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let index = fs::read_to_string(root.join(ADR_INDEX)).unwrap_or_default();
    check_adr_numbers(&names, &index)
}

/// [`duplicate_adrs`] over the file names of `docs/adr/` and its index's
/// text.
fn check_adr_numbers(names: &[String], index: &str) -> Vec<String> {
    let mut files: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
    for name in names {
        if let Some(number) = leading_number(name) {
            files.entry(number).or_default().push(name);
        }
    }
    let mut errors: Vec<String> = files
        .iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|(number, names)| {
            format!(
                "{ADR_DIR}: ADR-{number:04} is taken by {}; renumber the newer one \
                 (`cargo xtask next-id adr`) and its links",
                names.join(" and ")
            )
        })
        .collect();
    let mut rows: BTreeMap<u32, usize> = BTreeMap::new();
    for number in index.lines().filter_map(index_row) {
        *rows.entry(number).or_default() += 1;
    }
    errors.extend(
        rows.iter()
            .filter(|(_, count)| **count > 1)
            .map(|(number, count)| {
                format!("{ADR_INDEX}: ADR-{number:04} has {count} rows; keep one")
            }),
    );
    errors
}

/// The number of an index row, `| [NNNN](NNNN-<slug>.md) | …`.
fn index_row(line: &str) -> Option<u32> {
    let rest = line.trim_start().strip_prefix("| [")?;
    let digits = rest.get(..4)?;
    (rest.get(4..)?.starts_with("](") && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(list: &[&str]) -> Vec<String> {
        list.iter().map(|p| (*p).to_string()).collect()
    }

    #[test]
    fn parses_the_kinds() {
        let args = |list: &[&str]| paths(list);
        assert_eq!(Kind::parse(&args(&["adr"])), Some(Kind::Adr));
        assert_eq!(
            Kind::parse(&args(&["ticket", "20"])),
            Some(Kind::Ticket("20".into()))
        );
        assert_eq!(Kind::parse(&args(&["ticket", "2"])), None);
        assert_eq!(Kind::parse(&args(&["ticket", "2x"])), None);
        assert_eq!(Kind::parse(&args(&["ticket"])), None);
        assert_eq!(Kind::parse(&args(&["adr", "20"])), None);
    }

    #[test]
    fn the_next_ticket_is_one_past_the_highest_in_its_block() {
        let found = paths(&[
            "tickets/done/2001-a.md",
            "tickets/open/2007-b.md",
            "tickets/done/2101-c.md",
            "tickets/open/1150-d.md",
        ]);
        assert_eq!(next_ticket_id("20", &found), Ok("2008".into()));
        assert_eq!(next_ticket_id("21", &found), Ok("2102".into()));
        assert_eq!(next_ticket_id("11", &found), Ok("1151".into()));
    }

    #[test]
    fn a_gap_is_never_filled() {
        let found = paths(&["tickets/done/2001-a.md", "tickets/done/2050-b.md"]);
        assert_eq!(next_ticket_id("20", &found), Ok("2051".into()));
    }

    #[test]
    fn an_open_prs_ticket_is_skipped() {
        let mut found = paths(&["tickets/open/2052-a.md"]);
        found.extend(
            pr_paths(r#"[{"number":1,"files":[{"path":"tickets/open/2053-b.md"}]}]"#).unwrap(),
        );
        assert_eq!(next_ticket_id("20", &found), Ok("2054".into()));
    }

    #[test]
    fn an_empty_block_starts_at_01_and_a_full_one_fails() {
        assert_eq!(next_ticket_id("26", &[]), Ok("2601".into()));
        let full = paths(&["tickets/done/2699-z.md"]);
        assert!(next_ticket_id("26", &full).is_err());
    }

    #[test]
    fn only_ticket_folders_and_ticket_names_count() {
        let found = paths(&[
            "docs/adr/2090-x.md",
            "tickets/TEMPLATE.md",
            "tickets/open/20-x.md",
            "tickets/open/2003-a.txt",
            "tickets/open/2002-a.md",
        ]);
        assert_eq!(next_ticket_id("20", &found), Ok("2003".into()));
    }

    #[test]
    fn the_next_adr_is_one_past_the_highest() {
        let found = paths(&[
            "docs/adr/0000-template.md",
            "docs/adr/0089-a.md",
            "docs/adr/README.md",
            "docs/adr/0042-b.md",
            "tickets/open/2052-c.md",
        ]);
        assert_eq!(next_adr_number(&found), Ok("0090".into()));
        assert_eq!(next_adr_number(&[]), Ok("0001".into()));
    }

    #[test]
    fn an_open_prs_adr_is_skipped() {
        let mut found = paths(&["docs/adr/0089-a.md"]);
        found.extend(
            pr_paths(
                r#"[{"number":7,"files":[{"path":"crates/x.rs"},{"path":"docs/adr/0090-b.md"}]},
                    {"number":8,"files":[]}]"#,
            )
            .unwrap(),
        );
        assert_eq!(next_adr_number(&found), Ok("0091".into()));
    }

    #[test]
    fn gh_answering_no_json_is_an_error() {
        assert!(pr_paths("not json").is_err());
    }

    #[test]
    fn two_adrs_with_one_number_are_named() {
        let names = paths(&["0089-x.md", "0090-a.md", "0090-b.md", "README.md"]);
        assert_eq!(
            check_adr_numbers(&names, ""),
            vec![
                "docs/adr: ADR-0090 is taken by 0090-a.md and 0090-b.md; renumber the newer one \
                 (`cargo xtask next-id adr`) and its links"
                    .to_string()
            ]
        );
    }

    #[test]
    fn an_index_listing_a_number_twice_fails() {
        let index = "| ADR | Title | Status |\n|---|---|---|\n\
                     | [0089](0089-x.md) | X | Accepted |\n\
                     | [0090](0090-a.md) | A | Accepted |\n\
                     | [0090](0090-b.md) | B | Accepted |\n\
                     Text naming [0090](0090-a.md) again.\n";
        assert_eq!(
            check_adr_numbers(&[], index),
            vec!["docs/adr/README.md: ADR-0090 has 2 rows; keep one".to_string()]
        );
    }

    #[test]
    fn duplicates_are_found_in_a_temporary_adr_folder() {
        let root = std::env::temp_dir().join(format!("xtask-ids-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(ADR_DIR)).unwrap();
        for name in ["0090-a.md", "0090-b.md", "0091-c.md"] {
            fs::write(root.join(ADR_DIR).join(name), "# X\n").unwrap();
        }
        let errors = duplicate_adrs(&root);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("0090-a.md and 0090-b.md"), "{errors:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn real_repo_has_no_duplicate_adr() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask");
        let errors = duplicate_adrs(repo_root);
        assert!(errors.is_empty(), "{errors:#?}");
    }
}
