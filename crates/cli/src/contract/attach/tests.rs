//! `attach`: its arguments, and the operation against a test host that
//! runs the link's server on the toy game in a thread, as a backend's
//! listener does (`crate::toy::link`).

use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;

use shuyi_ui::perf::PerfLine;

use super::*;
use crate::toy::link::{LinkHost as Host, TOKEN, other_version};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-attach-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn input(address: &str, steps: Vec<AttachStep>) -> AttachInput {
    AttachInput {
        address: address.to_owned(),
        token: TOKEN.to_owned(),
        steps,
        ..AttachInput::default()
    }
}

#[test]
fn its_arguments_are_an_address_a_token_steps_and_what_to_read() {
    let parsed = parse_args(&args(&[
        "127.0.0.1:7357",
        "--token",
        TOKEN,
        "--keys",
        "Enter Up",
        "--act",
        "Confirm",
        "--pad",
        "South+East",
        "--wait",
        "0.5",
        "--frames",
        "3",
        "--screenshot",
        "shot.png",
        "--log-since",
        "12",
    ]))
    .unwrap();
    assert_eq!(parsed.address, "127.0.0.1:7357");
    assert_eq!(parsed.token, TOKEN);
    assert_eq!(
        parsed.steps,
        [
            AttachStep::Keys("Enter Up".into()),
            AttachStep::Act("Confirm".into()),
            AttachStep::Pad("South+East".into()),
            AttachStep::Wait(0.5),
        ]
    );
    assert_eq!(
        (
            parsed.frames,
            parsed.screenshot.as_deref(),
            parsed.log_since
        ),
        (3, Some("shot.png"), 12)
    );
    assert_eq!(
        parsed.link_steps(),
        [
            link::Step::Keys("Enter Up".into()),
            link::Step::Act("Confirm".into()),
            link::Step::Pad("South+East".into()),
            link::Step::Wait(30),
            link::Step::Wait(3),
        ]
    );
    // `--script` is `--keys`, as for `run`.
    let script = parse_args(&args(&["h:1", "--token", "t", "--script", "f"])).unwrap();
    assert_eq!(script.steps, [AttachStep::Keys("f".into())]);
    for (bad, said) in [
        (vec!["--token", TOKEN], "address is missing"),
        (vec!["127.0.0.1:1"], "--token is missing"),
        (vec!["a:1", "b:2", "--token", "t"], "one address"),
        (
            vec!["a:1", "--token", "t", "--nope"],
            "unknown option: --nope",
        ),
        (
            vec!["a:1", "--token", "t", "--keys", "Nope"],
            "--keys: unknown key",
        ),
        (
            vec!["a:1", "--token", "t", "--wait", "x"],
            "--wait: not a number",
        ),
        (
            vec!["a:1", "--token", "t", "--wait", "99"],
            "--wait: 99 is not",
        ),
        (
            vec!["a:1", "--token", "t", "--act", " "],
            "--act: no action named",
        ),
        (vec!["a:1", "--token", "t", "--act"], "--act needs a value"),
        (
            vec!["a:1", "--token", "t", "--frames", "-1"],
            "--frames: a whole number",
        ),
        (
            vec!["a:1", "--token", "t", "--log-since", "x"],
            "--log-since: a whole",
        ),
    ] {
        let error = parse_args(&args(&bad)).unwrap_err();
        assert!(error.contains(said), "{bad:?}: {error}");
    }
}

#[test]
fn a_wait_is_the_builds_frames_at_60_a_second() {
    assert_eq!(frames_of(0.0), 0);
    assert_eq!(frames_of(1.0 / 60.0), 1);
    assert_eq!(frames_of(0.5), 30);
    assert_eq!(frames_of(2.0), 120);
    assert_eq!(frames_of(-1.0), 0);
}

