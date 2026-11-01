//! The archives packages are shipped in: a zip (Windows, the web) and a
//! `tar.gz` (Linux). Both are written the same for the same files: every
//! entry at one fixed time (1980-01-01), in the order given, so an export
//! made twice gives the same bytes.

use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use flate2::write::DeflateEncoder;
use flate2::{Compression, GzBuilder};

/// A file to put in an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its name in the archive, with `/` between folders.
    pub name: String,
    /// The file on disk.
    pub path: PathBuf,
    /// Whether it is a program (mode 755 rather than 644).
    pub executable: bool,
}

/// Every file under `dir`, named from `prefix` (`""` for the archive's
/// root, `"game-1.0-linux/"` for a folder), sorted by name; `executable`
/// says which are programs.
pub fn entries(
    dir: &Path,
    prefix: &str,
    executable: &dyn Fn(&Path) -> bool,
) -> Result<Vec<Entry>, String> {
    let mut found = Vec::new();
    walk(dir, prefix, executable, &mut found)?;
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

fn walk(
    dir: &Path,
    prefix: &str,
    executable: &dyn Fn(&Path) -> bool,
    found: &mut Vec<Entry>,
) -> Result<(), String> {
    let list = fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for item in list {
        let path = item
            .map_err(|e| format!("read {}: {e}", dir.display()))?
            .path();
        let name = format!(
            "{prefix}{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        if path.is_dir() {
            walk(&path, &format!("{name}/"), executable, found)?;
        } else {
            found.push(Entry {
                executable: executable(&path),
                name,
                path,
            });
        }
    }
    Ok(())
}

/// The DOS date of 1980-01-01, zip's first day; its time is 00:00.
const DOS_DATE: u16 = (1 << 5) | 1;

/// Writes `entries` into a zip at `out`, each deflated, with UTF-8 names.
pub fn zip(out: &Path, entries: &[Entry]) -> Result<(), String> {
    let mut body = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let data =
            fs::read(&entry.path).map_err(|e| format!("read {}: {e}", entry.path.display()))?;
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(&data)
            .map_err(|e| format!("deflate {}: {e}", entry.name))?;
        let packed = encoder
            .finish()
            .map_err(|e| format!("deflate {}: {e}", entry.name))?;
        let too_big = || format!("zip: {} is over 4 GiB", entry.name);
        let size = u32::try_from(data.len()).map_err(|_| too_big())?;
        let packed_size = u32::try_from(packed.len()).map_err(|_| too_big())?;
        let offset = u32::try_from(body.len()).map_err(|_| too_big())?;
        let crc = crc32fast::hash(&data);
        let name = entry.name.as_bytes();
        let name_len = u16::try_from(name.len())
            .map_err(|_| format!("zip: {} is too long a name", entry.name))?;
        // The fields both headers share, from "version needed" to the
        // name's length (extra field: none).
        let mut shared = Vec::new();
        for half in [20u16, 0x0800, 8, 0, DOS_DATE] {
            shared.extend(half.to_le_bytes());
        }
        for word in [crc, packed_size, size] {
            shared.extend(word.to_le_bytes());
        }
        shared.extend(name_len.to_le_bytes());
        shared.extend(0u16.to_le_bytes());

        body.extend(0x0403_4b50u32.to_le_bytes());
        body.extend(&shared);
        body.extend(name);
        body.extend(&packed);

        let mode: u32 = if entry.executable {
            0o100_755
        } else {
            0o100_644
        };
        central.extend(0x0201_4b50u32.to_le_bytes());
        // Made by Unix (3), zip 3.0: its external attributes are a mode.
        central.extend(0x031eu16.to_le_bytes());
        central.extend(&shared);
        // Comment, disk, internal attributes: none.
        central.extend([0u8; 6]);
        central.extend((mode << 16).to_le_bytes());
        central.extend(offset.to_le_bytes());
        central.extend(name);
    }
    let count = u16::try_from(entries.len()).map_err(|_| "zip: over 65535 files".to_owned())?;
    let too_big = || "zip: over 4 GiB".to_owned();
    let central_size = u32::try_from(central.len()).map_err(|_| too_big())?;
    let central_at = u32::try_from(body.len()).map_err(|_| too_big())?;
    body.extend(central);
    body.extend(0x0605_4b50u32.to_le_bytes());
    body.extend([0u8; 4]);
    body.extend(count.to_le_bytes());
    body.extend(count.to_le_bytes());
    body.extend(central_size.to_le_bytes());
    body.extend(central_at.to_le_bytes());
    body.extend(0u16.to_le_bytes());
    fs::write(out, body).map_err(|e| format!("write {}: {e}", out.display()))
}

