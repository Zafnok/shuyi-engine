//! `private-assets [--library | --pin]`: fetches the files a game may not
//! hand out (bought art, say) from a private repository, checked out into
//! a git-ignored folder of the project ([`PrivateAssets::checkout`]) at the
//! commit the project's pin file names ([`PrivateAssets::pin_file`]).
//!
//! With no flag the command clones the checkout if it isn't there (only
//! [`PrivateAssets::folders`], and without the history's files), fetches,
//! and puts it at the pinned commit; then it copies
//! [`PrivateAssets::copies`] out of it into the project (voice clips that
//! ship as files beside the game, say). `--library` also brings
//! [`PrivateAssets::library`]. `--pin` writes the commit the checkout is
//! at into the pin file, after checking that it is pushed to the
//! repository's `main` and that every file in the credited folders has a
//! credit ([`PrivateAssets::credits`]). A full clone made by hand is left
//! whole.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use shuyi_assets::credits::{self, Places};

use crate::project::{Credited, PrivateAssets};
use crate::web::copy_tree;

/// The usage line, with `command` as the tool's name.
pub fn usage(rules: &PrivateAssets, command: &str) -> String {
    let flags = if rules.library.is_some() {
        "[--library | --pin]"
    } else {
        "[--pin]"
    };
    format!("usage: {command} private-assets {flags}")
}

/// What the command was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Clone or update the checkout and put it at the pinned commit.
    Fetch {
        /// Whether to bring the library folder too.
        library: bool,
    },
    /// Write the commit the checkout is at into the pin file.
    Pin,
}

/// Parses the arguments after `private-assets`. The error is the usage
/// line.
pub fn parse_args(rules: &PrivateAssets, command: &str, args: &[String]) -> Result<Mode, String> {
    match args {
        [] => Ok(Mode::Fetch { library: false }),
        [flag] if flag == "--library" && rules.library.is_some() => {
            Ok(Mode::Fetch { library: true })
        }
        [flag] if flag == "--pin" => Ok(Mode::Pin),
        _ => Err(usage(rules, command)),
    }
}

/// Runs `git <args>` in `dir`: its output, or what it printed as the error.
/// It never stops to ask for a password on the terminal.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("spawn git: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The error for a clone or fetch of `url` that git refused with `said`.
fn no_access(rules: &PrivateAssets, url: &str, said: &str) -> String {
    format!(
        "can't read {url}\n  git said: {said}\n  {}",
        rules.no_access
    )
}

/// The commit named by the pin file: 40 lower-case hex digits.
fn read_pin(rules: &PrivateAssets, root: &Path) -> Result<String, String> {
    let file = &rules.pin_file;
    let text = fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))?;
    let pin = text.trim();
    let is_hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
    if pin.len() != 40 || !pin.chars().all(is_hex) {
        return Err(format!(
            "{file}: expected a 40-character commit id, found `{pin}`"
        ));
    }
    Ok(pin.to_string())
}

