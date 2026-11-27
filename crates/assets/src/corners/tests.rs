use super::field::{Lattice, field};
use super::recipe::{self, Make};
use super::*;

use proptest::prelude::*;

/// A `w × h` tile whose every pixel is opaque and its own.
fn tile(w: u32, h: u32) -> Image {
    let mut tile = Image::filled(w, h, [0; 4]);
    for y in 0..h {
        for x in 0..w {
            let byte = |v: u32| u8::try_from(v % 256).unwrap();
            let pixel = [byte(x * 7), byte(y * 11), byte(x + y), 255];
            put(&mut tile, (x, y), pixel);
        }
    }
    tile
}

/// Mix `m`'s picture of `sheet`, whose pictures are `size`.
fn picture(sheet: &Image, size: (u32, u32), m: u8) -> Image {
    let at = sheet_rect((0, 0), size, m);
    let mut picture = Image::filled(size.0, size.1, [0; 4]);
    for y in 0..size.1 {
        for x in 0..size.0 {
            put(
                &mut picture,
                (x, y),
                sheet.pixel(at.x + x, at.y + y).unwrap(),
            );
        }
    }
    picture
}

fn is_in(picture: &Image, x: u32, y: u32) -> bool {
    picture.pixel(x, y).unwrap()[3] != 0
}

/// The in pixels of `picture` that `(x, y)` reaches through in pixels side
/// by side (never corner to corner).
fn region(picture: &Image, from: (u32, u32)) -> BTreeSet<(u32, u32)> {
    let mut seen = BTreeSet::new();
    let mut next = vec![from];
    while let Some((x, y)) = next.pop() {
        if x >= picture.width
            || y >= picture.height
            || !is_in(picture, x, y)
            || !seen.insert((x, y))
        {
            continue;
        }
        next.extend([(x + 1, y), (x, y + 1)]);
        next.extend(x.checked_sub(1).map(|x| (x, y)));
        next.extend(y.checked_sub(1).map(|y| (x, y)));
    }
    seen
}

fn in_count(picture: &Image) -> usize {
    picture.rgba.chunks(4).filter(|p| p[3] != 0).count()
}

const SIZE: (u32, u32) = (24, 24);

fn plain() -> Image {
    generate(&tile(24, 24), None, None).unwrap()
}

#[test]
fn mix_15_is_the_tile_half_a_tile_over_wrapping() {
    let (tile, sheet) = (tile(24, 24), plain());
    assert_eq!((sheet.width, sheet.height), (192, 48));
    let full = picture(&sheet, SIZE, 15);
    for y in 0..24 {
        for x in 0..24 {
            assert_eq!(full.pixel(x, y), tile.pixel((x + 12) % 24, (y + 12) % 24));
        }
    }
}

#[test]
fn two_side_by_side_in_fill_the_upper_half() {
    let top = picture(&plain(), SIZE, 0b1100);
    for y in 0..24 {
        for x in 0..24 {
            assert_eq!(is_in(&top, x, y), y < 12, "({x}, {y})");
        }
    }
}

#[test]
fn one_corner_in_is_rounded() {
    let corner = picture(&plain(), SIZE, 0b1000);
    assert!(is_in(&corner, 0, 0));
    // A square quarter would hold it.
    assert!(!is_in(&corner, 11, 11));
    assert!(!is_in(&corner, 23, 0) && !is_in(&corner, 0, 23));
}

#[test]
fn two_across_stay_two_pieces() {
    let sheet = plain();
    for (m, a, b) in [(0b1001, (0, 0), (23, 23)), (0b0110, (23, 0), (0, 23))] {
        let across = picture(&sheet, SIZE, m);
        let one = region(&across, a);
        assert!(!one.is_empty() && !one.contains(&b), "mix {m}");
        let other = region(&across, b);
        assert_eq!(one.len() + other.len(), in_count(&across), "mix {m}");
    }
}

