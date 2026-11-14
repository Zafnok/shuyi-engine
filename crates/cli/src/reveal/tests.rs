use std::fs;
use std::path::{Path, PathBuf};

use super::kit::{
    self, ATTRIBUTES, Build, CAPTURES, CHECKLIST, INDEX, MANIFEST, RECEIPTS, SEAL, VERSION, civil,
    read_manifest,
};
use super::{Asked, Salts, main, open, opens, parse, prepare, verify};
use crate::project::Project;

/// The committed fixture: a game's folder and the kit it prepares.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reveal")
}

/// The digest of the committed kit, as it would be published.
const FIXTURE_DIGEST: &str = include_str!("../../tests/fixtures/reveal/kit/reveal.ron.sha256");

/// The build the committed kit was prepared from.
fn fixture_build() -> Build {
    Build {
        version: Some("0.3.0".into()),
        commit: Some("0123456789abcdef0123456789abcdef01234567".into()),
        dirty: false,
    }
}

/// A new empty folder for test `name`.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-reveal-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Copies the folder `from` into `to`, as another machine would get it.
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            copy_dir(&path, &to.join(entry.file_name()));
        } else {
            fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
}

/// Writes `text` to `root/path`, making its folder.
fn put(root: &Path, path: &str, bytes: &[u8]) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// A game in `root` with a capture and its store re-encode, a demo build,
/// a release's sums and an attestation, and one commitment.
fn game(root: &Path) -> Project {
    put(
        root,
        "shuyi.ron",
        br#"(
    app: (name: "Moon Garden", id: "io.github.example.moon-garden", version: "0.3.0"),
    preview: (args: ["--keys", "Enter", "--every", "0.05", "--count", "4", "--apng"]),
    reveal: (
        store_url: "https://store.example.com/app/1/Moon_Garden/",
        keywords: ["moon", "garden"],
        captures: [
            (file: "preview.png", what: "the walk to the well", encodes: ["shots/preview-store.webp"]),
            (file: "shots/title.png", what: "the title | menu", args: ["--scale", "3"]),
        ],
        artifacts: ["dist/moon-garden-demo.zip"],
        sums: "release/SHA256SUMS",
        receipts: ["release/moon-garden.sigstore.json"],
        salts: "private/reveal-salts.ron",
    ),
)
"#,
    );
    put(root, "preview.png", b"an animated picture");
    put(root, "shots/preview-store.webp", b"the store's copy");
    put(root, "shots/title.png", b"a title");
    put(root, "dist/moon-garden-demo.zip", b"a demo");
    let sum = crate::sha256::hex(b"a package");
    put(
        root,
        "release/SHA256SUMS",
        format!("{sum}  moon-garden-windows.zip\n{sum} *moon-garden-web.zip\n").as_bytes(),
    );
    put(root, "release/moon-garden.sigstore.json", b"{}");
    put(root, "design/boss.txt", b"The last boss is the gardener.");
    let mut salts = Salts::load_or_new(&root.join("private/reveal-salts.ron")).unwrap();
    salts
        .commit(&root.join("design/boss.txt"), "design/boss.txt")
        .unwrap();
    salts.save(&root.join("private/reveal-salts.ron")).unwrap();
    Project::load(root).unwrap()
}

/// No problem, no note.
const NONE: [String; 0] = [];

/// The problems of the kit in `dir`.
fn problems(dir: &Path, digest: Option<&str>) -> Vec<String> {
    verify(dir, digest).problems
}

