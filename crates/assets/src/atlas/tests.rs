//! The atlas's tests: no two packed rectangles overlap and each is inside
//! its page with its padding, the same images in any order give the same
//! atlas, `insert` never moves a sprite placed before, and an `atlas.ron`
//! packs its folder through the image formats.
use proptest::prelude::*;

use super::*;
use crate::format::Format;
use crate::format::builtin_tests::bundle;
use crate::target::{TargetProfile, TargetRon};

/// A `w` × `h` picture whose pixels say which picture and where they are.
fn picture(seed: u8, (w, h): (u32, u32)) -> Image {
    let mut image = Image::filled(w, h, [seed, 0, 0, 255]);
    for (i, pixel) in image.rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        // Wraps on purpose: only a pattern.
        #[allow(clippy::cast_possible_truncation)]
        let i = i as u8;
        pixel[1] = i;
    }
    image
}

/// `image` as a PNG file.
pub(crate) fn png_of(image: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&image.rgba).unwrap();
    }
    out
}

/// Names for `n` sprites, `s0`, `s1`…, kept for the run.
fn names(n: usize) -> Vec<&'static str> {
    (0..n).map(|i| leak(format!("s{i}"))).collect()
}

/// An atlas of `sizes`, added in that order.
fn packed(sizes: &[(u32, u32)], side: u32, padding: u32) -> Atlas {
    let mut builder = AtlasBuilder::new("test", side).with_padding(padding);
    for (i, (&name, &size)) in names(sizes.len()).iter().zip(sizes).enumerate() {
        #[allow(clippy::cast_possible_truncation)] // a pattern
        builder.add(name, &picture(i as u8, size)).unwrap();
    }
    builder.build().unwrap()
}

/// Every placed rectangle is inside its page with `padding` clear at its
/// edges, and keeps `padding` clear of every other on its page; and each
/// sprite's pixels are on its page.
fn check_layout(atlas: &Atlas, pictures: &BTreeMap<&'static str, Image>) -> Result<(), String> {
    let (side, pad) = (atlas.page_size(), atlas.padding());
    let all: Vec<(&str, Placed)> = atlas.sprites().collect();
    for &(name, Placed { page, rect: r }) in &all {
        if page >= atlas.pages().len() {
            return Err(format!("{name} on page {page}, past the last"));
        }
        if r.x < pad || r.y < pad || r.x + r.w + pad > side || r.y + r.h + pad > side {
            return Err(format!(
                "{name} at {r:?} leaves its page of {side} with padding {pad}"
            ));
        }
        if let Some(image) = pictures.get(name) {
            let on = &atlas.pages()[page].image();
            for y in 0..r.h {
                for x in 0..r.w {
                    if on.pixel(r.x + x, r.y + y) != image.pixel(x, y) {
                        return Err(format!("{name}'s pixel ({x}, {y}) isn't on its page"));
                    }
                }
            }
        }
    }
    for (i, &(a, pa)) in all.iter().enumerate() {
        for &(b, pb) in &all[i + 1..] {
            let (r, s) = (pa.rect, pb.rect);
            let apart = r.x + r.w + pad <= s.x
                || s.x + s.w + pad <= r.x
                || r.y + r.h + pad <= s.y
                || s.y + s.h + pad <= r.y;
            if pa.page == pb.page && !apart {
                return Err(format!("{a} at {r:?} and {b} at {s:?} overlap"));
            }
        }
    }
    Ok(())
}

