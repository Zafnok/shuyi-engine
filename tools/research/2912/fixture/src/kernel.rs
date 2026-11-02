//! A deliberately small, cold integer rule for comparing research builds.
//!
//! The production engine never calls this fixture. The VM is a treatment to
//! measure, not a claim that client-side bytecode conceals a game's rules.

#[cfg(all(feature = "layout-kernel", feature = "vm-kernel"))]
compile_error!("layout-kernel and vm-kernel are separate experimental arms");

#[cfg(not(any(feature = "layout-kernel", feature = "vm-kernel")))]
pub(crate) fn gather_bonus(gathers: u8, bonus_phase: u8) -> bool {
    (u16::from(gathers) + u16::from(bonus_phase)).is_multiple_of(3)
}

#[cfg(any(test, all(feature = "layout-kernel", not(feature = "vm-kernel"))))]
fn layout_index(gathers: u8, bonus_phase: u8) -> usize {
    usize::from(gathers ^ bonus_phase)
}

#[cfg(any(test, all(feature = "layout-kernel", not(feature = "vm-kernel"))))]
pub(crate) fn layout_bonus(gathers: u8, bonus_phase: u8) -> bool {
    const LAYOUT: &[u8; 256] = include_bytes!(concat!(env!("OUT_DIR"), "/kernel_layout.bin"));
    let _ = std::hint::black_box(LAYOUT[layout_index(gathers, bonus_phase)]);
    (u16::from(gathers) + u16::from(bonus_phase)).is_multiple_of(3)
}

#[cfg(all(feature = "layout-kernel", not(feature = "vm-kernel")))]
pub(crate) use layout_bonus as gather_bonus;

#[cfg(any(test, all(feature = "vm-kernel", not(feature = "layout-kernel"))))]
mod vm {
    include!(concat!(env!("OUT_DIR"), "/kernel_opcodes.rs"));

    const PROGRAM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kernel_program.bin"));

    pub(crate) fn gather_bonus(gathers: u8, bonus_phase: u8) -> bool {
        let mut stack = [0_u16; 4];
        let mut size = 0;
        let program = std::hint::black_box(PROGRAM);
        for instruction in program.as_chunks::<2>().0 {
            match instruction[0] {
                OP_GATHERS => {
                    stack[size] = u16::from(gathers);
                    size += 1;
                }
                OP_PHASE => {
                    stack[size] = u16::from(bonus_phase);
                    size += 1;
                }
                OP_ADD => {
                    size -= 1;
                    stack[size - 1] += stack[size];
                }
                OP_CONSTANT => {
                    stack[size] = u16::from(instruction[1] ^ KEY);
                    size += 1;
                }
                OP_REMAINDER => {
                    size -= 1;
                    stack[size - 1] %= stack[size];
                }
                OP_IS_ZERO => stack[size - 1] = u16::from(stack[size - 1] == 0),
                OP_NOP => {}
                OP_HALT => return stack[size - 1] != 0,
                _ => unreachable!("the generated program has an unknown opcode"),
            }
        }
        unreachable!("the generated program must halt")
    }

    #[cfg(test)]
    mod tests {
        use super::{
            OP_ADD, OP_CONSTANT, OP_GATHERS, OP_HALT, OP_IS_ZERO, OP_NOP, OP_PHASE, OP_REMAINDER,
            PROGRAM,
        };

        #[test]
        fn generated_program_has_halt_and_seeded_padding() {
            assert_eq!(PROGRAM.len() % 2, 0);
            assert_eq!(PROGRAM[PROGRAM.len() - 2], OP_HALT);
            assert!(PROGRAM.len() >= 14);
            let valid = [
                OP_GATHERS,
                OP_PHASE,
                OP_ADD,
                OP_CONSTANT,
                OP_REMAINDER,
                OP_IS_ZERO,
                OP_NOP,
                OP_HALT,
            ];
            assert!(
                PROGRAM
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .all(|pair| valid.contains(&pair[0]))
            );
        }
    }
}

#[cfg(all(feature = "vm-kernel", not(feature = "layout-kernel")))]
pub(crate) use vm::gather_bonus;

#[cfg(test)]
mod tests {
    use super::{layout_bonus, layout_index, vm};

    #[test]
    fn layout_touches_the_seeded_slot_selected_by_both_inputs() {
        assert_eq!(layout_index(0, 0), 0);
        assert_eq!(layout_index(0b1100, 0b1010), 0b0110);
        assert_eq!(layout_index(0xff, 0x55), 0xaa);
    }

    #[test]
    fn every_u8_pair_matches_the_integer_rule_in_all_arms() {
        for gathers in u8::MIN..=u8::MAX {
            for phase in u8::MIN..=u8::MAX {
                let expected = (u16::from(gathers) + u16::from(phase)).is_multiple_of(3);
                assert_eq!(
                    super::gather_bonus(gathers, phase),
                    expected,
                    "active: {gathers}, {phase}"
                );
                assert_eq!(
                    layout_bonus(gathers, phase),
                    expected,
                    "layout: {gathers}, {phase}"
                );
                assert_eq!(
                    vm::gather_bonus(gathers, phase),
                    expected,
                    "VM: {gathers}, {phase}"
                );
            }
        }
    }
}