/// The names of the entries of the zip `bytes`, in order, from its
/// central directory.
pub fn zip_names(bytes: &[u8]) -> Result<Vec<String>, String> {
    let end = bytes
        .windows(4)
        .rposition(|w| w == 0x0605_4b50u32.to_le_bytes())
        .ok_or("zip: no end of central directory")?;
    let half = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let word =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let count = half(end + 10);
    let mut at = word(end + 16) as usize;
    let mut names = Vec::new();
    for _ in 0..count {
        if bytes.get(at..at + 4) != Some(&0x0201_4b50u32.to_le_bytes()[..]) {
            return Err("zip: a broken central directory".to_owned());
        }
        let name_len = half(at + 28) as usize;
        let skip = half(at + 30) as usize + half(at + 32) as usize;
        let name = bytes
            .get(at + 46..at + 46 + name_len)
            .ok_or("zip: a name past the end")?;
        names.push(String::from_utf8_lossy(name).into_owned());
        at += 46 + name_len + skip;
    }
    Ok(names)
}

/// The files of the archive at `path`, a zip or a `tar.gz` as these functions
/// write them: each name and its bytes, in order. An archive that unpacks to
/// more than `limit` bytes is an error, as is one that doesn't read.
pub fn read(path: &Path, limit: u64) -> Result<Vec<(String, Vec<u8>)>, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let read = if path.to_string_lossy().ends_with(".tar.gz") {
        read_tar_gz(&bytes, limit)
    } else {
        read_zip(&bytes, limit)
    };
    read.map_err(|e| format!("{}: {e}", path.display()))
}

