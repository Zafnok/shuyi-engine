//! `cargo xtask tutorial-tries [--check] [<name>...]`: the tutorial's
//! pictures (ticket 2818), each from `site/tutorial/tries.toml`.
//!
//! A try names a step of the tutorial and the keys to press. With no edits
//! it is one picture, what the reader should see. With edits it is two: the
//! step as it is, and the step with each edit made, built and run. The edits
//! are the ones a page tells the reader to make, so an edit that no longer
//! applies or builds fails here, and a picture can't go stale.
//!
//! The pictures are checked in, under `site/static/tutorial-tries/<name>/`,
//! so a change to one shows in its pull request. `--check` renders them
//! again and compares the bytes: the Harness's frames are exact, the same
//! on every OS.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

#[cfg(test)]
mod tests;

/// The list of tries, from the repository's root.
const TRIES: &str = "site/tutorial/tries.toml";
/// The steps' projects, from the repository's root.
const STEPS: &str = "site/tutorial";
/// The pictures, from the repository's root.
const PICTURES: &str = "site/static/tutorial-tries";
/// The pages that show them, from the repository's root.
const PAGES: &str = "site/content/learn/tutorial";
/// Where the copies are built: three folders down, like a step, so a step's
/// path dependencies on the engine's crates still resolve.
const COPIES: &str = "target/tutorial-tries";
/// What a copy leaves out of a step.
const LEFT_OUT: [&str; 2] = ["target", "dist"];
/// The picture of the step as it is, and the one of the step edited.
pub const BEFORE: &str = "before.png";
/// See [`BEFORE`].
pub const AFTER: &str = "after.png";

/// A picture, or a before and an after.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Try {
    /// The step's folder under `site/tutorial/`.
    pub step: String,
    /// The keys to press, as `frame-png --keys` takes them.
    #[serde(default)]
    pub keys: String,
    /// What the first picture shows, for a reader who can't see it.
    pub alt: String,
    /// What the second picture shows: only with edits.
    #[serde(default)]
    pub after: Option<String>,
    /// What to change for the second picture.
    #[serde(default)]
    pub edits: Vec<Edit>,
}

/// One change to one file of a step.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edit {
    /// The file, from the step's folder.
    pub file: String,
    /// The text to find, which must be there exactly once.
    pub find: String,
    /// What takes its place.
    pub replace: String,
}

/// `cargo xtask tutorial-tries`'s options.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// `--check`: compare with the pictures kept, and write none.
    pub check: bool,
    /// Only these tries; every one when empty.
    pub names: Vec<String>,
}

impl Options {
    /// The options in `args`, or `None` on a flag it doesn't know.
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut options = Self::default();
        for arg in args {
            match arg.as_str() {
                "--check" => options.check = true,
                flag if flag.starts_with('-') => return None,
                name => options.names.push(name.to_owned()),
            }
        }
        Some(options)
    }
}

/// The tries in `text`, the list's file, by name; every problem with them.
pub fn parse(text: &str) -> Result<BTreeMap<String, Try>, Vec<String>> {
    let tries: BTreeMap<String, Try> =
        toml::from_str(text).map_err(|e| vec![format!("{TRIES}: {e}")])?;
    let mut errors = Vec::new();
    for (name, one) in &tries {
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            errors.push(format!(
                "{TRIES}: `{name}`: a name is lowercase letters, digits and `-`"
            ));
        }
        if one.edits.is_empty() == one.after.is_some() {
            errors.push(format!(
                "{TRIES}: `{name}`: `after` says what its edits show, so it needs edits, and edits need it"
            ));
        }
    }
    if errors.is_empty() {
        Ok(tries)
    } else {
        Err(errors)
    }
}

/// `text` with `edit` made, its line endings read as `\n`; an error unless
/// the text to find is there exactly once.
pub fn apply(text: &str, edit: &Edit) -> Result<String, String> {
    let text = text.replace("\r\n", "\n");
    match text.matches(&edit.find).count() {
        1 => Ok(text.replacen(&edit.find, &edit.replace, 1)),
        count => Err(format!(
            "{}: `{}` is there {count} times, and an edit needs it exactly once",
            edit.file, edit.find
        )),
    }
}

