use alloc::rc::Rc;

use super::*;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Item, Rect};

const ID: PaintedId = PaintedId::new("plasma");

fn white() -> Rgb {
    Rgb::new(255, 255, 255)
}

fn buf() -> GlyphBuffer {
    GlyphBuffer::new(10, 4, Cell::new(' ', white(), Rgb::new(0, 0, 0)))
}

#[test]
fn each_kind_is_made_its_size_with_its_bytes() {
    let rgba = Painted::rgba(5, 3);
    assert_eq!((rgba.size(), rgba.kind()), ((5, 3), PaintedKind::Rgba));
    assert_eq!((rgba.stride(), rgba.pixels().len()), (20, 60));
    assert!(rgba.pixels().iter().all(|&b| b == 0));
    assert!(rgba.palette().is_empty() && rgba.mask().is_none());

    let colours = [Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)];
    let indexed = Painted::indexed(5, 3, &colours);
    assert_eq!(indexed.kind(), PaintedKind::Indexed);
    assert_eq!((indexed.stride(), indexed.pixels().len()), (5, 15));
    assert_eq!(indexed.palette().len(), PALETTE_LEN);
    assert_eq!(&indexed.palette()[..2], &colours);
    assert!(
        indexed.palette()[2..]
            .iter()
            .all(|&c| c == Rgb::new(0, 0, 0))
    );
    let many = vec![white(); 300];
    assert_eq!(Painted::indexed(1, 1, &many).palette().len(), PALETTE_LEN);

    let one_bit = Painted::one_bit(9, 3);
    assert_eq!(one_bit.kind(), PaintedKind::OneBit);
    assert_eq!((one_bit.stride(), one_bit.pixels().len()), (2, 6));
    assert!(one_bit.mask().is_none());
    let masked = one_bit.with_mask();
    assert_eq!(masked.mask(), Some(&[u8::MAX; 6][..]));
    // Only a one-bit picture takes a mask.
    assert!(Painted::rgba(2, 2).with_mask().mask().is_none());

    let empty = Painted::rgba(0, 7);
    assert_eq!((empty.size(), empty.pixels().len()), ((0, 7), 0));
    for picture in [rgba, indexed, masked, empty] {
        assert_eq!(picture.revision(), 0, "{picture:?}");
    }
}

#[test]
fn borrowing_for_writing_moves_the_revision_and_reading_does_not() {
    let mut picture = Painted::indexed(4, 4, &[white()]);
    let _ = (picture.pixels(), picture.palette(), picture.size());
    assert_eq!(picture.revision(), 0);
    picture.pixels_mut()[0] = 1;
    assert_eq!(picture.revision(), 1);
    // Borrowed without a change still counts.
    let _ = picture.pixels_mut();
    assert_eq!(picture.revision(), 2);
    picture.palette_mut()[1] = Rgb::new(9, 9, 9);
    assert_eq!(picture.revision(), 3);
    let mut one_bit = Painted::one_bit(8, 1).with_mask();
    one_bit.mask_mut().unwrap()[0] = 0;
    assert_eq!(one_bit.revision(), 1);
    // It wraps.
    picture.revision = u32::MAX;
    let _ = picture.pixels_mut();
    assert_eq!(picture.revision(), 0);
}

#[test]
fn every_kind_reads_as_rgba() {
    let mut rgba = Painted::rgba(2, 1);
    rgba.pixels_mut().copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(rgba.rgba_at(1, 0), [5, 6, 7, 8]);

    let mut indexed = Painted::indexed(2, 1, &[Rgb::new(10, 20, 30), Rgb::new(40, 50, 60)]);
    indexed.pixels_mut()[1] = 1;
    assert_eq!(indexed.rgba_at(0, 0), [10, 20, 30, 255]);
    assert_eq!(indexed.rgba_at(1, 0), [40, 50, 60, 255]);
    // An index past the colours given is black.
    indexed.pixels_mut()[1] = 200;
    assert_eq!(indexed.rgba_at(1, 0), [0, 0, 0, 255]);

    let mut one_bit = Painted::one_bit(10, 1).with_mask();
    one_bit
        .pixels_mut()
        .copy_from_slice(&[0b1000_0000, 0b0100_0000]);
    one_bit
        .mask_mut()
        .unwrap()
        .copy_from_slice(&[0b0111_1111, 0xff]);
    // White but left out, black, and white on the second byte.
    assert_eq!(one_bit.rgba_at(0, 0), [255, 255, 255, 0]);
    assert_eq!(one_bit.rgba_at(1, 0), [0, 0, 0, 255]);
    assert_eq!(one_bit.rgba_at(9, 0), [255, 255, 255, 255]);

    let mut out = vec![7; 3];
    one_bit.rgba_into(&mut out);
    assert_eq!(out.len(), 40);
    assert_eq!(&out[36..], &[255, 255, 255, 255]);
    rgba.rgba_into(&mut out);
    assert_eq!(out, rgba.pixels());
}