#[test]
fn a_kit_verifies_after_copying_to_another_folder() {
    let root = temp("copy");
    let project = game(&root.join("game"));
    let dir = root.join("game/dist/reveal");
    let prepared = prepare(&project, &dir, "2026-11-13", &Build::default()).unwrap();
    assert_eq!(problems(&dir, None), NONE);

    // Another machine: the kit alone, with no project and no sources.
    let elsewhere = root.join("elsewhere/kit");
    copy_dir(&dir, &elsewhere);
    fs::remove_dir_all(root.join("game")).unwrap();
    let verified = verify(&elsewhere, Some(&prepared.digest));
    assert_eq!(verified.problems, NONE);
    assert_eq!(verified.digest.as_deref(), Some(prepared.digest.as_str()));
    let manifest = verified.manifest.unwrap();
    assert_eq!(manifest, prepared.manifest);

    // What a store watch reads: the name, the keywords and the captures.
    assert_eq!(manifest.version, VERSION);
    assert_eq!(manifest.game, "Moon Garden");
    assert_eq!(manifest.keywords, ["moon", "garden"]);
    assert_eq!(manifest.captures.len(), 2);
    assert_eq!(manifest.captures[0].original.path, "captures/preview.png");
    assert_eq!(
        manifest.captures[0].original.sha256,
        crate::sha256::hex(b"an animated picture")
    );
    assert_eq!(
        manifest.captures[0].encodes[0].path,
        "captures/preview-store.webp"
    );
    // `preview.png` takes the `preview` part's arguments; another its own.
    assert_eq!(manifest.captures[0].args[..2], ["--keys", "Enter"]);
    assert_eq!(manifest.captures[1].args, ["--scale", "3"]);
    // The demo by its hash, never copied, and the release's two by theirs.
    let names: Vec<&str> = manifest.artifacts.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "moon-garden-demo.zip",
            "moon-garden-windows.zip",
            "moon-garden-web.zip"
        ]
    );
    assert_eq!(manifest.artifacts[0].bytes, Some(6));
    assert_eq!(manifest.artifacts[1].bytes, None);
    assert!(!elsewhere.join("moon-garden-demo.zip").exists());
    let receipts: Vec<&str> = manifest.receipts.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        receipts,
        ["receipts/SHA256SUMS", "receipts/moon-garden.sigstore.json"]
    );
    // The commitment's digest, never its salt.
    assert_eq!(manifest.commitments.len(), 1);
    assert_eq!(manifest.commitments[0].label, "design/boss.txt");
    let manifest_text = fs::read_to_string(elsewhere.join(MANIFEST)).unwrap();
    assert!(!manifest_text.contains("salt"), "{manifest_text}");
    // The seal checks with `sha256sum -c`.
    assert_eq!(
        fs::read_to_string(elsewhere.join(SEAL)).unwrap(),
        format!("{}  reveal.ron\n", prepared.digest)
    );
    assert_eq!(
        fs::read_to_string(elsewhere.join(ATTRIBUTES)).unwrap(),
        "# Every file of the reveal kit is kept byte for byte.\n* -text\n"
    );
}

#[test]
fn changing_a_captured_frame_fails_naming_it() {
    let root = temp("frame");
    let project = game(&root);
    let dir = root.join("kit");
    prepare(&project, &dir, "2026-11-13", &Build::default()).unwrap();
    let frame = dir.join(CAPTURES).join("title.png");
    let mut bytes = fs::read(&frame).unwrap();
    bytes[0] ^= 1;
    fs::write(&frame, bytes).unwrap();
    let found = problems(&dir, None);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("captures/title.png: its SHA-256 is "),
        "{found:?}"
    );
}

#[test]
fn a_missing_receipt_fails_naming_it() {
    let root = temp("missing");
    let project = game(&root);
    let dir = root.join("kit");
    prepare(&project, &dir, "2026-11-13", &Build::default()).unwrap();
    fs::remove_file(dir.join(RECEIPTS).join("SHA256SUMS")).unwrap();
    let found = problems(&dir, None);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("receipts/SHA256SUMS: missing or unreadable"),
        "{found:?}"
    );
}

