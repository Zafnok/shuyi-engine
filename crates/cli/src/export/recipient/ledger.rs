//! The ledger of who got which closed build: `recipients.ron`, beside the
//! reveal kit's folder (never in it: `reveal verify` refuses a file its
//! manifest doesn't name), signed with the game's SSH key in
//! `recipients.ron.sig` (`ssh-keygen -Y sign`, the signature `git` signs
//! commits with). ADR-0118 says why a key of the game's own and not a
//! Sigstore attestation: a closed build is made on a developer's machine,
//! not in the release workflow.
//!
//! Each entry names a recipient, the label they were given under, the day,
//! the code their copy carries, the base build's digest (the public
//! package's, which a release attests) and the overlay's (their copy's).
//! The ledger is what turns a code into a person, and it is circumstantial
//! evidence: it says a copy was made for someone, not who leaked it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::export::ExportError;
use crate::sha256;

/// The ledger's file name.
pub const FILE: &str = "recipients.ron";

/// Its signature's file name.
pub const SIGNATURE: &str = "recipients.ron.sig";

/// The namespace its signature is made in (`ssh-keygen -Y sign -n`), so a
/// signature over it means nothing anywhere else.
pub const NAMESPACE: &str = "shuyi-recipients";

/// The namespace a recipient's code is drawn in.
const CODE_NAMESPACE: &str = "shuyi-recipient-code";

/// The ledger's first line of text, which every copy of it carries.
pub const ABOUT: &str = "Who got which closed build. A code found in a copy that leaked is \
                         circumstantial evidence that this copy was the one given to that \
                         recipient, never proof of who leaked it.";

/// The ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    /// [`ABOUT`].
    pub about: String,
    /// The public key that signs it (`ssh-ed25519 AAAA…`), as its first
    /// entry was signed: every later entry is signed with the same key.
    pub signer: String,
    /// Each closed build made, oldest first.
    pub entries: Vec<Entry>,
}

/// One closed build, given to one recipient.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The recipient's id (`--recipient`).
    pub recipient: String,
    /// Who they are, as the developer wrote it (`--label`).
    pub label: String,
    /// The day the copy was made, `YYYY-MM-DD`, by this machine's clock.
    pub date: String,
    /// The code their copy carries.
    pub code: String,
    /// The target.
    pub target: String,
    /// The edition, if one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// The game's version.
    pub version: String,
    /// The SHA-256 of the base build's package: the public one, which a
    /// release attests.
    pub build: String,
    /// The SHA-256 of their copy's package.
    pub overlay: String,
    /// The reveal kit's digest when the copy was made.
    pub kit: String,
}

/// Where the ledger of the kit in `kit` is: beside it.
pub fn path_beside(kit: &Path) -> PathBuf {
    kit.parent()
        .map_or_else(|| PathBuf::from(FILE), |dir| dir.join(FILE))
}

/// Its signature's path, beside the ledger at `ledger`.
pub fn signature_path(ledger: &Path) -> PathBuf {
    ledger.with_file_name(SIGNATURE)
}

/// The ledger at `path`, if there is one.
pub fn load(path: &Path) -> Result<Option<Ledger>, ExportError> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)
        .map_err(|e| ExportError::Build(format!("read {}: {e}", path.display())))?;
    ron::from_str(&text)
        .map(Some)
        .map_err(|e| ExportError::Setup(format!("{}: {e}", path.display())))
}

/// The ledger as text.
pub fn text(ledger: &Ledger) -> String {
    let pretty = ron::ser::PrettyConfig::new().struct_names(false);
    let body = ron::ser::to_string_pretty(ledger, pretty).unwrap_or_default();
    format!("// {FILE}: written by `shuyi export --recipient`; signed in {SIGNATURE}.\n{body}\n")
}

/// The program that signs: `ssh-keygen`.
const SSH_KEYGEN: &str = "ssh-keygen";

/// How to install it.
const INSTALL: &str = "it comes with OpenSSH: on Windows the OpenSSH Client feature (Settings > \
                       System > Optional features), with Git for Windows too; on Linux the \
                       `openssh-client` package; on macOS it is there";