/// Every try no page shows with the `try_it` shortcode, given the pages'
/// text.
pub fn unused<'a>(tries: &'a BTreeMap<String, Try>, pages: &str) -> Vec<&'a str> {
    tries
        .keys()
        .filter(|name| !pages.contains(&format!("try_it(name=\"{name}\")")))
        .map(String::as_str)
        .collect()
}

/// The files `one`, called `name`, makes: their names under its folder.
pub fn pictures(one: &Try) -> &'static [&'static str] {
    if one.edits.is_empty() {
        &[BEFORE]
    } else {
        &[BEFORE, AFTER]
    }
}

/// What is wrong with the picture kept, given what it should be.
pub fn compare(name: &str, file: &str, kept: Option<&[u8]>, made: &[u8]) -> Option<String> {
    let remake = format!("cargo xtask tutorial-tries {name}");
    match kept {
        None => Some(format!(
            "{PICTURES}/{name}/{file} is missing: run `{remake}`"
        )),
        Some(kept) if kept != made => Some(format!(
            "{PICTURES}/{name}/{file} is stale, what the game draws has changed: run `{remake}`, and look at it"
        )),
        Some(_) => None,
    }
}

/// Every folder in `found` that no try names.
pub fn orphans<'a>(tries: &BTreeMap<String, Try>, found: &'a [String]) -> Vec<&'a str> {
    found
        .iter()
        .filter(|name| !tries.contains_key(*name))
        .map(String::as_str)
        .collect()
}

/// Reads the list, renders what `options` asks and writes or compares it;
/// every problem found.
pub fn run(root: &Path, options: &Options) -> Vec<String> {
    let text = match fs::read_to_string(root.join(TRIES)) {
        Ok(text) => text,
        Err(e) => return vec![format!("{TRIES}: {e}")],
    };
    let tries = match parse(&text) {
        Ok(tries) => tries,
        Err(errors) => return errors,
    };
    let mut errors = Vec::new();
    for name in &options.names {
        if !tries.contains_key(name) {
            errors.push(format!("{TRIES}: no try `{name}`"));
        }
    }
    let every = options.names.is_empty();
    if every {
        errors.extend(whole_list_problems(root, &tries));
    }
    let chosen: BTreeMap<&String, &Try> = tries
        .iter()
        .filter(|(name, _)| every || options.names.contains(name))
        .collect();
    let mut steps = BTreeSet::new();
    for (name, one) in &chosen {
        let step = root.join(STEPS).join(&one.step);
        if !step.is_dir() {
            errors.push(format!("{TRIES}: `{name}`: no step {STEPS}/{}", one.step));
            continue;
        }
        steps.insert(one.step.as_str());
        errors.extend(edit_problems(&step, one));
    }
    if !errors.is_empty() {
        return errors;
    }
    let renderer = Renderer::new(root);
    for step in steps {
        if let Err(e) = renderer.prepare(&format!("step-{step}"), step, &[]) {
            return vec![e];
        }
    }
    for (name, one) in chosen {
        match render(&renderer, name, one) {
            Ok(made) => errors.extend(keep(root, name, options.check, &made)),
            Err(e) => errors.push(e),
        }
    }
    errors
}

/// What only the whole list can show: an orphan, and a try no page shows.
fn whole_list_problems(root: &Path, tries: &BTreeMap<String, Try>) -> Vec<String> {
    let mut errors = Vec::new();
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join(PICTURES)) {
        for entry in entries.flatten() {
            found.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    for orphan in orphans(tries, &found) {
        errors.push(format!(
            "{PICTURES}/{orphan}: {TRIES} names no such try: delete the folder"
        ));
    }
    let mut pages = String::new();
    if let Ok(entries) = fs::read_dir(root.join(PAGES)) {
        for entry in entries.flatten() {
            pages.push_str(&fs::read_to_string(entry.path()).unwrap_or_default());
        }
    }
    for name in unused(tries, &pages) {
        errors.push(format!(
            "{TRIES}: `{name}`: no page of {PAGES} shows it with `try_it(name=\"{name}\")`"
        ));
    }
    errors
}

/// Each edit of `one` that doesn't apply to the step at `step`.
fn edit_problems(step: &Path, one: &Try) -> Vec<String> {
    let mut errors = Vec::new();
    for edit in &one.edits {
        let shown = format!("{}/{}", one.step, edit.file);
        match fs::read_to_string(step.join(&edit.file)) {
            Ok(text) => {
                if let Err(e) = apply(&text, edit) {
                    errors.push(format!("{STEPS}/{}: {e}", one.step));
                }
            }
            Err(e) => errors.push(format!("{STEPS}/{shown}: {e}")),
        }
    }
    errors
}

/// The pictures of `one`, as bytes, by file name.
fn render(
    renderer: &Renderer,
    name: &str,
    one: &Try,
) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
    let before = renderer.frame(&format!("step-{}", one.step), name, BEFORE, &one.keys)?;
    let mut made = vec![(BEFORE, before)];
    if !one.edits.is_empty() {
        let copy = format!("try-{name}");
        renderer.prepare(&copy, &one.step, &one.edits)?;
        made.push((AFTER, renderer.frame(&copy, name, AFTER, &one.keys)?));
    }
    Ok(made)
}