/// Sizes that fit a page of 64 with padding up to 2.
fn sizes() -> impl Strategy<Value = Vec<(u32, u32)>> {
    proptest::collection::vec((1..=40_u32, 1..=40_u32), 0..40)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn no_two_packed_rectangles_overlap_and_each_is_inside_its_page(
        sizes in sizes(),
        padding in 0..=2_u32,
    ) {
        let atlas = packed(&sizes, 64, padding);
        let pictures: BTreeMap<&'static str, Image> = names(sizes.len())
            .into_iter()
            .zip(&sizes)
            .enumerate()
            .map(|(i, (name, &size))| {
                #[allow(clippy::cast_possible_truncation)] // a pattern
                let image = picture(i as u8, size);
                (name, image)
            })
            .collect();
        prop_assert_eq!(atlas.sprites().count(), sizes.len());
        if let Err(why) = check_layout(&atlas, &pictures) {
            return Err(TestCaseError::fail(why));
        }
    }

    #[test]
    fn the_same_images_in_any_order_give_the_same_atlas(
        sizes in sizes(),
        shuffle in any::<prop::sample::Index>(),
        padding in 0..=2_u32,
    ) {
        let named: Vec<(&'static str, Image)> = names(sizes.len())
            .into_iter()
            .zip(&sizes)
            .enumerate()
            .map(|(i, (name, &size))| {
                #[allow(clippy::cast_possible_truncation)] // a pattern
                let image = picture(i as u8, size);
                (name, image)
            })
            .collect();
        let build = |order: &[(&'static str, Image)]| {
            let mut builder = AtlasBuilder::new("test", 64).with_padding(padding);
            for (name, image) in order {
                builder.add(name, image).unwrap();
            }
            builder.build().unwrap()
        };
        let mut other = named.clone();
        other.reverse();
        if !other.is_empty() {
            let at = shuffle.index(other.len());
            other.rotate_left(at);
        }
        prop_assert_eq!(build(&named), build(&other));
    }

    #[test]
    fn insert_never_moves_a_placed_sprite(
        first in sizes(),
        later in proptest::collection::vec((1..=40_u32, 1..=40_u32), 1..20),
        padding in 0..=2_u32,
    ) {
        let mut atlas = packed(&first, 64, padding);
        let mut pictures = BTreeMap::new();
        for (i, &size) in later.iter().enumerate() {
            let before: Vec<(&str, Placed)> = atlas.sprites().collect();
            let pages: Vec<Image> = atlas.pages().iter().map(|p| p.image().clone()).collect();
            let name = leak(format!("later{i}"));
            #[allow(clippy::cast_possible_truncation)] // a pattern
            let image = picture(200 + i as u8, size);
            let placed = atlas.insert(name, &image).unwrap();
            pictures.insert(name, image);
            for (name, was) in before {
                prop_assert_eq!(atlas.get(name), Some(was), "{} moved", name);
            }
            // Only the page it names changed.
            for (n, page) in pages.iter().enumerate() {
                if n != placed.page {
                    prop_assert_eq!(atlas.pages()[n].image(), page);
                }
            }
            if let Err(why) = check_layout(&atlas, &pictures) {
                return Err(TestCaseError::fail(why));
            }
        }
    }
}

#[test]
fn a_sprite_goes_in_free_space_or_on_a_new_page() {
    let mut atlas = packed(&[(30, 30)], 64, 1);
    assert_eq!(atlas.pages().len(), 1);
    assert_eq!(atlas.pages()[0].name(), "test#0");
    let beside = atlas.insert("beside", &picture(1, (20, 20))).unwrap();
    assert_eq!(beside.page, 0);
    assert_eq!((beside.rect.x, beside.rect.y), (32, 1));
    let big = atlas.insert("big", &picture(2, (62, 62))).unwrap();
    assert_eq!(big.page, 1);
    assert_eq!(atlas.pages()[1].name(), "test#1");
    assert_eq!(atlas.pages()[1].image().width, 64);
}

#[test]
fn a_sprite_that_cant_go_in_says_why() {
    let mut builder = AtlasBuilder::new("test", 16).with_padding(1);
    builder.add("a", &picture(0, (15, 4))).unwrap();
    assert_eq!(
        builder.add("a", &picture(0, (1, 1))),
        Err(AtlasError::Taken { name: "a" })
    );
    let error = builder.build().unwrap_err();
    assert_eq!(
        error.to_string(),
        "sprite a is 15×4 px, too big for a page of 16×16 px with its padding"
    );
    let mut atlas = packed(&[(4, 4)], 16, 1);
    assert_eq!(
        atlas.insert("s0", &picture(0, (1, 1))),
        Err(AtlasError::Taken { name: "s0" })
    );
    assert_eq!(
        atlas.insert("none", &Image::default()),
        Err(AtlasError::Empty { name: "none" })
    );
    assert!(matches!(
        atlas.insert("wide", &picture(0, (15, 1))),
        Err(AtlasError::TooBig { side: 16, .. })
    ));
}

#[test]
fn pages_are_never_bigger_than_a_targets_texture_limit() {
    let profile = |name: &str, text: &str| -> TargetProfile {
        TargetRon
            .decode(&AssetPath::new(name), text.as_bytes(), &DecodeCx::new())
            .unwrap()
    };
    let web = profile("web.ron", include_str!("../../../../targets/web.ron"));
    let desktop = profile(
        "desktop.ron",
        include_str!("../../../../targets/desktop.ron"),
    );
    assert_eq!(web.display.max_texture_side, 2048);
    assert_eq!(desktop.display.max_texture_side, MAX_SIDE);
    let atlas = AtlasBuilder::new("big", 8192)
        .with_max_side(web.display.max_texture_side)
        .build()
        .unwrap();
    assert_eq!(atlas.page_size(), 2048);
    assert_eq!(
        AtlasBuilder::new("big", 8192).build().unwrap().page_size(),
        MAX_SIDE
    );
}

/// A game with an atlas of three sprites, a picture in a folder below
/// that isn't packed, and a text file beside them.
pub(crate) fn sprites() -> &'static crate::bundle::Bundle {
    bundle(vec![
        (
            "sprites/atlas.ron",
            b"(from: \"sprites/\", page_size: 32, padding: 1)".to_vec(),
        ),
        ("sprites/coin.png", png_of(&picture(1, (8, 8)))),
        ("sprites/heart.png", png_of(&picture(2, (10, 9)))),
        ("sprites/wide.png", png_of(&picture(3, (20, 4)))),
        ("sprites/notes.txt", b"not a picture".to_vec()),
        ("sprites/below/skip.png", png_of(&picture(4, (2, 2)))),
    ])
}

#[test]
fn an_atlas_ron_packs_every_image_of_its_folder() {
    let formats = Formats::standard();
    let atlas: Atlas = formats.load(sprites(), "sprites/atlas.ron").unwrap();
    let mut builder = AtlasBuilder::new("sprites/atlas.ron", 32).with_padding(1);
    for (name, seed, size) in [
        ("sprites/coin.png", 1, (8, 8)),
        ("sprites/heart.png", 2, (10, 9)),
        ("sprites/wide.png", 3, (20, 4)),
    ] {
        builder.add(name, &picture(seed, size)).unwrap();
    }
    assert_eq!(atlas, builder.build().unwrap());
    let names: Vec<&str> = atlas.sprites().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        ["sprites/coin.png", "sprites/heart.png", "sprites/wide.png"]
    );
    // Capped by a target's limit.
    let mut web = Formats::new();
    web.register(AtlasRon::new().with_max_side(24)).unwrap();
    let small: Atlas = web.load(sprites(), "sprites/atlas.ron").unwrap();
    assert_eq!(small.page_size(), 24);
}