#[test]
fn the_fingerprint_follows_the_look_not_the_revision() {
    let mut picture = Painted::rgba(3, 3);
    let blank = picture.fingerprint();
    let _ = picture.pixels_mut();
    assert_eq!(picture.fingerprint(), blank);
    picture.pixels_mut()[5] = 1;
    assert_ne!(picture.fingerprint(), blank);
    // Same bytes, another kind or shape: another hash.
    assert_ne!(
        Painted::one_bit(8, 1).fingerprint(),
        Painted::indexed(1, 1, &[]).fingerprint()
    );
    assert_ne!(
        Painted::rgba(1, 2).fingerprint(),
        Painted::rgba(2, 1).fingerprint()
    );
    assert_ne!(
        Painted::one_bit(8, 1).fingerprint(),
        Painted::one_bit(8, 1).with_mask().fingerprint()
    );
    let mut indexed = Painted::indexed(1, 1, &[]);
    let before = indexed.fingerprint();
    indexed.palette_mut()[0] = white();
    assert_ne!(indexed.fingerprint(), before);
}

#[test]
fn a_picture_put_over_another_takes_a_revision_past_it() {
    let mut table = PaintedTable::default();
    assert!(table.is_empty());
    let first = table.get_or_insert_with(ID, || Painted::rgba(2, 2));
    let _ = first.pixels_mut();
    let _ = first.pixels_mut();
    // There already: the maker isn't called, and its size doesn't matter.
    let again = table.get_or_insert_with(ID, || panic!("made twice"));
    assert_eq!((again.size(), again.revision()), ((2, 2), 2));
    let replaced = table.insert(ID, Painted::rgba(4, 4));
    assert_eq!((replaced.size(), replaced.revision()), ((4, 4), 3));
    assert_eq!(table.len(), 1);
    assert_eq!(
        table.iter().map(|(id, _)| id.name()).collect::<Vec<_>>(),
        ["plasma"]
    );
    assert!(table.get_mut(ID).is_some());
    assert_eq!(table.remove(ID).map(|p| p.size()), Some((4, 4)));
    assert!(table.get(ID).is_none());
}

#[test]
fn absorbing_the_same_look_keeps_the_revision_and_another_moves_it() {
    let mut table = PaintedTable::default();
    let mut picture = Painted::rgba(2, 2);
    picture.pixels_mut()[0] = 9;
    table.absorb(ID, &picture);
    assert_eq!(table.get(ID).map(Painted::revision), Some(1));
    // A copy made again, at its own revision: nothing moves.
    let mut same = Painted::rgba(2, 2);
    for _ in 0..5 {
        same.pixels_mut()[0] = 9;
    }
    table.absorb(ID, &same);
    assert_eq!(table.get(ID).map(Painted::revision), Some(1));
    same.pixels_mut()[1] = 1;
    table.absorb(ID, &same);
    let now = table.get(ID).unwrap();
    assert_eq!((now.revision(), now.pixels()[1]), (2, 1));
}

#[test]
fn updating_with_the_same_look_keeps_the_picture_and_its_revision() {
    let mut table = PaintedTable::default();
    let grey = |level: u8| {
        let mut picture = Painted::indexed(2, 2, &[Rgb::new(level, level, level)]);
        let _ = picture.pixels_mut();
        picture
    };
    assert_eq!(table.update(ID, grey(1)).revision(), 1);
    // Made again the same, its own revision moved: the table's stays.
    let mut again = grey(1);
    let _ = again.palette_mut();
    assert_eq!(table.update(ID, again).revision(), 1);
    assert_eq!(table.update(ID, grey(2)).revision(), 2);
    assert_eq!(
        table.get(ID).map(|p| p.palette()[0]),
        Some(Rgb::new(2, 2, 2))
    );
}

