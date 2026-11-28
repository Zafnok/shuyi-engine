//! A Windows program `shuyi export windows` wrote, read back for the game's
//! identity. CI's `export` job exports its fixture game, Lune
//! Café (`tools/export-check/lune-cafe`), names the program in
//! `SHUYI_EXPORTED_EXE` and runs this with `--ignored`
//! (`tools/export-check/identity.sh`).

/// `text` in UTF-16LE, as a Windows resource holds a string.
fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// Whether `bytes` holds `needle`.
fn holds(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

#[test]
#[ignore = "CI's export job runs it on the program it exported, named in SHUYI_EXPORTED_EXE"]
fn the_exported_program_says_who_the_game_is() {
    let path = std::env::var_os("SHUYI_EXPORTED_EXE").expect("SHUYI_EXPORTED_EXE names no file");
    let exe = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(holds(&exe, b".rsrc\0\0\0"), "no .rsrc section in the table");
    for text in ["VS_VERSION_INFO", "Lune Café", "Shuyi Engine", "1.2.3"] {
        assert!(holds(&exe, &utf16(text)), "no {text:?} in UTF-16LE");
    }
    // `VS_FIXEDFILEINFO`: its signature, its structure's version, then the
    // file's version 1.2.3.4 as two 32-bit numbers, major and minor first.
    let fixed: Vec<u8> = [0xFEEF_04BD_u32, 0x0001_0000, 0x0001_0002, 0x0003_0004]
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();
    assert!(holds(&exe, &fixed), "no FILEVERSION 1,2,3,4");
}