/// Runs `ssh-keygen` with `args`, `input` on its standard input: its output.
fn ssh_keygen(args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, ExportError> {
    use std::io::Write as _;
    let mut child = Command::new(SSH_KEYGEN)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ExportError::MissingTool(format!("{SSH_KEYGEN} isn't installed: {INSTALL}"))
            } else {
                ExportError::Build(format!("{SSH_KEYGEN} doesn't start: {e}"))
            }
        })?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin
            .write_all(input)
            .map_err(|e| ExportError::Build(format!("{SSH_KEYGEN}: {e}")))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| ExportError::Build(format!("{SSH_KEYGEN}: {e}")))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(ExportError::Setup(format!(
            "{SSH_KEYGEN} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

/// A path as `ssh-keygen` takes it: with `/`, which every build of it
/// reads on Windows too.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The public half of the private key at `key`: `<key>.pub` beside it, else
/// what `ssh-keygen -y` derives (a key with no passphrase). Its type and
/// base64, without the comment.
pub fn public_key(key: &Path) -> Result<String, ExportError> {
    let beside = PathBuf::from(format!("{}.pub", key.display()));
    let text = if beside.is_file() {
        fs::read_to_string(&beside)
            .map_err(|e| ExportError::Build(format!("read {}: {e}", beside.display())))?
    } else {
        String::from_utf8_lossy(&ssh_keygen(&["-y", "-f", &shown(key)], None)?).into_owned()
    };
    let mut words = text.split_whitespace();
    match (words.next(), words.next()) {
        (Some(kind), Some(base64)) => Ok(format!("{kind} {base64}")),
        _ => Err(ExportError::Setup(format!(
            "{}: not an SSH public key",
            beside.display()
        ))),
    }
}

/// Signs `file` with `key` in `namespace`: the signature's text, which
/// `ssh-keygen` writes beside it as `<file>.sig` (taken away here).
fn sign_file(key: &Path, namespace: &str, file: &Path) -> Result<String, ExportError> {
    let sig = PathBuf::from(format!("{}.sig", file.display()));
    let _ = fs::remove_file(&sig);
    ssh_keygen(
        &[
            "-Y",
            "sign",
            "-f",
            &shown(key),
            "-n",
            namespace,
            &shown(file),
        ],
        None,
    )?;
    let text = fs::read_to_string(&sig)
        .map_err(|e| ExportError::Build(format!("read {}: {e}", sig.display())))?;
    let _ = fs::remove_file(&sig);
    Ok(text)
}

/// Whether `signature` is `signer`'s over `bytes` in `namespace`.
pub fn verifies(signer: &str, namespace: &str, bytes: &[u8], signature: &str) -> bool {
    let dir = scratch("verify");
    let allowed = dir.join("allowed_signers");
    let sig = dir.join("file.sig");
    let written = fs::create_dir_all(&dir)
        .and_then(|()| fs::write(&allowed, format!("shuyi {signer}\n")))
        .and_then(|()| fs::write(&sig, signature));
    let ok = written.is_ok()
        && ssh_keygen(
            &[
                "-Y",
                "verify",
                "-f",
                &shown(&allowed),
                "-I",
                "shuyi",
                "-n",
                namespace,
                "-s",
                &shown(&sig),
            ],
            Some(bytes),
        )
        .is_ok();
    let _ = fs::remove_dir_all(&dir);
    ok
}

/// A folder of this process's own under the system's temporary folder.
fn scratch(what: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("shuyi-{what}-{}-{n}", std::process::id()))
}

/// The 32 bits of the code of `recipient`'s copy of the build whose
/// package's SHA-256 is `build`, as `key` signs them (`try` counts up when
/// one is taken): only the key's holder can draw them, and a key that signs
/// the same way twice (Ed25519) draws the same.
pub fn draw_code(key: &Path, recipient: &str, build: &str, round: u32) -> Result<u32, ExportError> {
    let dir = scratch("code");
    let file = dir.join("code.txt");
    fs::create_dir_all(&dir)
        .and_then(|()| fs::write(&file, format!("{recipient}\n{build}\n{round}\n")))
        .map_err(|e| ExportError::Build(format!("write {}: {e}", file.display())))?;
    let signed = sign_file(key, CODE_NAMESPACE, &file);
    let _ = fs::remove_dir_all(&dir);
    let digest = sha256::hex(signed?.as_bytes());
    // The digest is 64 hex digits.
    Ok(u32::from_str_radix(&digest[..8], 16).unwrap_or(0))
}

/// Writes `ledger` to `path` and signs it with `key` (`recipients.ron.sig`).
pub fn save(path: &Path, ledger: &Ledger, key: &Path) -> Result<(), ExportError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|e| ExportError::Build(format!("create {}: {e}", dir.display())))?;
    }
    fs::write(path, text(ledger))
        .map_err(|e| ExportError::Build(format!("write {}: {e}", path.display())))?;
    let signature = sign_file(key, NAMESPACE, path)?;
    let sig = signature_path(path);
    fs::write(&sig, signature)
        .map_err(|e| ExportError::Build(format!("write {}: {e}", sig.display())))
}

/// Whether the ledger at `path` verifies: signed by its `signer`, over its
/// bytes as they are. `None` when it has no signature.
pub fn check(path: &Path, ledger: &Ledger) -> Option<bool> {
    let signature = fs::read_to_string(signature_path(path)).ok()?;
    let bytes = fs::read(path).ok()?;
    Some(verifies(&ledger.signer, NAMESPACE, &bytes, &signature))
}

#[cfg(test)]
pub(crate) mod tests;
