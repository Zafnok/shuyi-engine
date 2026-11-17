//! Salted commitments: a digest of a file not yet shown, recorded in the
//! kit, which the file's bytes and a salt kept privately open later.
//!
//! The digest is the SHA-256 of a fixed label, 32 random bytes of salt and
//! the file's bytes. Without the salt, nobody can test a guess at the
//! file against the digest (a short design note is easy to guess); with
//! it, anyone can check that the file is the one committed to, and a file
//! changed by one byte opens nothing.

use std::fs;
use std::io::Read as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::Failed;
use crate::contract::codes;
use crate::sha256::Hasher;

/// What every commitment's hash starts with, so it can't be read as
/// another hash of the same bytes.
const LABEL: &[u8] = b"shuyi reveal commitment v1\0";

/// The salts file's version.
const VERSION: u32 = 1;

/// The length of a salt, in bytes.
const SALT_BYTES: usize = 32;

/// The salts file: each commitment with its salt. Private: whoever holds
/// it can test guesses at the files committed to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Salts {
    /// The file's version.
    pub version: u32,
    /// The commitments, oldest first.
    pub commitments: Vec<Salted>,
}

/// A commitment and its salt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Salted {
    /// Its label (the file's path when it was made).
    pub label: String,
    /// The salt, 32 bytes in hexadecimal.
    pub salt: String,
    /// The digest the kit records.
    pub sha256: String,
}

/// The digest of the file at `path` under `salt`.
fn digest_of(path: &Path, salt: &[u8]) -> Result<String, Failed> {
    let failed =
        |e: std::io::Error| Failed::new(codes::FILE, format!("can't read {}: {e}", path.display()));
    let mut file = fs::File::open(path).map_err(failed)?;
    let mut hasher = Hasher::default();
    hasher.update(LABEL);
    hasher.update(salt);
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = file.read(&mut buffer).map_err(failed)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.hex())
}

/// `salt` read from hexadecimal, or why not.
fn salt_of(hex: &str) -> Result<Vec<u8>, Failed> {
    let wrong = || {
        Failed::new(
            codes::WRONG_ARGUMENTS,
            format!("the salt `{hex}` isn't 32 bytes in hexadecimal (64 digits)"),
        )
    };
    let hex = hex.trim();
    if hex.len() != SALT_BYTES * 2 || !hex.is_ascii() {
        return Err(wrong());
    }
    hex.as_bytes()
        .chunks(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).map_err(|_| wrong())?;
            u8::from_str_radix(pair, 16).map_err(|_| wrong())
        })
        .collect()
}

/// A new salt of [`SALT_BYTES`], from the system's random source.
fn fresh_salt() -> Result<Vec<u8>, Failed> {
    let mut salt = Vec::with_capacity(SALT_BYTES);
    while salt.len() < SALT_BYTES {
        let word = getrandom::u64().map_err(|e| {
            Failed::new(
                codes::FILE,
                format!("the system's random source failed: {e}"),
            )
        })?;
        salt.extend(word.to_le_bytes());
    }
    Ok(salt)
}

/// `bytes` in lowercase hexadecimal.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        // Writing to a `String` can't fail.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// Whether the file at `path` opens the commitment `digest` with `salt`
/// (both in hexadecimal).
///
/// # Errors
///
/// A salt or digest that isn't hexadecimal of the right length, or a file
/// that can't be read.
pub fn opens(path: &Path, salt: &str, digest: &str) -> Result<bool, Failed> {
    let salt = salt_of(salt)?;
    if !super::kit::is_sha256(digest.trim()) {
        return Err(Failed::new(
            codes::WRONG_ARGUMENTS,
            format!("the digest `{digest}` isn't a SHA-256 in hexadecimal (64 digits)"),
        ));
    }
    Ok(digest_of(path, &salt)?.eq_ignore_ascii_case(digest.trim()))
}

/// The commitment of `salts` that the file at `path` opens, if any.
///
/// # Errors
///
/// A salt that isn't hexadecimal, or a file that can't be read.
pub fn open<'a>(path: &Path, salts: &'a Salts) -> Result<Option<&'a Salted>, Failed> {
    for salted in &salts.commitments {
        if digest_of(path, &salt_of(&salted.salt)?)?.eq_ignore_ascii_case(&salted.sha256) {
            return Ok(Some(salted));
        }
    }
    Ok(None)
}

impl Salts {
    /// The salts file at `path`.
    ///
    /// # Errors
    ///
    /// A file that can't be read ([`codes::FILE`]) or isn't a salts file.
    pub fn load(path: &Path) -> Result<Self, Failed> {
        let text = fs::read_to_string(path).map_err(|e| {
            Failed::new(
                codes::FILE,
                format!("can't read the salts file {}: {e}", path.display()),
            )
        })?;
        let salts: Self = ron::from_str(&text).map_err(|e| {
            Failed::new(
                codes::FILE,
                format!("{} isn't a salts file: {e}", path.display()),
            )
        })?;
        if salts.version > VERSION {
            return Err(Failed::new(
                codes::FILE,
                format!(
                    "{} is version {}, newer than this shuyi reads ({VERSION})",
                    path.display(),
                    salts.version
                ),
            ));
        }
        Ok(salts)
    }

    /// The salts file at `path`, or a new one if there is none.
    ///
    /// # Errors
    ///
    /// As [`Salts::load`], for a file that is there.
    pub fn load_or_new(path: &Path) -> Result<Self, Failed> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self {
                version: VERSION,
                commitments: Vec::new(),
            })
        }
    }

    /// A new commitment to the file at `path`, labelled `label`, with a
    /// fresh salt from the system's random source.
    ///
    /// # Errors
    ///
    /// A label already committed ([`codes::WRONG_ARGUMENTS`]), a file that
    /// can't be read, or no random source.
    pub fn commit(&mut self, path: &Path, label: &str) -> Result<&Salted, Failed> {
        if self.commitments.iter().any(|salted| salted.label == label) {
            return Err(Failed::new(
                codes::WRONG_ARGUMENTS,
                format!(
                    "`{label}` is committed to already: a commitment is never replaced \
                     (one published may be opened later)"
                ),
            ));
        }
        let salt = fresh_salt()?;
        let sha256 = digest_of(path, &salt)?;
        let at = self.commitments.len();
        self.commitments.push(Salted {
            label: label.to_owned(),
            salt: hex(&salt),
            sha256,
        });
        Ok(&self.commitments[at])
    }

    /// Writes the salts file to `path`.
    ///
    /// # Errors
    ///
    /// A file that can't be written.
    pub fn save(&self, path: &Path) -> Result<(), Failed> {
        let pretty = ron::ser::PrettyConfig::default().new_line("\n");
        let mut text = ron::ser::to_string_pretty(self, pretty)
            .map_err(|e| Failed::new(codes::FILE, format!("can't write the salts: {e}")))?;
        text.push('\n');
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|e| {
                Failed::new(codes::FILE, format!("can't make {}: {e}", parent.display()))
            })?;
        }
        fs::write(path, text)
            .map_err(|e| Failed::new(codes::FILE, format!("can't write {}: {e}", path.display())))
    }
}
