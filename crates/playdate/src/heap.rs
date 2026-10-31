//! The arithmetic of the allocator over the SDK's `realloc`
//! (`crate::device`).
//!
//! The SDK's `realloc` returns blocks aligned to [`MIN_ALIGN`] bytes. A
//! bigger alignment gets a block [`padded_size`] long; the pointer handed
//! out is [`offset`] bytes into it, and the block's own address is kept in
//! the word just before that pointer, so freeing finds it again.

/// The alignment every block from the SDK's `realloc` has.
pub const MIN_ALIGN: usize = 8;

/// Whether a block of alignment `align` needs padding.
pub fn needs_padding(align: usize) -> bool {
    align > MIN_ALIGN
}

/// The size to ask the SDK for, for `size` bytes aligned to `align`
/// (padded); `None` when that overflows.
pub fn padded_size(size: usize, align: usize) -> Option<usize> {
    size.checked_add(align)
}

/// How far into a padded block at address `raw` the pointer handed out
/// is, for alignment `align` (a power of two above [`MIN_ALIGN`]).
///
/// The result is between [`MIN_ALIGN`] and `align`: the pointer is aligned,
/// stays inside the padded block, and leaves room before it for the
/// block's address.
pub fn offset(raw: usize, align: usize) -> usize {
    align - raw % align
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_alignments_above_the_sdks_are_padded() {
        assert!(!needs_padding(1));
        assert!(!needs_padding(MIN_ALIGN));
        assert!(needs_padding(16));
    }

    #[test]
    fn the_padded_size_adds_the_alignment() {
        assert_eq!(padded_size(100, 16), Some(116));
        assert_eq!(padded_size(usize::MAX, 16), None);
    }

    #[test]
    fn the_pointer_is_aligned_inside_the_block_with_room_for_an_address() {
        for align in [16, 32, 64, 4096] {
            for raw in (0..2 * align).step_by(MIN_ALIGN).map(|r| 0x2000_0000 + r) {
                let at = offset(raw, align);
                assert_eq!((raw + at) % align, 0, "raw {raw:#x} align {align}");
                assert!(at >= MIN_ALIGN && at >= size_of::<usize>(), "{at}");
                assert!(at <= align, "{at}");
            }
        }
    }

    #[test]
    fn an_already_aligned_block_still_moves_a_whole_alignment() {
        assert_eq!(offset(0x1000, 16), 16);
        assert_eq!(offset(0x1008, 16), 8);
    }
}