#[test]
fn a_painted_item_is_clipped_and_drawing_again_keeps_its_picture() {
    let mut b = buf();
    b.pictures_mut().insert(ID, Painted::rgba(4, 4));
    let src = Rect::new(0, 0, 4, 4);
    // Half off the right edge (the buffer is 80 pixels wide).
    b.add_painted(PaintedSprite::new(
        ID,
        src,
        Rect::new(72, 0, 16, 16),
        Layer::Over,
    ));
    // Nothing to show: dropped.
    b.add_painted(PaintedSprite::new(
        ID,
        Rect::new(0, 0, 0, 4),
        src,
        Layer::Over,
    ));
    b.add_painted(PaintedSprite::new(
        ID,
        src,
        Rect::new(200, 0, 4, 4),
        Layer::Over,
    ));
    let painted = b.painted_sprites();
    assert_eq!(painted.len(), 1);
    assert_eq!(
        (painted[0].dest, painted[0].clip),
        (Rect::new(72, 0, 16, 16), Rect::new(72, 0, 8, 16))
    );
    assert_eq!(b.items()[0].visible(), Rect::new(72, 0, 8, 16));
    assert!(b.sprites().is_empty() && b.overlays().is_empty());
    // Cells replaced under it cut it; the picture stays.
    b.fill_rect(b.bounds(), Cell::new(' ', white(), white()));
    assert_eq!(b.items(), []);
    assert_eq!(b.pictures().len(), 1);
}

#[test]
fn fading_darkens_a_painted_item() {
    let mut b = buf();
    let item = PaintedSprite::new(
        ID,
        Rect::new(0, 0, 4, 4),
        Rect::new(0, 0, 8, 8),
        Layer::Under,
    );
    b.add_painted(item);
    b.fade(0.4);
    assert_eq!(b.painted_sprites()[0].brightness, 102);
}

#[test]
fn a_blit_brings_its_pictures_and_moves_its_items() {
    let mut frame = buf();
    let make = |pixel: u8| {
        let mut part = GlyphBuffer::new(2, 1, Cell::new(' ', white(), white()));
        let picture = part
            .pictures_mut()
            .get_or_insert_with(ID, || Painted::rgba(1, 1));
        picture.pixels_mut()[0] = pixel;
        let one = Rect::new(0, 0, 1, 1);
        part.add_painted(PaintedSprite::new(
            ID,
            one,
            Rect::new(2, 2, 4, 4),
            Layer::Over,
        ));
        part
    };
    frame.blit(&make(5), 3, 1);
    let moved = frame.painted_sprites()[0];
    assert_eq!(moved.dest, Rect::new(26, 18, 4, 4));
    assert_eq!(
        frame
            .pictures()
            .get(ID)
            .map(|p| (p.revision(), p.pixels()[0])),
        Some((1, 5))
    );
    // The same picture made again comes in without a new revision.
    frame.blit(&make(5), 3, 1);
    assert_eq!(frame.pictures().get(ID).map(Painted::revision), Some(1));
    frame.blit(&make(6), 3, 1);
    assert_eq!(
        frame
            .pictures()
            .get(ID)
            .map(|p| (p.revision(), p.pixels()[0])),
        Some((2, 6))
    );
    // A buffer without painted items brings none.
    let mut other = buf();
    other
        .pictures_mut()
        .insert(PaintedId::new("unused"), Painted::rgba(1, 1));
    frame.blit(&other, 0, 0);
    assert_eq!(frame.pictures().len(), 1);
}

#[test]
fn a_backdrops_scene_brings_its_pictures() {
    let mut frame = buf();
    let mut scene = buf();
    scene.pictures_mut().insert(ID, Painted::one_bit(8, 8));
    let one = Rect::new(0, 0, 8, 8);
    scene.add_painted(PaintedSprite::new(ID, one, one, Layer::Over));
    frame.set_backdrop(Rc::new(scene), frame.bounds(), (0.0, 0.0), 1);
    assert_eq!(
        frame.pictures().get(ID).map(Painted::kind),
        Some(PaintedKind::OneBit)
    );
    let scene_items = frame.backdrop().map(|b| b.scene().items().to_vec());
    assert!(matches!(scene_items.as_deref(), Some([Item::Painted(_)])));
}

#[test]
fn its_debug_text_names_it_without_its_pixels() {
    let text = format!("{:?}", Painted::rgba(320, 200));
    assert!(text.starts_with("Painted(320x200 rgba r0 "), "{text}");
    assert!(text.len() < 60, "{text}");
    assert_eq!(ID.to_string(), "plasma");
}
