//! `preview`'s arguments, the worker's taken from them, and what it says
//! before it serves.

use super::*;
use crate::contract::fake::Fake;
use crate::project::Config;
use crate::toy::ToyTools;

/// `line`'s words.
fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_owned).collect()
}

/// `line`'s words read as `preview`'s arguments, with the toy's options.
fn parse(line: &str) -> Result<Options, String> {
    parse_args(&words(line), ToyTools.frame_options())
}

#[test]
fn by_default_it_listens_on_this_machine_alone_and_each_limit_is_a_flag() {
    let plain = parse("").unwrap();
    assert_eq!(plain.bind, IpAddr::V4(Ipv4Addr::LOCALHOST));
    let numbers = (
        plain.port,
        plain.frames,
        plain.scale,
        plain.until_stdin_closes,
    );
    assert_eq!(numbers, (8_000, 300, 1, false));
    let defaults = Limits {
        sessions: 8,
        per_address: 2,
        starts: 10,
        actions: 1_000,
        rate: 8,
        idle: Duration::from_secs(120),
        lifetime: Duration::from_mins(30),
        deadline: Duration::from_secs(5),
        cpu: Duration::from_secs(120),
        memory: 512 << 20,
        egress: 64 << 20,
        connections: 64,
    };
    assert_eq!(plain.limits, defaults);
    let line = "--port 0 --sessions 3 --frames 8 --scale 2 --deadline 11 --bind ::1 \
                --until-stdin-closes --per-address 4 --starts 5 --actions 6 --rate 7 \
                --idle 9 --lifetime 10 --cpu 12 --memory 16 --egress 1";
    let options = parse(line).unwrap();
    let limits = Limits {
        sessions: 3,
        per_address: 4,
        starts: 5,
        actions: 6,
        rate: 7,
        idle: Duration::from_secs(9),
        lifetime: Duration::from_secs(10),
        deadline: Duration::from_secs(11),
        cpu: Duration::from_secs(12),
        memory: 16 << 20,
        egress: 1 << 20,
        connections: 64,
    };
    assert_eq!(options.limits, limits);
    assert_eq!((options.port, options.frames, options.scale), (0, 8, 2));
    assert_eq!(options.bind.to_string(), "::1");
    assert!(options.until_stdin_closes);
}

#[test]
fn a_wrong_value_is_refused_naming_its_range() {
    // Each the arguments and what is said of them.
    let refused = [
        "--sessions 0: --sessions: a whole number 1 to 1000, not 0",
        "--scale 9: --scale: a whole number 1 to 8, not 9",
        "--port 65536: --port: a whole number 0 to 65535, not 65536",
        "--deadline -1: --deadline: a whole number 1 to 600, not -1",
        "--memory 15: --memory: a whole number 16 to 65536, not 15",
        "--egress 0: --egress: a whole number 1 to 65536, not 0",
        "--rate 1001: --rate: a whole number 1 to 1000, not 1001",
        "--frames: --frames needs a value",
        "--bind: --bind needs a value",
        "--bind localhost: --bind: not an IP address: localhost",
        "--moon: unknown option: --moon",
    ];
    for line in refused {
        let (args, error) = line.split_once(": ").unwrap();
        assert_eq!(parse(args), Err(error.to_owned()), "{args:?}");
    }
    let usage = usage("cargo shuyi");
    let listed = |flag: &str| usage.contains(&format!("\n  {flag} "));
    assert!(NUMBERS.iter().all(|(flag, ..)| listed(flag)), "{usage}");
    for default in [
        "the most sessions at once (8)\n",
        "of one address at once (2)\n",
        "may start a minute (10)\n",
        "may press (1000)\n",
        "may press each second (8)\n",
        "since its last action ends (120)\n",
        "after it started (1800)\n",
        "its calls' summed (120)\n",
        "ends its session (512; Linux alone says it)\n",
        "answers may send (64)\n",
        "at most 64 connections at once",
    ] {
        assert!(usage.contains(default), "{default:?} in {usage}");
    }
}

#[test]
fn the_games_options_start_every_worker_alike() {
    let options = parse("--lobby --burn-frame 3 --frames 40").unwrap();
    let starts: Vec<(&str, Option<&str>)> = (options.start.iter())
        .map(|arg| (arg.flag.as_str(), arg.value.as_deref()))
        .collect();
    assert_eq!(starts, [("--lobby", None), ("--burn-frame", Some("3"))]);
    // After what the project puts before a command of its tool program.
    let args = options.worker_args(&words("shuyi"));
    let line = "shuyi preview-worker --frames 40 --scale 1 --lobby --burn-frame 3";
    assert_eq!(args.join(" "), line);
    let worker = worker::parse_args(&args[2..], ToyTools.frame_options()).unwrap();
    assert_eq!((worker.frames, worker.scale), (40, 1));
    assert_eq!(worker.start, options.start);
}

#[test]
fn before_it_serves_it_says_what_is_wrong() {
    let project = Project {
        root: std::env::temp_dir(),
        config: Config::default(),
    };
    assert_eq!(main(&project, &words("--help"), &ToyTools), 0);
    assert_eq!(main(&project, &words("--sessions 0"), &ToyTools), 2);
    // A game whose tools offer no preview says so before its visitors.
    let offered = codes::exit_code(Some(codes::NOT_OFFERED.0));
    assert_eq!(main(&project, &[], &Fake::default()), offered);
    // A port that is taken.
    let taken = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = words(&format!("--port {}", taken.local_addr().unwrap().port()));
    assert_eq!(main(&project, &port, &ToyTools), 2);
}