#[test]
fn changing_a_manifest_field_fails_and_a_new_seal_fails_the_published_digest() {
    let root = temp("field");
    let project = game(&root);
    let dir = root.join("kit");
    let prepared = prepare(&project, &dir, "2026-11-13", &Build::default()).unwrap();
    let manifest = fs::read_to_string(dir.join(MANIFEST)).unwrap();
    let changed = manifest.replace("store.example.com", "store.example.net");
    assert_ne!(manifest, changed);
    fs::write(dir.join(MANIFEST), &changed).unwrap();
    let found = problems(&dir, None);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("the manifest changed after it was sealed"),
        "{found:?}"
    );

    // Sealed again: the folder agrees with itself, and not with the digest
    // published before.
    let resealed = crate::sha256::hex(changed.as_bytes());
    fs::write(dir.join(SEAL), format!("{resealed}  reveal.ron\n")).unwrap();
    assert_eq!(problems(&dir, None), NONE);
    let found = problems(&dir, Some(&prepared.digest));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("not the digest given"), "{found:?}");
}

#[test]
fn an_edited_evidence_index_and_a_file_not_listed_fail() {
    let root = temp("extra");
    let project = game(&root);
    let dir = root.join("kit");
    prepare(&project, &dir, "2026-11-13", &Build::default()).unwrap();
    let index = fs::read_to_string(dir.join(INDEX)).unwrap();
    fs::write(dir.join(INDEX), index.replace("Moon Garden", "Sun Garden")).unwrap();
    put(&dir, "captures/more.png", b"another");
    // The checklist is a working copy: ticking it changes nothing.
    let checklist = fs::read_to_string(dir.join(CHECKLIST)).unwrap();
    fs::write(dir.join(CHECKLIST), checklist.replacen("- [ ]", "- [x]", 1)).unwrap();
    let found = problems(&dir, None);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].starts_with("EVIDENCE.md: its SHA-256 is"),
        "{found:?}"
    );
    assert!(
        found[1].starts_with("captures/more.png: in the folder, and not in the manifest"),
        "{found:?}"
    );
}

#[test]
fn a_manifest_naming_a_path_outside_the_kit_fails_even_sealed() {
    let root = temp("outside");
    let project = game(&root);
    let dir = root.join("kit");
    prepare(&project, &dir, "2026-11-13", &Build::default()).unwrap();
    let manifest = fs::read_to_string(dir.join(MANIFEST)).unwrap();
    for (path, bad) in [
        ("captures/title.png", "captures/../../shuyi.ron"),
        ("captures/title.png", "/etc/title.png"),
        ("captures/title.png", "captures\\\\title.png"),
        ("captures/title.png", "elsewhere/title.png"),
    ] {
        let changed = manifest.replace(&format!("\"{path}\""), &format!("\"{bad}\""));
        assert_ne!(manifest, changed, "{bad}");
        fs::write(dir.join(MANIFEST), &changed).unwrap();
        let digest = crate::sha256::hex(changed.as_bytes());
        fs::write(dir.join(SEAL), format!("{digest}  reveal.ron\n")).unwrap();
        let found = problems(&dir, None);
        assert!(
            found.iter().any(|p| p.contains("not a path in the kit")),
            "{bad}: {found:?}"
        );
    }
}

#[test]
fn a_kit_of_a_newer_version_is_refused_by_its_version() {
    let text = format!("(version: {}, a_field_to_come: 1)", VERSION + 1);
    let e = read_manifest(&text).unwrap_err();
    assert!(e.contains("newer than this shuyi reads"), "{e}");
    assert!(read_manifest("(version: 1)").is_err());
}

#[test]
fn a_store_only_reveal_needs_no_build() {
    let root = temp("store-only");
    put(
        &root,
        "shuyi.ron",
        br#"(
    app: (name: "Moon Garden", id: "io.github.example.moon-garden"),
    reveal: (
        store_url: "https://store.example.com/app/1/",
        captures: [(file: "shot.png", what: "the garden")],
    ),
)
"#,
    );
    put(&root, "shot.png", b"a shot");
    let project = Project::load(&root).unwrap();
    let dir = root.join("kit");
    let prepared = prepare(&project, &dir, "2026-11-13", &Build::default()).unwrap();
    assert_eq!(prepared.manifest.artifacts.len(), 0);
    assert_eq!(prepared.manifest.receipts.len(), 0);
    assert_eq!(prepared.notes, NONE);
    assert_eq!(problems(&dir, Some(&prepared.digest)), NONE);
    let index = fs::read_to_string(dir.join(INDEX)).unwrap();
    assert!(
        index.contains("None: the reveal is a store page, with no build released."),
        "{index}"
    );
    let checklist = fs::read_to_string(dir.join(CHECKLIST)).unwrap();
    assert!(checklist.contains("17 U.S.C. 412"), "{checklist}");
    assert!(checklist.contains("not legal advice"), "{checklist}");
    // No commitment: no step about the salts.
    assert!(!checklist.contains("salts file"), "{checklist}");
}

