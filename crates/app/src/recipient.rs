//! The recipient's field (ADR-0118): 32 bytes every program built with this
//! crate reserves in its data, which a closed build's overlay (`shuyi export
//! --recipient`) writes a recipient's code into after the build. A public
//! build's field holds no code. [`recipient`] reads it, for a game that
//! draws a faint line naming the copy's recipient, as a deterrent.
//!
//! What the field holds is circumstantial evidence, never proof: a copy's
//! field can be overwritten, and two copies compared show where it is.

use std::sync::OnceLock;

use shuyi_assets::recipient::{FIELD, FIELD_BYTES, FieldRead, slot_code};

/// The field, as the build leaves it: kept in the program even when nothing
/// else reads it, so the overlay always finds it.
#[used]
static RECIPIENT: [u8; FIELD_BYTES] = FIELD;

/// The field's bytes as the program holds them now.
#[allow(unsafe_code)]
fn field() -> [u8; FIELD_BYTES] {
    // SAFETY: `RECIPIENT` is a static, so the pointer is valid, aligned and
    // never written while the program runs. The read is volatile so the
    // compiler can't fold in the value the build left: an overlay changes the
    // bytes in the program's file after the build.
    unsafe { core::ptr::read_volatile(&raw const RECIPIENT) }
}

/// The code of the recipient this copy was made for, if it is a closed
/// build's copy (`shuyi export --recipient`): 8 characters, such as
/// `7K2Q9MXA`. `None` in a public build, and in a copy whose field was
/// changed.
///
/// A game may draw it faintly in a corner, as a deterrent; a copy's code
/// names its recipient only in the ledger the export keeps.
pub fn recipient() -> Option<&'static str> {
    static CODE: OnceLock<Option<String>> = OnceLock::new();
    CODE.get_or_init(|| {
        let mut slot = [0u8; 16];
        slot.copy_from_slice(&field()[16..]);
        match slot_code(&slot) {
            FieldRead::Code(code) => Some(code.to_string()),
            FieldRead::Empty | FieldRead::Broken => None,
        }
    })
    .as_deref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_holds_the_empty_field_and_no_recipient() {
        assert_eq!(field(), FIELD);
        assert_eq!(recipient(), None);
    }
}
