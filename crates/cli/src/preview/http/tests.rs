//! Requests read whole or refused, and answers written.

use std::time::Duration;

use super::*;

/// `text` read as a request, or the status that refused it.
fn read_text(text: &[u8]) -> Result<Request, u16> {
    let deadline = Instant::now() + Duration::from_secs(30);
    read(&mut &text[..], deadline).map_err(|answer| answer.status)
}

/// A request whose head, up to the blank line, is `length` bytes.
fn head_of(length: usize) -> Vec<u8> {
    let line = "POST /session HTTP/1.1\r\nX: ";
    let value = "a".repeat(length - line.len());
    format!("{line}{value}\r\n\r\n").into_bytes()
}

#[test]
fn a_request_is_read_with_its_body_and_without_its_query() {
    let text = b"POST /session/ab?x=1 HTTP/1.1\r\nHost: here\r\nContent-Length: 4\r\n\r\nbody";
    let request = read_text(text).unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/session/ab");
    assert_eq!(request.body, b"body");
    let get = read_text(b"GET / HTTP/1.0\r\n\r\n").unwrap();
    assert_eq!((get.method.as_str(), get.path.as_str()), ("GET", "/"));
    assert_eq!(get.body, b"");
}

#[test]
fn a_head_and_a_body_are_read_up_to_their_most() {
    assert_eq!(read_text(&head_of(MAX_HEAD)).unwrap().path, "/session");
    assert_eq!(read_text(&head_of(MAX_HEAD + 1)).err(), Some(431));
    // A body is read to its length, in as many reads as it takes, and no
    // further.
    let body = |length: usize| {
        let head = format!("POST /session HTTP/1.1\r\nContent-Length: {length}\r\n\r\n");
        [head.into_bytes(), vec![b'a'; length], b"next".to_vec()].concat()
    };
    assert_eq!(read_text(&body(MAX_BODY)).unwrap().body, [b'a'; MAX_BODY]);
    assert_eq!(read_text(&body(MAX_BODY + 1)).err(), Some(413));
}

#[test]
fn what_isnt_a_plain_bounded_request_is_refused() {
    let head = |headers: &str| format!("POST /session HTTP/1.1\r\n{headers}\r\n\r\n");
    let line = |line: &str| format!("{line}\r\n\r\n");
    let refused = [
        (head("Content-Length: 9") + "short", 400),
        (head("Content-Length: 2"), 400),
        (line("GET / HTTP/1.1") + "more", 400),
        (head("Transfer-Encoding: chunked"), 501),
        (head("Content-Length: 1\r\nContent-Length: 1"), 400),
        (head("Content-Length: +1"), 400),
        (head("Bad Name: 1"), 400),
        (head(": no name"), 400),
        (head("no colon"), 400),
        (line("POST session HTTP/1.1"), 400),
        (line("POST /session HTTP/2"), 505),
        (line("POST /session"), 400),
        (line("POST /session HTTP/1.1 more"), 400),
        ("POST /session HTTP/1.1\r\n".to_owned(), 400),
    ];
    for (text, status) in refused {
        assert_eq!(read_text(text.as_bytes()).err(), Some(status), "{text}");
    }
    let not_text = [0xff, b' ', b'/', b'\r', b'\n', b'\r', b'\n'];
    assert_eq!(read_text(&not_text).err(), Some(400));
}

#[test]
fn a_request_past_its_deadline_is_refused() {
    let past = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    let refused = read(&mut &b"GET / HTTP/1.1\r\n\r\n"[..], past).unwrap_err();
    assert_eq!(refused.status, 408);
}

#[test]
fn an_answer_says_its_length_and_never_lets_a_page_cache_or_sniff_it() {
    let answer = Answer::error(503, "busy").retry_after(0);
    let text = String::from_utf8(answer.to_bytes()).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    assert_eq!(body, r#"{"error":"busy"}"#);
    let lines: Vec<&str> = head.lines().collect();
    assert_eq!(lines[0], "HTTP/1.1 503 Service Unavailable");
    for line in [
        "Content-Type: application/json",
        &format!("Content-Length: {}", body.len()),
        "Retry-After: 1",
        "Cache-Control: no-store",
        "X-Content-Type-Options: nosniff",
        "Referrer-Policy: no-referrer",
        "Connection: close",
    ] {
        assert!(lines.contains(&line), "{line} in {head}");
    }
    let empty = Answer {
        status: 204,
        headers: Vec::new(),
        body: Vec::new(),
    };
    let text = String::from_utf8(empty.to_bytes()).unwrap();
    assert!(text.starts_with("HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n"));
    assert!(!text.contains("\r\nContent-Type:"), "{text}");
    assert_eq!(reason(429), "Too Many Requests");
    assert_eq!(reason(500), "Internal Server Error");
}
