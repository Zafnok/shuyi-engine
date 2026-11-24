//! Each limit of the server, at its bound and past it: the sessions of an
//! address and its starts a minute, a session's actions, their rate, its
//! game's time, its bytes and its worker's memory; the idle and old
//! sessions ended, at the time given and by the reaper's thread; and the
//! connections at once, over real ones.

use std::process::Command;

use super::*;

/// The session at `path`, locked.
fn live(server: &Server, path: &str) -> Arc<Mutex<Live>> {
    let id = path.trim_start_matches("/session/");
    Arc::clone(&lock(&server.table).live[id].live)
}

#[test]
fn an_address_has_so_many_sessions_at_once_and_another_has_its_own() {
    let server = server(Limits {
        sessions: 4,
        per_address: 1,
        ..Limits::default()
    });
    let mine = start(&server);
    let refused = asked(&server, "POST", "/session", "");
    assert_eq!(refused.status, 429);
    let error = "{\"error\":\"an address may have 1 sessions at once: end one first\"}";
    assert_eq!(String::from_utf8_lossy(&refused.body), error);
    assert_eq!(header(&refused, "Retry-After"), None);
    start_from(&server, THERE);
    // One starting counts as its address's.
    let other = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
    lock(&server.table).starting.push(other);
    assert_eq!(
        asked_from(&server, other, "POST", "/session", "").status,
        429
    );
    lock(&server.table).starting.clear();
    // Ending one makes room for its address.
    assert_eq!(ask(&server, "DELETE", &mine, "").0, 204);
    start(&server);
    assert_eq!(open(&server), 2);
    // With one open and one starting, an address has two.
    let server = self::server(Limits {
        per_address: 2,
        ..Limits::default()
    });
    start_from(&server, THERE);
    lock(&server.table).starting.push(THERE);
    assert_eq!(lock(&server.table).of(THERE), 2);
    let third = asked_from(&server, THERE, "POST", "/session", "");
    assert_eq!(third.status, 429);
}

#[test]
fn an_ipv6_address_is_its_64_and_an_ipv4_one_written_in_ipv6_is_itself() {
    let v6 = |text: &str| text.parse::<IpAddr>().unwrap();
    assert_eq!(visitor(v6("2001:db8:1:2:aaaa::1")), v6("2001:db8:1:2::"));
    assert_eq!(
        visitor(v6("2001:db8:1:2:ffff:ffff:ffff:ffff")),
        v6("2001:db8:1:2::")
    );
    assert_ne!(visitor(v6("2001:db8:1:3::1")), v6("2001:db8:1:2::"));
    assert_eq!(visitor(v6("::ffff:10.0.0.2")), THERE);
    assert_eq!(visitor(THERE), THERE);
    // Two of one /64 share its limit.
    let server = server(Limits {
        per_address: 1,
        ..Limits::default()
    });
    start_from(&server, v6("2001:db8::1"));
    let second = asked_from(&server, v6("2001:db8::2"), "POST", "/session", "");
    assert_eq!(second.status, 429);
    start_from(&server, v6("2001:db8:0:1::1"));
}

#[test]
fn an_address_starts_so_many_sessions_a_minute_and_a_refused_start_isnt_one() {
    let server = server(Limits {
        starts: 2,
        per_address: 1,
        ..Limits::default()
    });
    let first = start(&server);
    // Refused for another limit, a start uses none of the minute's.
    assert_eq!(ask(&server, "POST", "/session", "").0, 429);
    assert_eq!(ask(&server, "DELETE", &first, "").0, 204);
    let second = start(&server);
    assert_eq!(ask(&server, "DELETE", &second, "").0, 204);
    let refused = asked(&server, "POST", "/session", "");
    assert_eq!(refused.status, 429);
    let error = "{\"error\":\"an address may start 2 sessions a minute\"}";
    assert_eq!(String::from_utf8_lossy(&refused.body), error);
    // A token comes back each half minute.
    let wait: u64 = header(&refused, "Retry-After").unwrap().parse().unwrap();
    assert!((1..=30).contains(&wait), "{wait}");
    start_from(&server, THERE);
    // Within the minute, the address is kept.
    server.reap(Instant::now());
    assert_eq!(asked(&server, "POST", "/session", "").status, 429);
    // A minute on, the address is forgotten, and starts again.
    server.reap(Instant::now() + MINUTE);
    assert!(lock(&server.table).starts.is_empty());
    start(&server);
}

