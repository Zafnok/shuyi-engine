//! The stage's look as text: the frame after a script, compared with the
//! snapshot kept in `tests/snapshots/stage.txt`. With `SNAPSHOTS=write`
//! the test writes the file instead: read it, and keep it only if the
//! change was meant. A test of what happened reads the state, never this.

use std::fs;
use std::path::Path;

#[test]
fn the_stage_after_one_step_looks_as_kept() {
    let mut h = game::game().harness();
    h.keys("Enter Right");
    let frame = h.snapshot();
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/stage.txt");
    if std::env::var_os("SNAPSHOTS").is_some_and(|v| v == "write") {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, &frame).unwrap();
    }
    let kept = fs::read_to_string(&file)
        .unwrap_or_else(|e| panic!("{}: {e}: write it with SNAPSHOTS=write", file.display()));
    assert_eq!(
        frame,
        kept.replace("\r\n", "\n"),
        "the frame differs from {}",
        file.display()
    );
}
