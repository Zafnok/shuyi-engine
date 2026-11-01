//! Writes a fixture's capture as numbered PNG files in a folder.

use std::fs;
use std::io::Write;
use std::path::Path;

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [game, seed, actions, out] = args.as_slice() else {
        return Err(
            "usage: clip-capture <courier|workshop> <u64-seed> <comma-separated-actions> <folder>"
                .to_owned(),
        );
    };
    let seed = seed
        .parse::<u64>()
        .map_err(|error| format!("seed: {error}"))?;
    let actions: Vec<&str> = actions.split(',').map(str::trim).collect();
    let out = Path::new(out);
    fs::create_dir_all(out).map_err(|error| error.to_string())?;
    for (index, png) in clip_capture::capture(game, seed, &actions)?
        .iter()
        .enumerate()
    {
        fs::write(out.join(format!("{index:03}.png")), png).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        let _ = writeln!(std::io::stderr().lock(), "{error}");
        std::process::exit(2);
    }
}
