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

/// The `size` picture of `image` at `at`.
fn crop(image: &Image, at: (u32, u32), size: (u32, u32)) -> Image {
    let rows = (at.1..at.1 + size.1).map(|y| (y * image.width + at.0) as usize * 4);
    let rgba = rows
        .flat_map(|i| image.rgba[i..i + size.0 as usize * 4].to_vec())
        .collect();
    Image {
        width: size.0,
        height: size.1,
        rgba,
    }
}

/// Mix `m`'s picture of `sheet`, whose pictures are `size`.
fn picture(sheet: &Image, size: (u32, u32), m: u8) -> Image {
    let at = sheet_rect((0, 0), size, m);
    crop(sheet, (at.x, at.y), size)
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

}

/// On mix 12 (`##..`), whose edge is between rows 11 and 12: each row
/// below `rim_from` is the tile's, each from there to `clear_from` the
/// rim's, and the rest clear.
fn assert_rim(side: RimSide, rim_from: u32, clear_from: u32) {
    let rim = Rim::new([1, 2, 3], side).with_width(2);
    let tile = tile(24, 24);
    let top = picture(&generate(&tile, None, Some(rim)).unwrap(), SIZE, 0b1100);
    for (x, y) in (0..24).flat_map(|y| (0..24).map(move |x| (x, y))) {
        let want = match y {
            y if y < rim_from => tile.pixel((x + 12) % 24, (y + 12) % 24).unwrap(),
            y if y < clear_from => [1, 2, 3, 255],
            _ => [0; 4],
        };
        assert_eq!(top.pixel(x, y), Some(want), "{side:?} ({x}, {y})");
    }
}

#[test]
fn a_rim_lines_its_side_of_the_edge() {
    assert_rim(RimSide::Outside, 12, 14);
    assert_rim(RimSide::Inside, 10, 12);
    // A rim of no width draws none.
    let none = Rim::new([1, 2, 3], RimSide::Inside).with_width(0);
    assert_eq!(generate(&tile(24, 24), None, Some(none)).unwrap(), plain());
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
    let made = |(w, h), wobble| generate(&tile(w, h), wobble, None).err();
    assert_eq!(made((0, 4), None), Some(CornersError::Empty((0, 4))));
    assert_eq!(made((257, 4), None), Some(CornersError::TooLarge((257, 4))));
    assert_eq!(made((4, 257), None), Some(CornersError::TooLarge((4, 257))));
    assert_eq!(made((256, 1), None), None);
    assert_eq!(made((1, 256), None), None);
    let wobble = Wobble::new(1);
    let cells = |cells| made((24, 24), Some(wobble.with_cells(cells)));
    assert_eq!(cells(0), Some(CornersError::Cells(0)));
    assert_eq!(cells(33), Some(CornersError::Cells(33)));
    assert_eq!(
        made((24, 24), Some(wobble.with_cells(32).with_strength(100))),
        None
    );
    let strong = made((24, 24), Some(wobble.with_strength(101)));
    assert_eq!(strong, Some(CornersError::Strength(101)));
}

#[test]
fn a_layer_names_each_mix_in_its_place() {
    let nine = sheet_rect((0, 96), (24, 24), 9);
    assert_eq!((nine.x, nine.y, nine.w, nine.h), (24, 120, 24, 24));
    let of = BTreeSet::from([TerrainId(1)]);
    let layer = layer(of.clone(), (3, 4), (16, 8));
    let want: Vec<_> = (0..16)
        .map(|m| (m > 0).then(|| sheet_rect((3, 4), (16, 8), m)))
        .collect();
    assert_eq!((layer.of, layer.tiles.to_vec()), (of, want));
}