/// Writes the pictures, or with `check` compares them with those kept.
fn keep(root: &Path, name: &str, check: bool, made: &[(&str, Vec<u8>)]) -> Vec<String> {
    let folder = root.join(PICTURES).join(name);
    let mut errors = Vec::new();
    for (file, bytes) in made {
        let path = folder.join(file);
        if check {
            let kept = fs::read(&path).ok();
            errors.extend(compare(name, file, kept.as_deref(), bytes));
        } else if let Err(e) = fs::create_dir_all(&folder).and_then(|()| fs::write(&path, bytes)) {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    errors
}

/// Builds copies of the steps and asks them for frames.
struct Renderer {
    root: PathBuf,
    /// One target folder, so the engine's crates build once.
    target_dir: PathBuf,
}

impl Renderer {
    fn new(root: &Path) -> Self {
        let target_dir = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| root.join("target/tutorial"), PathBuf::from);
        Self {
            root: root.to_owned(),
            target_dir,
        }
    }

    fn copy_dir(&self, copy: &str) -> PathBuf {
        self.root.join(COPIES).join(copy)
    }

    /// Makes the copy `copy` of `step`, with `edits` made, and moves its
    /// lock to the engine's crates, as CI's `tutorial` job does.
    fn prepare(&self, copy: &str, step: &str, edits: &[Edit]) -> Result<(), String> {
        let to = self.copy_dir(copy);
        if to.exists() {
            fs::remove_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()))?;
        }
        copy_tree(&self.root.join(STEPS).join(step), &to)
            .map_err(|e| format!("copying {step}: {e}"))?;
        for edit in edits {
            let path = to.join(&edit.file);
            let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let edited = apply(&text, edit).map_err(|e| format!("{step}: {e}"))?;
            fs::write(&path, edited).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        self.cargo(
            copy,
            &[
                "update",
                "-q",
                "-p",
                "shuyi",
                "-p",
                "shuyi-cli",
                "-p",
                "shuyi-app",
            ],
        )
    }

    /// The frame of the copy `copy` after `keys`, as PNG bytes.
    fn frame(&self, copy: &str, name: &str, file: &str, keys: &str) -> Result<Vec<u8>, String> {
        eprintln!("tutorial-tries: {name}/{file}");
        let out = self.copy_dir(copy).join(format!("{name}-{file}"));
        let out_text = out.to_string_lossy().into_owned();
        let mut args = vec!["shuyi", "frame-png", out_text.as_str(), "--scale", "2"];
        if !keys.is_empty() {
            args.extend(["--keys", keys]);
        }
        self.cargo(copy, &args)
            .map_err(|e| format!("try `{name}`, {file}: {e}"))?;
        fs::read(&out).map_err(|e| format!("{}: {e}", out.display()))
    }

    fn cargo(&self, copy: &str, args: &[&str]) -> Result<(), String> {
        let output = Command::new("cargo")
            .args(args)
            .current_dir(self.copy_dir(copy))
            .env("CARGO_TARGET_DIR", &self.target_dir)
            .output()
            .map_err(|e| format!("cargo did not run: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "`cargo {}` failed:\n{}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

/// Copies the folder `from` to `to`, but for [`LEFT_OUT`].
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if LEFT_OUT.iter().any(|left| name == *left) {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