#[test]
fn prepare_refuses_a_bad_store_page_two_files_of_one_name_and_a_folder_not_a_kit() {
    let root = temp("refuse");
    put(
        &root,
        "shuyi.ron",
        br#"(
    app: (name: "Moon Garden", id: "io.github.example.moon-garden"),
    reveal: (store_url: "http://store.example.com/app/1/"),
)
"#,
    );
    let project = Project::load(&root).unwrap();
    let e = prepare(&project, &root.join("kit"), "2026-11-13", &Build::default()).unwrap_err();
    assert_eq!(e.code, crate::contract::codes::PROJECT_FILE);
    assert!(
        e.message.contains("whole `https://` address"),
        "{}",
        e.message
    );

    put(
        &root,
        "shuyi.ron",
        br#"(
    app: (name: "Moon Garden", id: "io.github.example.moon-garden"),
    reveal: (
        store_url: "https://store.example.com/app/1/",
        captures: [(file: "a/shot.png"), (file: "b/shot.png")],
    ),
)
"#,
    );
    put(&root, "a/shot.png", b"one");
    put(&root, "b/shot.png", b"two");
    let project = Project::load(&root).unwrap();
    let e = prepare(
        &project,
        &root.join("kit2"),
        "2026-11-13",
        &Build::default(),
    )
    .unwrap_err();
    assert!(e.message.contains("`captures/shot.png`"), "{}", e.message);

    // A folder holding other files is never emptied.
    let e = prepare(&project, &root, "2026-11-13", &Build::default()).unwrap_err();
    assert_eq!(e.code, crate::contract::codes::FILE);
    assert!(root.join("a/shot.png").is_file());
}

#[test]
fn prepare_again_replaces_the_kit_whole() {
    let root = temp("again");
    let project = game(&root);
    let dir = root.join("kit");
    let first = prepare(&project, &dir, "2026-11-13", &Build::default()).unwrap();
    put(&dir, "captures/left-over.png", b"old");
    let second = prepare(&project, &dir, "2026-11-14", &Build::default()).unwrap();
    assert_ne!(first.digest, second.digest);
    assert!(!dir.join("captures/left-over.png").exists());
    assert_eq!(problems(&dir, None), NONE);
}

#[test]
fn a_salted_commitment_opens_with_its_bytes_and_salt_and_nothing_else() {
    let root = temp("commit");
    put(&root, "boss.md", b"The last boss is the gardener.");
    let salts_path = root.join("salts.ron");
    let mut salts = Salts::load_or_new(&salts_path).unwrap();
    let made = salts
        .commit(&root.join("boss.md"), "boss.md")
        .unwrap()
        .clone();
    salts.save(&salts_path).unwrap();
    let salts = Salts::load(&salts_path).unwrap();
    assert_eq!(salts.commitments, std::slice::from_ref(&made));

    assert!(opens(&root.join("boss.md"), &made.salt, &made.sha256).unwrap());
    assert_eq!(open(&root.join("boss.md"), &salts).unwrap(), Some(&made));

    // One byte changed opens nothing.
    put(&root, "changed.md", b"The last boss is the gardener!");
    assert!(!opens(&root.join("changed.md"), &made.salt, &made.sha256).unwrap());
    assert_eq!(open(&root.join("changed.md"), &salts).unwrap(), None);
    // Another salt opens nothing, and the digest isn't the bytes' plain hash.
    let other = format!("{}0", &made.salt[..63]);
    let other = if other == made.salt {
        format!("{}1", &made.salt[..63])
    } else {
        other
    };
    assert!(!opens(&root.join("boss.md"), &other, &made.sha256).unwrap());
    assert_ne!(
        made.sha256,
        crate::sha256::hex(b"The last boss is the gardener.")
    );

    // Two commitments to the same bytes differ: each has its own salt.
    put(&root, "same.md", b"The last boss is the gardener.");
    let mut more = Salts::load(&salts_path).unwrap();
    let again = more.commit(&root.join("same.md"), "same.md").unwrap();
    assert_ne!(again.sha256, made.sha256);
    // A label is committed to once.
    let e = more.commit(&root.join("boss.md"), "boss.md").unwrap_err();
    assert!(e.message.contains("never replaced"), "{}", e.message);
    // A salt or digest that isn't hexadecimal is refused.
    assert!(opens(&root.join("boss.md"), "zz", &made.sha256).is_err());
    assert!(opens(&root.join("boss.md"), &made.salt, "abc").is_err());
}

