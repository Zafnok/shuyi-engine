//! `cargo xtask check-devcontainers`: the book's dev container keeps in step
//! with the book's project and with the root's container (ticket 2837,
//! ADR-0126 §1).
//!
//! It reads the root's `.devcontainer/devcontainer.json`, the book's
//! `.devcontainer/book/devcontainer.json` (both JSON with `//` and `/* */`
//! comments, as dev containers allow) and the steps of `site/tutorial/` (each
//! folder there with a `shuyi.ron`), and fails when:
//!
//! 1. a file is missing, or isn't JSON with comments;
//! 2. the book's `image` isn't the root's;
//! 3. the book's `workspaceFolder` isn't `site/tutorial` in the repository's
//!    folder (`/workspaces/<repository>/site/tutorial`);
//! 4. the book's `onCreateCommand` doesn't start with the root's setup: the
//!    parts of the root's `postCreateCommand` (split at `&&`) up to the one
//!    that reads cargo's environment (`. "$HOME/.cargo/env"`), with each
//!    script a part runs (`bash <script>`) read from its own container's
//!    folder, so `bash scripts/setup-linux.sh` at the root is `bash
//!    ../../scripts/setup-linux.sh` in the book's;
//! 5. the book's `remoteEnv` isn't the root's;
//! 6. its builds (`bash <…>/.devcontainer/book/build.sh <step>…`, in the
//!    `onCreateCommand` and the `updateContentCommand`, the commands a
//!    prebuild runs) name a step that isn't there, miss one, or name one
//!    twice;
//! 7. two steps resolve a crate they build with different features, for
//!    the Codespace's own target or the web's ([`features`], ticket 2839):
//!    the Codespace's one target folder would build it twice.
//!
//! [`check`] is pure over the two files' text and the steps; [`run`] feeds
//! it the repository's; [`run_features`] runs [`features`] over the steps.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

mod features;

/// The root's dev container, which the quick start's Codespace opens.
pub const ROOT: &str = ".devcontainer/devcontainer.json";
/// The book's, which each step's page links.
pub const BOOK: &str = ".devcontainer/book/devcontainer.json";
/// The book's project: a folder of steps.
pub const TUTORIAL: &str = "site/tutorial";
/// The script that builds the steps, from the repository's root.
const BUILD: &str = ".devcontainer/book/build.sh";
/// Where a Codespace (and a local dev container) puts the repository.
const WORKSPACES: &str = "/workspaces/";

/// Checks the repository at `repo_root` and returns every error (empty
/// means they agree).
pub fn run(repo_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let mut read = |file: &str| match fs::read_to_string(repo_root.join(file)) {
        Ok(text) => Some(text),
        Err(e) => {
            errors.push(format!("{file}: {e}"));
            None
        }
    };
    let (root, book) = (read(ROOT), read(BOOK));
    let steps = match steps(&repo_root.join(TUTORIAL)) {
        Ok(steps) => steps,
        Err(e) => {
            errors.push(format!("{TUTORIAL}: {e}"));
            return errors;
        }
    };
    if let (Some(root), Some(book)) = (root, book) {
        errors.extend(check(&root, &book, &steps));
    }
    errors
}

/// Checks that the steps of the repository at `repo_root` resolve one set
/// of features ([`features`]), and returns every error.
pub fn run_features(repo_root: &Path) -> Vec<String> {
    let tutorial = repo_root.join(TUTORIAL);
    match steps(&tutorial) {
        Ok(steps) => features::run(&tutorial, &steps),
        Err(e) => vec![format!("{TUTORIAL}: {e}")],
    }
}

