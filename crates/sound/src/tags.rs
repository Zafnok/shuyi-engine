//! What a file's comments say of how it loops: the `LOOPSTART` and `LOOPLENGTH`
//! tags many music tools write in an Ogg Vorbis file, in samples of one channel
//! (frames), from the start of the sound (ticket 2397).
//!
//! A region the tags name is checked whole, never clamped or rounded: both tags or
//! neither, each a whole number, a length of at least one frame, a start and length
//! whose sum fits, and an end no later than the sound's own, where that is known.

use alloc::format;

use crate::error::Error;
use crate::mix::Region;

/// The tag of the region's first frame.
pub const LOOP_START: &str = "LOOPSTART";
/// The tag of how many frames the region has.
pub const LOOP_LENGTH: &str = "LOOPLENGTH";

/// The region `start` and `length`, the values of a file's [`LOOP_START`] and
/// [`LOOP_LENGTH`] tags, name in a sound `frames` long (if that is known): `None`
/// if it has neither tag, an error naming the tag if they don't name one.
pub fn loop_region(
    start: Option<&str>,
    length: Option<&str>,
    frames: Option<u64>,
) -> Result<Option<Region>, Error> {
    let (start, length) = match (start, length) {
        (None, None) => return Ok(None),
        (Some(_), None) => return Err(problem(LOOP_START, "is there without LOOPLENGTH")),
        (None, Some(_)) => return Err(problem(LOOP_LENGTH, "is there without LOOPSTART")),
        (Some(start), Some(length)) => (number(LOOP_START, start)?, number(LOOP_LENGTH, length)?),
    };
    let end = start.checked_add(length).ok_or_else(|| {
        problem(
            LOOP_LENGTH,
            &format!("{length} after LOOPSTART {start} is past the largest frame there is"),
        )
    })?;
    if let Some(frames) = frames.filter(|&frames| end > frames) {
        let why =
            format!("{length} ends the region at frame {end}, past the sound's end at {frames}");
        return Err(problem(LOOP_LENGTH, &why));
    }
    Region::new(start, end)
        .map(Some)
        .ok_or_else(|| problem(LOOP_LENGTH, "is 0: a region has a frame at least"))
}

/// `value`, the tag `tag`'s, as a number of frames.
fn number(tag: &'static str, value: &str) -> Result<u64, Error> {
    let digits = value.trim();
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(problem(
            tag,
            &format!("is {value:?}, not a whole number of frames"),
        ));
    }
    digits.parse().map_err(|_| {
        problem(
            tag,
            &format!("is {digits}, past the largest frame there is"),
        )
    })
}

fn problem(tag: &'static str, why: &str) -> Error {
    Error::LoopTag {
        tag,
        problem: why.into(),
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use proptest::prelude::*;

    use super::*;

    fn region(start: u64, end: u64) -> Option<Region> {
        Region::new(start, end)
    }

    /// What the problem says, or `"ok"`.
    fn said(
        start: Option<&str>,
        length: Option<&str>,
        frames: Option<u64>,
    ) -> alloc::string::String {
        match loop_region(start, length, frames) {
            Ok(_) => "ok".into(),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn both_tags_name_a_half_open_region_in_frames() {
        let got = loop_region(Some("44100"), Some("44100"), Some(88_200));
        assert_eq!(got, Ok(region(44_100, 88_200)));
        // Spaces around a value are an encoder's, and a length to the very end fits.
        assert_eq!(
            loop_region(Some(" 0 "), Some("10\n"), Some(10)),
            Ok(region(0, 10))
        );
        // A sound whose end isn't known is held to nothing more.
        assert_eq!(loop_region(Some("5"), Some("7"), None), Ok(region(5, 12)));
    }

    #[test]
    fn no_tags_is_no_region() {
        assert_eq!(loop_region(None, None, Some(10)), Ok(None));
    }

    #[test]
    fn a_tag_alone_or_unreadable_is_an_error_naming_it() {
        assert_eq!(
            said(Some("1"), None, None),
            "LOOPSTART is there without LOOPLENGTH"
        );
        assert_eq!(
            said(None, Some("1"), None),
            "LOOPLENGTH is there without LOOPSTART"
        );
        for bad in ["", " ", "-1", "+1", "1.5", "1e3", "0x10", "ten", "1 2"] {
            assert_eq!(
                said(Some(bad), Some("1"), None),
                format!("LOOPSTART is {bad:?}, not a whole number of frames"),
            );
            assert!(
                said(Some("1"), Some(bad), None).starts_with("LOOPLENGTH is "),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_region_of_nothing_past_the_end_or_overflowing_is_an_error() {
        assert_eq!(
            said(Some("3"), Some("0"), None),
            "LOOPLENGTH is 0: a region has a frame at least"
        );
        assert_eq!(
            said(Some("5"), Some("6"), Some(10)),
            "LOOPLENGTH 6 ends the region at frame 11, past the sound's end at 10"
        );
        let max = u64::MAX.to_string();
        assert_eq!(
            said(Some(&max), Some("1"), None),
            format!("LOOPLENGTH 1 after LOOPSTART {max} is past the largest frame there is")
        );
        assert_eq!(
            said(Some("1"), Some("18446744073709551616"), None),
            "LOOPLENGTH is 18446744073709551616, past the largest frame there is"
        );
    }

    proptest! {
        /// Every pair of numbers is a region exactly when it fits the sound, and
        /// then it is those frames, never clamped.
        #[test]
        fn any_numbers_are_a_region_only_when_it_fits(
            start in 0u64..1_000,
            length in 0u64..1_000,
            frames in proptest::option::of(0u64..2_000),
        ) {
            let got = loop_region(Some(&start.to_string()), Some(&length.to_string()), frames);
            let fits = length > 0 && frames.is_none_or(|frames| start + length <= frames);
            if fits {
                prop_assert_eq!(got, Ok(region(start, start + length)));
            } else {
                let refused = matches!(got, Err(Error::LoopTag { tag: LOOP_LENGTH, .. }));
                prop_assert!(refused, "{:?}", got);
            }
        }
    }
}