#[test]
fn mix_0s_place_is_clear_and_every_other_has_pixels() {
    let sheet = plain();
    assert_eq!(in_count(&picture(&sheet, SIZE, 0)), 0);
    for m in 1..16 {
        assert!(in_count(&picture(&sheet, SIZE, m)) > 0, "mix {m}");
    }
}

/// `2F + W` of mix `m` at `(X, Y)`.
fn wobbled(m: u8, size: (i64, i64), lattice: &Lattice, at: (i64, i64)) -> i64 {
    2 * field(m, size, at) + lattice.wobble(size, at)
}

/// Every pair of mixes that meet side by side, and every pair that meet
/// top to bottom, has the same wobbled field on the border they share.
fn assert_seamless(size: (i64, i64), wobble: Wobble) {
    let (w, h) = size;
    let lattice = Lattice::new(wobble).unwrap();
    for a in 0..16u8 {
        for b in 0..16u8 {
            // `a`'s right corners are `b`'s left: tr = tl, br = bl.
            if (a & 4) << 1 == b & 8 && (a & 1) << 1 == b & 2 {
                for y in 0..=2 * h {
                    let (right, left) = ((2 * w, y), (0, y));
                    assert_eq!(
                        wobbled(a, size, &lattice, right),
                        wobbled(b, size, &lattice, left)
                    );
                }
            }
            // `a`'s lower corners are `b`'s upper: bl = tl, br = tr.
            if (a & 2) << 2 == b & 8 && (a & 1) << 2 == b & 4 {
                for x in 0..=2 * w {
                    let (low, high) = ((x, 2 * h), (x, 0));
                    assert_eq!(
                        wobbled(a, size, &lattice, low),
                        wobbled(b, size, &lattice, high)
                    );
                }
            }
        }
    }
}

#[test]
fn pictures_meet_without_a_seam_with_any_wobble() {
    for seed in 0..20 {
        for cells in 1..=8 {
            assert_seamless(
                (24, 24),
                Wobble::new(seed).with_cells(cells).with_strength(100),
            );
        }
    }
}

proptest! {
    #[test]
    fn pictures_of_any_size_meet_without_a_seam(
        seed in any::<u64>(),
        cells in 1..=MAX_CELLS,
        strength in 0..=MAX_STRENGTH,
        w in 2..=64i64,
        h in 2..=48i64,
    ) {
        assert_seamless((w, h), Wobble::new(seed).with_cells(cells).with_strength(strength));
    }

    #[test]
    fn four_quarter_turns_give_the_piece_back(side in 1..=12u32, seed in any::<u8>()) {
        let mut piece = tile(side, side);
        piece.rgba.iter_mut().for_each(|b| *b ^= seed);
        let turned = turn(&turn(&turn(&turn(&piece))));
        prop_assert_eq!(turned, piece);
    }
}

#[test]
fn an_outside_rim_lines_the_out_side_of_the_edge() {
    let rim = Rim::new([1, 2, 3], RimSide::Outside).with_width(2);
    let tile = tile(24, 24);
    let top = picture(&generate(&tile, None, Some(rim)).unwrap(), SIZE, 0b1100);
    for x in 0..24 {
        for y in 0..12 {
            assert_eq!(top.pixel(x, y), tile.pixel((x + 12) % 24, (y + 12) % 24));
        }
        for y in 12..14 {
            assert_eq!(top.pixel(x, y), Some([1, 2, 3, 255]), "({x}, {y})");
        }
        for y in 14..24 {
            assert_eq!(top.pixel(x, y), Some([0; 4]), "({x}, {y})");
        }
    }
}

#[test]
fn an_inside_rim_lines_the_in_side_of_the_edge() {
    let rim = Rim::new([9, 8, 7], RimSide::Inside).with_width(2);
    let tile = tile(24, 24);
    let top = picture(&generate(&tile, None, Some(rim)).unwrap(), SIZE, 0b1100);
    for x in 0..24 {
        for y in 0..10 {
            assert_eq!(top.pixel(x, y), tile.pixel((x + 12) % 24, (y + 12) % 24));
        }
        for y in 10..12 {
            assert_eq!(top.pixel(x, y), Some([9, 8, 7, 255]), "({x}, {y})");
        }
        for y in 12..24 {
            assert_eq!(top.pixel(x, y), Some([0; 4]), "({x}, {y})");
        }
    }
    // A rim of no width draws none.
    let none = generate(&tile, None, Some(rim.with_width(0))).unwrap();
    assert_eq!(none, plain());
}

