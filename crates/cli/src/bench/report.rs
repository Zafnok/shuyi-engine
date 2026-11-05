//! What `bench` found: `bench-report.json` ([`Report`]), what is over a
//! budget ([`over`]) and the table for a pull request ([`markdown`]).

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use shuyi_ui::perf::bytes_text;

/// What `bench` found: `bench-report.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Report {
    /// The scenario format it read (and this report's).
    pub format_version: u32,
    /// The targets gated, with their budgets.
    pub targets: Vec<Budget>,
    /// Each scenario's runs: once for the colour targets, once on each
    /// one-bit one.
    pub runs: Vec<Run>,
    /// The packages measured.
    pub packages: Vec<Package>,
    /// Every number over its budget.
    pub over: Vec<Over>,
    /// What wasn't measured, or gated.
    pub limits: Vec<String>,
}

/// A target's budgets, from its profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Budget {
    /// The target's id.
    pub target: String,
    /// The most instructions one frame may take.
    pub frame_instructions: Option<u64>,
    /// The most heap at once, in bytes.
    pub peak_heap_bytes: Option<u64>,
    /// The biggest package, in bytes.
    pub package_bytes: Option<u64>,
}

/// One scenario played on some targets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Run {
    /// The scenario's name.
    pub scenario: String,
    /// Its file, from the game's folder.
    pub file: String,
    /// What it plays.
    pub about: String,
    /// The targets it was played for.
    pub on: Vec<String>,
    /// Whether it ran on a one-bit target, each frame converted.
    pub one_bit: bool,
    /// The instructions from the program's start to the first frame, if
    /// counted: never gated.
    pub start_instructions: Option<u64>,
    /// Its measured frames' instructions, if counted.
    pub instructions: Option<Spread>,
    /// The heap's highest peak in its measured frames, if counted.
    pub peak_heap: Option<Peak>,
    /// Each measured frame.
    pub frames: Vec<FrameCounts>,
    /// What it can't promise.
    pub limits: Vec<String>,
}

/// What one frame counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FrameCounts {
    /// Its number: the first frame the game runs is 1.
    pub frame: u32,
    /// The instructions it took, if counted.
    pub instructions: Option<u64>,
    /// The heap's peak at its end, if counted.
    pub peak_heap_bytes: Option<u64>,
}

/// The instructions of a run's frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Spread {
    /// The most one frame took.
    pub most: u64,
    /// The first frame that took the most.
    pub frame: u32,
    /// The 99th percentile (nearest rank).
    pub p99: u64,
    /// The mean, rounded down.
    pub mean: u64,
}

/// The heap's highest peak in a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Peak {
    /// The bytes.
    pub bytes: u64,
    /// The first frame it was reached by.
    pub frame: u32,
}

/// A package `export` made, measured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Package {
    /// The target profile it was checked against.
    pub target: String,
    /// The export target that made it (`web`, `playdate`).
    pub export: String,
    /// The package's file, from the export's folder.
    pub file: String,
    /// Its size.
    pub bytes: u64,
}

/// What a number over its budget is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum What {
    /// One frame's instructions.
    FrameInstructions,
    /// The heap's peak.
    PeakHeap,
    /// The package's size.
    Package,
}

/// A number over its budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Over {
    /// The target whose budget it is over.
    pub target: String,
    /// What it measures.
    pub what: What,
    /// The scenario, for a frame's number.
    pub scenario: Option<String>,
    /// The frame, for a frame's number.
    pub frame: Option<u32>,
    /// The package's file, for its size.
    pub package: Option<String>,
    /// The number.
    pub value: u64,
    /// The budget.
    pub budget: u64,
}

impl Over {
    /// What it says, in a line.
    pub(crate) fn text(&self) -> String {
        let scenario = self.scenario.as_deref().unwrap_or_default();
        let frame = self.frame.unwrap_or_default();
        let (value, budget) = (self.value, self.budget);
        match self.what {
            What::FrameInstructions => format!(
                "{scenario} on {}: frame {frame} took {} instructions, over the budget of {}",
                self.target,
                grouped(value),
                grouped(budget)
            ),
            What::PeakHeap => format!(
                "{scenario} on {}: the heap's peak reached {} by frame {frame}, over the budget \
                 of {}",
                self.target,
                bytes(value),
                bytes(budget)
            ),
            What::Package => format!(
                "{}: the package {} is {}, over the budget of {}",
                self.target,
                self.package.as_deref().unwrap_or_default(),
                bytes(value),
                bytes(budget)
            ),
        }
    }
}

/// The instructions of `frames` that were counted; `None` if none was.
pub(crate) fn spread(frames: &[FrameCounts]) -> Option<Spread> {
    let counted: Vec<(u32, u64)> = frames
        .iter()
        .filter_map(|f| f.instructions.map(|n| (f.frame, n)))
        .collect();
    let (frame, most) = counted
        .iter()
        .copied()
        .reduce(|best, next| if next.1 > best.1 { next } else { best })?;
    let mut sorted: Vec<u64> = counted.iter().map(|&(_, n)| n).collect();
    sorted.sort_unstable();
    let rank = (sorted.len() * 99).div_ceil(100).max(1);
    let sum: u128 = sorted.iter().map(|&n| u128::from(n)).sum();
    let mean = sum / sorted.len() as u128;
    Some(Spread {
        most,
        frame,
        p99: sorted[rank - 1],
        mean: u64::try_from(mean).unwrap_or(u64::MAX),
    })
}

