//! The task table: ids counted from 1, states, times and their JSON.

use super::*;

/// `secs` after the epoch.
fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

#[test]
fn a_stamp_is_iso_8601_in_utc_to_the_millisecond() {
    // 2026-11-27 is day 20,784; 14:03:07.250 is 50,587.25 seconds into it.
    let time = UNIX_EPOCH + Duration::from_millis(20_784 * 86_400_000 + 50_587_250);
    assert_eq!(stamp(time), "2026-11-27T14:03:07.250Z");
    assert_eq!(stamp(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
    assert_eq!(stamp(at(86_399)), "1970-01-01T23:59:59.000Z");
}

#[test]
fn ids_count_from_one_and_each_state_has_its_status() {
    let mut tasks = Tasks::<u32>::default();
    let a = tasks.make(Op::Start, at(10), State::Working(7));
    let b = tasks.make(Op::Bake, at(10), State::Working(8));
    let c = tasks.make(
        Op::Export,
        at(10),
        State::Completed(json!({ "isError": false })),
    );
    assert_eq!(
        (a.as_str(), b.as_str(), c.as_str()),
        ("shuyi-1", "shuyi-2", "shuyi-3")
    );
    assert_eq!(tasks.working(None), ["shuyi-1", "shuyi-2"]);
    assert_eq!(tasks.working(Some(Op::Bake)), ["shuyi-2"]);
    assert_eq!(tasks.work("shuyi-1"), Some(&mut 7));
    assert_eq!(tasks.work("shuyi-3"), None);
    assert_eq!(tasks.work("shuyi-9"), None);

    let error = json!({ "code": -32_603, "message": "the host stopped" });
    assert_eq!(
        tasks.end("shuyi-2", at(25), State::Failed(error.clone())),
        Some(8)
    );
    // An ended task isn't ended again.
    assert_eq!(tasks.end("shuyi-2", at(30), State::Cancelled), None);
    assert_eq!(tasks.end("shuyi-9", at(30), State::Cancelled), None);
    let failed = tasks.detailed("shuyi-2").unwrap();
    assert_eq!(
        (failed["status"].as_str(), failed["statusMessage"].as_str()),
        (Some("failed"), Some("the host stopped"))
    );
    assert_eq!(failed["error"], error);
    assert_eq!(failed["ttlMs"], 15_000 + 3_600_000);
    assert_eq!(failed["lastUpdatedAt"], "1970-01-01T00:00:25.000Z");
    assert_eq!(failed["pollIntervalMs"], 10_000);

    let completed = tasks.detailed("shuyi-3").unwrap();
    assert_eq!(completed["status"], "completed");
    assert_eq!(completed["result"], json!({ "isError": false }));
    assert_eq!(completed["ttlMs"], 3_600_000);
    // `tools/call`'s answer carries no result.
    assert!(tasks.json("shuyi-3").unwrap()["result"].is_null());

    let working = tasks.detailed("shuyi-1").unwrap();
    assert_eq!(
        (
            working["status"].as_str(),
            working["pollIntervalMs"].as_u64()
        ),
        (Some("working"), Some(2_000))
    );
    assert!(working["ttlMs"].is_null());
    assert_eq!(tasks.end("shuyi-1", at(40), State::Cancelled), Some(7));
    tasks.ended("shuyi-1", State::Cancelled);
    let cancelled = tasks.detailed("shuyi-1").unwrap();
    assert_eq!(cancelled["status"], "cancelled");
    assert!(cancelled["result"].is_null() && cancelled["error"].is_null());
    assert!(tasks.json("shuyi-9").is_none() && tasks.detailed("shuyi-9").is_none());
}

#[test]
fn a_task_is_forgotten_an_hour_after_it_ends_but_never_while_it_works() {
    let mut tasks = Tasks::<()>::default();
    tasks.make(Op::Start, at(0), State::Working(()));
    tasks.make(Op::Bake, at(0), State::Cancelled);
    tasks.make(Op::Export, at(100), State::Completed(json!({})));
    tasks.ended("shuyi-9", State::Cancelled);
    tasks.expire(at(3_600));
    assert!(tasks.has("shuyi-1") && tasks.has("shuyi-2") && tasks.has("shuyi-3"));
    tasks.expire(at(3_601));
    assert!(tasks.has("shuyi-1") && !tasks.has("shuyi-2") && tasks.has("shuyi-3"));
    tasks.expire(at(100_000));
    assert!(tasks.has("shuyi-1") && !tasks.has("shuyi-3"));
    assert!(!tasks.has("shuyi-9"));
}