/// The steps in `tutorial`: each folder with a `shuyi.ron`, by name.
fn steps(tutorial: &Path) -> std::io::Result<Vec<String>> {
    let mut steps = Vec::new();
    for entry in fs::read_dir(tutorial)? {
        let entry = entry?;
        if entry.path().join("shuyi.ron").is_file() {
            steps.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    steps.sort();
    Ok(steps)
}

/// What is wrong with the book's container (`book`, its text) against the
/// root's (`root`) and the project's `steps`, in order.
pub fn check(root: &str, book: &str, steps: &[String]) -> Vec<String> {
    let (root, book) = match (parse(ROOT, root), parse(BOOK, book)) {
        (Ok(root), Ok(book)) => (root, book),
        (root, book) => return [root.err(), book.err()].into_iter().flatten().collect(),
    };
    let mut errors = Vec::new();
    if book.get("image") != root.get("image") {
        errors.push(format!(
            "{BOOK}: its `image` is {}, and the root's ({ROOT}) is {}: they are kept the same",
            shown(book.get("image")),
            shown(root.get("image"))
        ));
    }
    if book.get("remoteEnv") != root.get("remoteEnv") {
        errors.push(format!(
            "{BOOK}: its `remoteEnv` is {}, and the root's ({ROOT}) is {}: they are kept the same",
            shown(book.get("remoteEnv")),
            shown(root.get("remoteEnv"))
        ));
    }
    let book_folder = match folder(&book) {
        Ok(folder) if folder == TUTORIAL => Some(folder),
        Ok(folder) => {
            errors.push(format!(
                "{BOOK}: its `workspaceFolder` opens `{folder}`, not the book's project, `{TUTORIAL}`"
            ));
            None
        }
        Err(e) => {
            errors.push(format!("{BOOK}: {e}"));
            None
        }
    };
    let root_folder = folder(&root).unwrap_or_default();
    let create = book.get("onCreateCommand").and_then(Value::as_str);
    let Some(create) = create else {
        errors.push(format!(
            "{BOOK}: no `onCreateCommand` (a string): it sets up and builds the steps, and a prebuild runs it"
        ));
        return errors;
    };
    let Some(book_folder) = book_folder else {
        return errors;
    };
    match setup(&root, &root_folder) {
        Ok(setup) => {
            let book_parts: Vec<String> = parts(create)
                .iter()
                .map(|part| normalize(&book_folder, part))
                .collect();
            if !book_parts.starts_with(&setup) {
                errors.push(format!(
                    "{BOOK}: its `onCreateCommand` doesn't start with the root's setup ({ROOT}): `{}`, \
                     each script from the repository's root",
                    setup.join(" && ")
                ));
            }
        }
        Err(e) => errors.push(format!("{ROOT}: {e}")),
    }
    let update = book.get("updateContentCommand").and_then(Value::as_str);
    let commands: Vec<&str> = [Some(create), update].into_iter().flatten().collect();
    errors.extend(check_steps(&commands, &book_folder, steps));
    errors
}

/// The steps the `commands` build, against `steps`.
fn check_steps(commands: &[&str], folder: &str, steps: &[String]) -> Vec<String> {
    let mut built: Vec<String> = Vec::new();
    for part in commands.iter().flat_map(|command| parts(command)) {
        let mut words = part.split_whitespace();
        if words.next() == Some("bash")
            && words
                .next()
                .and_then(|script| resolve(folder, script))
                .as_deref()
                == Some(BUILD)
        {
            built.extend(words.map(str::to_owned));
        }
    }
    if built.is_empty() {
        return vec![format!(
            "{BOOK}: neither its `onCreateCommand` nor its `updateContentCommand` runs `bash <…>/{BUILD} <step>…`: \
             they build every step of {TUTORIAL}"
        )];
    }
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    for step in &built {
        if !seen.insert(step.as_str()) {
            errors.push(format!("{BOOK}: it builds step `{step}` twice"));
        }
        if !steps.iter().any(|s| s == step) {
            errors.push(format!(
                "{BOOK}: it builds step `{step}`, which {TUTORIAL} doesn't have (a step is a folder with a `shuyi.ron`)"
            ));
        }
    }
    for step in steps {
        if !seen.contains(step.as_str()) {
            errors.push(format!(
                "{BOOK}: it doesn't build step `{step}` of {TUTORIAL}: add it to `build.sh`'s steps"
            ));
        }
    }
    errors
}

/// The root's setup: its `postCreateCommand`'s parts up to the one that
/// reads cargo's environment, each normalized from `folder`.
fn setup(root: &Value, folder: &str) -> Result<Vec<String>, String> {
    let command = root
        .get("postCreateCommand")
        .and_then(Value::as_str)
        .ok_or("no `postCreateCommand` (a string): it runs the setup")?;
    let parts = parts(command);
    let end = parts
        .iter()
        .position(|part| part.starts_with(". ") && part.contains(".cargo/env"))
        .ok_or("its `postCreateCommand` never reads cargo's environment (`. \"$HOME/.cargo/env\"`), where its setup ends")?;
    Ok(parts[..=end]
        .iter()
        .map(|part| normalize(folder, part))
        .collect())
}

/// A container's folder in the repository, from its `workspaceFolder`
/// (`/workspaces/<repository>/<folder>`); the root when it has none.
fn folder(container: &Value) -> Result<String, String> {
    let Some(value) = container.get("workspaceFolder") else {
        return Ok(String::new());
    };
    let path = value
        .as_str()
        .ok_or("its `workspaceFolder` isn't a string")?;
    let inside = path
        .strip_prefix(WORKSPACES)
        .and_then(|rest| rest.split_once('/').map(|(_, inside)| inside).or(Some("")))
        .ok_or_else(|| {
            format!("its `workspaceFolder`, `{path}`, isn't in the repository's folder, {WORKSPACES}<repository>")
        })?;
    Ok(inside.trim_end_matches('/').to_owned())
}

/// A command's parts, split at `&&` and trimmed.
fn parts(command: &str) -> Vec<String> {
    command
        .split("&&")
        .map(|part| part.trim().to_owned())
        .collect()
}

/// `part` with the script it runs (`bash <script> …`) read from `folder`,
/// so the same script reads the same from any container.
fn normalize(folder: &str, part: &str) -> String {
    let mut words = part.split_whitespace();
    match (words.next(), words.next()) {
        (Some("bash"), Some(script)) => {
            let script = resolve(folder, script).unwrap_or_else(|| script.to_owned());
            std::iter::once("bash".to_owned())
                .chain(std::iter::once(script))
                .chain(words.map(str::to_owned))
                .collect::<Vec<_>>()
                .join(" ")
        }
        _ => part.split_whitespace().collect::<Vec<_>>().join(" "),
    }
}

/// `path` from `folder`, both from the repository's root, with `.` and
/// `..` taken out; none if it climbs out of the repository or is absolute.
fn resolve(folder: &str, path: &str) -> Option<String> {
    if path.starts_with('/') {
        return None;
    }
    let mut out: Vec<&str> = folder.split('/').filter(|s| !s.is_empty()).collect();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            _ => out.push(segment),
        }
    }
    Some(out.join("/"))
}

/// A value as the error shows it: its JSON, or `none`.
fn shown(value: Option<&Value>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| format!("`{value}`"))
}

/// `text`, the file `file`, as JSON once its comments are taken out.
fn parse(file: &str, text: &str) -> Result<Value, String> {
    serde_json::from_str(&uncommented(text)).map_err(|e| format!("{file}: {e}"))
}

/// `text` with its `//` and `/* */` comments made spaces (a newline stays
/// a newline, so an error's line is the file's), outside strings.
fn uncommented(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let (mut string, mut escaped) = (false, false);
    while let Some(c) = chars.next() {
        if string {
            out.push(c);
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => string = false,
                _ => {}
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    out.push(if c == '\n' { '\n' } else { ' ' });
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests;