#[test]
fn a_wobble_is_its_seeds_and_no_strength_is_none() {
    let tile = tile(24, 24);
    let made = |wobble| generate(&tile, Some(wobble), None).unwrap();
    let seven = made(Wobble::new(7));
    assert_eq!(made(Wobble::new(7)), seven);
    assert_ne!(made(Wobble::new(8)), seven);
    assert_ne!(seven, plain());
    assert_eq!(made(Wobble::new(7).with_strength(0)), plain());
}

#[test]
fn an_odd_tile_lines_up_as_the_painter_centres_it() {
    // The painter puts a picture's top-left half a tile, rounded down, up
    // and left of its point: 4 of a 9 pixel tile.
    let tile = tile(9, 13);
    let full = picture(&generate(&tile, None, None).unwrap(), (9, 13), 15);
    assert_eq!(full.pixel(4, 6), tile.pixel(0, 0));
    assert_eq!(full.pixel(0, 0), tile.pixel(5, 7));
}

#[test]
fn the_tile_and_the_wobble_must_be_in_range() {
    let tile24 = tile(24, 24);
    assert_eq!(
        generate(&tile(0, 4), None, None),
        Err(CornersError::Empty((0, 4)))
    );
    assert_eq!(
        generate(&tile(257, 4), None, None),
        Err(CornersError::TooLarge((257, 4)))
    );
    assert!(generate(&tile(256, 1), None, None).is_ok());
    let wobble = Wobble::new(1);
    assert_eq!(
        generate(&tile24, Some(wobble.with_cells(0)), None),
        Err(CornersError::Cells(0))
    );
    assert_eq!(
        generate(&tile24, Some(wobble.with_cells(33)), None),
        Err(CornersError::Cells(33))
    );
    assert!(
        generate(
            &tile24,
            Some(wobble.with_cells(32).with_strength(100)),
            None
        )
        .is_ok()
    );
    assert_eq!(
        generate(&tile24, Some(wobble.with_strength(101)), None),
        Err(CornersError::Strength(101))
    );
}

/// Five square pieces, each its own.
fn pieces(side: u32) -> Vec<Image> {
    (0..5u8)
        .map(|i| {
            let mut piece = tile(side, side);
            piece.rgba.chunks_mut(4).for_each(|p| p[2] = i * 40);
            piece
        })
        .collect()
}

fn five(pieces: &[Image]) -> [&Image; 5] {
    [&pieces[0], &pieces[1], &pieces[2], &pieces[3], &pieces[4]]
}

#[test]
fn rotated_fills_every_mix_from_five_pieces_and_their_turns() {
    let drawn = pieces(16);
    let sheet = rotated(five(&drawn)).unwrap();
    assert_eq!((sheet.width, sheet.height), (128, 32));
    let size = (16, 16);
    for (m, piece) in PIECES.into_iter().zip(&drawn) {
        assert_eq!(&picture(&sheet, size, m), piece, "mix {m} is as drawn");
    }
    // `#...` turned once is `.#..`; `#..#` turned once is `.##.`.
    assert_eq!(picture(&sheet, size, 4), turn(&drawn[0]));
    assert_eq!(picture(&sheet, size, 6), turn(&drawn[2]));
    assert_eq!(picture(&sheet, size, 1), turn(&turn(&drawn[0])));
    assert_eq!(picture(&sheet, size, 5), turn(&drawn[1]));
    assert_eq!(picture(&sheet, size, 13), turn(&drawn[3]));
    assert_eq!(in_count(&picture(&sheet, size, 0)), 0);
    for m in 1..16 {
        assert_eq!(in_count(&picture(&sheet, size, m)), 256, "mix {m}");
    }
}