#[test]
fn the_fixtures_commitment_made_elsewhere_opens_with_its_file() {
    // Its salts file was made by Python's `hashlib`, not by this code:
    // SHA-256 of the label, the salt and the bytes.
    let game = fixtures().join("game");
    let salts = Salts::load(&game.join("private/reveal-salts.ron")).unwrap();
    let opened = open(&game.join("design/boss.txt"), &salts).unwrap();
    assert_eq!(
        opened.map(|salted| salted.label.as_str()),
        Some("design/boss.txt")
    );
    assert_eq!(open(&game.join("shuyi.ron"), &salts).unwrap(), None);
}

#[test]
fn the_committed_kit_verifies_with_its_published_digest() {
    // Made on one machine and checked out on each of CI's: another machine.
    let digest = FIXTURE_DIGEST.split_whitespace().next().unwrap();
    let verified = verify(&fixtures().join("kit"), Some(digest));
    assert_eq!(verified.problems, NONE);
    assert_eq!(verified.manifest.unwrap().game, "Moon Garden");
}

#[test]
fn the_committed_kit_is_what_prepare_writes() {
    let project = Project::load(&fixtures().join("game")).unwrap();
    let dir = temp("fixture").join("kit");
    prepare(&project, &dir, "2026-11-13", &fixture_build()).unwrap();
    let committed = fixtures().join("kit");
    if std::env::var_os("SHUYI_BLESS").is_some() {
        let _ = fs::remove_dir_all(&committed);
        copy_dir(&dir, &committed);
    }
    let mut made = Vec::new();
    kit_files(&dir, &dir, &mut made);
    let mut kept = Vec::new();
    kit_files(&committed, &committed, &mut kept);
    assert_eq!(
        made, kept,
        "the committed kit (tests/fixtures/reveal/kit) isn't what prepare writes: \
         rewrite it with `SHUYI_BLESS=1 cargo test -p shuyi-cli --lib reveal`"
    );
}

/// Every file under `dir` with its bytes, by its path from `from`.
fn kit_files(dir: &Path, from: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            kit_files(&path, from, out);
        } else {
            let rel = path.strip_prefix(from).unwrap();
            let rel = rel.to_string_lossy().replace('\\', "/");
            out.push((rel, fs::read(&path).unwrap()));
        }
    }
    out.sort();
}

