//! `reveal`: the reveal kit, the evidence a game keeps before its first
//! public footage (ADR-0116). A gameplay idea shown in public can be copied
//! from the footage alone, without the game; the kit can't stop that. It
//! keeps what a person needs to show later what was theirs, and when: the
//! approved captures in whole (originals and each platform's re-encode), the
//! canonical store page, the game's name and keywords, the hashes of its
//! builds, dated receipts, and salted commitments to what isn't shown yet.
//!
//! - `reveal prepare` writes the kit from `shuyi.ron`'s `reveal` part
//!   ([`crate::project::Reveal`]): a folder whose manifest, `reveal.ron`,
//!   gives every file's SHA-256, sealed by the manifest's own SHA-256 in
//!   `reveal.ron.sha256` (the kit's digest), with `EVIDENCE.md`, an index to
//!   copy into a report of a clip, and `CHECKLIST.md`, the steps before the
//!   reveal.
//! - `reveal verify` checks a kit wherever it was copied, with no project.
//! - `reveal commit` makes a salted commitment to a file kept private, and
//!   `reveal open` shows a file and its salt open one.
//!
//! The kit proves a folder unchanged since its digest was written; a date
//! comes from where the digest was published (a pushed commit, a release's
//! attestation), never from the kit's own clock. It is a command of the CLI
//! only: it judges nothing, sends nothing and is not legal advice.

use std::path::{Path, PathBuf};

use crate::contract::codes::{self, Code, help_of};
use crate::project::Project;

mod commitment;
mod kit;
mod text;

pub use commitment::{Salts, open, opens};
pub use kit::{Build, prepare, verify};

/// The folder of the kit when `shuyi.ron`'s `reveal` part names none.
pub const DEFAULT_OUT: &str = "dist/reveal";

/// The usage line of `reveal`.
fn usage(command: &str) -> String {
    format!(
        "usage: {command} reveal prepare [--out <dir>]\n       \
         {command} reveal verify [<dir>] [--digest <sha256>]\n       \
         {command} reveal commit <file>... [--salts <file>]\n       \
         {command} reveal open <file> (--salts <file> | --salt <hex> --digest <sha256>)"
    )
}

/// `reveal --help`.
fn help(command: &str) -> String {
    format!(
        "\
{usage}

The reveal kit: the evidence a game keeps before its first public footage.

  prepare   write the kit from {file}'s `reveal` part into its `out` folder
            (`{out}` unless it says): the approved captures, the store page,
            the builds' hashes, the receipts and the commitments' digests,
            sealed by the manifest's SHA-256 (the kit's digest)
  verify    check a kit, anywhere, with no project: every file's SHA-256 and
            the seal; --digest: and that the seal is the digest published
            before (in a commit, a tag, a release's notes)
  commit    a salted commitment to each file, kept in the salts file (the
            `reveal` part's `salts`, or --salts): keep that file private
  open      whether a file opens a commitment: with the salts file, or with
            its salt and digest

The kit judges nothing and sends nothing. It is not legal advice.",
        usage = usage(command),
        file = crate::project::FILE,
        out = DEFAULT_OUT,
    )
}

/// What the arguments asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Asked {
    Help,
    Prepare {
        out: Option<String>,
    },
    Verify {
        dir: Option<String>,
        digest: Option<String>,
    },
    Commit {
        files: Vec<String>,
        salts: Option<String>,
    },
    Open {
        file: String,
        salts: Option<String>,
        salt: Option<String>,
        digest: Option<String>,
    },
}

/// The value after `flag`, or why there is none.
fn value(flag: &str, rest: &mut std::slice::Iter<'_, String>) -> Result<String, String> {
    rest.next()
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}