/// Whether `dir` is a folder with something in it.
fn has_entries(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

/// Clones `url` into the checkout if it isn't there, fetches, and puts it
/// at the pinned commit. A clone made here holds only the rules' folders
/// (and the library once asked for with `library`); a full clone made by
/// hand is left whole. Returns a one-line summary.
fn fetch(rules: &PrivateAssets, root: &Path, url: &str, library: bool) -> Result<String, String> {
    let name = &rules.checkout;
    let pin = read_pin(rules, root)?;
    let checkout = root.join(name);
    let (first, others) = rules
        .folders
        .split_first()
        .ok_or("shuyi.ron: `private_assets.folders` is empty")?;
    if checkout.join(".git").exists() {
        git(&checkout, &["fetch", "--quiet", "origin"]).map_err(|e| no_access(rules, url, &e))?;
    } else {
        if has_entries(&checkout) {
            return Err(format!(
                "{name}/ has files but isn't a git checkout: move it away first"
            ));
        }
        let clone = ["clone", "--quiet", "--filter=blob:none", "--no-checkout"];
        let long_paths = ["--config", "core.longpaths=true"];
        git(root, &[&clone[..], &long_paths[..], &[url, name]].concat())
            .map_err(|e| no_access(rules, url, &e))?;
        git(&checkout, &["sparse-checkout", "set", "--cone", first])?;
    }

    // Unset (a full clone) is an error from `git config --get`.
    let sparse =
        git(&checkout, &["config", "--get", "core.sparseCheckout"]).as_deref() == Ok("true");
    if sparse && !others.is_empty() {
        // A clone made before the other folders existed holds only the
        // first.
        let others: Vec<&str> = others.iter().map(String::as_str).collect();
        git(
            &checkout,
            &[&["sparse-checkout", "add"][..], &others].concat(),
        )?;
    }
    if let Some(dir) = rules.library.as_deref().filter(|_| library && sparse) {
        git(&checkout, &["sparse-checkout", "add", dir])?;
    }

    // A clone whose files were never checked out (new, or a first fetch that
    // stopped half-way) has an empty index. Otherwise a checkout already at
    // the pin is left as it is (on its branch, if any).
    let never_checked_out = git(&checkout, &["ls-files"])?.is_empty();
    if never_checked_out || git(&checkout, &["rev-parse", "HEAD"])? != pin {
        git(&checkout, &["checkout", "--quiet", "--detach", &pin]).map_err(|e| {
            format!("can't check out the pinned commit {pin} in {name}/\n  git said: {e}")
        })?;
    }

    let holds = if sparse {
        let folders = git(&checkout, &["sparse-checkout", "list"])?;
        folders.lines().collect::<Vec<_>>().join(", ")
    } else {
        "the whole repository".to_string()
    };
    let mut copied = String::new();
    for copy in &rules.copies {
        if let Some(files) = copy_out(&checkout.join(&copy.from), &root.join(&copy.to))? {
            // Writing to a `String` can't fail.
            let _ = write!(
                copied,
                "; {}/ holds its {files} {} files",
                copy.to, copy.from
            );
        }
    }
    Ok(format!("{name}/ is at {pin} ({holds}){copied}"))
}

/// Replaces `dst` with a copy of the checkout's folder `src`, returning
/// how many files it holds. With no such folder, `dst` is left as it is
/// and `None` is returned.
fn copy_out(src: &Path, dst: &Path) -> Result<Option<usize>, String> {
    if !src.is_dir() {
        return Ok(None);
    }
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| format!("clear {}: {e}", dst.display()))?;
    }
    copy_tree(src, dst, &|_| true).map(Some)
}

/// The paths (`/`-separated, relative to `dir`, each after `prefix`) of
/// every file in `dir` and the folders below it.
fn files_below(dir: &Path, prefix: &str, paths: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.path().is_dir() {
            files_below(&entry.path(), &format!("{path}/"), paths)?;
        } else {
            paths.push(path);
        }
    }
    Ok(())
}

/// The files of the checkout's folder `dir` that no entry of the project's
/// credits file covers, sorted. None for a folder that isn't there.
fn uncredited(
    rules: &PrivateAssets,
    credited: &Credited,
    root: &Path,
    dir: &str,
) -> Result<Vec<String>, String> {
    let credits_file = &credited.file;
    let source =
        fs::read_to_string(root.join(credits_file)).map_err(|e| format!("{credits_file}: {e}"))?;
    let places = Places {
        audio_manifest: &credited.audio_manifest,
        private_root: &credited.private_root,
    };
    let file = credits::from_source(credits_file, &source, &places).map_err(|errors| {
        let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
        lines.join("\n")
    })?;
    let mut files = Vec::new();
    let dir = root.join(&rules.checkout).join(dir);
    if dir.is_dir() {
        files_below(&dir, "", &mut files)?;
    }
    files.sort_unstable();
    let paths: Vec<&str> = files.iter().map(String::as_str).collect();
    let uncredited = credits::uncredited(&file, &paths);
    Ok(uncredited.into_iter().map(str::to_owned).collect())
}

