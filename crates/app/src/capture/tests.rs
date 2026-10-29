//! The capture mode's pure parts: the console cut out of a window read
//! back bottom row first, and what a report writes.

use shuyi_ui::frame::raster::{Tolerance, compare};

use super::*;

/// A 4 × 3 window read back bottom row first: pixel `(x, y)` is
/// `[x, y, 10 * y + x, 0]` (see-through, as the window reads back).
fn window_4x3() -> Vec<u8> {
    (0..3u8)
        .rev()
        .flat_map(|y| (0..4u8).flat_map(move |x| [x, y, 10 * y + x, 0]))
        .collect()
}

fn pixel(image: &raster::Image, x: u32, y: u32) -> [u8; 4] {
    image.pixel(x, y).unwrap()
}

#[test]
fn the_console_is_cut_out_upright_and_opaque() {
    let all = cut_out(&window_4x3(), (4, 3), (0, 0), (4, 3)).unwrap();
    assert_eq!((all.width, all.height), (4, 3));
    assert_eq!(pixel(&all, 0, 0), [0, 0, 0, 255]);
    assert_eq!(pixel(&all, 3, 2), [3, 2, 23, 255]);
    let part = cut_out(&window_4x3(), (4, 3), (1, 1), (2, 2)).unwrap();
    assert_eq!((part.width, part.height), (2, 2));
    let rows: Vec<[u8; 4]> = part.rgba.as_chunks::<4>().0.to_vec();
    assert_eq!(
        rows,
        [
            [1, 1, 11, 255],
            [2, 1, 12, 255],
            [1, 2, 21, 255],
            [2, 2, 22, 255]
        ]
    );
}

#[test]
fn a_console_outside_the_window_or_a_short_read_is_an_error() {
    let window = window_4x3();
    for (at, size) in [((-1, 0), (2, 2)), ((3, 0), (2, 1)), ((0, 2), (1, 2))] {
        let error = cut_out(&window, (4, 3), at, size).unwrap_err();
        assert!(error.contains("isn't all in the window"), "{error}");
    }
    let error = cut_out(&window[4..], (4, 3), (0, 0), (1, 1)).unwrap_err();
    assert!(error.contains("read back 44 bytes"), "{error}");
}

fn solid(width: u32, height: u32, rgb: [u8; 3]) -> raster::Image {
    raster::Image::filled(width, height, Rgb::new(rgb[0], rgb[1], rgb[2]))
}

#[test]
fn a_report_writes_each_capture_and_a_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut report = Report::new(dir.path().join("out")).unwrap();
    let read = |file: &str| std::fs::read(dir.path().join("out").join(file));
    assert_eq!(read("capture.txt").unwrap(), b"");

    let reference = solid(4, 2, [10, 20, 30]);
    let close = solid(4, 2, [11, 20, 30]);
    let passed = compare(&close, &reference, Tolerance::default());
    assert!(report.add("title", &close, &reference, &passed, 3).unwrap());
    assert_eq!(report.exit_code(), PASSED);

    let far = solid(4, 2, [60, 20, 30]);
    let failed = compare(&far, &reference, Tolerance::default());
    assert!(!report.add("stage", &far, &reference, &failed, 3).unwrap());
    assert_eq!(report.exit_code(), FAILED);
    assert_eq!(report.captured(), 2);
    assert!(report.summary().starts_with("1 of 2 captures passed"));

    let text = String::from_utf8(read("capture.txt").unwrap()).unwrap();
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "title: 4 × 2 at scale 3; 0 pixels over (channel 2, pixels 0); worst 1; first none; passed",
            "stage: 4 × 2 at scale 3; 8 pixels over (channel 2, pixels 0); worst 50; first 0, 0; FAILED",
        ]
    );
    for file in ["title.window.png", "title.raster.png", "stage.diff.png"] {
        let png = read(file).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "{file}");
    }
    assert!(read("title.diff.png").is_err());
    let window = image::load_from_memory(&read("stage.window.png").unwrap()).unwrap();
    assert_eq!(window.to_rgba8().into_raw(), far.rgba);
}

#[test]
fn a_line_names_both_sizes_when_they_differ() {
    let reference = solid(4, 2, [0, 0, 0]);
    let window = solid(3, 2, [0, 0, 0]);
    let comparison = compare(&window, &reference, Tolerance::new(0, 5));
    assert_eq!(
        line("card", &window, &reference, &comparison, 1),
        "card: window 3 × 2, raster 4 × 2 at scale 1; 0 pixels over (channel 0, pixels 5); worst 0; first none; FAILED"
    );
}

/// The web's console has the lines alone (ticket 2019): each is the line
/// of `capture.txt`, after the prefix `check.py` looks for.
#[test]
fn a_console_line_is_the_files_line_after_the_prefix() {
    let reference = solid(4, 2, [10, 20, 30]);
    let close = solid(4, 2, [11, 20, 30]);
    let far = solid(4, 2, [60, 20, 30]);
    let narrow = solid(3, 2, [10, 20, 30]);
    let cases = [
        (&close, Tolerance::default(), "passed"),
        (&far, Tolerance::default(), "FAILED"),
        (&narrow, Tolerance::new(0, 5), "FAILED"),
    ];
    for (window, tolerance, verdict) in cases {
        let comparison = compare(window, &reference, tolerance);
        let file_line = line("title", window, &reference, &comparison, 3);
        let logged = console(&file_line);
        assert_eq!(logged, format!("shuyi-capture: {file_line}"));
        assert!(logged.ends_with(verdict), "{logged}");
    }
    assert!(DONE.starts_with(PREFIX));
}

#[test]
fn a_console_report_writes_nothing_and_counts_the_same() {
    let mut report = Report::console();
    let reference = solid(4, 2, [10, 20, 30]);
    let far = solid(4, 2, [60, 20, 30]);
    let failed = compare(&far, &reference, Tolerance::default());
    assert!(!report.add("stage", &far, &reference, &failed, 3).unwrap());
    assert_eq!(report.captured(), 1);
    assert_eq!(report.exit_code(), FAILED);
    assert_eq!(report.summary(), "0 of 1 captures passed");
}

#[test]
fn a_report_that_cannot_be_written_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a-file");
    std::fs::write(&file, b"").unwrap();
    assert!(Report::new(file.join("out")).is_err());
}

/// Without the `capture` feature the backend never reads `SHUYI_CAPTURE`:
/// only this module names it, and it is compiled with the feature alone.
#[test]
fn only_the_capture_feature_reads_shuyi_capture() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![src.clone()];
    let mut naming = Vec::new();
    while let Some(path) = files.pop() {
        if path.is_dir() {
            files.extend(std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
        } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("tests.rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            if text.contains(&format!("\"{PLAN_VAR}\"")) {
                naming.push(path.strip_prefix(&src).unwrap().to_owned());
            }
        }
    }
    assert_eq!(naming, [PathBuf::from("capture.rs")]);
    let lib = include_str!("../lib.rs");
    let gate = "#[cfg(feature = \"capture\")]
pub mod capture;";
    assert!(lib.contains(gate));
}
