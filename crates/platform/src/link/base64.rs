//! Base64 (RFC 4648 §4, the standard alphabet, with padding): how the link
//! carries a picture in a JSON line, and how MCP carries an image.

use crate::alloc_prelude::*;
use alloc::format;

/// The standard alphabet.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` in standard base64, with padding.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = u32::from_be_bytes([
            0,
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The value of one base64 character, if it is one.
fn value(c: u8) -> Option<u32> {
    let v = match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    };
    Some(u32::from(v))
}

/// The bytes `text` encodes in standard base64 with padding, or why it
/// isn't that.
pub fn decode(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err(format!(
            "base64: {} characters, not a multiple of 4",
            bytes.len()
        ));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let quads = bytes.len() / 4;
    for (i, quad) in bytes.chunks(4).enumerate() {
        let pad = quad.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && i + 1 != quads) {
            return Err("base64: padding before the end".to_owned());
        }
        let mut n = 0u32;
        for &c in &quad[..4 - pad] {
            let v = value(c)
                .ok_or_else(|| format!("base64: {:?} is no base64 character", char::from(c)))?;
            n = n * 64 + v;
        }
        n <<= 6 * pad;
        let [_, a, b, c] = n.to_be_bytes();
        out.extend_from_slice(&[a, b, c][..3 - pad]);
    }
    Ok(out)
}