/// The heap's highest peak in `frames`, and the first frame it was
/// reached by; `None` if the heap wasn't counted.
pub(crate) fn peak(frames: &[FrameCounts]) -> Option<Peak> {
    frames
        .iter()
        .filter_map(|f| {
            f.peak_heap_bytes.map(|bytes| Peak {
                bytes,
                frame: f.frame,
            })
        })
        .reduce(|best, next| if next.bytes > best.bytes { next } else { best })
}

/// Every number of `runs` and `packages` over a budget of `budgets`: a
/// run's against each target it was played for, a package's against its
/// target's.
pub(crate) fn over(budgets: &[Budget], runs: &[Run], packages: &[Package]) -> Vec<Over> {
    let mut out = Vec::new();
    for run in runs {
        for budget in budgets.iter().filter(|b| run.on.contains(&b.target)) {
            let mut add = |what, frame, value, limit: Option<u64>| {
                if let Some(limit) = limit.filter(|&limit| value > limit) {
                    out.push(Over {
                        target: budget.target.clone(),
                        what,
                        scenario: Some(run.scenario.clone()),
                        frame: Some(frame),
                        package: None,
                        value,
                        budget: limit,
                    });
                }
            };
            if let Some(spread) = run.instructions {
                let limit = budget.frame_instructions;
                add(What::FrameInstructions, spread.frame, spread.most, limit);
            }
            if let Some(peak) = run.peak_heap {
                add(
                    What::PeakHeap,
                    peak.frame,
                    peak.bytes,
                    budget.peak_heap_bytes,
                );
            }
        }
    }
    for package in packages {
        let limit = budgets
            .iter()
            .find(|b| b.target == package.target)
            .and_then(|b| b.package_bytes);
        if let Some(limit) = limit.filter(|&limit| package.bytes > limit) {
            out.push(Over {
                target: package.target.clone(),
                what: What::Package,
                scenario: None,
                frame: None,
                package: Some(package.file.clone()),
                value: package.bytes,
                budget: limit,
            });
        }
    }
    out
}

/// `n` with its thousands grouped: `1,234,567`.
pub(crate) fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `n` bytes for people, with the exact count when it isn't a few bytes.
fn bytes(n: u64) -> String {
    let text = bytes_text(n);
    if text.ends_with(" B") {
        text
    } else {
        format!("{text} ({} bytes)", grouped(n))
    }
}

/// `n`, or a dash when there's none.
fn or_dash(n: Option<String>) -> String {
    n.unwrap_or_else(|| "—".to_owned())
}

/// `report` as Markdown: the runs, the budgets, the packages, what is
/// over and what wasn't measured.
pub(crate) fn markdown(report: &Report) -> String {
    let mut out = String::from("## shuyi bench\n\n");
    out.push_str(
        "| Scenario | On | Frames | Instructions: most (frame) | p99 | Mean | Peak heap (frame) |\n\
         | --- | --- | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for run in &report.runs {
        let spread = run.instructions;
        let most = spread.map(|s| format!("{} ({})", grouped(s.most), s.frame));
        let peak = run
            .peak_heap
            .map(|p| format!("{} ({})", bytes_text(p.bytes), p.frame));
        // Writing to a `String` can't fail.
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} |",
            run.scenario,
            run.on.join(", "),
            run.frames.len(),
            or_dash(most),
            or_dash(spread.map(|s| grouped(s.p99))),
            or_dash(spread.map(|s| grouped(s.mean))),
            or_dash(peak),
        );
    }
    out.push_str(
        "\n| Target | Frame instructions | Peak heap | Package |\n| --- | ---: | ---: | ---: |\n",
    );
    for budget in &report.targets {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            budget.target,
            or_dash(budget.frame_instructions.map(grouped)),
            or_dash(budget.peak_heap_bytes.map(bytes_text)),
            or_dash(budget.package_bytes.map(bytes_text)),
        );
    }
    if !report.packages.is_empty() {
        out.push_str("\n| Package | Export | Size |\n| --- | --- | ---: |\n");
        for package in &report.packages {
            let _ = writeln!(
                out,
                "| {} | {} | {} |",
                package.file,
                package.export,
                bytes_text(package.bytes)
            );
        }
    }
    if report.over.is_empty() {
        out.push_str("\nEvery number is within its budget.\n");
    } else {
        out.push_str("\n**Over budget:**\n\n");
        for over in &report.over {
            let _ = writeln!(out, "- {}", over.text());
        }
    }
    let limits = report
        .limits
        .iter()
        .chain(report.runs.iter().flat_map(|run| &run.limits));
    let mut first = true;
    for limit in limits {
        if first {
            out.push_str("\nNot measured or not gated:\n\n");
            first = false;
        }
        let _ = writeln!(out, "- {limit}");
    }
    out
}
