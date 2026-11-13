//! A scripted machine and GitHub for `repos`' tests: it answers the calls
//! `repos` makes as `gh`, `git` and `ssh-keygen` would, keeps what they
//! change (so a second run sees the first's work), and records every
//! call. Nothing reaches the network.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Git, Host, Ran};
use crate::project::Project;

/// One call `repos` made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub program: String,
    pub args: Vec<String>,
    pub dir: PathBuf,
    pub stdin: Option<String>,
}

impl Call {
    /// The program and its arguments, joined by spaces.
    pub fn line(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A repository on the fake GitHub.
#[derive(Debug, Clone, Default)]
pub struct Repo {
    pub admin: bool,
    pub private: bool,
    pub description: String,
    pub branches: Vec<String>,
    pub main: Option<String>,
    /// Each deploy key: its id, title and public half.
    pub keys: Vec<(u64, String, String)>,
    pub actions: BTreeMap<String, String>,
    pub dependabot: BTreeMap<String, String>,
    pub rulesets: Vec<String>,
    /// GitHub refuses its rulesets (a free account's private repository).
    pub no_rulesets: bool,
    /// The files its first commit held, by path, with their text.
    pub files: BTreeMap<String, String>,
}

/// The machine, the game's folder and GitHub.
pub struct Fake {
    /// Programs that aren't installed.
    pub missing: Vec<&'static str>,
    pub signed_in: bool,
    /// The scopes `gh auth status` lists, if it lists any.
    pub scopes: Option<Vec<&'static str>>,
    pub login: String,
    pub repos: BTreeMap<String, Repo>,
    pub git: Git,
    pub branch: String,
    pub origin: Option<String>,
    /// The pin file's text at the last commit.
    pub committed_pin: String,
    pub commits: Vec<String>,
    /// A call whose line holds this fails.
    pub fail: Option<String>,
    pub calls: Vec<Call>,
    pub root: PathBuf,
    pub temp: PathBuf,
    pub fetches: usize,
    next_key: u64,
}

impl Fake {
    /// Nothing made yet: a signed-in `gh` with every scope, and the game's
    /// folder at `root` with an empty pin file.
    pub fn new(root: &Path, temp: &Path) -> Self {
        Self {
            missing: Vec::new(),
            signed_in: true,
            scopes: Some(vec!["gist", "read:org", "repo", "workflow"]),
            login: "you".to_owned(),
            repos: BTreeMap::new(),
            git: Git::None,
            branch: "main".to_owned(),
            origin: None,
            committed_pin: String::new(),
            commits: Vec::new(),
            fail: None,
            calls: Vec::new(),
            root: root.to_path_buf(),
            temp: temp.to_path_buf(),
            fetches: 0,
            next_key: 100,
        }
    }

    /// The repository `path` (`owner/name`).
    pub fn repo(&mut self, path: &str) -> &mut Repo {
        self.repos.entry(path.to_owned()).or_default()
    }

    /// The calls' lines that change something: neither a read of GitHub
    /// nor a look at git.
    pub fn changes(&self) -> Vec<String> {
        self.calls
            .iter()
            .map(Call::line)
            .filter(|line| !is_read(line))
            .collect()
    }

    fn pin_text(&self) -> String {
        fs::read_to_string(self.root.join("private.rev")).unwrap_or_default()
    }

    fn answer(&mut self, call: &Call) -> Ran {
        let args: Vec<&str> = call.args.iter().map(String::as_str).collect();
        let at_root = call.dir == self.root;
        match (call.program.as_str(), args.as_slice()) {
            ("gh" | "git", ["--version"]) | ("ssh-keygen", ["-?"]) => ok(""),
            ("gh", ["auth", "status", ..]) => self.auth_status(),
            ("gh", ["api", "user", "--jq", ".login"]) => ok(&format!("{}\n", self.login)),
            ("gh", ["api", "-X", "DELETE", path]) => self.delete_key(path),
            ("gh", ["api", path, "-X", "POST", "--input", "-"]) => {
                self.post_ruleset(path, call.stdin.as_deref())
            }
            ("gh", ["api", path, "--jq", ".sha"]) => self.main_sha(path),
            ("gh", ["api", path]) => self.get(path),
            ("gh", ["repo", "create", path, visibility, rest @ ..]) => {
                self.create(path, visibility, rest, &call.dir, at_root)
            }
            (
                "gh",
                [
                    "repo",
                    "deploy-key",
                    "add",
                    file,
                    "--repo",
                    repo,
                    "--title",
                    title,
                ],
            ) => {
                let public = fs::read_to_string(file).unwrap_or_default();
                self.next_key += 1;
                let id = self.next_key;
                self.repo(repo).keys.push((id, (*title).to_owned(), public));
                ok("")
            }
            ("gh", ["secret", "set", name, "--repo", repo, rest @ ..]) => {
                let value = call.stdin.clone().unwrap_or_default();
                let repo = self.repo(repo);
                let secrets = if rest == ["--app", "dependabot"] {
                    &mut repo.dependabot
                } else {
                    &mut repo.actions
                };
                secrets.insert((*name).to_owned(), value);
                ok("")
            }
            ("git", args) if at_root => self.git_here(args),
            // The private repository's first commit, in its own folder.
            ("git", ["init" | "add" | "commit", ..]) => ok(""),
            ("ssh-keygen", [.., "-C", comment, "-f", file]) => {
                self.next_key += 1;
                let n = self.next_key;
                fs::write(file, format!("PRIVATE-HALF-{n}\n")).unwrap();
                fs::write(
                    format!("{file}.pub"),
                    format!("ssh-ed25519 PUBLIC-HALF-{n} {comment}\n"),
                )
                .unwrap();
                ok("")
            }
            _ => panic!("the fake can't answer `{}`", call.line()),
        }
    }

    fn auth_status(&self) -> Ran {
        if !self.signed_in {
            return failed(
                "You are not logged into any GitHub hosts. To log in, run: gh auth login",
            );
        }
        let mut text = format!(
            "github.com\n  \u{2713} Logged in to github.com account {} (keyring)\n  - Active \
             account: true\n  - Git operations protocol: https\n",
            self.login
        );
        if let Some(scopes) = &self.scopes {
            let quoted: Vec<String> = scopes.iter().map(|s| format!("'{s}'")).collect();
            text.push_str("  - Token scopes: ");
            text.push_str(&quoted.join(", "));
            text.push('\n');
        }
        ok(&text)
    }

    fn get(&mut self, path: &str) -> Ran {
        let path = path.split('?').next().unwrap_or(path);
        let parts: Vec<&str> = path.split('/').collect();
        let ["repos", owner, name, rest @ ..] = parts.as_slice() else {
            panic!("the fake can't get {path}");
        };
        let Some(repo) = self.repos.get(&format!("{owner}/{name}")) else {
            return not_found();
        };
        let json = match rest {
            [] => json!({ "permissions": { "admin": repo.admin }, "private": repo.private }),
            ["branches", branch] => {
                if !repo.branches.iter().any(|b| b == branch) {
                    return not_found();
                }
                json!({ "name": branch })
            }
            [app @ ("actions" | "dependabot"), "secrets"] => {
                let secrets = if *app == "actions" {
                    &repo.actions
                } else {
                    &repo.dependabot
                };
                let list: Vec<Value> = secrets.keys().map(|n| json!({ "name": n })).collect();
                json!({ "total_count": list.len(), "secrets": list })
            }
            ["keys"] => Value::Array(
                repo.keys
                    .iter()
                    .map(|(id, title, key)| json!({ "id": id, "title": title, "key": key, "read_only": true }))
                    .collect(),
            ),
            ["rulesets"] => {
                if repo.no_rulesets {
                    return refused();
                }
                Value::Array(repo.rulesets.iter().map(|n| json!({ "name": n })).collect())
            }
            _ => panic!("the fake can't get {path}"),
        };
        ok(&json.to_string())
    }

    fn main_sha(&self, path: &str) -> Ran {
        let repo = path
            .strip_prefix("repos/")
            .and_then(|p| p.strip_suffix("/commits/main"))
            .and_then(|p| self.repos.get(p));
        match repo.and_then(|r| r.main.clone()) {
            Some(sha) => ok(&format!("{sha}\n")),
            None => not_found(),
        }
    }

    fn delete_key(&mut self, path: &str) -> Ran {
        let parts: Vec<&str> = path.split('/').collect();
        let ["repos", owner, name, "keys", id] = parts.as_slice() else {
            panic!("the fake can't delete {path}");
        };
        let id: u64 = id.parse().unwrap();
        let repo = self.repo(&format!("{owner}/{name}"));
        let before = repo.keys.len();
        repo.keys.retain(|(k, _, _)| *k != id);
        if repo.keys.len() == before {
            not_found()
        } else {
            ok("")
        }
    }

    fn post_ruleset(&mut self, path: &str, body: Option<&str>) -> Ran {
        let repo = path
            .strip_prefix("repos/")
            .and_then(|p| p.strip_suffix("/rulesets"))
            .unwrap();
        let repo = self.repo(repo);
        if repo.no_rulesets {
            return refused();
        }
        let body: Value = serde_json::from_str(body.unwrap()).unwrap();
        repo.rulesets
            .push(body["name"].as_str().unwrap().to_owned());
        ok("{}")
    }

    fn create(
        &mut self,
        path: &str,
        visibility: &str,
        rest: &[&str],
        dir: &Path,
        at_root: bool,
    ) -> Ran {
        if self.repos.contains_key(path) {
            return failed("GraphQL: Name already exists on this account (createRepository)");
        }
        let mut repo = Repo {
            admin: true,
            private: visibility == "--private",
            ..Repo::default()
        };
        if let Some(at) = rest.iter().position(|a| *a == "--description") {
            repo.description = rest[at + 1].to_owned();
        }
        if rest.contains(&"--push") {
            repo.branches.push("main".to_owned());
            repo.main = Some(format!("{:0>40}", self.repos.len() + 1));
            files_in(dir, "", &mut repo.files);
        }
        self.repos.insert(path.to_owned(), repo);
        if at_root {
            self.origin = Some(format!("https://github.com/{path}.git"));
        }
        ok(&format!("https://github.com/{path}\n"))
    }

    fn git_here(&mut self, args: &[&str]) -> Ran {
        let not_a_repo =
            || failed("fatal: not a git repository (or any of the parent directories): .git");
        match args {
            ["rev-parse", "--show-toplevel"] => {
                if self.git == Git::None {
                    return not_a_repo();
                }
                ok(&format!(
                    "{}\n",
                    self.root.to_string_lossy().replace('\\', "/")
                ))
            }
            ["rev-parse", "--verify", "--quiet", "HEAD"] => {
                if self.git == Git::Committed {
                    ok("0123\n")
                } else {
                    failed("")
                }
            }
            ["symbolic-ref", "--short", "HEAD"] => ok(&format!("{}\n", self.branch)),
            ["remote", "get-url", "origin"] => match &self.origin {
                Some(url) => ok(&format!("{url}\n")),
                None => failed("error: No such remote 'origin'"),
            },
            ["remote", "add", "origin", url] => {
                self.origin = Some((*url).to_owned());
                ok("")
            }
            ["status", "--porcelain", "--", _] => {
                if self.pin_text() == self.committed_pin {
                    ok("")
                } else {
                    ok(" M private.rev\n")
                }
            }
            ["init", ..] => {
                self.git = Git::NoCommit;
                ok("")
            }
            ["add", ..] => ok(""),
            ["commit", "--quiet", "-m", message, ..] => {
                self.git = Git::Committed;
                self.committed_pin = self.pin_text();
                self.commits.push((*message).to_owned());
                ok("")
            }
            ["push", "--quiet", "--set-upstream", "origin", branch] => {
                let origin = self.origin.clone().unwrap();
                let path = super::Slug::of_url(&origin).unwrap().path();
                self.repo(&path).branches.push((*branch).to_owned());
                ok("")
            }
            _ => panic!("the fake can't answer git {args:?}"),
        }
    }
}

/// Whether the call's line only reads: GitHub's state, or git's.
pub fn is_read(line: &str) -> bool {
    let reads = [
        "gh --version",
        "git --version",
        "ssh-keygen -?",
        "gh auth status",
        "gh api user",
        "git rev-parse",
        "git symbolic-ref",
        "git remote get-url",
        "git status",
    ];
    if reads.iter().any(|r| line.starts_with(r)) {
        return true;
    }
    // `gh api <path>` with no method, or a `--jq` read.
    line.starts_with("gh api ") && !line.contains(" -X ")
}

fn ok(stdout: &str) -> Ran {
    Ran {
        ok: true,
        stdout: stdout.to_owned(),
        stderr: String::new(),
    }
}

fn failed(stderr: &str) -> Ran {
    Ran {
        ok: false,
        stdout: String::new(),
        stderr: stderr.to_owned(),
    }
}

fn not_found() -> Ran {
    failed("gh: Not Found (HTTP 404)")
}

fn refused() -> Ran {
    failed(
        "gh: Upgrade to GitHub Pro or make this repository public to enable this feature. (HTTP 403)",
    )
}

/// Every file under `dir`, by its path from it.
fn files_in(dir: &Path, prefix: &str, out: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.path().is_dir() {
            files_in(&entry.path(), &format!("{name}/"), out);
        } else {
            out.insert(name, fs::read_to_string(entry.path()).unwrap());
        }
    }
}

impl Host for Fake {
    fn run(
        &mut self,
        program: &str,
        args: &[&str],
        dir: &Path,
        stdin: Option<&str>,
    ) -> std::io::Result<Ran> {
        let call = Call {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            dir: dir.to_path_buf(),
            stdin: stdin.map(str::to_owned),
        };
        self.calls.push(call.clone());
        if self.missing.contains(&program) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "program not found",
            ));
        }
        if let Some(fail) = &self.fail
            && call.line().contains(fail.as_str())
        {
            return Ok(failed("injected failure"));
        }
        Ok(self.answer(&call))
    }

    fn temp_root(&self) -> PathBuf {
        self.temp.clone()
    }

    fn fetch_private(&mut self, project: &Project) -> Result<String, String> {
        self.calls.push(Call {
            program: "shuyi".to_owned(),
            args: vec!["private-assets".to_owned()],
            dir: project.root.clone(),
            stdin: None,
        });
        if self.fail.as_deref() == Some("private-assets") {
            return Err("can't read the repository".to_owned());
        }
        self.fetches += 1;
        fs::create_dir_all(project.root.join("private/.git")).unwrap();
        Ok("private/ is at the pin".to_owned())
    }
}