#[test]
fn a_session_presses_so_many_actions_and_is_ended_past_them() {
    let server = server(Limits {
        actions: 2,
        ..Limits::default()
    });
    let path = start(&server);
    for _ in 0..2 {
        assert_eq!(press(&server, &path, "CursorDown").0, 200);
    }
    let (status, body) = press(&server, &path, "CursorDown");
    assert_eq!(
        (status, &body["error"]),
        (429, &Value::from("the session pressed its 2 actions"))
    );
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
}

#[test]
fn a_sessions_rate_allows_a_seconds_burst_and_refuses_past_it() {
    let server = server(Limits {
        rate: 3,
        ..Limits::default()
    });
    let path = start(&server);
    let session = live(&server, &path);
    // Its bucket, full, refills from an hour on: however long the toy's
    // frames take, a token doesn't come back.
    let hour = Instant::now() + Duration::from_secs(3_600);
    lock(&session).rate = Bucket::new(3, Duration::from_secs(1), hour);
    for _ in 0..3 {
        assert_eq!(press(&server, &path, "CursorDown").0, 200);
    }
    let body = serde_json::json!({ "action": "CursorDown" }).to_string();
    let refused = asked(&server, "POST", &path, &body);
    assert_eq!(refused.status, 429);
    let error = "{\"error\":\"a session may press 3 actions a second\"}";
    assert_eq!(String::from_utf8_lossy(&refused.body), error);
    assert_eq!(header(&refused, "Retry-After"), Some("1"));
    // Refused for its rate, the session goes on.
    assert_eq!(lock(&session).actions, 3);
    lock(&session).rate = Bucket::new(3, Duration::from_secs(1), Instant::now());
    assert_eq!(press(&server, &path, "CursorDown").0, 200);
}

#[test]
fn a_bucket_refills_at_its_rate_up_to_its_size_and_says_how_long_to_wait() {
    let now = Instant::now();
    let ms = Duration::from_millis;
    let mut bucket = Bucket::new(2, Duration::from_secs(1), now);
    assert_eq!((bucket.take(now), bucket.take(now)), (Ok(()), Ok(())));
    assert_eq!(bucket.take(now), Err(ms(500)));
    assert_eq!(bucket.take(now + ms(250)), Err(ms(250)));
    assert_eq!(bucket.take(now + ms(499)), Err(ms(1)));
    assert_eq!(bucket.take(now + ms(500)), Ok(()));
    // A time before its last is no refill.
    assert_eq!(bucket.take(now), Err(ms(500)));
    assert!(!bucket.full(now + ms(1_499)));
    assert!(bucket.full(now + ms(1_500)));
    // Long idle, it holds a window's worth and no more.
    let later = now + Duration::from_secs(60);
    assert_eq!((bucket.take(later), bucket.take(later)), (Ok(()), Ok(())));
    assert_eq!(bucket.take(later), Err(ms(500)));
    // Waits are whole seconds, never early.
    let waits = [ms(0), ms(1), ms(1_000), ms(1_001), ms(29_998)].map(seconds);
    assert_eq!(waits, [0, 1, 1, 2, 30]);
}

#[test]
fn a_session_uses_so_much_of_the_games_time_and_is_ended_past_it() {
    let server = server(Limits {
        cpu: Duration::from_secs(60),
        ..Limits::default()
    });
    let path = start(&server);
    let session = live(&server, &path);
    assert!(
        lock(&session).busy > Duration::ZERO,
        "its start is the game's time"
    );
    lock(&session).busy = Duration::from_secs(60) - Duration::from_nanos(1);
    assert_eq!(press(&server, &path, "CursorDown").0, 200);
    assert!(lock(&session).busy >= Duration::from_secs(60));
    lock(&session).busy = Duration::from_secs(60);
    let (status, body) = press(&server, &path, "CursorDown");
    let error = "the session used its 60 seconds of the game's time";
    assert_eq!((status, &body["error"]), (429, &Value::from(error)));
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
}