/// Reads `args`, after `reveal`.
fn parse(args: &[String]) -> Result<Asked, String> {
    let Some((sub, args)) = args.split_first() else {
        return Err("say what to do: prepare, verify, commit or open".into());
    };
    if matches!(sub.as_str(), "--help" | "-h" | "help") {
        return Ok(Asked::Help);
    }
    let mut flags: Vec<(String, String)> = Vec::new();
    let mut plain: Vec<String> = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Asked::Help),
            "--out" | "--digest" | "--salts" | "--salt" => {
                if flags.iter().any(|(flag, _)| flag == arg) {
                    return Err(format!("{arg} given twice"));
                }
                flags.push((arg.clone(), value(arg, &mut rest)?));
            }
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ => plain.push(arg.clone()),
        }
    }
    let take = |name: &str, flags: &mut Vec<(String, String)>| {
        flags
            .iter()
            .position(|(flag, _)| flag == name)
            .map(|at| flags.remove(at).1)
    };
    let asked = match sub.as_str() {
        "prepare" => {
            if !plain.is_empty() {
                return Err("prepare takes no file: it reads the `reveal` part".into());
            }
            Asked::Prepare {
                out: take("--out", &mut flags),
            }
        }
        "verify" => {
            if plain.len() > 1 {
                return Err("verify checks one folder".into());
            }
            Asked::Verify {
                dir: plain.pop(),
                digest: take("--digest", &mut flags),
            }
        }
        "commit" => {
            if plain.is_empty() {
                return Err("commit needs the files to commit to".into());
            }
            Asked::Commit {
                files: plain,
                salts: take("--salts", &mut flags),
            }
        }
        "open" => {
            let [file] =
                <[String; 1]>::try_from(plain).map_err(|_| "open checks one file".to_owned())?;
            let salts = take("--salts", &mut flags);
            let salt = take("--salt", &mut flags);
            let digest = take("--digest", &mut flags);
            if salts.is_some() && (salt.is_some() || digest.is_some()) {
                return Err("open takes --salts, or --salt and --digest, not both".into());
            }
            if salt.is_some() != digest.is_some() {
                return Err("--salt and --digest go together".into());
            }
            Asked::Open {
                file,
                salts,
                salt,
                digest,
            }
        }
        other => {
            return Err(format!(
                "no `reveal {other}`: prepare, verify, commit or open"
            ));
        }
    };
    if let Some((flag, _)) = flags.first() {
        return Err(format!("{sub} takes no {flag}"));
    }
    Ok(asked)
}

/// A failure, with its code.
#[derive(Debug)]
pub struct Failed {
    /// Its problem code.
    pub code: Code,
    /// What went wrong.
    pub message: String,
}

