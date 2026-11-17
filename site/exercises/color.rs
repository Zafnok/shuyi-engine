//! The palette's colors as a person names them, by which of red, green and
//! blue stand out.

/// How far a part must stand above another for the color to be named by it.
const MARGIN: i16 = 0x30;

/// `color` as the palette writes it, `#rrggbb`.
pub fn hex([red, green, blue]: [u8; 3]) -> String {
    format!("#{red:02x}{green:02x}{blue:02x}")
}

/// What a person calls `color`: `"red"`, `"green"` or `"blue"` when that
/// part stands well above the other two, `"yellow"` when red and green are
/// both bright and blue well below them; none for a grey, a white, or a
/// color between those.
pub fn name([red, green, blue]: [u8; 3]) -> Option<&'static str> {
    let (red, green, blue) = (i16::from(red), i16::from(green), i16::from(blue));
    let above = |part: i16, others: [i16; 2]| others.iter().all(|&other| part >= other + MARGIN);
    if red >= 0xa0 && green >= 0xa0 && blue + MARGIN * 2 <= red.min(green) {
        Some("yellow")
    } else if above(red, [green, blue]) {
        Some("red")
    } else if above(green, [red, blue]) {
        Some("green")
    } else if above(blue, [red, green]) {
        Some("blue")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_are_named_by_the_parts_that_stand_out() {
        assert_eq!(hex([0x5a, 0x9b, 0xff]), "#5a9bff");
        assert_eq!(name([0x20, 0x40, 0xc0]), Some("blue"));
        assert_eq!(name([0xff, 0xd7, 0x5a]), Some("yellow"));
        assert_eq!(name([0xe0, 0x30, 0x30]), Some("red"));
        assert_eq!(name([0x30, 0xc0, 0x40]), Some("green"));
        assert_eq!(name([0x26, 0x26, 0x2e]), None);
        assert_eq!(name([0xff, 0xff, 0xff]), None);
        assert_eq!(name([0xc8, 0x9a, 0x5a]), None);
    }
}
