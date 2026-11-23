//! `preview`'s arguments, and the worker's taken from them.

use super::*;
use crate::toy::ToyTools;

/// `line`'s words read as `preview`'s arguments, with the toy's options.
fn parse(line: &str) -> Result<Options, String> {
    let args: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
    parse_args(&args, ToyTools.frame_options())
}

#[test]
fn by_default_it_listens_on_this_machine_alone() {
    let options = parse("").unwrap();
    assert_eq!(options, Options::default());
    assert_eq!(options.bind, IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_eq!(
        (options.port, options.frames, options.scale),
        (8_000, 300, 1)
    );
    assert!(!options.until_stdin_closes);
}

#[test]
fn each_limit_is_a_flag_in_its_unit() {
    let options = parse(
        "--port 0 --sessions 3 --per-address 4 --starts 5 --actions 6 --rate 7 \
         --frames 8 --scale 2 --idle 9 --lifetime 10 --deadline 11 --cpu 12 \
         --memory 64 --egress 13 --bind ::1 --until-stdin-closes",
    )
    .unwrap();
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
        memory: 64 << 20,
        egress: 13 << 20,
        ..Limits::default()
    };
    assert_eq!(options.limits, limits);
    assert_eq!((options.port, options.frames, options.scale), (0, 8, 2));
    assert_eq!(options.bind.to_string(), "::1");
    assert!(options.until_stdin_closes);
}

#[test]
fn a_wrong_value_is_refused_naming_its_range() {
    let refused = [
        (
            "--sessions 0",
            "--sessions: a whole number 1 to 1000, not 0",
        ),
        ("--scale 9", "--scale: a whole number 1 to 8, not 9"),
        (
            "--port 65536",
            "--port: a whole number 0 to 65535, not 65536",
        ),
        (
            "--memory 8",
            "--memory: a whole number 16 to 1048576, not 8",
        ),
        ("--idle -1", "--idle: a whole number 1 to 86400, not -1"),
        ("--rate", "--rate needs a value"),
        ("--bind", "--bind needs a value"),
        ("--bind localhost", "--bind: not an IP address: localhost"),
        ("--moon", "unknown option: --moon"),
    ];
    for (args, error) in refused {
        assert_eq!(parse(args), Err(error.to_owned()), "{args:?}");
    }
    let usage = usage("cargo shuyi");
    for (flag, ..) in NUMBERS {
        assert!(usage.contains(&format!("  {flag} ")), "{flag} in the usage");
    }
}

#[test]
fn the_games_options_start_every_worker_alike() {
    let options = parse("--lobby --burn-frame 3 --frames 40").unwrap();
    let starts: Vec<(&str, Option<&str>)> = options
        .start
        .iter()
        .map(|arg| (arg.flag.as_str(), arg.value.as_deref()))
        .collect();
    assert_eq!(starts, [("--lobby", None), ("--burn-frame", Some("3"))]);
    let args = options.worker_args();
    assert_eq!(
        args.join(" "),
        "preview-worker --frames 40 --scale 1 --lobby --burn-frame 3"
    );
    let worker = parse_args(&args[1..], ToyTools.frame_options()).unwrap();
    assert_eq!((worker.frames, worker.scale), (40, 1));
    assert_eq!(worker.start, options.start);
}
