//! Tasks: MCP's tasks extension ([`EXTENSION`]) on 2026-07-28. A
//! `tools/call` of a long tool ([`super::tasked`]) from a client that
//! declared the extension answers a task at once, and the client polls it
//! with `tasks/get` while the host works; `tasks/cancel` stops it
//! (ADR-0159).
//!
//! [`Tasks`] is the table: each task's id (`shuyi-<n>`, counted from 1 in
//! the process), its tool, its times and its state. It knows nothing of
//! the work, `W`, which the server keeps while the task is
//! [`State::Working`] and polls.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::contract::Op;

/// The extension's name, as a client's capabilities and `server/discover`
/// carry it.
pub const EXTENSION: &str = "io.modelcontextprotocol/tasks";

/// How long a task is kept once it has ended: an hour, as Claude Code
/// asks of a server.
pub const KEEP: Duration = Duration::from_secs(3600);

/// How often a client is asked to poll `start`'s task, in milliseconds:
/// a build that is done in seconds is played at once.
const POLL_START_MS: u64 = 2_000;

/// How often a client is asked to poll any other task: they run for
/// minutes.
const POLL_MS: u64 = 10_000;

/// Where a task stands.
#[derive(Debug)]
pub enum State<W> {
    /// Its work runs.
    Working(W),
    /// It ended with this `tools/call` result (an error result too).
    Completed(Value),
    /// It ended with this JSON-RPC error, the one `tools/call` would have
    /// answered.
    Failed(Value),
    /// It was stopped before it ended.
    Cancelled,
}

/// A task.
#[derive(Debug)]
struct Task<W> {
    /// The tool it runs.
    op: Op,
    /// When it was made.
    created: SystemTime,
    /// When its state last changed.
    updated: SystemTime,
    state: State<W>,
}

/// The tasks of the process, by id.
#[derive(Debug)]
pub struct Tasks<W> {
    /// How many it has made.
    made: u64,
    tasks: BTreeMap<String, Task<W>>,
}

impl<W> Default for Tasks<W> {
    fn default() -> Self {
        Self {
            made: 0,
            tasks: BTreeMap::new(),
        }
    }
}

impl<W> Tasks<W> {
    /// A new task of the tool `op`, made at `now`, in `state`: its id.
    pub fn make(&mut self, op: Op, now: SystemTime, state: State<W>) -> String {
        self.made += 1;
        let id = format!("shuyi-{}", self.made);
        let task = Task {
            op,
            created: now,
            updated: now,
            state,
        };
        self.tasks.insert(id.clone(), task);
        id
    }

    /// The ids of the tasks still working, of the tool `op` if given.
    pub fn working(&self, op: Option<Op>) -> Vec<String> {
        self.tasks
            .iter()
            .filter(|(_, task)| matches!(task.state, State::Working(_)))
            .filter(|(_, task)| op.is_none_or(|op| task.op == op))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// The work of task `id`, if it is working.
    pub fn work(&mut self, id: &str) -> Option<&mut W> {
        match &mut self.tasks.get_mut(id)?.state {
            State::Working(work) => Some(work),
            _ => None,
        }
    }

    /// The work of each task still working.
    pub fn works(&self) -> impl Iterator<Item = &W> {
        self.tasks.values().filter_map(|task| match &task.state {
            State::Working(work) => Some(work),
            _ => None,
        })
    }

    /// Whether there is a task `id`.
    pub fn has(&self, id: &str) -> bool {
        self.tasks.contains_key(id)
    }

    /// Ends task `id`, if it is working, at `now` as `state`: its work,
    /// for the server to finish or drop.
    pub fn end(&mut self, id: &str, now: SystemTime, state: State<W>) -> Option<W> {
        let task = self.tasks.get_mut(id)?;
        if !matches!(task.state, State::Working(_)) {
            return None;
        }
        task.updated = now;
        match std::mem::replace(&mut task.state, state) {
            State::Working(work) => Some(work),
            _ => None,
        }
    }

    /// Sets how task `id` ended, once its work was taken ([`Self::end`]),
    /// keeping when it ended.
    pub fn ended(&mut self, id: &str, state: State<W>) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.state = state;
        }
    }

    /// Forgets each task that ended more than [`KEEP`] before `now`.
    pub fn expire(&mut self, now: SystemTime) {
        self.tasks.retain(|_, task| {
            matches!(task.state, State::Working(_))
                || now
                    .duration_since(task.updated)
                    .is_ok_and(|after| after <= KEEP)
        });
    }

    /// Task `id` as `tools/call`'s answer gives it: its id, status, times,
    /// how long it is kept and how often to poll it. None if there is no
    /// such task.
    pub fn json(&self, id: &str) -> Option<Value> {
        let task = self.tasks.get(id)?;
        let status = match task.state {
            State::Working(_) => "working",
            State::Completed(_) => "completed",
            State::Failed(_) => "failed",
            State::Cancelled => "cancelled",
        };
        // While it works, it is kept: it is forgotten an hour after it ends,
        // and a time to live counts from when it was made.
        let ttl = if let State::Working(_) = task.state {
            Value::Null
        } else {
            let ran = task
                .updated
                .duration_since(task.created)
                .unwrap_or_default();
            json!(millis(ran + KEEP))
        };
        let poll = if task.op == Op::Start {
            POLL_START_MS
        } else {
            POLL_MS
        };
        Some(json!({
            "taskId": id,
            "status": status,
            "createdAt": stamp(task.created),
            "lastUpdatedAt": stamp(task.updated),
            "ttlMs": ttl,
            "pollIntervalMs": poll,
        }))
    }

    /// Task `id` as `tasks/get` gives it: [`Self::json`], with its result
    /// once completed, or its error once failed.
    pub fn detailed(&self, id: &str) -> Option<Value> {
        let mut detailed = self.json(id)?;
        match &self.tasks.get(id)?.state {
            State::Completed(result) => detailed["result"] = result.clone(),
            State::Failed(error) => {
                detailed["error"] = error.clone();
                detailed["statusMessage"] = error["message"].clone();
            }
            State::Working(_) | State::Cancelled => {}
        }
        Some(detailed)
    }
}

/// `duration` in whole milliseconds.
fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// `time` in ISO 8601, in UTC to the millisecond:
/// `2026-11-27T14:03:07.250Z`.
pub fn stamp(time: SystemTime) -> String {
    let since = millis(time.duration_since(UNIX_EPOCH).unwrap_or_default());
    let (secs, ms) = (since / 1000, since % 1000);
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let (year, month, day) = crate::reveal::civil(days);
    let of_day = secs % 86_400;
    let (hour, minute, second) = (of_day / 3600, of_day / 60 % 60, of_day % 60);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{ms:03}Z")
}

#[cfg(test)]
mod tests;