#[test]
fn a_quarter_turn_moves_each_corner_clockwise() {
    assert_eq!([8, 4, 1, 2].map(turn_mix), [4, 1, 2, 8]);
    assert_eq!(turn_mix(0b1001), 0b0110);
    assert_eq!(turn_mix(15), 15);
    let mut piece = Image::filled(2, 2, [0; 4]);
    put(&mut piece, (0, 0), [1, 1, 1, 1]);
    assert_eq!(turn(&piece).pixel(1, 0), Some([1, 1, 1, 1]));
}

#[test]
fn rotated_takes_square_pieces_of_one_size() {
    let mut drawn = pieces(16);
    drawn[3] = tile(16, 8);
    assert_eq!(
        rotated(five(&drawn)),
        Err(CornersError::PiecesDiffer {
            piece: 3,
            size: (16, 8),
            first: (16, 16)
        })
    );
    let flat = vec![tile(16, 8); 5];
    assert_eq!(rotated(five(&flat)), Err(CornersError::NotSquare((16, 8))));
    let empty = vec![tile(0, 0); 5];
    assert_eq!(rotated(five(&empty)), Err(CornersError::Empty((0, 0))));
}

#[test]
fn a_layer_names_each_mix_in_its_place() {
    assert_eq!(
        sheet_rect((0, 96), (24, 24), 9),
        ImageRect {
            x: 24,
            y: 120,
            w: 24,
            h: 24
        }
    );
    let layer = layer(BTreeSet::from([TerrainId(1)]), (3, 4), (16, 8));
    assert_eq!(layer.tiles[0], None);
    for m in 1..16u8 {
        assert_eq!(
            layer.tiles[usize::from(m)],
            Some(sheet_rect((3, 4), (16, 8), m))
        );
    }
    assert_eq!(layer.of, BTreeSet::from([TerrainId(1)]));
}

const GENERATED: &str = r##"(
    make: Generated(
        tile: (image: "tiles/terrain.png", at: (24, 0), size: (24, 24)),
        wobble: Some((strength: 30, cells: 4, seed: 7)),
        rim: Some((colour: "#E8F4FF", width: 2, side: Outside)),
    ),
    out: "tiles/water-corners.png",
)
"##;

const ROTATED: &str = r#"(
    make: Rotated(
        image: "shore.png",
        size: (16, 16),
        pieces: [(0, 0), (16, 0), (32, 0), (48, 0), (64, 0)],
    ),
    out: "shore-corners.png",
)
"#;

/// The pictures the recipes above name: `terrain.png` 48 × 24, `shore.png`
/// 80 × 16.
fn pictures(path: &str) -> Result<Image, String> {
    match path {
        "tiles/terrain.png" => Ok(tile(48, 24)),
        "shore.png" => Ok(tile(80, 16)),
        _ => Err("no such file".to_owned()),
    }
}

#[test]
fn a_recipe_is_read_and_makes_its_sheet() {
    let recipe = recipe::parse("art/water.corners.ron", GENERATED).unwrap();
    assert_eq!(recipe.out, "tiles/water-corners.png");
    assert_eq!(recipe.file(), "art/water.corners.ron");
    let Make::Generated {
        tile: at,
        wobble,
        rim,
    } = &recipe.make
    else {
        panic!("{:?}", recipe.make);
    };
    assert_eq!((at.at, at.size), ((24, 0), (24, 24)));
    assert_eq!(*wobble, Some(Wobble::new(7)));
    assert_eq!(
        *rim,
        Some(Rim::new([0xE8, 0xF4, 0xFF], RimSide::Outside).with_width(2))
    );
    let mut shifted = tile(48, 24);
    shifted.rgba = shifted
        .rgba
        .chunks(48 * 4)
        .flat_map(|row| row[24 * 4..].to_vec())
        .collect();
    shifted.width = 24;
    let want = generate(&shifted, *wobble, *rim).unwrap();
    assert_eq!(recipe.sheet(pictures), Ok(want));
    // Left out, the wobble and the rim are none.
    let bare =
        "(make: Generated(tile: (image: \"a.png\", at: (0, 0), size: (8, 8))), out: \"b.png\")";
    let bare = recipe::parse("bare.corners.ron", bare).unwrap();
    assert!(matches!(
        bare.make,
        Make::Generated {
            wobble: None,
            rim: None,
            ..
        }
    ));

    let turned = recipe::parse("shore.corners.ron", ROTATED).unwrap();
    let drawn = tile(80, 16);
    let piece = |i: u32| {
        let mut piece = Image::filled(16, 16, [0; 4]);
        for y in 0..16 {
            for x in 0..16 {
                put(&mut piece, (x, y), drawn.pixel(16 * i + x, y).unwrap());
            }
        }
        piece
    };
    let drawn: Vec<_> = (0..5).map(piece).collect();
    assert_eq!(turned.sheet(pictures), Ok(rotated(five(&drawn)).unwrap()));
}

