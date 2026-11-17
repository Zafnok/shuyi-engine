//! Where an atlas's glyphs put ink (ticket 3215).

use shuyi_assets::font::{FontInk, InkBox};

use super::*;

/// A 4 × 4 font of `glyphs`, each drawn as four rows of `#` and `.`.
fn font(glyphs: &[(char, [&str; 4])]) -> BitmapFont {
    let pixels = |rows: &[&str; 4]| {
        rows.iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect()
    };
    BitmapFont {
        cell_w: 4,
        cell_h: 4,
        glyphs: glyphs.iter().map(|(c, rows)| (*c, pixels(rows))).collect(),
    }
}

const A: [&str; 4] = ["....", "##..", "#...", "...."];

/// Ink of capitals `(left, top, right, bottom)`, and `columns`.
fn ink((l, t, r, b): (u32, u32, u32, u32), columns: &[(char, [u32; 2])]) -> FontInk {
    let mut ink = FontInk::new(InkBox::new(l, t, r, b));
    ink.columns.extend(columns.iter().copied());
    ink
}

#[test]
fn the_ink_is_the_edges_most_capitals_share_and_the_columns_that_differ() {
    let b = ["....", "##..", "##..", "...."];
    let q = ["....", ".##.", ".##.", "..#."];
    let lower = ["####", "####", "####", "####"];
    let face = font(&[('A', A), ('B', b), ('Q', q), ('q', lower)]);
    let all: BTreeMap<char, u32> = face.glyphs.keys().copied().zip(0..).collect();
    // Q's columns and q's differ from the capitals': they are listed.
    let found = ink((0, 1, 1, 2), &[('Q', [1, 2]), ('q', [0, 3])]);
    assert_eq!(ink_of(&face, &all), Some(found));
    // Only the glyphs the atlas keeps count.
    let q_only = BTreeMap::from([('Q', 0), ('q', 1)]);
    assert_eq!(
        ink_of(&face, &q_only),
        Some(ink((1, 1, 2, 3), &[('q', [0, 3])]))
    );
    assert_eq!(ink_of(&face, &BTreeMap::from([('q', 0)])), None);
    let blank = font(&[('A', ["....", "....", "....", "...."])]);
    assert_eq!(ink_of(&blank, &BTreeMap::from([('A', 0)])), None);
}

#[test]
fn the_atlas_ron_writes_the_ink_a_glyphs_columns_to_a_line() {
    let regular = font(&[('A', A), ('b', ["....", "....", "....", "..#."])]);
    let atlas = build_atlas(&regular, &AtlasChars::default());
    assert_eq!(atlas.def.ink, Some(ink((0, 1, 1, 2), &[('b', [2, 2])])));
    let ron = atlas_ron(&atlas.def, "a", "shuyi", None).unwrap();
    let written = "    ink: Some((\n        capitals: (\n            left: 0,\n            top: 1,\n            \
                   right: 1,\n            bottom: 2,\n        ),\n        columns: {\n            \
                   'b': (2, 2),\n        },\n    )),\n)\n";
    assert!(ron.ends_with(written), "{ron}");
    let read: FontAtlasDef = ron::from_str(&ron).unwrap();
    assert_eq!(read, atlas.def);
}