/// Writes the commit the checkout is at into the pin file. Refuses when
/// there are uncommitted changes, when a file in a credited folder has no
/// credit, or when the commit isn't on the private repository's `main`
/// yet: the builds that ship fetch the pin from there.
fn pin(rules: &PrivateAssets, command: &str, root: &Path, url: &str) -> Result<String, String> {
    let name = &rules.checkout;
    let checkout = root.join(name);
    if !checkout.join(".git").exists() {
        return Err(format!(
            "{name}/ isn't there: run `{command} private-assets` first"
        ));
    }
    let changes = git(&checkout, &["status", "--porcelain"])?;
    if !changes.is_empty() {
        return Err(format!(
            "{name}/ has changes that aren't committed:\n{changes}\n\
             commit and push them, then run this again"
        ));
    }
    if let Some(credited) = &rules.credits {
        for dir in &credited.folders {
            let uncredited = uncredited(rules, credited, root, dir)?;
            if !uncredited.is_empty() {
                let hint = credited
                    .hint
                    .as_ref()
                    .map(|hint| format!(" ({hint})"))
                    .unwrap_or_default();
                return Err(format!(
                    "these files in {name}/{dir}/ have no credit:\n  {}\n\
                     add their path to an entry's `private` list in {}{hint}, then run \
                     this again",
                    uncredited.join("\n  "),
                    credited.file
                ));
            }
        }
    }
    git(&checkout, &["fetch", "--quiet", "origin"]).map_err(|e| no_access(rules, url, &e))?;
    let head = git(&checkout, &["rev-parse", "HEAD"])?;
    let pushed = git(&checkout, &["branch", "--remotes", "--contains", &head])?;
    if !pushed.lines().any(|branch| branch.trim() == "origin/main") {
        return Err(format!(
            "commit {head} isn't on the private repository's main branch: push it \
             (`git -C {name} push origin HEAD:main`), then run this again"
        ));
    }
    let file = &rules.pin_file;
    fs::write(root.join(file), format!("{head}\n")).map_err(|e| format!("{file}: {e}"))?;
    Ok(format!(
        "{file} now names {head}: commit it with your change"
    ))
}