/// The files of the zip `bytes`, by its central directory.
fn read_zip(bytes: &[u8], limit: u64) -> Result<Vec<(String, Vec<u8>)>, String> {
    let at = |at: usize, n: usize| bytes.get(at..at + n).ok_or("zip: cut short".to_owned());
    let half = |offset: usize| at(offset, 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let word = |offset: usize| {
        at(offset, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let end = bytes
        .windows(4)
        .rposition(|w| w == 0x0605_4b50u32.to_le_bytes())
        .ok_or("zip: no end of central directory")?;
    let mut next = word(end + 16)?;
    let mut total = 0u64;
    let mut files = Vec::new();
    for _ in 0..half(end + 10)? {
        if at(next, 4)? != 0x0201_4b50u32.to_le_bytes() {
            return Err("zip: a broken central directory".to_owned());
        }
        let method = half(next + 10)?;
        let packed = word(next + 20)?;
        let size = word(next + 24)? as u64;
        let name_len = half(next + 28)?;
        let skip = half(next + 30)? + half(next + 32)?;
        let local = word(next + 42)?;
        let name = String::from_utf8_lossy(at(next + 46, name_len)?).into_owned();
        total = total.saturating_add(size);
        if total > limit {
            return Err(format!("zip: over {limit} bytes unpacked"));
        }
        let start = local + 30 + half(local + 26)? + half(local + 28)?;
        let data = at(start, packed)?;
        let inner = match method {
            0 => data.to_vec(),
            8 => {
                let mut inner = Vec::new();
                flate2::read::DeflateDecoder::new(data)
                    .take(size)
                    .read_to_end(&mut inner)
                    .map_err(|e| format!("zip: {name}: {e}"))?;
                inner
            }
            other => return Err(format!("zip: {name}: compression method {other}")),
        };
        files.push((name, inner));
        next += 46 + name_len + skip;
    }
    Ok(files)
}

/// The files of the `tar.gz` `bytes`, by their ustar headers.
fn read_tar_gz(bytes: &[u8], limit: u64) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut tar = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .take(limit.saturating_add(1))
        .read_to_end(&mut tar)
        .map_err(|e| format!("tar: {e}"))?;
    if tar.len() as u64 > limit {
        return Err(format!("tar: over {limit} bytes unpacked"));
    }
    let text = |field: &[u8]| {
        let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
        String::from_utf8_lossy(&field[..end]).into_owned()
    };
    let mut files = Vec::new();
    let mut at = 0;
    while let Some(header) = tar.get(at..at + 512).filter(|header| header[0] != 0) {
        let name = text(&header[..100]);
        let prefix = text(&header[345..500]);
        let size = usize::from_str_radix(text(&header[124..136]).trim(), 8)
            .map_err(|_| format!("tar: {name}: a broken size"))?;
        let data = at + 512;
        let inner = data
            .checked_add(size)
            .and_then(|end| tar.get(data..end))
            .ok_or_else(|| format!("tar: {name}: cut short"))?;
        let name = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        files.push((name, inner.to_vec()));
        at = data + size.div_ceil(512) * 512;
    }
    Ok(files)
}

/// Writes `entries` into a gzip-compressed ustar archive at `out`.
pub fn tar_gz(out: &Path, entries: &[Entry]) -> Result<(), String> {
    let file = fs::File::create(out).map_err(|e| format!("write {}: {e}", out.display()))?;
    let mut gz = GzBuilder::new().mtime(0).write(file, Compression::best());
    let failed = |e: std::io::Error| format!("write {}: {e}", out.display());
    for entry in entries {
        let data =
            fs::read(&entry.path).map_err(|e| format!("read {}: {e}", entry.path.display()))?;
        let mode = if entry.executable { 0o755 } else { 0o644 };
        gz.write_all(&tar_header(&entry.name, mode, data.len() as u64)?)
            .map_err(failed)?;
        gz.write_all(&data).map_err(failed)?;
        let pad = (512 - data.len() % 512) % 512;
        gz.write_all(&vec![0; pad]).map_err(failed)?;
    }
    gz.write_all(&[0; 1024]).map_err(failed)?;
    gz.finish().map_err(failed)?;
    Ok(())
}

/// A ustar header for a regular file `name` of `size` bytes and `mode`,
/// owned by root, at time 0.
fn tar_header(name: &str, mode: u32, size: u64) -> Result<[u8; 512], String> {
    let mut header = [0u8; 512];
    let (prefix, name) = if name.len() <= 100 {
        ("", name)
    } else {
        name.char_indices()
            .filter(|&(at, c)| c == '/' && at <= 155 && name.len() - at - 1 <= 100)
            .map(|(at, _)| (&name[..at], &name[at + 1..]))
            .next()
            .ok_or_else(|| format!("tar: the name {name} is too long"))?
    };
    let put = |header: &mut [u8; 512], at: usize, text: &[u8]| {
        header[at..at + text.len()].copy_from_slice(text);
    };
    put(&mut header, 0, name.as_bytes());
    put(&mut header, 100, format!("{mode:07o}\0").as_bytes());
    put(&mut header, 108, b"0000000\0");
    put(&mut header, 116, b"0000000\0");
    put(&mut header, 124, format!("{size:011o}\0").as_bytes());
    put(&mut header, 136, b"00000000000\0");
    put(&mut header, 148, b"        ");
    header[156] = b'0';
    put(&mut header, 257, b"ustar\0");
    put(&mut header, 263, b"00");
    put(&mut header, 265, b"root");
    put(&mut header, 297, b"root");
    put(&mut header, 345, prefix.as_bytes());
    let sum: u32 = header.iter().map(|&b| u32::from(b)).sum();
    put(&mut header, 148, format!("{sum:06o}\0 ").as_bytes());
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shuyi-archive-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("pkg/sub")).unwrap();
        fs::write(dir.join("pkg/game"), b"\x7fELF program").unwrap();
        fs::write(dir.join("pkg/NOTICES.txt"), "notices\n".repeat(50)).unwrap();
        fs::write(dir.join("pkg/sub/a.ogg"), b"ogg").unwrap();
        dir
    }

    fn listed(dir: &Path, prefix: &str) -> Vec<Entry> {
        entries(&dir.join("pkg"), prefix, &|path| path.ends_with("game")).unwrap()
    }

    #[test]
    fn entries_are_sorted_and_named_from_the_prefix() {
        let dir = fixture("entries");
        let found = listed(&dir, "g-1.0-linux/");
        let names: Vec<&str> = found.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "g-1.0-linux/NOTICES.txt",
                "g-1.0-linux/game",
                "g-1.0-linux/sub/a.ogg"
            ]
        );
        assert!(found[1].executable && !found[0].executable);
        assert!(entries(&dir.join("missing"), "", &|_| false).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_zip_lists_its_files_and_inflates_to_them_the_same_each_time() {
        let dir = fixture("zip");
        let found = listed(&dir, "");
        zip(&dir.join("a.zip"), &found).unwrap();
        zip(&dir.join("b.zip"), &found).unwrap();
        let bytes = fs::read(dir.join("a.zip")).unwrap();
        assert_eq!(bytes, fs::read(dir.join("b.zip")).unwrap());
        assert_eq!(
            zip_names(&bytes).unwrap(),
            ["NOTICES.txt", "game", "sub/a.ogg"]
        );
        // The first entry's data, inflated, is the file.
        let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        let packed = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]) as usize;
        let start = 30 + name_len;
        let mut out = String::new();
        flate2::read::DeflateDecoder::new(&bytes[start..start + packed])
            .read_to_string(&mut out)
            .unwrap();
        assert_eq!(out, "notices\n".repeat(50));
        assert!(zip_names(b"not a zip").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_tar_gz_holds_each_file_with_its_mode() {
        let dir = fixture("tar");
        tar_gz(&dir.join("a.tar.gz"), &listed(&dir, "g/")).unwrap();
        let mut tar = Vec::new();
        flate2::read::GzDecoder::new(fs::File::open(dir.join("a.tar.gz")).unwrap())
            .read_to_end(&mut tar)
            .unwrap();
        // Three headers and their data, padded, then two zero blocks.
        assert_eq!(tar.len(), 512 * (2 + 2 + 2 + 2));
        assert_eq!(&tar[..13], b"g/NOTICES.txt");
        assert_eq!(&tar[100..107], b"0000644");
        assert_eq!(&tar[1024..1030], b"g/game");
        assert_eq!(&tar[1024 + 100..1024 + 107], b"0000755");
        assert_eq!(&tar[257..262], b"ustar");
        // The checksum is the header's bytes summed, its own field as
        // spaces.
        let mut header = tar[..512].to_vec();
        header[148..156].copy_from_slice(b"        ");
        let sum: u32 = header.iter().map(|&b| u32::from(b)).sum();
        assert_eq!(&tar[148..155], format!("{sum:06o}\0").as_bytes());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_long_tar_name_goes_in_the_prefix() {
        let long = format!("{}/{}", "d".repeat(120), "f".repeat(50));
        let header = tar_header(&long, 0o644, 1).unwrap();
        assert_eq!(&header[..50], "f".repeat(50).as_bytes());
        assert_eq!(&header[345..465], "d".repeat(120).as_bytes());
        assert!(tar_header(&"x".repeat(300), 0o644, 1).is_err());
    }
}
