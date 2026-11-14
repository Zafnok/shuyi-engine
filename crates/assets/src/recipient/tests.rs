use super::*;
use proptest::prelude::*;

fn code(bits: u32) -> Code {
    Code::new(bits).expect("not zero")
}

#[test]
fn zero_is_no_code() {
    assert_eq!(Code::new(0), None);
    assert_eq!(Code::from_bytes([0; CODE_BYTES]), None);
}

#[test]
fn the_check_refuses_every_one_bit_change() {
    for bits in [1, 0x1234_5678, 0xffff_ffff, 0x8000_0000] {
        let bytes = code(bits).to_bytes();
        assert_eq!(Code::from_bytes(bytes), Some(code(bits)));
        for at in 0..CODE_BYTES * 8 {
            let mut broken = bytes;
            broken[at / 8] ^= 1 << (at % 8);
            assert_eq!(Code::from_bytes(broken), None, "{bits:#x}, bit {at}");
        }
    }
}

#[test]
fn the_check_refuses_every_two_bit_change_in_one_byte() {
    let bytes = code(0x0bad_cafe).to_bytes();
    for byte in 0..CODE_BYTES {
        for a in 0..8 {
            for b in a + 1..8 {
                let mut broken = bytes;
                broken[byte] ^= (1 << a) | (1 << b);
                assert_eq!(Code::from_bytes(broken), None);
            }
        }
    }
}

#[test]
fn it_reads_as_it_is_written_in_any_case_and_with_hyphens() {
    let c = code(0x1234_5678);
    let text = c.to_string();
    assert_eq!(text.len(), 8);
    assert!(text.bytes().all(|b| ALPHABET.contains(&b)), "{text}");
    assert_eq!(text.parse::<Code>(), Ok(c));
    assert_eq!(text.to_lowercase().parse::<Code>(), Ok(c));
    let hyphened = format!("{}-{}", &text[..4], &text[4..]);
    assert_eq!(hyphened.parse::<Code>(), Ok(c));
}

#[test]
fn text_that_isnt_a_code_is_refused() {
    assert_eq!("".parse::<Code>(), Err(CodeError::Shape));
    assert_eq!("7K2Q9MX".parse::<Code>(), Err(CodeError::Shape));
    assert_eq!("7K2Q9MXA9".parse::<Code>(), Err(CodeError::Shape));
    assert_eq!("7K2Q9MX!".parse::<Code>(), Err(CodeError::Shape));
    let text = code(0x1234_5678).to_string();
    let last = text.chars().last().expect("8 characters");
    let other = if last == '0' { '1' } else { '0' };
    let wrong = format!("{}{other}", &text[..7]);
    assert_eq!(wrong.parse::<Code>(), Err(CodeError::Check));
}

#[test]
fn a_public_programs_field_holds_no_code_and_no_zero_byte() {
    assert_eq!(field_code(&FIELD), FieldRead::Empty);
    assert!(FIELD.iter().all(|&b| b != 0), "{FIELD:?}");
    assert!(
        FIELD.iter().all(|b| !b.is_ascii()),
        "the field reads as text"
    );
}

#[test]
fn a_field_holds_its_code() {
    let c = code(0x00c0_ffee);
    assert_eq!(field_code(&field_of(Some(c))), FieldRead::Code(c));
    assert_eq!(field_of(None), FIELD);
}

#[test]
fn a_changed_slot_is_broken_not_another_code() {
    let field = field_of(Some(code(0x00c0_ffee)));
    for at in 16..FIELD_BYTES {
        let mut changed = field;
        changed[at] ^= 0x10;
        assert_eq!(field_code(&changed), FieldRead::Broken, "byte {at}");
    }
    let mut zeroed = field;
    zeroed[16..].fill(0);
    assert_eq!(field_code(&zeroed), FieldRead::Broken);
}

#[test]
fn writing_the_field_changes_only_its_slot() {
    let mut program: Vec<u8> = (0..4096u32).map(|i| (i * 31 % 251) as u8).collect();
    program[1000..1000 + FIELD_BYTES].copy_from_slice(&FIELD);
    let before = program.clone();
    let c = code(0x5eed_1234);
    assert_eq!(write_field(&mut program, c), 1);
    for (at, (a, b)) in before.iter().zip(&program).enumerate() {
        if a != b {
            assert!((1016..1032).contains(&at), "byte {at} changed");
        }
    }
    assert_eq!(read_fields(&program), vec![(1000, FieldRead::Code(c))]);
}

#[test]
fn a_program_with_two_slices_gets_both_and_one_without_gets_none() {
    let mut program = vec![7u8; 300];
    program[10..10 + FIELD_BYTES].copy_from_slice(&FIELD);
    program[200..200 + FIELD_BYTES].copy_from_slice(&FIELD);
    assert_eq!(write_field(&mut program, code(9)), 2);
    let mut none = vec![7u8; 300];
    assert_eq!(write_field(&mut none, code(9)), 0);
    assert_eq!(none, vec![7u8; 300]);
}

proptest! {
    #[test]
    fn any_code_reads_back_from_its_bytes_text_bits_and_field(bits in 1u32..) {
        let c = code(bits);
        prop_assert_eq!(Code::from_bytes(c.to_bytes()), Some(c));
        prop_assert_eq!(c.to_string().parse::<Code>(), Ok(c));
        let mut all = [false; CODE_BYTES * 8];
        for (at, bit) in all.iter_mut().enumerate() {
            *bit = c.bit(at);
        }
        prop_assert_eq!(Code::from_bits(&all), Some(c));
        prop_assert_eq!(field_code(&field_of(Some(c))), FieldRead::Code(c));
    }
}
