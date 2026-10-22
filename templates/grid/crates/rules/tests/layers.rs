//! The rules stay pure (AGENTS.md, "The layers"): nothing below them draws,
//! touches a device or rolls luck of its own.

use std::collections::BTreeSet;
use std::process::Command;

/// The crates the rules never depend on, even through another crate: the
/// engine's screens, its backend and its tools, the device crates, and
/// luck that isn't the `SimRng` kept in the state. A trailing `*` is a
/// prefix. The device crates, from `macroquad` to `hound`, are a copy of
/// `devices` in the engine's `tools/layers.ron`.
const NEVER: &[&str] = &[
    "shuyi-ui",
    "shuyi-platform",
    "shuyi-app",
    "shuyi-cli",
    "macroquad",
    "miniquad",
    "quad-*",
    "gilrs",
    "gilrs-core",
    "cpal",
    "rodio",
    "kira",
    "alsa",
    "alsa-sys",
    "coreaudio-rs",
    "coreaudio-sys",
    "oboe",
    "symphonia",
    "lewton",
    "hound",
    "rand",
    "getrandom",
];

fn is_never(name: &str) -> bool {
    NEVER.iter().any(|never| match never.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == *never,
    })
}

#[test]
fn the_rules_reach_no_screen_no_device_and_no_other_luck() {
    let package = env!("CARGO_PKG_NAME");
    let output = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            package,
            "--edges",
            "normal,build",
            "--target",
            "all",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree runs");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout);
    let found: BTreeSet<&str> = tree
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| is_never(name))
        .collect();
    let problems: Vec<String> = found
        .iter()
        .map(|name| {
            format!(
                "{package} depends on {name}: the rules stay pure, so screens, devices and \
                 tools go in crates/game, and the only luck is the SimRng kept in the state. \
                 See the way in with `cargo tree -p {package} -i {name}`."
            )
        })
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