#[test]
fn a_session_sends_so_many_bytes_and_is_ended_past_them() {
    let server = server(Limits {
        egress: 1 << 20,
        ..Limits::default()
    });
    let first = asked(&server, "POST", "/session", "");
    let body: Value = serde_json::from_slice(&first.body).unwrap();
    let path = format!("/session/{}", body["session"].as_str().unwrap());
    let session = live(&server, &path);
    let sent = u64::try_from(first.to_bytes().len()).unwrap();
    assert_eq!(
        lock(&session).sent,
        sent,
        "its first frame's bytes, head and all"
    );
    lock(&session).sent = (1 << 20) - 1;
    assert_eq!(press(&server, &path, "CursorDown").0, 200);
    lock(&session).sent = 1 << 20;
    let (status, body) = press(&server, &path, "CursorDown");
    let error = "the session's answers sent their 1 MB";
    assert_eq!((status, &body["error"]), (429, &Value::from(error)));
    assert_eq!(press(&server, &path, "CursorDown").0, 404);
    // What it spent, for the server's log.
    let mut done = lock(&session);
    (done.actions, done.busy, done.sent) = (3, Duration::from_millis(1_500), 2 << 20);
    done.peak = None;
    let unread = "3 actions, 1500 ms of the game's time, 2 MB sent, its memory unread";
    assert_eq!(done.summary(), unread);
    done.peak = Some(5 << 20);
    assert!(done.summary().ends_with(", 2 MB sent, 5 MB held at most"));
    drop(done);
    let sizes = [0, 1_536, 1 << 20, (1 << 20) + 1_024, 3 << 20].map(size);
    assert_eq!(sizes, ["0 KB", "1 KB", "1 MB", "1025 KB", "3 MB"]);
}

/// Workers on threads, each the toy started in its lobby, whose memory is
/// that of a process that idles, `sort` reading an input never closed.
fn toy_in_a_process() -> Launch {
    Box::new(|| {
        let mut worker = (toy_ending(channel().0))()?;
        let idle = Command::new("sort")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()?;
        worker.child = Some(idle);
        Ok(worker)
    })
}

#[test]
fn a_worker_past_its_memory_ends_its_session_where_the_system_says_it() {
    let says = cfg!(target_os = "linux");
    // Past it at its start, it never is a session.
    let mut server = Server::new(
        Limits {
            memory: 0,
            ..Limits::default()
        },
        toy_in_a_process(),
    );
    let (status, body) = ask(&server, "POST", "/session", "");
    if says {
        let error = "the game held more than its 0 KB";
        assert_eq!((status, &body["error"]), (503, &Value::from(error)));
        assert_eq!(open(&server), 0);
    } else {
        // Elsewhere it isn't kept.
        assert_eq!(status, 200, "{body}");
    }
    server.limits.memory = 1 << 30;
    let path = start(&server);
    let session = live(&server, &path);
    let peak = lock(&session).peak;
    assert_eq!(peak.is_some(), says, "{peak:?}");
    // Past it after an action, its session ends.
    server.limits.memory = 1 << 10;
    let (status, body) = press(&server, &path, "CursorDown");
    if says {
        let error = "the game held more than its 1 KB";
        assert_eq!((status, &body["error"]), (503, &Value::from(error)));
        assert_eq!(press(&server, &path, "CursorDown").0, 404);
    } else {
        assert_eq!(status, 200, "{body}");
    }
}

#[test]
fn the_memory_a_process_holds_is_read_from_linuxs_proc_and_none_elsewhere() {
    // A worker past its bound holds more than it.
    assert!(!past(512 << 20, 512 << 20));
    assert!(past((512 << 20) + 1, 512 << 20));
    let mine = resident(std::process::id());
    if cfg!(target_os = "linux") {
        assert!(mine.unwrap() > 1 << 20, "{mine:?}");
    } else {
        assert_eq!(mine, None);
    }
    let status = "Name:\tsort\nVmPeak:\t  9000 kB\nVmRSS:\t    2048 kB\nThreads:\t1\n";
    assert_eq!(rss_of(status), Some(2 << 20));
    for broken in ["Name:\tsort\n", "VmRSS:\t2048\n", "VmRSS:\tmany kB\n", ""] {
        assert_eq!(rss_of(broken), None, "{broken:?}");
    }
}

