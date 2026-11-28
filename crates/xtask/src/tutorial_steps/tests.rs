use super::*;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh folder for test `name`, outside the repository.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "xtask-tutorial-steps-{name}-{}",
        std::process::id()
    ));
    remove(&dir).unwrap();
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn tree(files: &[(&str, &str)]) -> Tree {
    files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec()))
        .collect()
}

/// A root holding a copy of the real steps and their source.
fn copy_of_the_steps(name: &str) -> PathBuf {
    let root = fresh(name);
    for dir in [STEPS, SOURCE] {
        write_tree(&root.join(dir), &read_tree(&repo().join(dir)).unwrap()).unwrap();
    }
    root
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn the_modes_are_write_check_record_and_locks() {
    let args = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(Mode::parse(&args(&["--write"])), Some(Mode::Write));
    assert_eq!(Mode::parse(&args(&["--check"])), Some(Mode::Check));
    assert_eq!(Mode::parse(&args(&["--locks"])), Some(Mode::Locks));
    assert_eq!(
        Mode::parse(&args(&["--record", "03"])),
        Some(Mode::Record("03".to_owned()))
    );
    assert_eq!(Mode::parse(&args(&["--record", "3"])), None);
    assert_eq!(Mode::parse(&args(&["--record"])), None);
    assert_eq!(Mode::parse(&args(&[])), None);
    assert_eq!(Mode::parse(&args(&["--check", "--write"])), None);
}

#[test]
fn a_patch_applied_in_a_scratch_tree_makes_the_new_tree() {
    let root = fresh("apply");
    let old = tree(&[
        ("src/lib.rs", "fn a() {}\n\nfn b() {}\n"),
        ("gone.txt", "bye\n"),
    ]);
    let new = tree(&[
        ("src/lib.rs", "fn a() {}\n\nfn c() {}\n"),
        ("src/more/new.rs", "pub fn new() {}\n"),
    ]);
    let patch = diff(&root.join("diff"), ("01-a", &old), ("02-b", &new)).unwrap();
    assert!(
        patch.contains("--- 01-a/src/lib.rs\n+++ 02-b/src/lib.rs\n"),
        "{patch}"
    );
    assert!(
        !patch.contains("\nindex "),
        "a text file's patch names no blob: {patch}"
    );
    let file = root.join("02-b.patch");
    fs::write(&file, &patch).unwrap();
    let work = root.join("work");
    write_tree(&work, &old).unwrap();
    apply(&work, &file).unwrap();
    assert_eq!(read_tree(&work).unwrap(), new);
    // The same patch again doesn't apply, and says which.
    let again = apply(&work, &file).unwrap_err();
    assert!(again.contains("02-b.patch: doesn't apply"), "{again}");
    remove(&root).unwrap();
}

#[test]
fn clean_keeps_a_binary_files_index_line_only() {
    let patch = "\
diff --git a/x.rs b/x.rs
index 1111..2222 100644
--- a/x.rs
+++ b/x.rs
@@ -1 +1 @@
-index 1
+index 2
diff --git a/p.png b/p.png
index 3333..4444 100644
GIT binary patch
literal 1
";
    let cleaned = clean(patch);
    assert!(!cleaned.contains("index 1111"), "{cleaned}");
    // A changed line that starts `index ` is the file's, kept.
    assert!(cleaned.contains("\n-index 1\n+index 2\n"), "{cleaned}");
    assert!(
        cleaned.contains("index 3333..4444 100644\nGIT binary patch"),
        "{cleaned}"
    );
    assert_eq!(clean(""), "");
}

#[test]
fn the_first_difference_names_the_file_and_the_fix() {
    let want = tree(&[("a.rs", "a"), ("b.rs", "b"), ("c.rs", "c")]);
    assert_eq!(
        first_difference("site/tutorial/03-walk", &want, &want),
        None
    );
    let changed = tree(&[("a.rs", "a"), ("b.rs", "B"), ("c.rs", "C")]);
    let error = first_difference("site/tutorial/03-walk", &want, &changed).unwrap();
    assert!(
        error.starts_with("site/tutorial/03-walk/b.rs: not what the source makes"),
        "{error}"
    );
    assert!(error.contains("--record 03"), "{error}");
    let missing = tree(&[("a.rs", "a"), ("c.rs", "c")]);
    let error = first_difference("site/tutorial/03-walk", &want, &missing).unwrap();
    assert!(
        error.starts_with("site/tutorial/03-walk/b.rs: missing"),
        "{error}"
    );
    let extra = tree(&[("a.rs", "a"), ("aa.rs", ""), ("b.rs", "b"), ("c.rs", "c")]);
    let error = first_difference("site/tutorial/03-walk", &want, &extra).unwrap();
    assert!(
        error.starts_with("site/tutorial/03-walk/aa.rs: not made by the source"),
        "{error}"
    );
    let lock = tree(&[("Cargo.lock", "old")]);
    let error = first_difference(
        "site/tutorial/03-walk",
        &tree(&[("Cargo.lock", "new")]),
        &lock,
    )
    .unwrap();
    assert!(
        error.contains("site/tutorial-steps/locks/02-board/Cargo.lock"),
        "{error}"
    );
}

#[test]
fn every_step_takes_one_lock_and_every_lock_is_kept() {
    let steps = steps(&repo()).unwrap();
    assert_eq!(steps.first().map(String::as_str), Some(FIRST));
    let listed: Vec<&str> = LOCK_GROUPS
        .iter()
        .flat_map(|(_, steps)| steps.iter().copied())
        .collect();
    assert_eq!(listed, steps, "LOCK_GROUPS lists each step once, in order");
    for (lock, group) in LOCK_GROUPS {
        assert_eq!(
            group.first(),
            Some(lock),
            "a lock is named after its first step"
        );
        assert!(repo().join(LOCKS).join(lock).join(LOCK).is_file(), "{lock}");
    }
    assert_eq!(lock_of("05-undo"), Some("02-board"));
    assert_eq!(lock_of("11-more"), None);
}

#[test]
fn a_step_folder_with_no_patch_is_named() {
    let root = fresh("no-patch");
    fs::create_dir_all(root.join(PATCHES)).unwrap();
    fs::write(root.join(PATCHES).join("02-board.patch"), "").unwrap();
    for step in ["01-new", "02-board", "03-walk"] {
        write_tree(&root.join(STEPS).join(step), &tree(&[("shuyi.ron", "()")])).unwrap();
    }
    let error = steps(&root).unwrap_err();
    assert!(
        error.starts_with("site/tutorial/03-walk: a step with no patch"),
        "{error}"
    );
    fs::write(root.join(PATCHES).join("board.patch"), "").unwrap();
    let error = steps(&root).unwrap_err();
    assert!(error.contains("board.patch: a patch is named"), "{error}");
    remove(&root).unwrap();
}

#[test]
fn mirror_removes_what_the_tree_hasnt_and_keeps_the_builds() {
    let root = fresh("mirror");
    write_tree(
        &root,
        &tree(&[
            ("keep.rs", "same"),
            ("edit.rs", "old"),
            ("old/deep/gone.rs", "x"),
            ("target/debug/build", "a build"),
        ]),
    )
    .unwrap();
    mirror(
        &root,
        &tree(&[("keep.rs", "same"), ("edit.rs", "new"), ("new/one.rs", "1")]),
    )
    .unwrap();
    assert_eq!(
        read_tree(&root).unwrap(),
        tree(&[("edit.rs", "new"), ("keep.rs", "same"), ("new/one.rs", "1")])
    );
    assert!(!root.join("old").exists(), "the emptied folders go too");
    assert!(root.join("target/debug/build").is_file(), "a build stays");
    remove(&root).unwrap();
}

/// The real steps, copied: `--check` passes on them as checked in, fails
/// on an edit not recorded, naming the file, and `--write` puts it back.
#[test]
fn check_and_write_on_a_copy_of_the_steps() {
    let root = copy_of_the_steps("check");
    assert_eq!(run(&root, &Mode::Check), Vec::<String>::new());
    let rules = "site/tutorial/05-undo/crates/game/src/rules.rs";
    let text = read(&root, rules);
    fs::write(root.join(rules), format!("{text}// an edit\n")).unwrap();
    let errors = run(&root, &Mode::Check);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with(&format!("{rules}: not what the source makes")),
        "{errors:?}"
    );
    let lock = "site/tutorial/08-polish/Cargo.lock";
    fs::write(root.join(lock), "").unwrap();
    assert_eq!(run(&root, &Mode::Check).len(), 2);
    assert_eq!(run(&root, &Mode::Write), Vec::<String>::new());
    assert_eq!(read(&root, rules), text);
    assert_eq!(run(&root, &Mode::Check), Vec::<String>::new());
    remove(&root).unwrap();
}