#[test]
fn an_atlas_ron_says_what_is_wrong() {
    let read = |files: Vec<(&'static str, Vec<u8>)>| {
        let bundle = bundle(files);
        Formats::standard()
            .load::<Atlas>(bundle, "a/atlas.ron")
            .unwrap_err()
            .into_iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let spec = |text: &str| ("a/atlas.ron", text.as_bytes().to_vec());
    assert!(read(vec![spec("(from: \"a/\", page_size: 32)")]).contains("no image in folder a/"),);
    assert!(read(vec![spec("(from: \"a/\")")]).contains("page_size"));
    let broken = read(vec![
        spec("(from: \"a/\", page_size: 32)"),
        ("a/bad.png", b"\x89PNG\r\n\x1a\nbroken".to_vec()),
    ]);
    assert!(broken.contains("a/bad.png"), "{broken}");
    let big = read(vec![
        spec("(from: \"a/\", page_size: 8, padding: 1)"),
        ("a/big.png", png_of(&picture(0, (7, 7)))),
    ]);
    assert!(big.contains("sprite a/big.png is 7×7 px"), "{big}");
    // Without a bundle, its images can't be read.
    let alone = AtlasRon::new().decode(
        &AssetPath::new("a/atlas.ron"),
        b"(from: \"a/\", page_size: 8)",
        &DecodeCx::new(),
    );
    assert!(
        alone.unwrap_err()[0]
            .to_string()
            .contains("without the bundle")
    );
}

#[test]
fn an_atlas_survives_its_binary_form() {
    let atlas: Atlas = Formats::standard()
        .load(sprites(), "sprites/atlas.ron")
        .unwrap();
    let bytes = postcard::to_allocvec(&atlas).unwrap();
    let back: Atlas = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back, atlas);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn any_bytes_as_an_atlas_ron_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..200)) {
        let _ = AtlasRon::new().decode(
            &AssetPath::in_bundle(sprites(), "sprites/atlas.ron"),
            &bytes,
            &DecodeCx::in_bundle(sprites()),
        );
    }
}

