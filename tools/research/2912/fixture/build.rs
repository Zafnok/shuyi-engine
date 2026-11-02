//! Generate two reproducible, seed-dependent research builds of one tiny rule.

use std::env;
use std::fs;
use std::path::PathBuf;

const OP_GATHERS: usize = 0;
const OP_PHASE: usize = 1;
const OP_ADD: usize = 2;
const OP_CONSTANT: usize = 3;
const OP_REMAINDER: usize = 4;
const OP_IS_ZERO: usize = 5;
const OP_NOP: usize = 6;
const OP_HALT: usize = 7;

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn seed() -> Result<u64, Box<dyn std::error::Error>> {
    let raw = env::var("SHUYI_KERNEL_BUILD_SEED").unwrap_or_else(|_| "0".to_owned());
    if let Some(hex) = raw.strip_prefix("0x") {
        Ok(u64::from_str_radix(hex, 16)?)
    } else {
        Ok(raw.parse::<u64>()?)
    }
}

fn instruction(program: &mut Vec<u8>, opcode: u8, operand: u8) {
    program.extend_from_slice(&[opcode, operand]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=SHUYI_KERNEL_BUILD_SEED");
    let out = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| std::io::Error::other("Cargo supplies OUT_DIR"))?,
    );
    let mut stream = seed()? ^ 0x8514_5b1a_9347_117d;

    // The layout control executes the original arithmetic. These bytes are
    // touched but do not affect its answer, so byte diversity earns no credit.
    let layout: Vec<u8> = (0..256)
        .map(|_| next(&mut stream).to_le_bytes()[0])
        .collect();
    fs::write(out.join("kernel_layout.bin"), layout)?;

    // Assign a fresh encoding to each VM operation and encode its constant.
    let mut opcodes = [0x12, 0x2d, 0x43, 0x5a, 0x71, 0x8b, 0xa6, 0xe4];
    for index in (1..opcodes.len()).rev() {
        let bound = u64::try_from(index + 1)?;
        let other = usize::try_from(next(&mut stream) % bound)?;
        opcodes.swap(index, other);
    }
    let key = next(&mut stream).to_le_bytes()[0];
    let definitions = format!(
        "const OP_GATHERS: u8 = {g};\nconst OP_PHASE: u8 = {p};\n\
         const OP_ADD: u8 = {a};\nconst OP_CONSTANT: u8 = {c};\n\
         const OP_REMAINDER: u8 = {r};\nconst OP_IS_ZERO: u8 = {z};\n\
         const OP_NOP: u8 = {n};\nconst OP_HALT: u8 = {h};\nconst KEY: u8 = {key};\n",
        g = opcodes[OP_GATHERS],
        p = opcodes[OP_PHASE],
        a = opcodes[OP_ADD],
        c = opcodes[OP_CONSTANT],
        r = opcodes[OP_REMAINDER],
        z = opcodes[OP_IS_ZERO],
        n = opcodes[OP_NOP],
        h = opcodes[OP_HALT],
    );
    fs::write(out.join("kernel_opcodes.rs"), definitions)?;

    let mut program = Vec::new();
    for (operation, operand) in [
        (OP_GATHERS, 0),
        (OP_PHASE, 0),
        (OP_ADD, 0),
        (OP_CONSTANT, 3 ^ key),
        (OP_REMAINDER, 0),
        (OP_IS_ZERO, 0),
    ] {
        if next(&mut stream) & 1 == 0 {
            instruction(
                &mut program,
                opcodes[OP_NOP],
                next(&mut stream).to_le_bytes()[0],
            );
        }
        instruction(&mut program, opcodes[operation], operand);
    }
    instruction(&mut program, opcodes[OP_HALT], 0);
    fs::write(out.join("kernel_program.bin"), program)?;
    Ok(())
}