#[test]
fn it_plays_the_steps_in_the_running_build_and_reads_what_it_shows() {
    let host = Host::start();
    let dir = scratch("plays");
    let mut asked = input(&host.address(), vec![AttachStep::Keys("f".into())]);
    asked.frames = 2;
    asked.screenshot = Some("shot.png".to_owned());
    let answer = attach(&asked, &dir);
    assert!(answer.ok, "{answer:?}");
    let output = answer.result.unwrap();
    assert_eq!((output.link, output.game.as_str()), (link::VERSION, "Toy"));
    assert_eq!(output.screens, ["home", "menu"]);
    assert_eq!(
        (output.top_screen.as_str(), output.phase.as_str()),
        ("menu", "running")
    );
    assert!(output.focused && !output.suspended && !output.quit);
    assert!(output.frame.contains("--- "), "{}", output.frame);
    let shot = output
        .screenshot
        .clone()
        .unwrap_or_else(|| panic!("{output:?}"));
    assert_eq!(
        (shot.path.as_str(), shot.width, shot.height),
        ("shot.png", 2, 1)
    );
    assert_eq!(fs::read(dir.join("shot.png")).unwrap(), b"\x89PNG");
    assert!(
        answer.limits.iter().any(|l| l.contains("own frames")),
        "{:?}",
        answer.limits
    );
    // Reading the log from where it ended gives nothing new.
    host.let_go();
    let mut again = input(&host.address(), Vec::new());
    again.log_since = output.log_next;
    let answer = attach(&again, &dir);
    let output = answer.result.unwrap();
    assert_eq!(output.log, []);
    assert_eq!(output.screens, ["home", "menu"]);
    assert_eq!(answer.limits, Vec::<String>::new());
    let text = text(&output);
    assert!(text.starts_with("attach: Toy on frame "), "{text}");
    assert!(text.contains("top screen: menu (home > menu)"), "{text}");
}

#[test]
fn an_action_is_pressed_by_its_key_and_one_it_cant_press_is_named() {
    let host = Host::start();
    let dir = scratch("act");
    let answer = attach(
        &input(&host.address(), vec![AttachStep::Act("Confirm".into())]),
        &dir,
    );
    assert_eq!(answer.result.unwrap().screens, ["home", "menu"]);
    host.let_go();
    let answer = attach(
        &input(&host.address(), vec![AttachStep::Act("Menu".into())]),
        &dir,
    );
    assert_eq!(
        answer.code.as_deref(),
        Some(codes::WRONG_ARGUMENTS.0),
        "{answer:?}"
    );
    let error = answer.error.unwrap();
    assert!(
        error.contains("Menu") && error.contains("neither"),
        "{error}"
    );
}

#[test]
fn no_build_listening_is_a_setup_to_fix() {
    let free = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = free.local_addr().unwrap().to_string();
    drop(free);
    let answer = attach(&input(&address, Vec::new()), &scratch("none"));
    assert_eq!(answer.code.as_deref(), Some(codes::NO_BUILD.0));
    assert_eq!(answer.exit_code(), 3);
    assert!(answer.error.unwrap().contains(&address));
}

#[test]
fn a_wrong_token_and_another_link_version_have_their_codes() {
    let host = Host::start();
    let mut wrong = input(&host.address(), Vec::new());
    wrong.token = "fedcba9876543210fedcba9876543210".to_owned();
    let answer = attach(&wrong, &scratch("token"));
    assert_eq!(answer.code.as_deref(), Some(codes::LINK_TOKEN.0));
    assert_eq!(answer.exit_code(), 2);
    assert!(answer.error.unwrap().contains("token is wrong"));

    let (address, thread) = other_version();
    let answer = attach(&input(&address, Vec::new()), &scratch("version"));
    thread.join().unwrap();
    assert_eq!(answer.code.as_deref(), Some(codes::LINK_VERSION.0));
    assert_eq!(answer.exit_code(), 3);
    let error = answer.error.unwrap();
    assert!(
        error.contains(&format!("version {}", link::VERSION + 1)),
        "{error}"
    );
}