/// A page of `side` whose skyline is `runs`, each `(x, y, w)`.
fn page_of(side: u32, runs: &[(u32, u32, u32)]) -> Page {
    let mut page = Page::new("p#0", side, 0);
    page.skyline = runs.iter().map(|&(x, y, w)| Segment { x, y, w }).collect();
    page
}

/// `page`'s skyline as `(x, y, w)`.
fn runs(page: &Page) -> Vec<(u32, u32, u32)> {
    page.skyline.iter().map(|s| (s.x, s.y, s.w)).collect()
}

#[test]
fn a_box_goes_lowest_then_leftmost_up_to_the_pages_edge() {
    // Reaching the right edge exactly fits.
    let edge = page_of(16, &[(0, 8, 8), (8, 0, 8)]);
    assert_eq!(edge.fit((8, 8), 16), Some((8, 0)));
    // A run that starts where the box ends doesn't hold it up.
    let step = page_of(16, &[(0, 4, 8), (8, 8, 4), (12, 0, 4)]);
    assert_eq!(step.fit((8, 1), 16), Some((0, 4)));
    // Two places as low: the leftmost.
    let tie = page_of(16, &[(0, 4, 4), (4, 8, 4), (8, 4, 8)]);
    assert_eq!(tie.fit((4, 4), 16), Some((0, 4)));
    // Reaching the bottom edge exactly fits; past it, nowhere.
    assert_eq!(page_of(16, &[(0, 8, 16)]).fit((4, 8), 16), Some((0, 8)));
    assert_eq!(page_of(16, &[(0, 8, 16)]).fit((4, 9), 16), None);
}

#[test]
fn raising_the_skyline_keeps_what_is_left_and_right_and_merges_equal_runs() {
    let mut page = page_of(16, &[(2, 0, 14)]);
    page.raise(6, 4, 5);
    assert_eq!(runs(&page), [(2, 0, 4), (6, 5, 4), (10, 0, 6)]);
    let mut flush = page_of(16, &[(0, 0, 8), (8, 3, 8)]);
    flush.raise(4, 4, 5);
    assert_eq!(runs(&flush), [(0, 0, 4), (4, 5, 4), (8, 3, 8)]);
    let mut level = page_of(16, &[(0, 5, 4), (4, 0, 12)]);
    level.raise(4, 4, 5);
    assert_eq!(runs(&level), [(0, 5, 8), (8, 0, 8)]);
}

#[test]
fn a_picture_with_no_pixels_is_refused() {
    let mut atlas = packed(&[(4, 4)], 16, 1);
    assert_eq!((atlas.id(), atlas.padding()), ("test", 1));
    let empty = |width, height, bytes| Image {
        width,
        height,
        rgba: vec![0; bytes],
    };
    for (name, image) in [
        ("no width", empty(0, 2, 0)),
        ("no height", empty(2, 0, 0)),
        ("short", empty(2, 2, 8)),
    ] {
        assert_eq!(atlas.insert(name, &image), Err(AtlasError::Empty { name }));
    }
}