#[test]
fn a_sheet_layer_names_its_own_image_from_its_top_left() {
    let images = crate::image::ImageTable::new(
        [(
            "tiles/water-corners.png",
            crate::image::ImageInfo::new(128, 16),
        )]
        .into(),
    );
    let sheet = images.id("tiles/water-corners.png").unwrap();
    let of = BTreeSet::from([TerrainId(1)]);
    let made = sheet_layer(of.clone(), sheet, (16, 8));
    assert_eq!(made, layer(of, (0, 0), (16, 8)).with_image(sheet));
    assert_eq!(made.image, Some(sheet));
    assert_eq!(layer(BTreeSet::new(), (0, 0), (16, 8)).image, None);
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

/// The pictures the recipes name: `terrain.png`, 48 × 24, and
/// `shore.png`, 80 × 16.
fn pictures(path: &str) -> Result<Image, String> {
    match path {
        "tiles/terrain.png" => Ok(tile(48, 24)),
        "tiles/shore.png" => Ok(tile(80, 16)),
        _ => Err("no such file".to_owned()),
    }
}

/// The sheet the recipe `text`, the file `w.corners.ron`, makes.
fn made(text: &str) -> Result<Image, String> {
    let recipe = recipe::parse("w.corners.ron", text).map_err(|e| e.to_string())?;
    recipe.sheet(pictures)
}

#[test]
fn a_recipe_is_read_and_makes_its_sheet() {
    let recipe = recipe::parse("w.corners.ron", GENERATED).unwrap();
    assert_eq!(recipe.out, "tiles/water-corners.png");
    assert_eq!(recipe.file(), "w.corners.ron");
    let Make::Generated {
        tile: at,
        wobble,
        rim,
    } = &recipe.make
    else {
        panic!("{:?}", recipe.make);
    };
    assert_eq!((at.at, at.size), ((24, 0), (24, 24)));
    let foam = Rim::new([0xE8, 0xF4, 0xFF], RimSide::Outside).with_width(2);
    assert_eq!((*wobble, *rim), (Some(Wobble::new(7)), Some(foam)));
    let shifted = crop(&tile(48, 24), (24, 0), (24, 24));
    let want = generate(&shifted, *wobble, *rim).unwrap();
    assert_eq!(recipe.sheet(pictures), Ok(want));
    // Left out, the wobble and the rim are none.
    let bare =
        "(make: Generated(tile: (image: \"a.png\", at: (0, 0), size: (8, 8))), out: \"b.png\")";
    let bare = recipe::parse("bare.corners.ron", bare).unwrap();
    let Make::Generated { wobble, rim, .. } = bare.make else {
        panic!("{:?}", bare.make);
    };
    assert_eq!((wobble, rim), (None, None));
}

#[test]
fn a_fields_line_is_where_its_name_is_followed_by_a_colon() {
    // Not inside a longer name, nor where the word isn't a field.
    let line = |text: &str| recipe::line_of(text, "at");
    assert_eq!(line("flat_at: 1\nformat: 2\nat: 3"), Some(3));
    assert_eq!(line("// at the top\n  (at : 2)"), Some(2));
    assert_eq!(line("size: (1, 1)"), None);
}

#[test]
fn a_bad_recipe_says_its_file_and_line() {
    let bad = |from: &str, to: &str| made(&GENERATED.replace(from, to)).unwrap_err();
    let e = bad("seed: 7", "seed: 7, size: 3");
    assert!(
        e.starts_with("w.corners.ron:4:") && e.contains("size"),
        "{e}"
    );
    let e = bad("#E8F4FF", "E8F4FF");
    assert!(
        e.starts_with("w.corners.ron:5:") && e.contains("#RRGGBB"),
        "{e}"
    );
    // A tile outside its picture: the line of its `at`.
    let e = bad("at: (24, 0)", "at: (25, 0)");
    assert!(
        e.starts_with("w.corners.ron:3: the tile,") && e.contains("terrain.png, 48×24"),
        "{e}"
    );
    // A picture that isn't there, and a sheet that can't be made.
    assert_eq!(
        bad("tiles/terrain.png", "gone.png"),
        "gone.png: no such file"
    );
    let e = bad("strength: 30", "strength: 120");
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
    golden.check("generated", hash(&made(GENERATED).unwrap()));
    golden.check("rotated", hash(&made(ROTATED).unwrap()));
    golden.assert();
}

/// `picture` turned a quarter clockwise `times` times, as the ticket's
/// rule says: `(x, y)` to `(n − 1 − y, x)`.
fn turned(picture: &Image, times: u32) -> Image {
    (0..times).fold(picture.clone(), |p, _| turn(&p))
}

/// Five `n × n` pieces, each its own: `tile` shifted by the piece's place.
fn five_pieces(n: u32) -> [Image; 5] {
    let all = tile(n * 5, n);
    core::array::from_fn(|i| crop(&all, (n * u32::try_from(i).unwrap(), 0), (n, n)))
}

#[test]
fn a_quarter_turn_moves_the_corners_clockwise() {
    let mut piece = Image::filled(3, 3, [0; 4]);
    put(&mut piece, (0, 0), [1, 0, 0, 255]);
    put(&mut piece, (2, 1), [2, 0, 0, 255]);
    let once = turn(&piece);
    // Top-left to top-right; the middle of the right side to the middle of
    // the bottom.
    assert_eq!(once.pixel(2, 0), Some([1, 0, 0, 255]));
    assert_eq!(once.pixel(1, 2), Some([2, 0, 0, 255]));
    assert_eq!(in_count(&once), 2);
    // The mixes' bits: 8 to 4, 4 to 1, 1 to 2, 2 to 8.
    assert_eq!([8, 4, 1, 2].map(turn_mix), [4, 1, 2, 8]);
    assert_eq!([12, 5, 3, 10].map(turn_mix), [5, 3, 10, 12]);
    assert_eq!([9, 6, 15, 0].map(turn_mix), [6, 9, 15, 0]);
    assert_eq!([14, 13, 7, 11].map(turn_mix), [13, 7, 11, 14]);
}

#[test]
fn a_rotated_sheet_turns_each_piece_into_its_mixes() {
    let pieces = five_pieces(16);
    let [one, two, across, three, four] = &pieces;
    let sheet = rotated([one, two, across, three, four]).unwrap();
    assert_eq!((sheet.width, sheet.height), (128, 32));
    let at = |m| picture(&sheet, (16, 16), m);
    // The pieces' own mixes are the pieces as drawn.
    for (m, piece) in PIECES.into_iter().zip(&pieces) {
        assert_eq!(at(m), *piece, "mix {m}");
    }
    // Mix 4 is mix 8 turned once, 1 twice, 2 three times; 6 is 9 turned.
    assert_eq!(at(4), turned(one, 1));
    assert_eq!(at(1), turned(one, 2));
    assert_eq!(at(2), turned(one, 3));
    assert_eq!(at(6), turned(across, 1));
    assert_eq!([at(5), at(3), at(10)], [1, 2, 3].map(|t| turned(two, t)));
    assert_eq!([at(13), at(7), at(11)], [1, 2, 3].map(|t| turned(three, t)));
    // Every mix but 0 filled, 0 clear.
    assert_eq!(in_count(&at(0)), 0);
    for m in 1..16 {
        assert_eq!(in_count(&at(m)), 256, "mix {m}");
    }
}

#[test]
fn rotated_pieces_must_be_square_of_one_size() {
    let pieces = five_pieces(8);
    let with = |i: usize, other: &Image| {
        let mut all: [&Image; 5] = core::array::from_fn(|n| &pieces[n]);
        all[i] = other;
        rotated(all).err()
    };
    let small = tile(4, 4);
    let size = |piece, size, first| CornersError::PieceSize { piece, size, first };
    assert_eq!(with(3, &small), Some(size(3, (4, 4), (8, 8))));
    let tall = tile(8, 9);
    assert_eq!(with(1, &tall), Some(size(1, (8, 9), (8, 8))));
    let not_square = |piece, size| CornersError::NotSquare { piece, size };
    assert_eq!(rotated([&tall; 5]).err(), Some(not_square(0, (8, 9))));
    let empty = tile(0, 0);
    assert_eq!(with(0, &empty), Some(CornersError::Empty((0, 0))));
    let large = tile(257, 257);
    assert_eq!(with(0, &large), Some(CornersError::TooLarge((257, 257))));
    assert_eq!(with(0, &pieces[0]), None);
    let e = with(2, &small).unwrap().to_string();
    assert_eq!(
        e,
        "piece 2 is (4, 4) pixels across and down, and the first is (8, 8)"
    );
}

proptest! {
    #[test]
    fn four_quarter_turns_give_the_piece_back(n in 1..=24u32, seed in any::<u8>()) {
        let mut piece = tile(n, n);
        for byte in &mut piece.rgba {
            *byte ^= seed;
        }
        prop_assert_eq!(turned(&piece, 4), piece);
    }
}

const ROTATED: &str = r#"(
    make: Rotated(
        image: "tiles/shore.png",
        size: (16, 16),
        pieces: [(0, 0), (16, 0), (32, 0), (48, 0), (64, 0)],
    ),
    out: "tiles/shore-corners.png",
)
"#;

