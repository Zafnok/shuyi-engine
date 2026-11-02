//! Browser-target replay bridge for the synthetic Workshop research fixture.
//!
//! The WebAssembly exports pass UTF-8 bytes one at a time to keep the ABI
//! independent of a JavaScript binding generator. The whole replay runs in
//! Rust; the page only transports a request and reads the JSON response.

use std::cell::RefCell;

use cloning_fixture::workshop::{self, Workshop};
use serde::{Deserialize, Serialize};
use shuyi_core::{TurnRules, state_hash};

const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
struct Request {
    seed: String,
    actions: Vec<String>,
}

#[derive(Clone, Serialize)]
struct Record {
    ok: bool,
    action: String,
    events: Vec<String>,
    state: serde_json::Value,
    frame: String,
    hash: String,
    outcome: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct Batch {
    game: &'static str,
    seed: u64,
    trace: Vec<Record>,
    r#final: Record,
}

fn record(
    game: &Workshop,
    action: &str,
    events: Vec<String>,
    error: Option<String>,
) -> Result<Record, String> {
    Ok(Record {
        ok: error.is_none(),
        action: action.to_owned(),
        events,
        state: game.visible(),
        frame: game.frame(),
        hash: state_hash(game)
            .map_err(|hash_error| hash_error.to_string())?
            .to_string(),
        outcome: game.outcome().map(|outcome| format!("{outcome:?}")),
        error,
    })
}

fn step(game: &mut Workshop, action: &str) -> Result<Record, String> {
    if action == "state" {
        return record(game, action, Vec::new(), None);
    }
    let result = workshop::Action::parse(action)
        .ok_or_else(|| format!("unknown action: {action}"))
        .and_then(|parsed| {
            game.apply(&parsed)
                .map(|events| {
                    events
                        .into_iter()
                        .map(|event| format!("{event:?}"))
                        .collect()
                })
                .map_err(|error| format!("{error:?}"))
        });
    match result {
        Ok(events) => record(game, action, events, None),
        Err(error) => record(game, action, Vec::new(), Some(error)),
    }
}

/// Replay a Workshop session using the same record schema as the native fixture.
pub fn replay(seed: u64, actions: &[String]) -> Result<String, String> {
    let mut game = Workshop::new(seed);
    let mut trace = vec![record(&game, "state", Vec::new(), None)?];
    for action in actions {
        trace.push(step(&mut game, action)?);
    }
    let r#final = trace.last().cloned().ok_or("missing initial record")?;
    serde_json::to_string(&Batch {
        game: "workshop",
        seed,
        trace,
        r#final,
    })
    .map_err(|error| error.to_string())
}

#[derive(Default)]
struct Bridge {
    request: Vec<u8>,
    response: Vec<u8>,
}

thread_local! {
    static BRIDGE: RefCell<Bridge> = RefCell::new(Bridge::default());
}

// Rust 2024 marks unmangled names unsafe because they could collide with
// another export. These five `shuyi_*` functions are the browser ABI.
/// Clear the request buffer before sending a new UTF-8 JSON request.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_input_clear() {
    BRIDGE.with(|bridge| bridge.borrow_mut().request.clear());
}

/// Append one request byte; return 0 on success, 1 on invalid byte or size.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_input_push(byte: u32) -> u32 {
    BRIDGE.with(|bridge| {
        let mut bridge = bridge.borrow_mut();
        let Ok(byte) = u8::try_from(byte) else {
            return 1;
        };
        if bridge.request.len() >= MAX_REQUEST_BYTES {
            return 1;
        }
        bridge.request.push(byte);
        0
    })
}

/// Run the buffered request; return 0 on success or 1 with a JSON error.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_run() -> u32 {
    BRIDGE.with(|bridge| {
        let mut bridge = bridge.borrow_mut();
        let answer = serde_json::from_slice::<Request>(&bridge.request)
            .map_err(|error| error.to_string())
            .and_then(|request| {
                request
                    .seed
                    .parse::<u64>()
                    .map_err(|error| error.to_string())
                    .and_then(|seed| replay(seed, &request.actions))
            });
        match answer {
            Ok(json) => {
                bridge.response = json.into_bytes();
                0
            }
            Err(error) => {
                bridge.response = serde_json::json!({ "error": error })
                    .to_string()
                    .into_bytes();
                1
            }
        }
    })
}

/// Number of bytes in the last JSON response.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_output_len() -> u32 {
    BRIDGE.with(|bridge| u32::try_from(bridge.borrow().response.len()).unwrap_or(u32::MAX))
}

/// Byte at `index` in the last response, or 256 when out of bounds.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn shuyi_output_byte(index: u32) -> u32 {
    BRIDGE.with(|bridge| {
        bridge
            .borrow()
            .response
            .get(usize::try_from(index).unwrap_or(usize::MAX))
            .map_or(256, |byte| u32::from(*byte))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        replay, shuyi_input_clear, shuyi_input_push, shuyi_output_byte, shuyi_output_len, shuyi_run,
    };
    use serde_json::{Value, json};

    #[test]
    fn bridge_replays_and_rejects_bad_requests() {
        let actions = vec!["gather".to_owned(), "invalid".to_owned()];
        let expected = replay(11, &actions).unwrap();
        let request = br#"{"seed":"11","actions":["gather","invalid"]}"#;
        shuyi_input_clear();
        for byte in request {
            assert_eq!(shuyi_input_push(u32::from(*byte)), 0);
        }
        assert_eq!(shuyi_run(), 0);
        let actual: Vec<u8> = (0..shuyi_output_len())
            .map(|index| u8::try_from(shuyi_output_byte(index)).unwrap())
            .collect();
        assert_eq!(String::from_utf8(actual).unwrap(), expected);
        assert_eq!(shuyi_output_byte(shuyi_output_len()), 256);

        shuyi_input_clear();
        assert_eq!(shuyi_input_push(256), 1);
        assert_eq!(shuyi_run(), 1);
        assert!(shuyi_output_len() > 0);
    }

    #[test]
    fn state_action_reads_without_advancing_the_workshop() {
        let actions = vec!["state".to_owned(), "gather".to_owned()];
        let batch: Value = serde_json::from_str(&replay(11, &actions).unwrap()).unwrap();
        let trace = batch["trace"].as_array().unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace[1]["action"], "state");
        assert_eq!(trace[1]["ok"], true);
        assert_eq!(trace[1]["events"], json!([]));
        assert_eq!(trace[1]["error"], Value::Null);
        assert_eq!(trace[1]["state"], trace[0]["state"]);
        assert_eq!(trace[1]["frame"], trace[0]["frame"]);
        assert_eq!(trace[1]["hash"], trace[0]["hash"]);
        assert_eq!(trace[2]["action"], "gather");
        assert_ne!(trace[2]["state"], trace[1]["state"]);
    }

    #[test]
    fn request_buffer_accepts_exactly_sixty_four_kib() {
        shuyi_input_clear();
        for _ in 0..65_536 {
            assert_eq!(shuyi_input_push(u32::from(b' ')), 0);
        }
        assert_eq!(shuyi_input_push(u32::from(b' ')), 1);
    }
}
