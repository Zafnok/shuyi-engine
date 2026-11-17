use super::*;

/// A new empty folder for test `name`.
pub(crate) fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-ledger-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

/// A fresh Ed25519 key with no passphrase in a new folder for test `name`:
/// the folder and the key's path.
pub(crate) fn new_key(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp(name);
    let key = dir.join("signing_key");
    ssh_keygen(
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "test",
            "-f",
            &shown(&key),
        ],
        None,
    )
    .expect("ssh-keygen makes a key");
    (dir, key)
}

fn entry(recipient: &str) -> Entry {
    Entry {
        recipient: recipient.to_owned(),
        label: format!("{recipient}, Example Press"),
        date: "2026-11-14".to_owned(),
        code: "7K2Q9MXA".to_owned(),
        target: "windows".to_owned(),
        version: "0.1.0".to_owned(),
        build: "ab".repeat(32),
        overlay: "cd".repeat(32),
        kit: "ef".repeat(32),
        ..Entry::default()
    }
}

#[test]
fn a_saved_ledger_reads_back_and_verifies_until_it_changes() {
    let (dir, key) = new_key("saved");
    let path = dir.join(FILE);
    let ledger = Ledger {
        about: ABOUT.to_owned(),
        signer: public_key(&key).expect("its public key"),
        entries: vec![entry("jane")],
    };
    save(&path, &ledger, &key).expect("saved and signed");
    let back = load(&path).expect("reads").expect("is there");
    assert_eq!(back, ledger);
    assert_eq!(check(&path, &back), Some(true));
    let changed = fs::read_to_string(&path)
        .expect("text")
        .replace("jane", "joan");
    fs::write(&path, changed).expect("written");
    assert_eq!(check(&path, &back), Some(false));
    fs::remove_file(signature_path(&path)).expect("removed");
    assert_eq!(check(&path, &back), None);
}

#[test]
fn another_keys_signature_doesnt_verify() {
    let (dir, key) = new_key("another-a");
    let (_other_dir, other) = new_key("another-b");
    let path = dir.join(FILE);
    let ledger = Ledger {
        about: ABOUT.to_owned(),
        signer: public_key(&other).expect("its public key"),
        entries: vec![entry("jane")],
    };
    save(&path, &ledger, &key).expect("signed with the first key");
    assert_eq!(check(&path, &ledger), Some(false));
}

#[test]
fn a_code_is_drawn_the_same_by_one_key_and_differently_by_another() {
    let (_dir, key) = new_key("drawn-a");
    let (_other_dir, other) = new_key("drawn-b");
    let build = "ab".repeat(32);
    let one = draw_code(&key, "jane", &build, 0).expect("drawn");
    assert_eq!(draw_code(&key, "jane", &build, 0).expect("drawn"), one);
    assert_ne!(draw_code(&key, "jane", &build, 1).expect("drawn"), one);
    assert_ne!(draw_code(&key, "joan", &build, 0).expect("drawn"), one);
    assert_ne!(draw_code(&other, "jane", &build, 0).expect("drawn"), one);
}

#[test]
fn the_ledger_sits_beside_the_kit() {
    assert_eq!(
        path_beside(Path::new("game/dist/reveal")),
        Path::new("game/dist").join(FILE)
    );
}
