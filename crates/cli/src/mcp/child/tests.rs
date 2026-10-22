//! The game's tool program as a host: one that doesn't build stops with
//! its exit code and the last of its output.

use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::contract::Op;
use crate::project::Config;

/// A project in a fresh folder with no `Cargo.toml`, whose `shuyi.ron`
/// says `config`.
fn project(name: &str, config: &str) -> Project {
    let root: PathBuf =
        std::env::temp_dir().join(format!("shuyi-mcp-child-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    Project {
        root,
        config: Config::parse(config).unwrap(),
    }
}

#[test]
fn a_project_with_no_game_program_launches_nothing() {
    let project = project("none", "()");
    let Err((code, error)) = Cargo.launch(&project) else {
        panic!("launched a host of no program");
    };
    assert_eq!(code, codes::PROJECT_FILE);
    assert!(error.contains("names no game tool program"), "{error}");
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn a_program_that_doesnt_build_stops_with_its_exit_code_and_output() {
    let project = project("broken", r#"(game: (package: "no-such-game"))"#);
    let mut host = Cargo
        .launch(&project)
        .unwrap_or_else(|(_, e)| panic!("{e}"));
    let request = Request {
        op: Op::Start,
        input: json!({}),
    };
    let error = host.call(&request).unwrap_err();
    assert!(
        error.starts_with("the game's tool program stopped before it answered (exit code "),
        "{error}"
    );
    // Cargo says why: there is no `Cargo.toml` here.
    assert!(
        error.contains("): ") && error.contains("Cargo.toml"),
        "{error}"
    );
    // It has stopped: a second call says so again.
    assert!(
        host.call(&request)
            .unwrap_err()
            .starts_with("the game's tool program stopped")
    );
    drop(host);
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn the_tail_keeps_the_last_bytes_and_copies_them_all() {
    let tail = Mutex::new(Vec::new());
    let output = vec![b'a'; TAIL]
        .into_iter()
        .chain(*b"end")
        .collect::<Vec<u8>>();
    keep_tail(output.as_slice(), &tail);
    let kept = tail.into_inner().unwrap();
    assert_eq!(kept.len(), TAIL);
    assert!(kept.ends_with(b"aend"));
}
