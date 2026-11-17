//! The ink of an atlas's glyphs (ticket 3215).

use super::*;

/// An 8 × 16 atlas of `A` and `b`, `b` inked in columns 1 to 5.
fn atlas() -> FontAtlasDef {
    let mut ink = FontInk::new(InkBox::new(1, 2, 6, 11));
    ink.columns.insert('b', [1, 5]);
    FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 4,
        glyphs: BTreeMap::from([('A', 0), ('b', 1)]),
        ink: Some(ink),
        ..FontAtlasDef::default()
    }
}

#[test]
fn ink_must_lie_inside_the_cell() {
    assert_eq!(atlas().problems("Ab"), Vec::<String>::new());
    let mut outside = atlas();
    let ink = outside.ink.as_mut().unwrap();
    ink.capitals.right = 8;
    ink.columns.insert('b', [5, 1]);
    assert_eq!(
        outside.problems(""),
        [
            "ink's capitals must lie inside the cell, left of right and above bottom",
            "ink's columns of 'b' (U+0062) must lie inside the cell",
        ]
    );
    let mut upside_down = atlas();
    upside_down.ink.as_mut().unwrap().capitals.top = 12;
    assert_eq!(upside_down.problems("").len(), 1);
    let mut inside_out = atlas();
    inside_out.ink.as_mut().unwrap().capitals.left = 7;
    assert_eq!(inside_out.problems("").len(), 1);
    // One column, one row, or a glyph one column wide, is ink.
    let mut thin = atlas();
    let ink = thin.ink.as_mut().unwrap();
    ink.capitals = InkBox::new(3, 5, 3, 5);
    ink.columns.insert('b', [2, 2]);
    assert_eq!(thin.problems("Ab"), Vec::<String>::new());
}

#[test]
fn widths_carry_the_ink_and_a_caps_cells() {
    let widths = CellWidths::of(&atlas());
    let ink = widths.ink().unwrap();
    assert_eq!(ink.columns_of('b'), (1, 5));
    assert_eq!(ink.columns_of('A'), (1, 6));
    assert_eq!(widths.key_cap_cells(), None);
    let capped = widths.clone().with_key_cap_cells(Some(1));
    assert_eq!(capped.key_cap_cells(), Some(1));
    assert_ne!(capped, widths);
    assert_eq!(CellWidths::NARROW.ink(), None);
}

#[test]
fn an_atlas_file_without_ink_still_reads() {
    let old = "(cell_w: 8, cell_h: 16, columns: 4, glyphs: {'A': 0})";
    let def: FontAtlasDef = ron::from_str(old).unwrap();
    assert!(def.ink.is_none());
    let new = "(cell_w: 8, cell_h: 16, columns: 4, glyphs: {'A': 0}, \
               ink: Some((capitals: (left: 1, top: 2, right: 6, bottom: 11), columns: {'i': (2, 4)})))";
    let def: FontAtlasDef = ron::from_str(new).unwrap();
    assert_eq!(def.ink.unwrap().columns_of('i'), (2, 4));
}