#[test]
fn a_bad_recipe_says_its_file_and_line() {
    let unknown = GENERATED.replace("seed: 7", "seed: 7, size: 3");
    let e = recipe::parse("w.corners.ron", &unknown)
        .unwrap_err()
        .to_string();
    assert!(
        e.starts_with("w.corners.ron:4:") && e.contains("size"),
        "{e}"
    );
    let colour = GENERATED.replace("#E8F4FF", "E8F4FF");
    let e = recipe::parse("w.corners.ron", &colour)
        .unwrap_err()
        .to_string();
    assert!(
        e.starts_with("w.corners.ron:5:") && e.contains("#RRGGBB"),
        "{e}"
    );
    let four = ROTATED.replace(", (64, 0)]", "]");
    let e = recipe::parse("s.corners.ron", &four)
        .unwrap_err()
        .to_string();
    assert!(
        e.starts_with("s.corners.ron:5:") && e.contains("5 pieces, not 4"),
        "{e}"
    );
    // A piece outside its picture: the line of the pieces.
    let outside = ROTATED.replace("(64, 0)]", "(72, 0)]");
    let e = recipe::parse("s.corners.ron", &outside)
        .unwrap()
        .sheet(pictures)
        .unwrap_err();
    assert!(
        e.starts_with("s.corners.ron:5: piece 5,") && e.contains("shore.png, 80×16"),
        "{e}"
    );
    let outside = GENERATED.replace("at: (24, 0)", "at: (25, 0)");
    let e = recipe::parse("w.corners.ron", &outside)
        .unwrap()
        .sheet(pictures)
        .unwrap_err();
    assert!(e.starts_with("w.corners.ron:3: the tile,"), "{e}");
    // A picture that isn't there, and a sheet that can't be made.
    let missing = GENERATED.replace("tiles/terrain.png", "gone.png");
    let e = recipe::parse("w.corners.ron", &missing)
        .unwrap()
        .sheet(pictures)
        .unwrap_err();
    assert_eq!(e, "gone.png: no such file");
    let strong = GENERATED.replace("strength: 30", "strength: 120");
    let e = recipe::parse("w.corners.ron", &strong)
        .unwrap()
        .sheet(pictures)
        .unwrap_err();
    assert_eq!(e, "w.corners.ron: a wobble's strength is 0 to 100, not 120");
}

/// A sheet's hash: its size, then its pixels.
fn hash(sheet: &Image) -> String {
    let mut hasher = crate::fnv::Hasher::new();
    hasher.word(u64::from(sheet.width));
    hasher.word(u64::from(sheet.height));
    hasher.bytes(&sheet.rgba);
    format!("{:016x}", hasher.finish())
}

#[test]
fn golden_a_recipes_sheet_is_the_same_on_every_platform() {
    let mut golden = crate::golden!("tests/golden/corners.ron");
    for (row, text) in [("generated", GENERATED), ("rotated", ROTATED)] {
        let sheet = recipe::parse("fixed.corners.ron", text)
            .unwrap()
            .sheet(pictures)
            .unwrap();
        golden.check(row, hash(&sheet));
    }
    golden.assert();
}
