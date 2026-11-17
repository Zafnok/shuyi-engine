//! The bold face of an atlas (ticket 3216).

use super::*;

/// An 8 × 16 atlas of `A` and `b`, `A`'s bold shape in cell 2.
fn atlas() -> FontAtlasDef {
    let mut ink = FontInk::new(InkBox::new(1, 2, 6, 11));
    ink.columns.insert('b', [1, 5]);
    FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 4,
        glyphs: BTreeMap::from([('A', 0), ('b', 1)]),
        bold: BTreeMap::from([('A', 2)]),
        ink: Some(ink),
        bold_ink: Some(FontInk::new(InkBox::new(1, 2, 7, 11))),
        ..FontAtlasDef::default()
    }
}

#[test]
fn bold_is_the_bold_shape_once_or_the_glyph_twice() {
    let atlas = atlas();
    let regular = atlas.glyph_rect('b').unwrap();
    assert_eq!(atlas.bold_rect('A'), Some(atlas.cell_rect(2)));
    assert_eq!(atlas.bold_rect('b'), None);
    let a = atlas.glyph_rect('A').unwrap();
    assert_eq!(atlas.bold_strikes('A', a, true), (atlas.cell_rect(2), 1));
    assert_eq!(atlas.bold_strikes('b', regular, true), (regular, 2));
    assert_eq!(atlas.bold_strikes('A', a, false), (a, 1));
    assert_eq!(atlas.bold_strikes('b', regular, false), (regular, 1));
    assert!(atlas.problems("Ab").is_empty());
    assert!(atlas.image_problems(32, 16).is_empty());
    // The bold shape's cell counts: the picture needs it.
    assert_eq!(atlas.cell_end(), 3);
    let two_wide = FontAtlasDef {
        columns: 2,
        ..atlas
    };
    assert_eq!(
        two_wide.image_problems(16, 16),
        ["'A' (U+0041) has cell index 2, but the image only holds 2 cells"]
    );
}

#[test]
fn a_bold_shape_needs_its_glyph_its_own_cell_and_ink_inside_the_cell() {
    let mut lone = atlas();
    lone.bold.insert('z', 3);
    lone.bold.insert('b', 0);
    let found = lone.problems("");
    assert!(
        found.contains(&"bold 'z' (U+007A) has no narrow glyph".to_owned()),
        "{found:?}"
    );
    assert!(
        found.iter().any(|p| p.contains("share cell index 0")),
        "{found:?}"
    );
    let mut outside = atlas();
    let bold_ink = outside.bold_ink.as_mut().unwrap();
    bold_ink.capitals.top = 12;
    bold_ink.columns.insert('b', [2, 8]);
    assert_eq!(
        outside.problems(""),
        [
            "bold_ink's capitals must lie inside the cell, left of right and above bottom",
            "bold_ink's columns of 'b' (U+0062) must lie inside the cell",
        ]
    );
}

#[test]
fn widths_carry_the_bold_faces_ink() {
    let widths = CellWidths::of(&atlas());
    assert_eq!(widths.ink(true).unwrap().capitals.right, 7);
    assert_eq!(widths.ink(false).unwrap().capitals.right, 6);
    let regular_only = FontAtlasDef {
        bold_ink: None,
        ..atlas()
    };
    assert_eq!(CellWidths::of(&regular_only).ink(true), None);
    assert_ne!(CellWidths::of(&regular_only), widths);
}

#[test]
fn an_atlas_file_without_bold_still_reads() {
    let old = "(cell_w: 8, cell_h: 16, columns: 4, glyphs: {'A': 0})";
    let def: FontAtlasDef = ron::from_str(old).unwrap();
    assert!(def.bold.is_empty() && def.bold_ink.is_none());
    let new = "(cell_w: 8, cell_h: 16, columns: 4, glyphs: {'A': 0}, bold: {'A': 1}, \
               bold_ink: Some((capitals: (left: 1, top: 2, right: 7, bottom: 11), columns: {})))";
    let def: FontAtlasDef = ron::from_str(new).unwrap();
    assert_eq!(def.bold_rect('A'), Some(def.cell_rect(1)));
    assert_eq!(def.bold_ink.unwrap().capitals.right, 7);
}