/// Runs the command in the project at `root` against the private
/// repository at `url` (the rules' own, but for tests). `command` is the
/// tool's name in what it says.
pub fn run(
    rules: &PrivateAssets,
    command: &str,
    root: &Path,
    url: &str,
    mode: Mode,
) -> Result<String, String> {
    match mode {
        Mode::Fetch { library } => fetch(rules, root, url, library),
        Mode::Pin => pin(rules, command, root, url),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::project::CopyOut;

    /// A game's private repository: `game/`, `test/` and `voice/` checked
    /// out into `private/`, `library/` on demand, the voices copied out,
    /// `game/` and `test/` credited.
    fn rules() -> PrivateAssets {
        let strings = |items: &[&str]| items.iter().map(|s| (*s).to_owned()).collect();
        PrivateAssets {
            url: "https://example.org/private.git".to_owned(),
            checkout: "private".to_owned(),
            pin_file: "private.rev".to_owned(),
            folders: strings(&["game", "test", "voice"]),
            library: Some("library".to_owned()),
            copies: vec![CopyOut {
                from: "voice".to_owned(),
                to: "voice".to_owned(),
            }],
            credits: Some(Credited {
                file: "data/credits.ron".to_owned(),
                folders: strings(&["game", "test"]),
                audio_manifest: "audio/audio.ron".to_owned(),
                private_root: "private/game".to_owned(),
                hint: Some("a new pack also needs a row in the assets list".to_owned()),
            }),
            no_access: "Only the team can read this private repository; without it the \
                        game runs with the public placeholders."
                .to_owned(),
        }
    }

    /// Who commits in the scratch repositories (a runner has no git identity).
    const AS_TESTER: [&str; 6] = [
        "-c",
        "user.name=tester",
        "-c",
        "user.email=tester@example.com",
        "-c",
        "commit.gpgsign=false",
    ];

    /// Settings that keep git's own housekeeping out of a scratch
    /// repository. A commit or a push starts it in the background, and its
    /// repack then moves files under `objects/` while the next command
    /// copies them (ticket 0120: seen on a Mac runner with git 2.55).
    const NO_UPKEEP: [&str; 2] = ["gc.auto=0", "maintenance.auto=false"];

    /// `git <args>` in `dir`, which must work.
    fn g(dir: &Path, args: &[&str]) -> String {
        git(dir, args).unwrap_or_else(|e| panic!("git {args:?}: {e}"))
    }

    /// Commits everything in `dir` and returns the commit id.
    fn commit(dir: &Path, message: &str) -> String {
        g(dir, &["add", "--all"]);
        g(
            dir,
            &[&AS_TESTER[..], &["commit", "--quiet", "-m", message]].concat(),
        );
        g(dir, &["rev-parse", "HEAD"])
    }

    /// A scratch folder under `env::temp_dir()` holding a stand-in for the
    /// private repository (`remote.git`, filled from `src/`) and for this
    /// one (`public/`).
    struct Scratch {
        root: PathBuf,
        /// The private repository's first commit: `game/` says `one`.
        one: String,
        /// Its second commit, the tip of `main`: `game/` says `two`, and
        /// there is a `voice/` folder with a manifest and a clip.
        two: String,
    }

    impl Scratch {
        fn new(name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("shuyi-private-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            let src = root.join("src");
            for dir in ["game/portraits", "library/tiny-tales", "originals"] {
                fs::create_dir_all(src.join(dir)).unwrap();
            }
            fs::create_dir_all(root.join("public")).unwrap();
            g(&root, &["init", "--quiet", "--initial-branch=main", "src"]);
            for setting in NO_UPKEEP {
                let (key, value) = setting.split_once('=').unwrap();
                g(&src, &["config", key, value]);
            }
            fs::write(src.join("README.md"), "private").unwrap();
            fs::write(src.join("game/portraits/a.portrait"), "one").unwrap();
            fs::write(src.join("library/tiny-tales/face.png"), "face").unwrap();
            fs::write(src.join("originals/pack.zip"), "zip").unwrap();
            let one = commit(&src, "one");
            fs::write(src.join("game/portraits/a.portrait"), "two").unwrap();
            fs::create_dir_all(src.join("voice/en/scene")).unwrap();
            fs::write(src.join("voice/en/voice.ron"), "manifest").unwrap();
            fs::write(src.join("voice/en/scene/line.ogg"), "clip").unwrap();
            let two = commit(&src, "two");
            // `--no-local`: over git's own protocol, as from a real remote,
            // not by copying the files under `src/.git/objects`.
            let [gc, maintenance] = NO_UPKEEP;
            let bare = ["clone", "--quiet", "--bare", "--no-local"];
            let quiet = ["--config", gc, "--config", maintenance];
            g(
                &root,
                &[&bare[..], &quiet[..], &["src", "remote.git"]].concat(),
            );
            Self { root, one, two }
        }

        /// The stand-in for this repository's root.
        fn public(&self) -> PathBuf {
            self.root.join("public")
        }

        /// The address of the stand-in private repository.
        fn url(&self) -> String {
            self.root.join("remote.git").to_string_lossy().into_owned()
        }

        /// A path inside the checkout, `public/private/`.
        fn checkout(&self, path: &str) -> PathBuf {
            self.public().join("private").join(path)
        }

        /// Writes this repository's credits file: one bought work that
        /// covers `private` (RON strings, e.g. `"portraits/"`).
        fn set_credits(&self, private: &str) {
            let dir = self.public().join("data");
            fs::create_dir_all(&dir).unwrap();
            let credits = format!(
                "(credits: [(id: \"pack\", group: Art, title: \"Pack\", author: \"Dee\", \
                 source: \"https://example.org\", license: \"Custom (Dee)\", \
                 private: [{private}])])"
            );
            fs::write(dir.join("credits.ron"), credits).unwrap();
        }

        fn set_pin(&self, commit: &str) {
            fs::write(self.public().join("private.rev"), format!("{commit}\n")).unwrap();
        }

        fn fetch(&self, library: bool) -> Result<String, String> {
            run(
                &rules(),
                "shuyi",
                &self.public(),
                &self.url(),
                Mode::Fetch { library },
            )
        }

        fn pin(&self) -> Result<String, String> {
            run(&rules(), "shuyi", &self.public(), &self.url(), Mode::Pin)
        }

        /// What `game/portraits/a.portrait` says in the checkout.
        fn game_says(&self) -> String {
            fs::read_to_string(self.checkout("game/portraits/a.portrait")).unwrap()
        }

        /// A full clone of the private repository in place of the checkout,
        /// as on the machine the art was bought on.
        fn clone_whole(&self) {
            g(
                &self.public(),
                &["clone", "--quiet", &self.url(), "private"],
            );
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn parse_args_reads_the_mode() {
        let parse = |items: &[&str]| parse_args(&rules(), "shuyi", &args(items));
        assert_eq!(parse(&[]), Ok(Mode::Fetch { library: false }));
        assert_eq!(parse(&["--library"]), Ok(Mode::Fetch { library: true }));
        assert_eq!(parse(&["--pin"]), Ok(Mode::Pin));
        let usage = "usage: shuyi private-assets [--library | --pin]";
        for bad in [&["--bogus"][..], &["--library", "--pin"], &["--pin", "x"]] {
            assert_eq!(parse(bad), Err(usage.to_owned()), "{bad:?}");
        }
        // Without a library there is no `--library`.
        let bare = PrivateAssets {
            library: None,
            ..rules()
        };
        let usage = "usage: tools private-assets [--pin]";
        assert_eq!(usage, super::usage(&bare, "tools"));
        let library = parse_args(&bare, "tools", &args(&["--library"]));
        assert_eq!(library, Err(usage.to_owned()));
    }

    #[test]
    fn the_pin_file_must_name_a_commit() {
        let scratch = Scratch::new("pin-file");
        let root = scratch.public();
        assert!(
            read_pin(&rules(), &root)
                .unwrap_err()
                .starts_with("private.rev: ")
        );
        scratch.set_pin(&scratch.one);
        assert_eq!(read_pin(&rules(), &root), Ok(scratch.one.clone()));
        for bad in [
            "",
            "main",
            &scratch.one[..39],
            &format!("{}0", scratch.one),
            &scratch.one.to_uppercase().replace('0', "A"),
            &format!("g{}", &scratch.one[1..]),
        ] {
            scratch.set_pin(bad);
            assert_eq!(
                read_pin(&rules(), &root),
                Err(format!(
                    "private.rev: expected a 40-character commit id, found `{bad}`"
                )),
            );
        }
    }

    #[test]
    fn a_first_fetch_clones_only_the_game_and_voice_folders() {
        let scratch = Scratch::new("first");
        scratch.set_pin(&scratch.two);
        // An empty folder is fine to clone into.
        fs::create_dir_all(scratch.checkout("")).unwrap();
        assert_eq!(
            scratch.fetch(false),
            Ok(format!(
                "private/ is at {} (game, test, voice); voice/ holds its 2 voice files",
                scratch.two
            ))
        );
        assert_eq!(scratch.game_says(), "two");
        // Files in the private repository's root come along; nothing else.
        assert!(scratch.checkout("README.md").is_file());
        assert!(!scratch.checkout("library").exists());
        assert!(!scratch.checkout("originals").exists());
    }

    /// The private repository's `voice/` is copied to the project's
    /// `voice/`, replacing what was there; without one, `voice/` is left
    /// alone.
    #[test]
    fn a_fetch_copies_the_voice_folder_beside_the_game() {
        let scratch = Scratch::new("voice");
        let voice = scratch.public().join("voice");
        let read = |path: &str| fs::read_to_string(voice.join(path)).ok();
        // The first commit has no voices: a folder made by hand stays.
        scratch.set_pin(&scratch.one);
        fs::create_dir_all(voice.join("en")).unwrap();
        fs::write(voice.join("en/mine.ogg"), "mine").unwrap();
        assert_eq!(
            scratch.fetch(false),
            Ok(format!(
                "private/ is at {} (game, test, voice)",
                scratch.one
            ))
        );
        assert_eq!(read("en/mine.ogg").as_deref(), Some("mine"));
        // The second has: its files replace the folder's.
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        assert_eq!(read("en/voice.ron").as_deref(), Some("manifest"));
        assert_eq!(read("en/scene/line.ogg").as_deref(), Some("clip"));
        assert_eq!(read("en/mine.ogg"), None);
        // A clone made before voices existed (only `game/`) gets them too.
        let old = Scratch::new("voice-old-clone");
        old.set_pin(&old.two);
        old.fetch(false).unwrap();
        let checkout = old.checkout("");
        git(&checkout, &["sparse-checkout", "set", "--cone", "game"]).unwrap();
        assert!(!old.checkout("voice").exists());
        fs::remove_dir_all(old.public().join("voice")).unwrap();
        old.fetch(false).unwrap();
        assert!(old.public().join("voice/en/voice.ron").is_file());
    }

    #[test]
    fn copy_out_needs_a_source_folder() {
        let root = std::env::temp_dir().join(format!("shuyi-copy-out-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("dst")).unwrap();
        fs::write(root.join("dst/kept.ogg"), "kept").unwrap();
        // No source, or a file by that name: nothing happens.
        assert_eq!(copy_out(&root.join("src"), &root.join("dst")), Ok(None));
        fs::write(root.join("src"), "a file").unwrap();
        assert_eq!(copy_out(&root.join("src"), &root.join("dst")), Ok(None));
        assert!(root.join("dst/kept.ogg").is_file());
        // An empty source empties the destination.
        fs::remove_file(root.join("src")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        assert_eq!(copy_out(&root.join("src"), &root.join("dst")), Ok(Some(0)));
        assert!(!root.join("dst/kept.ogg").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_fetch_puts_the_checkout_at_the_pinned_commit() {
        let scratch = Scratch::new("pinned");
        // The pin is an older commit than the tip of `main`.
        scratch.set_pin(&scratch.one);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "one");
        // Fetching again changes nothing.
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "one");
        // The pin moves: so does the checkout.
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "two");
    }

    #[test]
    fn a_fetch_sees_commits_pushed_after_the_clone() {
        let scratch = Scratch::new("later");
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        let src = scratch.root.join("src");
        fs::write(src.join("game/portraits/a.portrait"), "three").unwrap();
        let three = commit(&src, "three");
        g(&src, &["push", "--quiet", "../remote.git", "main"]);
        scratch.set_pin(&three);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "three");
    }

    #[test]
    fn library_brings_the_library_folder_and_keeps_it() {
        let scratch = Scratch::new("library");
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        assert_eq!(
            scratch.fetch(true),
            Ok(format!(
                "private/ is at {} (game, library, test, voice); voice/ holds its 2 voice files",
                scratch.two
            ))
        );
        assert!(scratch.checkout("library/tiny-tales/face.png").is_file());
        assert!(!scratch.checkout("originals").exists());
        // A later plain fetch keeps it.
        scratch.fetch(false).unwrap();
        assert!(scratch.checkout("library/tiny-tales/face.png").is_file());

        // Asked for on the first fetch, it comes with the clone.
        let first = Scratch::new("library-first");
        first.set_pin(&first.one);
        first.fetch(true).unwrap();
        assert!(first.checkout("library/tiny-tales/face.png").is_file());
        assert_eq!(first.game_says(), "one");
    }

    #[test]
    fn a_whole_clone_is_left_whole_and_on_its_branch() {
        let scratch = Scratch::new("whole");
        scratch.clone_whole();
        scratch.set_pin(&scratch.two);
        assert_eq!(
            scratch.fetch(true),
            Ok(format!(
                "private/ is at {} (the whole repository); voice/ holds its 2 voice files",
                scratch.two
            ))
        );
        assert!(scratch.checkout("originals/pack.zip").is_file());
        assert!(scratch.checkout("library/tiny-tales/face.png").is_file());
        let branch = |scratch: &Scratch| git(&scratch.checkout(""), &["symbolic-ref", "HEAD"]);
        assert_eq!(branch(&scratch), Ok("refs/heads/main".to_string()));
        // A pin elsewhere moves it off the branch, which stays where it was.
        scratch.set_pin(&scratch.one);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "one");
        assert!(branch(&scratch).is_err());
        assert_eq!(
            g(&scratch.checkout(""), &["rev-parse", "main"]),
            scratch.two
        );
        assert!(scratch.checkout("originals/pack.zip").is_file());
    }

    #[test]
    fn a_fetch_says_why_it_failed() {
        let scratch = Scratch::new("fails");
        // No pin file.
        assert!(
            scratch
                .fetch(false)
                .unwrap_err()
                .starts_with("private.rev: ")
        );
        assert!(!scratch.checkout("").exists());

        // A folder that isn't a checkout is never cloned over.
        scratch.set_pin(&scratch.two);
        fs::create_dir_all(scratch.checkout("game")).unwrap();
        assert_eq!(
            scratch.fetch(false),
            Err("private/ has files but isn't a git checkout: move it away first".into())
        );
        fs::remove_dir_all(scratch.checkout("")).unwrap();

        // A repository that can't be read.
        let nowhere = scratch.root.join("nowhere.git");
        let nowhere = nowhere.to_string_lossy();
        let fetch = Mode::Fetch { library: false };
        let error = run(&rules(), "shuyi", &scratch.public(), &nowhere, fetch).unwrap_err();
        assert!(
            error.starts_with(&format!("can't read {nowhere}\n  git said: ")),
            "{error}"
        );
        assert!(error.contains("private repository"), "{error}");
        assert!(error.contains("public placeholders"), "{error}");

        // A pin that isn't a commit of the repository.
        let unknown = "0123456789abcdef0123456789abcdef01234567";
        scratch.set_pin(unknown);
        let error = scratch.fetch(false).unwrap_err();
        assert!(
            error.starts_with(&format!(
                "can't check out the pinned commit {unknown} in private/\n  git said: "
            )),
            "{error}"
        );

        // That left a clone with no files checked out. With a good pin the
        // next fetch finishes the job, though the clone is already "at" it.
        assert!(!scratch.checkout("game").exists());
        scratch.set_pin(&scratch.two);
        scratch.fetch(false).unwrap();
        assert_eq!(scratch.game_says(), "two");
        assert!(!scratch.checkout("library").exists());

        // The repository going away fails the fetch.
        fs::remove_dir_all(scratch.root.join("remote.git")).unwrap();
        let error = scratch.fetch(false).unwrap_err();
        assert!(error.starts_with("can't read "), "{error}");
    }

    #[test]
    fn pin_writes_the_commit_once_it_is_pushed() {
        let scratch = Scratch::new("pin");
        assert_eq!(
            scratch.pin(),
            Err("private/ isn't there: run `shuyi private-assets` first".into())
        );

        scratch.clone_whole();
        let checkout = scratch.checkout("");
        scratch.set_pin(&scratch.one);
        scratch.set_credits("\"portraits/\"");

        // A new file that isn't committed.
        fs::write(scratch.checkout("game/portraits/b.portrait"), "new").unwrap();
        let error = scratch.pin().unwrap_err();
        assert!(
            error.starts_with("private/ has changes that aren't committed:\n"),
            "{error}"
        );
        assert!(error.contains("game/portraits/b.portrait"), "{error}");

        // Committed, not pushed.
        let new = commit(&checkout, "b");
        let error = scratch.pin().unwrap_err();
        assert!(
            error.starts_with(&format!(
                "commit {new} isn't on the private repository's main branch"
            )),
            "{error}"
        );
        assert_eq!(
            read_pin(&rules(), &scratch.public()),
            Ok(scratch.one.clone())
        );

        // Pushed, but to another branch than `main`: the builds that ship
        // can't be sure to find it there.
        g(&checkout, &["push", "--quiet", "origin", "HEAD:side"]);
        let error = scratch.pin().unwrap_err();
        assert!(
            error.starts_with(&format!(
                "commit {new} isn't on the private repository's main branch"
            )),
            "{error}"
        );
        assert_eq!(
            read_pin(&rules(), &scratch.public()),
            Ok(scratch.one.clone())
        );

        // Pushed to `main`.
        g(&checkout, &["push", "--quiet", "origin", "HEAD:main"]);
        assert_eq!(
            scratch.pin(),
            Ok(format!(
                "private.rev now names {new}: commit it with your change"
            ))
        );
        assert_eq!(
            fs::read_to_string(scratch.public().join("private.rev")).unwrap(),
            format!("{new}\n")
        );

        // The private repository can't be read: no pin is written.
        scratch.set_pin(&scratch.one);
        fs::remove_dir_all(scratch.root.join("remote.git")).unwrap();
        assert!(scratch.pin().unwrap_err().starts_with("can't read "));
        assert_eq!(
            read_pin(&rules(), &scratch.public()),
            Ok(scratch.one.clone())
        );
    }

    #[test]
    fn pin_refuses_a_bought_file_without_a_credit() {
        let scratch = Scratch::new("pin-credit");
        scratch.clone_whole();
        let checkout = scratch.checkout("");
        scratch.set_pin(&scratch.one);
        for (path, text) in [
            ("game/README.md", "ours"),
            ("game/units/deep/b.png", "b"),
            ("game/units/a.png", "a"),
        ] {
            fs::create_dir_all(scratch.checkout(path).parent().unwrap()).unwrap();
            fs::write(scratch.checkout(path), text).unwrap();
        }
        let new = commit(&checkout, "units");
        g(&checkout, &["push", "--quiet", "origin", "HEAD:main"]);

        // No credits file in this repository.
        let error = scratch.pin().unwrap_err();
        assert!(error.starts_with("data/credits.ron: "), "{error}");

        // A credits file that doesn't load.
        scratch.set_credits("\"/units/\"");
        assert_eq!(
            scratch.pin(),
            Err(
                "data/credits.ron:1: credit \"pack\": private path \"/units/\" must \
                 be a path inside private/game/"
                    .into()
            )
        );

        // The credit covers the portrait only: the units are named, in
        // order, and the folder's own note isn't.
        scratch.set_credits("\"portraits/\"");
        assert_eq!(
            scratch.pin(),
            Err(
                "these files in private/game/ have no credit:\n  units/a.png\n  \
                 units/deep/b.png\nadd their path to an entry's `private` list in \
                 data/credits.ron (a new pack also needs a row in the assets list), \
                 then run this again"
                    .into()
            )
        );
        assert_eq!(
            read_pin(&rules(), &scratch.public()),
            Ok(scratch.one.clone())
        );

        scratch.set_credits("\"portraits/\", \"units/\"");
        assert!(scratch.pin().is_ok());
        assert_eq!(read_pin(&rules(), &scratch.public()), Ok(new.clone()));

        // The second credited folder needs its credits too.
        fs::create_dir_all(scratch.checkout("test/tiles")).unwrap();
        fs::write(scratch.checkout("test/tiles/t.png"), "t").unwrap();
        let with_test = commit(&checkout, "test tiles");
        g(&checkout, &["push", "--quiet", "origin", "HEAD:main"]);
        assert_eq!(
            scratch.pin(),
            Err(
                "these files in private/test/ have no credit:\n  tiles/t.png\n\
                 add their path to an entry's `private` list in \
                 data/credits.ron (a new pack also needs a row in the assets list), \
                 then run this again"
                    .into()
            )
        );
        assert_eq!(read_pin(&rules(), &scratch.public()), Ok(new));
        scratch.set_credits("\"portraits/\", \"units/\", \"tiles/\"");
        assert!(scratch.pin().is_ok());
        assert_eq!(read_pin(&rules(), &scratch.public()), Ok(with_test));
    }
}