#[test]
fn the_cli_verifies_anywhere_and_fails_a_changed_kit_with_its_code() {
    let root = temp("cli");
    game(&root.join("game"));
    let args = |list: &[&str]| list.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>();
    assert_eq!(main(&root.join("game"), &args(&["prepare"])), 0);
    let dir = root.join("game/dist/reveal");
    assert!(dir.join(MANIFEST).is_file());
    let digest = fs::read_to_string(dir.join(SEAL)).unwrap();
    let digest = digest.split_whitespace().next().unwrap().to_owned();

    // No project: a folder named, or the folder it runs in.
    assert_eq!(main(&root, &args(&["verify", "game/dist/reveal"])), 0);
    assert_eq!(main(&dir, &args(&["verify", "--digest", &digest])), 0);
    assert_eq!(
        main(&dir, &args(&["verify", "--digest", &"0".repeat(64)])),
        1
    );
    // With the project, its `out`.
    assert_eq!(main(&root.join("game"), &args(&["verify"])), 0);

    put(&dir, "captures/title.png", b"another title");
    assert_eq!(main(&dir, &args(&["verify"])), 1);
    // `prepare` needs the project: a setup to fix.
    assert_eq!(main(&root, &args(&["prepare"])), 3);
    assert_eq!(main(&root, &args(&["frobnicate"])), 2);

    // Commit and open by the CLI, with --salts and with the pair.
    put(&root, "notes.md", b"a secret");
    let salts = "salts.ron";
    assert_eq!(
        main(&root, &args(&["commit", "notes.md", "--salts", salts])),
        0
    );
    assert_eq!(
        main(&root, &args(&["open", "notes.md", "--salts", salts])),
        0
    );
    let kept = Salts::load(&root.join(salts)).unwrap();
    let made = &kept.commitments[0];
    let pair = [
        "open",
        "notes.md",
        "--salt",
        &made.salt,
        "--digest",
        &made.sha256,
    ];
    assert_eq!(main(&root, &args(&pair)), 0);
    put(&root, "notes.md", b"a secret!");
    assert_eq!(
        main(&root, &args(&["open", "notes.md", "--salts", salts])),
        1
    );
}

#[test]
fn the_arguments_read_as_asked() {
    let parsed = |list: &[&str]| parse(&list.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>());
    assert_eq!(parsed(&["--help"]), Ok(Asked::Help));
    assert_eq!(parsed(&["verify", "-h"]), Ok(Asked::Help));
    assert_eq!(
        parsed(&["prepare", "--out", "kit"]),
        Ok(Asked::Prepare {
            out: Some("kit".into())
        })
    );
    assert_eq!(
        parsed(&["verify", "kit", "--digest", "ab"]),
        Ok(Asked::Verify {
            dir: Some("kit".into()),
            digest: Some("ab".into())
        })
    );
    assert_eq!(
        parsed(&["commit", "a", "b"]),
        Ok(Asked::Commit {
            files: vec!["a".into(), "b".into()],
            salts: None
        })
    );
    for wrong in [
        &[][..],
        &["publish"],
        &["prepare", "file"],
        &["prepare", "--digest", "ab"],
        &["verify", "a", "b"],
        &["verify", "--digest"],
        &["verify", "--digest", "a", "--digest", "b"],
        &["commit"],
        &["open"],
        &["open", "a", "--salt", "00"],
        &[
            "open", "a", "--salts", "s", "--salt", "00", "--digest", "ab",
        ],
        &["verify", "--frob"],
    ] {
        assert!(parsed(wrong).is_err(), "{wrong:?}");
    }
}

#[test]
fn days_count_from_1970_as_the_calendar_does() {
    assert_eq!(civil(0), (1970, 1, 1));
    assert_eq!(civil(59), (1970, 3, 1));
    assert_eq!(civil(11_016), (2000, 2, 29));
    assert_eq!(civil(20_770), (2026, 11, 13));
    assert_eq!(civil(-1), (1969, 12, 31));
    assert_eq!(kit::today().len(), 10);
}

#[test]
fn the_sums_of_a_release_read_as_sha256sum_writes_them() {
    let root = temp("sums");
    put(
        &root,
        "shuyi.ron",
        br#"(
    app: (name: "Moon Garden", id: "io.github.example.moon-garden"),
    reveal: (store_url: "https://store.example.com/app/1/", sums: "SHA256SUMS"),
)
"#,
    );
    put(&root, "SHA256SUMS", b"not a sum\n");
    let project = Project::load(&root).unwrap();
    let e = prepare(&project, &root.join("kit"), "2026-11-13", &Build::default()).unwrap_err();
    assert!(e.message.contains("SHA256SUMS, line 1"), "{}", e.message);
}
