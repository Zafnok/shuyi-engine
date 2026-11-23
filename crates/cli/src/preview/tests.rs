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
    let line = "--port 0 --sessions 3 --frames 8 --scale 2 --deadline 11 --bind ::1 \
                --until-stdin-closes";
    let options = parse(line).unwrap();
    let limits = Limits {
        sessions: 3,
        deadline: Duration::from_secs(11),
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
    assert!(usage.contains("the most sessions at once (8)\n"), "{usage}");
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