impl Failed {
    /// A failure `code` saying `message`.
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// The project found from the folder `reveal` runs in, or why there is
/// none.
type Found = Result<Project, (Code, String)>;

/// The `reveal` part of the project `found`, or why there is none.
fn part(found: &Found) -> Result<(&Project, &crate::project::Reveal), Failed> {
    let project = found
        .as_ref()
        .map_err(|(code, e)| Failed::new(*code, e.clone()))?;
    let reveal = project
        .part(&project.config.reveal, "reveal")
        .map_err(|e| Failed::new(codes::PROJECT_FILE, e))?;
    Ok((project, reveal))
}

/// The kit's folder of `project`, from `here`'s point of view.
fn kit_dir(project: &Project, reveal: &crate::project::Reveal) -> PathBuf {
    project
        .root
        .join(reveal.out.as_deref().unwrap_or(DEFAULT_OUT))
}

/// Runs `reveal` with `args` (after `reveal`) in the folder `here`, on the
/// project of the first folder from there up that holds a `shuyi.ron` if
/// there is one, and returns the exit code.
pub fn main(here: &Path, args: &[String]) -> u8 {
    let found = Project::find(here);
    let command = found
        .as_ref()
        .map_or("shuyi", |project| project.config.command());
    let asked = match parse(args) {
        Ok(asked) => asked,
        Err(e) => {
            eprint!(
                "{}",
                codes::said("reveal", codes::WRONG_ARGUMENTS.0, &e, None)
            );
            eprintln!("{}", usage(command));
            return 2;
        }
    };
    let ran = match asked {
        Asked::Help => {
            println!("{}", help(command));
            return 0;
        }
        Asked::Prepare { out } => run_prepare(&found, here, out.as_deref(), command),
        Asked::Verify { dir, digest } => {
            run_verify(&found, here, dir.as_deref(), digest.as_deref())
        }
        Asked::Commit { files, salts } => run_commit(&found, here, &files, salts.as_deref()),
        Asked::Open {
            file,
            salts,
            salt,
            digest,
        } => run_open(&found, here, &file, salts.as_deref(), salt.zip(digest)),
    };
    match ran {
        Ok(code) => code,
        Err(failed) => {
            eprint!(
                "{}",
                codes::said(
                    "reveal",
                    failed.code.0,
                    &failed.message,
                    help_of(failed.code.0)
                )
            );
            codes::exit_code(Some(failed.code.0))
        }
    }
}

fn run_prepare(found: &Found, here: &Path, out: Option<&str>, command: &str) -> Result<u8, Failed> {
    let (project, reveal) = part(found)?;
    let dir = out.map_or_else(|| kit_dir(project, reveal), |out| here.join(out));
    let build = Build::of(&project.root);
    let prepared = prepare(project, &dir, &kit::today(), &build)?;
    let manifest = &prepared.manifest;
    println!(
        "reveal: the kit of {} is in {}",
        manifest.game,
        dir.display()
    );
    println!("  {}", kit::counts(manifest));
    println!("  digest: {}", prepared.digest);
    println!(
        "  next: work through {}/{}, then publish the digest somewhere that dates it \
         (a pushed commit or tag); `{command} reveal verify {}` checks the kit anywhere",
        dir.display(),
        kit::CHECKLIST,
        dir.display(),
    );
    for note in &prepared.notes {
        println!("  note: {note}");
    }
    Ok(0)
}

fn run_verify(
    found: &Found,
    here: &Path,
    dir: Option<&str>,
    digest: Option<&str>,
) -> Result<u8, Failed> {
    let dir = match dir {
        Some(dir) => here.join(dir),
        None if here.join(kit::MANIFEST).is_file() => here.to_path_buf(),
        None => match part(found) {
            Ok((project, reveal)) => kit_dir(project, reveal),
            Err(_) => here.to_path_buf(),
        },
    };
    let verified = verify(&dir, digest);
    if verified.problems.is_empty() {
        let manifest = verified.manifest.as_ref();
        println!(
            "reveal: {} verifies{}",
            dir.display(),
            manifest.map_or(String::new(), |manifest| format!(
                ": {}, prepared {}, {}",
                manifest.game,
                manifest.prepared,
                kit::counts(manifest)
            ))
        );
        if let Some(digest) = &verified.digest {
            println!("  digest: {digest}");
        }
        return Ok(0);
    }
    println!("reveal: {} doesn't verify:", dir.display());
    for problem in &verified.problems {
        println!("  {problem}");
    }
    Err(Failed::new(
        codes::REVEAL_KIT,
        format!(
            "the kit doesn't verify ({} problem{})",
            verified.problems.len(),
            if verified.problems.len() == 1 {
                ""
            } else {
                "s"
            }
        ),
    ))
}

/// The salts file `salts` names from `here`, else the `reveal` part's.
fn salts_path(found: &Found, here: &Path, salts: Option<&str>) -> Result<PathBuf, Failed> {
    if let Some(salts) = salts {
        return Ok(here.join(salts));
    }
    let (project, reveal) = part(found)?;
    let salts = reveal.salts.as_deref().ok_or_else(|| {
        Failed::new(
            codes::PROJECT_FILE,
            format!(
                "{}'s `reveal` names no `salts` file: name one (kept private), or pass --salts",
                crate::project::FILE
            ),
        )
    })?;
    Ok(project.root.join(salts))
}

fn run_commit(
    found: &Found,
    here: &Path,
    files: &[String],
    salts: Option<&str>,
) -> Result<u8, Failed> {
    let path = salts_path(found, here, salts)?;
    let mut kept = Salts::load_or_new(&path)?;
    for file in files {
        let made = kept.commit(&here.join(file), &file.replace('\\', "/"))?;
        println!("reveal: committed to {file}: {}", made.sha256);
    }
    kept.save(&path)?;
    println!(
        "  the salts are in {}: keep it private, and keep it; without it no commitment opens",
        path.display()
    );
    Ok(0)
}

fn run_open(
    found: &Found,
    here: &Path,
    file: &str,
    salts: Option<&str>,
    pair: Option<(String, String)>,
) -> Result<u8, Failed> {
    let path = here.join(file);
    let opened = if let Some((salt, digest)) = pair {
        opens(&path, &salt, &digest)?.then_some(digest)
    } else {
        let salts = Salts::load(&salts_path(found, here, salts)?)?;
        open(&path, &salts)?.map(|salted| format!("{} ({})", salted.sha256, salted.label))
    };
    if let Some(which) = opened {
        println!("reveal: {file} opens the commitment {which}");
        Ok(0)
    } else {
        println!("reveal: {file} opens no commitment: its bytes or the salt differ");
        Ok(1)
    }
}

#[cfg(test)]
mod tests;