/// A line changed in `01-new` and `--write`: every later step that has the
/// line has it changed.
#[test]
fn a_line_changed_in_the_first_step_reaches_every_step_with_it() {
    let root = copy_of_the_steps("first");
    let steps = steps(&root).unwrap();
    let file = "crates/game/src/main.rs";
    let first = read(&root, &format!("{STEPS}/{FIRST}/{file}"));
    // A line every step keeps as the first has it.
    let line = first
        .lines()
        .find(|line| {
            !line.trim().is_empty()
                && first.matches(*line).count() == 1
                && steps.iter().all(|step| {
                    read(&root, &format!("{STEPS}/{step}/{file}"))
                        .matches(*line)
                        .count()
                        == 1
                })
        })
        .unwrap()
        .to_owned();
    let changed = format!("{line} // changed once");
    fs::write(
        root.join(STEPS).join(FIRST).join(file),
        first.replace(&line, &changed),
    )
    .unwrap();
    assert_eq!(run(&root, &Mode::Write), Vec::<String>::new());
    for step in &steps {
        let text = read(&root, &format!("{STEPS}/{step}/{file}"));
        assert!(text.contains(&changed), "{step}");
    }
    assert_eq!(run(&root, &Mode::Check), Vec::<String>::new());
    remove(&root).unwrap();
}