#[test]
fn a_rotated_recipe_makes_its_sheet() {
    let recipe = recipe::parse("s.corners.ron", ROTATED).unwrap();
    let Make::Rotated { size, pieces, .. } = &recipe.make else {
        panic!("{:?}", recipe.make);
    };
    assert_eq!((*size, pieces[4]), ((16, 16), (64, 0)));
    let pieces = five_pieces(16);
    let [one, two, across, three, four] = &pieces;
    let want = rotated([one, two, across, three, four]).unwrap();
    assert_eq!(made(ROTATED), Ok(want));
}

#[test]
fn a_bad_rotated_recipe_says_its_file_and_line() {
    let bad = |from: &str, to: &str| made(&ROTATED.replace(from, to)).unwrap_err();
    // Four pieces, and six.
    let e = bad(", (64, 0)]", "]");
    assert!(
        e.starts_with("w.corners.ron:5:") && e.contains("5 places, not 4"),
        "{e}"
    );
    let e = bad("(64, 0)]", "(64, 0), (0, 0)]");
    assert!(
        e.starts_with("w.corners.ron:5:") && e.contains("not 6"),
        "{e}"
    );
    // A piece outside its picture: the line of `pieces`.
    let e = bad("(64, 0)]", "(72, 0)]");
    let outside =
        "w.corners.ron:5: piece 4, 16×16 pixels at (72, 0), lies outside tiles/shore.png, 80×16";
    assert!(e.starts_with(outside), "{e}");
    // Pieces that aren't square.
    let e = bad("size: (16, 16)", "size: (16, 8)");
    assert!(e.starts_with("w.corners.ron: piece 0 is (16, 8)"), "{e}");
}
