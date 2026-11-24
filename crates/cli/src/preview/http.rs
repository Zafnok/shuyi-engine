//! The little HTTP/1.1 the preview speaks: one request a connection, read
//! whole within bounds ([`read`]), and one answer ([`Answer`]), after which
//! the connection closes. It reads a request line, headers, and a body of
//! `Content-Length` bytes, and refuses the rest: a head over [`MAX_HEAD`]
//! bytes, a body over [`MAX_BODY`], a chunked body, two lengths, bytes
//! after the body, a request that takes longer than its deadline. Every
//! answer says it isn't to be cached, sniffed or sent on as a referrer.

use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::time::Instant;

/// The longest request line and headers, in bytes: 8 KiB.
pub(crate) const MAX_HEAD: usize = 8_192;

/// The longest body, in bytes: an action's name is short.
pub(crate) const MAX_BODY: usize = 1_024;

/// The end of a line and an empty line, which end a request's head:
/// CR LF CR LF.
const BLANK_LINE: [u8; 4] = [13, 10, 13, 10];

/// A request read whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    /// Its method: `GET`, `POST`, …
    pub(crate) method: String,
    /// Its path, without a query.
    pub(crate) path: String,
    /// Its body.
    pub(crate) body: Vec<u8>,
}

/// An answer: its status, its headers beyond the ones every answer has,
/// and its body, JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Answer {
    /// The status.
    pub(crate) status: u16,
    /// More headers: a `Retry-After`, say.
    pub(crate) headers: Vec<(&'static str, String)>,
    /// The body; none for `204`.
    pub(crate) body: Vec<u8>,
}

impl Answer {
    /// An answer of `status` with the JSON `body`.
    pub(crate) fn json(status: u16, body: &impl serde::Serialize) -> Self {
        Self {
            status,
            headers: Vec::new(),
            // A struct of strings, numbers and lists: serializing can't fail.
            body: serde_json::to_vec(body).unwrap_or_default(),
        }
    }

    /// A refusal of `status`, saying `error`.
    pub(crate) fn error(status: u16, error: impl Into<String>) -> Self {
        Self::json(status, &serde_json::json!({ "error": error.into() }))
    }

    /// This answer, telling the client to wait `seconds` before it asks
    /// again.
    #[must_use]
    pub(crate) fn retry_after(mut self, seconds: u64) -> Self {
        self.headers
            .push(("Retry-After", seconds.max(1).to_string()));
        self
    }

    /// Its bytes, head and body: what goes over the connection.
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut head = format!("HTTP/1.1 {} {}\r\n", self.status, reason(self.status));
        if !self.body.is_empty() {
            head.push_str("Content-Type: application/json\r\n");
        }
        // Writing to a `String` can't fail.
        let _ = write!(head, "Content-Length: {}\r\n", self.body.len());
        for (name, value) in &self.headers {
            let _ = write!(head, "{name}: {value}\r\n");
        }
        head.push_str(
            "Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\
             Referrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        );
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }

    /// Writes it to `stream`.
    pub(crate) fn write_to(&self, stream: &mut impl Write) -> io::Result<()> {
        stream.write_all(&self.to_bytes())?;
        stream.flush()
    }
}

/// The statuses the preview answers, each with its reason phrase; any
/// other is a server's error, 500.
const REASONS: [(u16, &str); 12] = [
    (200, "OK"),
    (204, "No Content"),
    (400, "Bad Request"),
    (404, "Not Found"),
    (405, "Method Not Allowed"),
    (408, "Request Timeout"),
    (409, "Conflict"),
    (413, "Content Too Large"),
    (431, "Request Header Fields Too Large"),
    (501, "Not Implemented"),
    (503, "Service Unavailable"),
    (505, "HTTP Version Not Supported"),
];

/// The reason phrase of `status`.
fn reason(status: u16) -> &'static str {
    let known = REASONS.iter().find(|(known, _)| *known == status);
    known.map_or("Internal Server Error", |(_, reason)| reason)
}

/// Reads one request from `stream` before `deadline`, or the answer that
/// refuses it. A stream that waits gives up at its read timeout, which
/// the caller sets.
pub(crate) fn read(stream: &mut impl Read, deadline: Instant) -> Result<Request, Answer> {
    let (mut bytes, mut chunk) = (Vec::new(), [0_u8; 1_024]);
    let mut more = |bytes: &mut Vec<u8>, most: usize| {
        if Instant::now() >= deadline {
            return Err(Answer::error(408, "the request took too long"));
        }
        let most = most.min(chunk.len());
        match stream.read(&mut chunk[..most]) {
            Ok(0) => Err(Answer::error(400, "the request ended early")),
            Ok(n) => {
                bytes.extend_from_slice(&chunk[..n]);
                Ok(())
            }
            Err(_) => Err(Answer::error(408, "the request didn't arrive")),
        }
    };
    // Before the body, at most the head and the blank line after it.
    let cap = MAX_HEAD + BLANK_LINE.len();
    let end = loop {
        let blank = bytes
            .windows(BLANK_LINE.len())
            .position(|w| w == BLANK_LINE);
        if let Some(end) = blank {
            break end;
        }
        if bytes.len() >= cap {
            return Err(Answer::error(431, "the request's head is too long"));
        }
        let left = cap - bytes.len();
        more(&mut bytes, left)?;
    };
    let head = std::str::from_utf8(&bytes[..end])
        .map_err(|_| Answer::error(400, "the request's head isn't text"))?;
    let (method, path, length) = parse_head(head)?;
    let mut body = bytes[end + BLANK_LINE.len()..].to_vec();
    if body.len() > length {
        return Err(Answer::error(400, "more bytes than the body's length"));
    }
    while body.len() < length {
        let left = length - body.len();
        more(&mut body, left)?;
    }
    Ok(Request { method, path, body })
}

/// The method, the path without its query, and the body's length of a
/// request's `head`.
fn parse_head(head: &str) -> Result<(String, String, usize), Answer> {
    let mut lines = head.split("\r\n");
    let line = lines.next().unwrap_or_default();
    let mut parts = line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Answer::error(400, "not an HTTP request line"));
    };
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(Answer::error(505, "only HTTP/1.1 is spoken"));
    }
    if !target.starts_with('/') {
        return Err(Answer::error(400, "the target isn't a path"));
    }
    let mut length = None;
    for header in lines {
        let Some((name, value)) = header.split_once(':') else {
            return Err(Answer::error(400, "a header without a colon"));
        };
        if name.is_empty() || name.contains(char::is_whitespace) {
            return Err(Answer::error(400, "a header's name with a space"));
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(Answer::error(501, "a chunked body isn't read"));
        }
        if name.eq_ignore_ascii_case("content-length") {
            let value = value.trim();
            let n = value
                .parse::<usize>()
                .ok()
                .filter(|_| value.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| Answer::error(400, "a length that isn't a number"))?;
            if length.replace(n).is_some() {
                return Err(Answer::error(400, "two lengths"));
            }
        }
    }
    let length = length.unwrap_or(0);
    if length > MAX_BODY {
        return Err(Answer::error(413, "the body is too long"));
    }
    let path = target.split('?').next().unwrap_or_default();
    Ok((method.to_owned(), path.to_owned(), length))
}

#[cfg(test)]
mod tests;
