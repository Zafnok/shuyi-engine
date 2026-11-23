//! Requests read whole or refused, and answers written.

use std::time::Duration;

use super::*;

/// `text` read as a request, or the status that refused it.
fn read_text(text: &[u8]) -> Result<Request, u16> {
    let deadline = Instant::now() + Duration::from_secs(5);
    read(&mut &text[..], deadline).map_err(|answer| answer.status)
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
fn what_isnt_a_plain_bounded_request_is_refused() {
    let head = |headers: &str| format!("POST /session HTTP/1.1\r\n{headers}\r\n\r\n");
    let refused = [
        (
            b"POST /session HTTP/1.1\r\nContent-Length: 9\r\n\r\nshort".to_vec(),
            400,
        ),
        (head("Content-Length: 2").into_bytes(), 400),
        (b"GET / HTTP/1.1\r\n\r\nmore".to_vec(), 400),
        (head("Transfer-Encoding: chunked").into_bytes(), 501),
        (
            head("Content-Length: 1\r\nContent-Length: 1").into_bytes(),
            400,
        ),
        (head("Content-Length: +1").into_bytes(), 400),
        (
            head(&format!("Content-Length: {}", MAX_BODY + 1)).into_bytes(),
            413,
        ),
        (
            head(&format!("X: {}", "a".repeat(MAX_HEAD))).into_bytes(),
            431,
        ),
        (head("Bad Name: 1").into_bytes(), 400),
        (head("no colon").into_bytes(), 400),
        (b"POST session HTTP/1.1\r\n\r\n".to_vec(), 400),
        (b"POST /session HTTP/2\r\n\r\n".to_vec(), 505),
        (b"POST /session\r\n\r\n".to_vec(), 400),
        (b"POST /session HTTP/1.1\r\n".to_vec(), 400),
        (vec![0xff, b' ', b'/', b'\r', b'\n', b'\r', b'\n'], 400),
    ];
    for (text, status) in refused {
        assert_eq!(
            read_text(&text).err(),
            Some(status),
            "{}",
            String::from_utf8_lossy(&text)
        );
    }
}

#[test]
fn a_request_past_its_deadline_is_refused() {
    let past = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    let refused = read(&mut &b"GET / HTTP/1.1\r\n\r\n"[..], past).unwrap_err();
    assert_eq!(refused.status, 408);
}

#[test]
fn an_answer_says_its_length_and_never_lets_a_page_cache_or_sniff_it() {
    let answer = Answer::error(429, "slow down").retry_after(0);
    let text = String::from_utf8(answer.to_bytes()).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    assert_eq!(body, r#"{"error":"slow down"}"#);
    let lines: Vec<&str> = head.lines().collect();
    assert_eq!(lines[0], "HTTP/1.1 429 Too Many Requests");
    for line in [
        "Content-Type: application/json",
        &format!("Content-Length: {}", body.len()),
        "Retry-After: 1",
        "Cache-Control: no-store",
        "X-Content-Type-Options: nosniff",
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
}
