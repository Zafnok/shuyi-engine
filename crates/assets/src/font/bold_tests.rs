//! The bold face and the ink of an atlas (ticket 3215).

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
    assert!(atlas.problems("Ab").is_empty());
    assert!(atlas.image_problems(32, 16).is_empty());
    assert_eq!(atlas.cell_end(), 3);
}

#[test]
fn a_bold_shape_needs_its_glyph_its_own_cell_and_ink_inside_the_cell() {
    let mut lone = atlas();
    lone.bold.insert('z', 3);
    lone.bold.insert('b', 0);
    let found = lone.problems("");
    assert!(found.contains(&"bold 'z' (U+007A) has no narrow glyph".to_owned()), "{found:?}");
    assert!(found.iter().any(|p| p.contains("share cell index 0")), "{found:?}");
    let mut outside = atlas();
    outside.ink.as_mut().unwrap().capitals.right = 8;
    outside.ink.as_mut().unwrap().columns.insert('b', [5, 1]);
    outside.bold_ink.as_mut().unwrap().capitals.top = 12;
    let found = outside.problems("");
    assert_eq!(
        found,
        [
            "ink's capitals must lie inside the cell, left of right and above bottom",
            "ink's columns of 'b' (U+0062) must lie inside the cell",
            "bold_ink's capitals must lie inside the cell, left of right and above bottom",
        ]
    );
}

#[test]
fn widths_carry_the_ink_and_a_caps_cells() {
    let widths = CellWidths::of(&atlas());
    let ink = widths.ink(false).unwrap();
    assert_eq!(ink.columns_of('b'), (1, 5));
    assert_eq!(ink.columns_of('A'), (1, 6));
    assert_eq!(widths.ink(true).unwrap().capitals.right, 7);
    assert_eq!(widths.key_cap_cells(), None);
    let capped = widths.clone().with_key_cap_cells(Some(1));
    assert_eq!(capped.key_cap_cells(), Some(1));
    assert_ne!(capped, widths);
    assert_eq!(CellWidths::NARROW.ink(false), None);
}

#[test]
fn an_atlas_file_without_bold_or_ink_still_reads() {
    let old = "(cell_w: 8, cell_h: 16, columns: 4, glyphs: {'A': 0})";
    let def: FontAtlasDef = ron::from_str(old).unwrap();
    assert!(def.bold.is_empty() && def.ink.is_none() && def.bold_ink.is_none());
    let new = "(cell_w: 8, cell_h: 16, columns: 4, glyphs: {'A': 0}, bold: {'A': 1}, \
               ink: Some((capitals: (left: 1, top: 2, right: 6, bottom: 11), columns: {'i': (2, 4)})))";
    let def: FontAtlasDef = ron::from_str(new).unwrap();
    assert_eq!(def.bold_rect('A'), Some(def.cell_rect(1)));
    assert_eq!(def.ink.unwrap().columns_of('i'), (2, 4));
}