#[test]
fn a_build_with_a_client_attached_is_busy() {
    let host = Host::start();
    let mut first = TcpStream::connect((Ipv4Addr::LOCALHOST, host.port())).unwrap();
    let hello = Request::Hello {
        token: TOKEN.to_owned(),
        link: link::VERSION,
    };
    first
        .write_all(format!("{}\n", hello.to_line()).as_bytes())
        .unwrap();
    let mut reader = BufReader::new(first.try_clone().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let answer = attach(&input(&host.address(), Vec::new()), &scratch("busy"));
    assert_eq!(answer.code.as_deref(), Some(codes::LINK_LOST.0));
    assert!(answer.error.unwrap().contains("another client"));
}

#[test]
fn a_wrong_input_is_refused_before_connecting() {
    let mut asked = input("127.0.0.1:1", Vec::new());
    asked.token = String::new();
    let answer = attach(&asked, &scratch("refused"));
    assert_eq!(answer.code.as_deref(), Some(codes::WRONG_ARGUMENTS.0));
    let mut asked = input("127.0.0.1:1", vec![AttachStep::Wait(-1.0)]);
    asked.frames = MAX_IDLE_FRAMES + 1;
    assert_eq!(attach(&asked, &scratch("refused")).exit_code(), 2);
}

#[test]
fn its_perf_arguments_read_follow_and_snapshot() {
    let parsed = parse_args(&args(&[
        "h:1",
        "--token",
        "t",
        "--perf-since",
        "40",
        "--perf-follow",
        "perf.jsonl",
        "--for",
        "30",
        "--perf-snapshot",
    ]))
    .unwrap();
    assert_eq!(
        (
            parsed.perf_since,
            parsed.perf_follow.as_deref(),
            parsed.follow_for,
            parsed.perf_snapshot
        ),
        (Some(40), Some("perf.jsonl"), Some(30), true)
    );
    for (bad, said) in [
        (vec!["--perf-since", "x"], "--perf-since: a whole number"),
        (
            vec!["--perf-follow", "f", "--for", "0"],
            "--for: whole seconds, 1 to 3600",
        ),
        (
            vec!["--perf-follow", "f", "--for", "3601"],
            "--for: whole seconds",
        ),
        (vec!["--for", "3"], "--for: only with --perf-follow"),
        (vec!["--perf-snapshot"], "--perf-snapshot: only with"),
    ] {
        let mut all = vec!["h:1", "--token", "t"];
        all.extend(bad.iter().copied());
        let error = parse_args(&args(&all)).unwrap_err();
        assert!(error.contains(said), "{bad:?}: {error}");
    }
    let mut asked = input("h:1", Vec::new());
    asked.follow_for = Some(5);
    assert!(asked.check().unwrap_err().contains("only with perf_follow"));
    asked.perf_follow = Some(String::new());
    assert!(asked.check().unwrap_err().contains("file is missing"));
    asked.perf_follow = Some("f".to_owned());
    asked.follow_for = Some(0);
    assert!(asked.check().unwrap_err().contains("1 to 3600"));
}

/// The perf trace's lines in `text`, each read as a `PerfLine`.
fn trace(text: &str) -> Vec<PerfLine> {
    text.lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{line}: {e}")))
        .collect()
}

/// The frames of the `frame` lines of `lines`.
fn frames(lines: &[PerfLine]) -> Vec<u32> {
    lines
        .iter()
        .filter_map(|line| match line {
            PerfLine::Frame(record) => Some(record.frame),
            _ => None,
        })
        .collect()
}

/// Whether `frames` count up by one, with none missed.
fn one_each(frames: &[u32]) -> bool {
    frames.windows(2).all(|w| w[1] == w[0] + 1)
}

#[test]
fn perf_since_reads_the_builds_records_and_their_summary() {
    let host = Host::start();
    let dir = scratch("perf-since");
    let mut asked = input(&host.address(), Vec::new());
    asked.perf_since = Some(0);
    let answer = attach(&asked, &dir);
    let output = answer.result.clone().unwrap();
    assert!(!output.perf.is_empty(), "{output:?}");
    let records: Vec<u32> = output.perf.iter().map(|r| r.frame).collect();
    assert!(one_each(&records), "{records:?}");
    assert!(output.perf.iter().all(|r| r.heap.is_some()));
    let summary = output.perf_summary.clone().unwrap();
    assert_eq!(usize::try_from(summary.frames).unwrap(), output.perf.len());
    assert!(
        answer.limits.contains(&perf::TIMES.to_owned()),
        "{answer:?}"
    );
    assert_eq!(
        (output.perf_snapshot.clone(), output.follow.clone()),
        (None, None)
    );
    let next = output.perf_next.unwrap();
    assert!(text(&output).contains(&format!("perf: next record {next}")));

    // From `perf_next`, the frames run since, with a snapshot after them.
    host.let_go();
    asked.perf_since = Some(next);
    asked.perf_snapshot = true;
    let output = attach(&asked, &dir).result.unwrap();
    let first = output.perf.first().map(|r| r.frame);
    assert_eq!(first, records.last().map(|f| f + 1), "{output:?}");
    let snapshot = output.perf_snapshot.unwrap();
    assert_eq!(Some(snapshot.frame), output.perf.last().map(|r| r.frame));
}

#[test]
fn perf_follow_writes_a_line_for_each_frame_the_build_ran_and_sums_them_up() {
    let host = Host::start();
    let dir = scratch("perf-follow");
    let mut asked = input(&host.address(), Vec::new());
    asked.perf_follow = Some("perf.jsonl".to_owned());
    asked.follow_for = Some(1);
    asked.perf_snapshot = true;
    let answer = attach(&asked, &dir);
    let output = answer
        .result
        .clone()
        .unwrap_or_else(|| panic!("{answer:?}"));
    let lines = trace(&fs::read_to_string(dir.join("perf.jsonl")).unwrap());
    let follow = output.follow.clone().unwrap();
    assert_eq!(usize::try_from(follow.lines).unwrap(), lines.len());
    assert!(!follow.closed && follow.dropped == 0, "{follow:?}");
    assert!(follow.file.ends_with("perf.jsonl") && Path::new(&follow.file).is_absolute());
    // A frame line each, one per frame, then the two snapshots and the
    // diff.
    let frame_lines = frames(&lines);
    assert!(frame_lines.len() > 10, "{frame_lines:?}");
    assert!(one_each(&frame_lines), "{frame_lines:?}");
    let [
        ..,
        PerfLine::Snapshot(first),
        PerfLine::Snapshot(last),
        PerfLine::Diff(diff),
    ] = lines.as_slice()
    else {
        panic!("{:?}", &lines[lines.len() - 3..]);
    };
    assert_eq!(*diff, first.diff(last));
    assert_eq!(output.perf_snapshot.as_ref(), Some(last));
    let summary = output.perf_summary.clone().unwrap();
    assert_eq!(usize::try_from(summary.frames).unwrap(), frame_lines.len());
    assert_eq!(summary.file.as_deref(), Some(follow.file.as_str()));
    assert!(summary.heap_growth.is_some());
    assert_eq!(output.perf, []);
    assert_eq!(
        output.perf_next,
        frame_lines.last().map(|frame| u64::from(*frame))
    );
    let said = text(&output);
    assert!(
        said.contains("followed: ") && said.contains("trace: "),
        "{said}"
    );
}

#[test]
fn a_build_faster_than_the_reads_has_its_dropped_frames_counted() {
    // More than 600 frames between two reads.
    let host = Host::running(1_000);
    let dir = scratch("perf-dropped");
    let mut asked = input(&host.address(), Vec::new());
    asked.perf_follow = Some("perf.jsonl".to_owned());
    asked.follow_for = Some(1);
    let answer = attach(&asked, &dir);
    let output = answer.result.clone().unwrap();
    let follow = output.follow.unwrap();
    assert!(follow.dropped > 0, "{follow:?}");
    let said = format!("{} frame(s) were dropped between reads", follow.dropped);
    assert!(
        answer.limits.iter().any(|l| l.starts_with(&said)),
        "{:?}",
        answer.limits
    );
    assert_eq!(output.perf_summary.unwrap().dropped, follow.dropped);
}

/// A clock that moves only when slept on.
#[derive(Default)]
struct FakeClock {
    now: Duration,
    slept: Vec<Duration>,
}

impl Clock for FakeClock {
    fn now(&mut self) -> Duration {
        self.now
    }

    fn sleep(&mut self, time: Duration) {
        self.now += time;
        self.slept.push(time);
    }
}

/// A read's answer: the records of `frames` (numbered one less), and
/// `dropped` and `snapshot`.
fn perf_answer(
    frames: std::ops::Range<u32>,
    dropped: u64,
    snapshot: Option<&HeapSnapshot>,
) -> PerfAnswer {
    let records: Vec<FrameRecord> = frames.clone().map(FrameRecord::new).collect();
    serde_json::from_value(serde_json::json!({
        "records": records, "next": frames.end - 1, "dropped": dropped,
        "measured": true, "frames": frames.end - 1, "snapshot": snapshot,
    }))
    .unwrap()
}

#[test]
fn a_follow_reads_every_100_ms_until_its_time_is_up_and_once_more_then() {
    let mut clock = FakeClock::default();
    let mut asked = Vec::new();
    // Each read finds six frames more.
    let mut read = |since: u64, snapshot: bool| {
        asked.push((since, snapshot));
        let from = u32::try_from(since).unwrap() + 1;
        Ok(Some(perf_answer(from..from + 6, 0, None)))
    };
    let mut out = Vec::new();
    let followed = follow(&mut read, 0, 1, false, &mut out, &mut clock).unwrap();
    // At 0, 100, …, 1,000 ms.
    assert_eq!(asked.len(), 11);
    assert!(asked.iter().all(|(_, snapshot)| !snapshot));
    assert_eq!(clock.slept, vec![FOLLOW_EVERY; 10]);
    assert_eq!(
        (followed.lines, followed.next, followed.closed),
        (66, 66, false)
    );
    let frames = frames(&trace(std::str::from_utf8(&out).unwrap()));
    assert_eq!(frames, (1..=66).collect::<Vec<_>>());
}

#[test]
fn a_follow_stops_when_the_build_closes_the_link() {
    let mut clock = FakeClock::default();
    let mut reads = 0;
    let mut read = |since: u64, _| {
        reads += 1;
        let from = u32::try_from(since).unwrap() + 1;
        Ok((reads < 3).then(|| perf_answer(from..from + 2, 0, None)))
    };
    let mut out = Vec::new();
    let followed = follow(&mut read, 0, 10, true, &mut out, &mut clock).unwrap();
    assert!(followed.closed);
    assert_eq!((reads, followed.lines), (3, 4));
    // No last snapshot: the build had gone.
    assert_eq!(followed.snapshots.1, None);
    let asked = AttachInput {
        perf_snapshot: true,
        ..AttachInput::default()
    };
    let seen = followed_seen(followed, &asked, "f.jsonl");
    assert!(seen.limits.iter().any(|l| l.contains("it quit")));
    assert!(!seen.limits.contains(&perf::HEAP_UNTAGGED.to_owned()));
}

#[test]
fn a_full_read_is_read_again_at_once_and_snapshots_end_the_trace() {
    let mut clock = FakeClock::default();
    let mut asked = Vec::new();
    let snapshot = |frame| {
        let mut snapshot = HeapSnapshot::new(1024 * u64::from(frame), 0, Vec::new());
        snapshot.frame = frame;
        snapshot
    };
    let mut read = |since: u64, snap: bool| {
        asked.push((since, snap));
        let from = u32::try_from(since).unwrap() + 1;
        // The first read is full: 600 records, and 50 dropped before them.
        let (to, dropped) = if since == 0 {
            (from + 600, 50)
        } else {
            (from + 1, 0)
        };
        let taken = snap.then(|| snapshot(to - 1));
        Ok(Some(perf_answer(from..to, dropped, taken.as_ref())))
    };
    let mut out = Vec::new();
    let followed = follow(&mut read, 0, 1, true, &mut out, &mut clock).unwrap();
    // Read again at once after the full read: no sleep before the second.
    assert_eq!(asked[1], (600, false));
    assert_eq!(clock.slept.len(), asked.len() - 2);
    assert_eq!((asked[0].1, asked.last().unwrap().1), (true, true));
    assert!(asked[1..asked.len() - 1].iter().all(|(_, snap)| !snap));
    assert_eq!(followed.dropped, 50);
    let lines = trace(std::str::from_utf8(&out).unwrap());
    let [
        ..,
        PerfLine::Snapshot(first),
        PerfLine::Snapshot(last),
        PerfLine::Diff(diff),
    ] = lines.as_slice()
    else {
        panic!("no snapshots at the end");
    };
    assert_eq!((first.frame, diff.from, diff.to), (600, 600, last.frame));
    assert_eq!(usize::try_from(followed.lines).unwrap(), lines.len());
    let seen = followed_seen(followed, &AttachInput::default(), "f.jsonl");
    assert!(
        seen.limits
            .iter()
            .any(|l| l.starts_with("50 frame(s) were dropped"))
    );
    let summary = seen.summary.unwrap();
    assert_eq!(
        (summary.dropped, summary.file.as_deref()),
        (50, Some("f.jsonl"))
    );
    assert!(summary.heap_growth.is_some());
}
