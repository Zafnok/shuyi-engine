use proptest::prelude::*;

use super::*;

/// `count` shots of `width` × `height`, each filled with its own grey.
fn shots(count: u8, width: u32, height: u32) -> Vec<Shot> {
    (0..count)
        .map(|i| {
            Shot::new(
                Image::filled(width, height, [i * 10, i * 10, i * 10, 255]),
                "s",
            )
        })
        .collect()
}

/// The pixel at `(x, y)` of `image`.
fn pixel(image: &Image, x: u32, y: u32) -> [u8; 4] {
    let at = index(image, x, y);
    image.rgba[at..at + 4].try_into().unwrap()
}

/// `image` as text, a font pixel (its top-left) a character: `#` in ink,
/// `.` otherwise.
fn font_pixels(image: &Image) -> String {
    (0..image.height / FONT_SCALE)
        .map(|y| {
            (0..image.width / FONT_SCALE)
                .map(|x| {
                    if pixel(image, x * FONT_SCALE, y * FONT_SCALE) == INK {
                        '#'
                    } else {
                        '.'
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn each_glyph_of_the_label_font_draws_as_drawn_here() {
    let glyphs = [
        ('0', "###|#.#|#.#|#.#|###"),
        ('1', ".#.|##.|.#.|.#.|###"),
        ('2', "###|..#|###|#..|###"),
        ('3', "###|..#|###|..#|###"),
        ('4', "#.#|#.#|###|..#|..#"),
        ('5', "###|#..|###|..#|###"),
        ('6', "###|#..|###|#.#|###"),
        ('7', "###|..#|..#|..#|..#"),
        ('8', "###|#.#|###|#.#|###"),
        ('9', "###|#.#|###|..#|###"),
        ('+', "...|.#.|###|.#.|..."),
        ('m', "...|##.|###|#.#|#.#"),
        ('s', "...|.##|.#.|..#|##."),
    ];
    for (c, want) in glyphs {
        let mut image = Image::filled(6, 10, BACKGROUND);
        draw_text(&mut image, &c.to_string(), (0, 0), 6);
        assert_eq!(font_pixels(&image), want.replace('|', "\n"), "{c:?}");
        // Each font pixel is 2 × 2.
        for y in 0..10 {
            for x in 0..6 {
                let corner = pixel(&image, x / 2 * 2, y / 2 * 2);
                assert_eq!(pixel(&image, x, y), corner, "{c:?} at {x},{y}");
            }
        }
    }
    // A letter the font lacks draws nothing.
    let mut image = Image::filled(6, 10, BACKGROUND);
    draw_text(&mut image, "x", (0, 0), 6);
    assert_eq!(image, Image::filled(6, 10, BACKGROUND));
}

#[test]
fn cells_go_left_to_right_then_top_to_bottom() {
    // A full last row: 8 cells of 10 × 6, 4 in a row.
    let full = Layout::new(8, Some(4), 10, 6).unwrap();
    let cells: Vec<_> = (0..8).map(|i| full.cell(i)).collect();
    assert_eq!(
        cells,
        [
            (2, 2),
            (14, 2),
            (26, 2),
            (38, 2),
            (2, 24),
            (14, 24),
            (26, 24),
            (38, 24)
        ]
    );
    assert_eq!((full.width(), full.height()), (50, 46));
    // A partial last row: 5 cells, 3 in a row by default.
    let partial = Layout::new(5, None, 10, 6).unwrap();
    assert_eq!((partial.columns, partial.rows), (3, 2));
    assert_eq!(partial.cell(4), (14, 24));
    assert_eq!((partial.width(), partial.height()), (38, 46));
    // The default is the smallest square that holds them.
    let defaults: Vec<_> = [1, 2, 4, 5, 9, 10, 64].map(default_columns).into();
    assert_eq!(defaults, [1, 2, 2, 3, 3, 4, 8]);
    assert!(Layout::new(0, None, 10, 6).is_err());
    assert!(Layout::new(5, Some(0), 10, 6).is_err());
    assert!(Layout::new(5, Some(6), 10, 6).is_err());
}

#[test]
fn a_sheet_puts_each_frame_under_its_time() {
    let shots = shots(5, 40, 3);
    let sheet = build(&shots, 3, None).unwrap();
    assert_eq!((sheet.width, sheet.height), (2 + 3 * 42, 2 + 2 * 19));
    // The gutter's corner is the background, and each frame is copied.
    assert_eq!(pixel(&sheet, 0, 0), BACKGROUND);
    let layout = Layout::new(5, None, 40, 3).unwrap();
    for (i, shot) in (0..).zip(&shots) {
        let (x, y) = layout.cell(i);
        assert_eq!(pixel(&sheet, x, y + LABEL_HEIGHT), pixel(&shot.image, 0, 0));
        assert_eq!(
            pixel(&sheet, x + 39, y + LABEL_HEIGHT + 2),
            pixel(&shot.image, 39, 2)
        );
    }
    // The fifth frame, 12 frames on, is labelled +200ms.
    let (x, y) = layout.cell(4);
    let mut strip = Image::filled(40, 10, BACKGROUND);
    for dy in 0..10 {
        for dx in 0..40 {
            let at = index(&strip, dx, dy);
            strip.rgba[at..at + 4].copy_from_slice(&pixel(&sheet, x + dx, y + LABEL_TOP + dy));
        }
    }
    let mut want = Image::filled(40, 10, BACKGROUND);
    draw_text(&mut want, "+200ms", (0, 0), 40);
    assert_eq!(strip, want);
    assert!(font_pixels(&strip).contains('#'));
}

#[test]
fn a_label_is_cut_at_its_cells_right_edge() {
    // Two cells of 12 pixels: "+16ms" is wider and stops at the edge.
    let sheet = build(&shots(2, 12, 2), 1, Some(2)).unwrap();
    let (x, y) = (2 + 12 + 2, 2 + LABEL_TOP);
    let ink_in = |from: u32, to: u32| {
        (from..to).any(|px| (y..y + 10).any(|py| pixel(&sheet, px, py) == INK))
    };
    assert!(ink_in(x, x + 12));
    // The gutter after the second cell keeps the background.
    assert!(!ink_in(x + 12, sheet.width));
    // Between the cells too.
    assert!(!ink_in(14, 16));
}

#[test]
fn a_sheet_over_16384_pixels_is_refused_with_its_size() {
    let error = build(&shots(8, 3000, 1), 1, Some(8)).unwrap_err();
    assert!(error.contains("24018×19"), "{error}");
    assert!(error.contains("16384"), "{error}");
    assert!(
        error.contains("--scale") && error.contains("--columns"),
        "{error}"
    );
    assert!(build(&[], 1, None).is_err());
}

proptest! {
    /// For any count and columns, the cells (each strip and its frame) lie
    /// inside the sheet and don't overlap.
    #[test]
    fn cells_lie_inside_and_apart(
        count in 1_u32..=64,
        columns in 1_u32..=64,
        width in 1_u32..=120,
        height in 1_u32..=120,
    ) {
        let columns = columns.min(count);
        let layout = Layout::new(count, Some(columns), width, height).unwrap();
        let cells: Vec<_> = (0..count)
            .map(|i| {
                let (x, y) = layout.cell(i);
                (u64::from(x), u64::from(y))
            })
            .collect();
        let (w, h) = (u64::from(width), u64::from(LABEL_HEIGHT + height));
        for &(x, y) in &cells {
            prop_assert!(x + w <= layout.width() && y + h <= layout.height());
        }
        for (i, &(x, y)) in cells.iter().enumerate() {
            for &(ox, oy) in &cells[i + 1..] {
                let apart = x + w <= ox || ox + w <= x || y + h <= oy || oy + h <= y;
                prop_assert!(apart, "{:?} and {:?}", (x, y), (ox, oy));
            }
        }
    }
}