#[test]
fn a_session_idle_or_old_is_ended_at_its_limit_and_not_before() {
    let nanosecond = Duration::from_nanos(1);
    let idle = Duration::from_secs(120);
    let server = server(Limits::default());
    let path = start(&server);
    let last = lock(&live(&server, &path)).last;
    server.reap((last + idle).checked_sub(nanosecond).unwrap());
    assert_eq!(open(&server), 1);
    server.reap(last + idle);
    assert_eq!(open(&server), 0);
    // Old, though it is pressed.
    let lifetime = Duration::from_secs(600);
    let server = self::server(Limits {
        idle: Duration::from_hours(24),
        lifetime,
        ..Limits::default()
    });
    let path = start(&server);
    let born = lock(&live(&server, &path)).born;
    assert_eq!(press(&server, &path, "CursorDown").0, 200);
    server.reap((born + lifetime).checked_sub(nanosecond).unwrap());
    assert_eq!(open(&server), 1);
    server.reap(born + lifetime);
    assert_eq!(open(&server), 0);
    // A session busy with an action is neither.
    let server = self::server(Limits::default());
    let path = start(&server);
    let session = live(&server, &path);
    let held = lock(&session);
    server.reap(Instant::now() + Duration::from_hours(24));
    assert_eq!(open(&server), 1);
    drop(held);
    server.reap(Instant::now() + Duration::from_hours(24));
    assert_eq!(open(&server), 0);
}

#[test]
fn the_reaper_looks_often_enough_to_be_at_most_a_second_late() {
    let every = |idle: u64, lifetime: u64| {
        tick(&Limits {
            idle: Duration::from_millis(idle),
            lifetime: Duration::from_millis(lifetime),
            ..Limits::default()
        })
    };
    assert_eq!(tick(&Limits::default()), Duration::from_secs(1));
    assert_eq!(every(400, 1_000_000), Duration::from_millis(100));
    assert_eq!(every(1_000_000, 200), Duration::from_millis(50));
    assert_eq!(every(4_000, 4_000), Duration::from_secs(1));
    assert_eq!(every(8_000, 8_000), Duration::from_secs(1));
    assert_eq!(every(10, 10), Duration::from_millis(20));
}

/// Waits up to 10 seconds for `done`, and says whether it came.
fn until(done: impl Fn() -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs(10);
    while !done() {
        if Instant::now() > end {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
    true
}

#[test]
fn the_reapers_thread_ends_an_idle_session_by_itself() {
    let server = Arc::new(server(Limits {
        idle: Duration::from_millis(100),
        ..Limits::default()
    }));
    serving(&server);
    start(&server);
    assert_eq!(open(&server), 1);
    assert!(until(|| open(&server) == 0), "it is still open");
}

#[test]
fn past_64_connections_at_once_a_connection_is_answered_503() {
    let server = Arc::new(server(Limits::default()));
    let address = serving(&server);
    // 64 connections that send nothing fill the cap.
    let held: Vec<TcpStream> = (0..64)
        .map(|_| TcpStream::connect(address).unwrap())
        .collect();
    let count = || server.connections.load(Ordering::SeqCst);
    assert!(until(|| count() == 64), "{} held", count());
    let mut refused = String::new();
    let mut late = TcpStream::connect(address).unwrap();
    late.read_to_string(&mut refused).unwrap();
    assert!(refused.starts_with("HTTP/1.1 503 "), "{refused}");
    assert!(refused.contains("\r\nRetry-After: 5\r\n"), "{refused}");
    assert!(refused.ends_with("{\"error\":\"too many connections: try again later\"}"));
    assert_eq!(count(), 64);
    // Their connections closed, the count falls, and the next is served.
    drop(held);
    assert!(until(|| count() == 0), "{} held", count());
    let (status, _) = over_http(address, "GET", "/", "");
    assert_eq!(status, 404);
}