/// `--record 03` after an edit in `03-walk`: patches 03 and 04 make 03
/// with the edit and 04 as it was.
#[test]
fn record_writes_the_steps_patch_and_the_next() {
    let root = copy_of_the_steps("record");
    let rules = "site/tutorial/03-walk/crates/game/src/rules.rs";
    let edited = format!("{}// recorded\n", read(&root, rules));
    fs::write(root.join(rules), &edited).unwrap();
    let four = read_tree(&root.join(STEPS).join("04-push")).unwrap();
    let five = read(&root, "site/tutorial-steps/patches/05-undo.patch");
    assert_eq!(
        run(&root, &Mode::Record("03".to_owned())),
        Vec::<String>::new()
    );
    assert_eq!(run(&root, &Mode::Check), Vec::<String>::new());
    assert_eq!(run(&root, &Mode::Write), Vec::<String>::new());
    assert_eq!(read(&root, rules), edited);
    assert_eq!(read_tree(&root.join(STEPS).join("04-push")).unwrap(), four);
    assert!(read(&root, "site/tutorial-steps/patches/03-walk.patch").contains("+// recorded"));
    assert_eq!(
        read(&root, "site/tutorial-steps/patches/05-undo.patch"),
        five
    );
    let error = run(&root, &Mode::Record("42".to_owned()));
    assert!(error[0].contains("no step 42"), "{error:?}");
    remove(&root).unwrap();
}

/// `--locks` keeps a group's lock when its steps agree, and refuses when
/// they don't.
#[test]
fn locks_are_kept_from_steps_that_agree() {
    let root = copy_of_the_steps("locks");
    let group = |step: &str| format!("{STEPS}/{step}/Cargo.lock");
    for step in ["07-save", "08-polish", "09-test", "10-ship"] {
        fs::write(root.join(group(step)), "refreshed\n").unwrap();
    }
    assert_eq!(run(&root, &Mode::Locks), Vec::<String>::new());
    assert_eq!(
        read(&root, "site/tutorial-steps/locks/07-save/Cargo.lock"),
        "refreshed\n"
    );
    assert_eq!(run(&root, &Mode::Check), Vec::<String>::new());
    fs::write(root.join(group("09-test")), "other\n").unwrap();
    let error = run(&root, &Mode::Locks);
    assert!(
        error[0]
            .starts_with("site/tutorial/09-test/Cargo.lock isn't site/tutorial/07-save/Cargo.lock"),
        "{error:?}"
    );
    remove(&root).unwrap();
}

/// A patch that no longer applies: `--write` makes the steps before it,
/// and names it and how to fix it; `--check` names it too.
#[test]
fn write_stops_at_a_patch_that_no_longer_applies() {
    let root = copy_of_the_steps("stops");
    let patches = root.join(PATCHES);
    // Step 03's change, already made in 04: it can't be made again.
    fs::copy(patches.join("03-walk.patch"), patches.join("05-undo.patch")).unwrap();
    let lib = "crates/game/src/main.rs";
    let first = read(&root, &format!("{STEPS}/{FIRST}/{lib}"));
    let line = first
        .lines()
        .find(|l| l.starts_with("//"))
        .unwrap()
        .to_owned();
    let changed = format!("{line} (changed)");
    fs::write(
        root.join(STEPS).join(FIRST).join(lib),
        first.replacen(&line, &changed, 1),
    )
    .unwrap();
    let errors = run(&root, &Mode::Write);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("05-undo.patch: doesn't apply"),
        "{errors:?}"
    );
    assert!(
        errors[0].contains("tutorial-steps --record 05"),
        "{errors:?}"
    );
    assert!(read(&root, &format!("{STEPS}/04-push/{lib}")).contains(&changed));
    assert!(!read(&root, &format!("{STEPS}/05-undo/{lib}")).contains(&changed));
    let errors = run(&root, &Mode::Check);
    assert!(
        errors[0].contains("05-undo.patch: doesn't apply"),
        "{errors:?}"
    );
    remove(&root).unwrap();
}
