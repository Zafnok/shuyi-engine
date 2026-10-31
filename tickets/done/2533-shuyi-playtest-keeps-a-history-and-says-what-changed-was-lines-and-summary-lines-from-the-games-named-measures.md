---
id: "2533"
title: "shuyi playtest keeps a history and says what changed: `was …` beside each entrant, and summary lines from the game's named measures"
type: feature
milestone: After MVP
model: implementation
effort: high
status: done
blocked_by: ["2532"]
nick_input: none
completed: 2026-10-24
---

# 2533 — The playtest history and its summary lines

## Context

A request from the game.

1. **The game ticket:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0506-persona-bots-casual-normal-hardcore.md`).
   The game's follow-up that moves onto this ticket is game#1180.
   Neither is a blocker of this one.
2. **The job in one sentence:** after a playtest, show per bot the few
   numbers a designer reads first (how many tries won, how often one,
   two or three pieces were lost, which piece was lost most, the median
   items used and turns taken) and what the same numbers were on the
   last run of the same stage and bots, with that run's date and commit.
3. **No friction review triggered:** game#0506 measured 1.8×. Nick
   (2026-10-22) asked that the generic parts of the game's playtesting
   move into the engine; this is one ticket of that sweep (game
   `docs/engine/friction.md`, row "0506 (sweep)"). In the game's
   `crates/xtask/src/playtest.rs`, `render`, `Lines`, `median`, `was`,
   `history_file`, `last_record`, `append`, `git_commit` and
   `utc_stamp` are about 220 production lines, none of them about the
   game: each line names a measure.
4. **The generic shape:** the playtest plan (2532) gains, per entrant, a
   list of **summary items** over 2126's named measures, as data:
   `Sum` ("won 83/100"), `Spread` ("deaths per try 0: 30 1: 21 2: 25
   3+: 24"), `Most` (the key under a prefix set in the most tries:
   "most: scout 41×" from measures `fell scout`), `Median` and `Max`. And
   `shuyi playtest` keeps every run in a JSON-lines **history**, one
   file per stage, entrants and game options, and adds to each entrant's
   line the headline number of the last run ("· was 47/100 succeed",
   from the entrant's first rate check, or its first summary item), and
   a line with that run's date and commit.
5. **Examples from games that aren't tactics:** a platformer's
   playtest: "died per try 0: 61 1: 30 2+: 9 · most: spike pit 3 22×";
   a deck-builder's: "won 71/100 · median cards played 34"; a racing
   game: "median lap 3 time". Each wants "was …" after a balance change.

## Nick input

None.

## Scope

**In:**
- `crates/cli/src/contract/playtest.rs` (or a `playtest/` folder with
  `summary.rs` and `history.rs` if the file passes about 800 lines):
  `Summary` items, their text, the history, the `was …` lines.
- `PlaytestPlan` (2532) gains `summary: Vec<Vec<Summary>>` (per entrant,
  in `entrants` order; empty means none) and `Summary` is serde, so a
  game may keep its items in a data file.
- Options: `--history <dir>` (default `target/playtest-history/` under
  the project root), `--no-history`, and `--brief` (leave out the
  table's one line per try; the summary stays).
- `docs/contract.md`; the usage text.
- **From 1134 (done):** the `shuyi-balance-bots` skill
  (`skills/shuyi-balance-bots/SKILL.md`) shows `shuyi playtest`'s
  history: a plan's `summary` items, the `was …` and `previous` lines,
  `--history`, `--no-history` and `--brief`; `check-skills` passes.

**Out (do not do):**
- Charts, HTML or a dashboard.
- Comparing two arbitrary runs (only the last one before this).
- A stamp of the engine version and the content's fingerprint (2111).
  **Either order with 2111:** if 2111 is done, the record carries its
  stamp too (`made`, from `GameTools::made()`); if not, only the date
  and the commit, and 2111 adds the stamp (its clause says so).
- The game's use of it (game#1180).

## Implementation steps

1. **Read** 2532's `PlaytestPlan` and answer, and the game's
   `crates/xtask/src/playtest.rs` `render`, `Lines`, `was`,
   `history_file`, `utc_stamp` (game repository, `main`) and their tests
   (`crates/xtask/src/playtest/tests.rs`) for the exact lines.
2. **`Summary`**:
   ```rust
   pub enum Summary {
       /// "won 83/100": the sum of `measure` over the tries, of how many.
       Sum { measure: String, label: Option<String> },
       /// "deaths per try 0: 30  1: 21  2: 25  3+: 24": tries by value, the last bucket open.
       Spread { measure: String, up_to: u32, label: Option<String> },
       /// "most: scout 41×": among keys "<prefix><key>" set to 1 or more
       /// (keys with a further `skip_suffix` left out), the key set in
       /// the most tries, then the first by name; "nobody" if none.
       Most { prefix: String, skip_suffix: Option<String>, label: Option<String> },
       /// "items median 1": the lower middle value, 0 for no tries.
       Median { measure: String, label: Option<String> },
       Max { measure: String, label: Option<String> },
   }
   ```
   A missing measure counts as 0. `label` defaults to the measure's
   name (`"most"` for `Most`). The items print on one line after the
   entrant's name and label, ` · ` between them, then 2532's time per
   try; wrap to a second line, indented, past 100 characters.
3. **History**: a record is `{ date: "2026-10-22 14:05 UTC", commit,
   title, stage, entrants, options, output }` (one stage's
   `PlaytestOutput` of 2532's answer, a list of them), one JSON
   line per run. The file is `<stage or "stage">-<entrants joined by
   "+">[-<game options>].jsonl`, the options as `flag=value` with
   anything outside `[A-Za-z0-9_.=-]` turned into `_`. The commit is
   `git -C <project root> rev-parse --short HEAD`, or `unknown`; the
   date from `SystemTime`, written without a date crate (copy the
   game's `utc_stamp` and its tests). Read the file's last line that
   parses before writing the new one.
4. **`was …`**: per entrant that played in the last run: its first
   `Rate` check's successes ("· was 47/100 succeed"), else its first
   summary item's number ("· was won 63/100"). Then a line "previous
   <date> · commit <hash>". The JSON answer gains `previous: Option<{
   date, commit }>`.
5. **`--brief`**: the text without 2126's per-try rows (what the game's
   CI cuts with `grep -v '^try '`).
6. **hello-grid**: its plan gets a summary (`Sum` of `won`, `Median` of
   `turns`); its playtest test runs twice in a temporary history folder
   and checks the second run's `was` line.
7. **The skill**: the `shuyi-balance-bots` lines (Scope).

## Acceptance criteria

- [x] Each `Summary` kind prints as in its doc comment on a fixture of
      four tries (unit tests with the exact strings).
- [x] `Most` with prefix `"fell "` and `skip_suffix: Some(" turn")`
      ignores `fell scout turn` and breaks ties by name.
- [x] A second run with the same stage, entrants and options shows
      `was` numbers and the first run's date and commit; a run with
      other entrants writes another file and shows none.
- [x] `--no-history` writes nothing; a history folder that can't be
      written is an error naming the path.
- [x] `--brief` has no per-try rows and everything else.
- [x] The UTC stamp is right on 2000-02-29, 2026-12-31 23:59 and
      1970-01-01 00:00 (the game's tests).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: summary text, history file names, `was` lines, the stamp.
- Integration: hello-grid's playtest run twice through the binary.

## Completion notes

What was done:

- `contract::playtest::Summary` (`playtest/summary.rs`): `Sum`, `Spread`,
  `Most`, `Median`, `Max` over the named measures, serde (a game may keep
  them in a RON file), each with its text as its doc comment shows,
  built with `Summary::sum` … `max`, `with_label`, `with_skip_suffix`
  (ADR-0039 kind 6). `PlaytestPlan::summary: Vec<Vec<Summary>>`, per
  entrant in `entrants` order, empty from `PlaytestPlan::new`.
- Each entrant's line: name, label, the summary items, the time per try
  and `was …`, ` · ` between them, wrapped past 100 characters onto lines
  indented under the first item. `Played` gains `summary` (the items'
  text) and `was`; `PlaytestOutput` gains `previous: Option<Previous>`
  (`date`, `commit`). Contract version 19.
- The history (`playtest/history.rs`): one JSON line per run and stage,
  `{date, commit, title, stage, entrants, options, output}`, in
  `<stage>-<entrants joined by +>[-<options>].jsonl` under
  `target/playtest-history/` (or `--history`), the commit from
  `git rev-parse --short HEAD` or `unknown`, the date by the game's
  `utc_stamp` and its tests (plus 2026-12-31 23:59). The last line that
  parses is read before the new one is written.
- `playtest::kept(game, root, input)` plays and keeps the history; the
  CLI and the MCP `playtest` tool call it. `playtest::playtest` keeps
  none, as before. `PlaytestInput` gains `history` and `no_history`;
  `PlaytestArgs` gains `brief`. A history that can't be written fails
  the answer with `SH3004`, naming the file, with the reports still in
  it.
- `--brief` prints `playtest::brief`: the core's new
  `PlaytestReport::brief()`, the table without its line per try, and
  everything else.
- Hello, Grid's plan sums `won` and gives the median of `turns` for
  each hero; its test runs `playtest` twice through `main_with` in a
  temporary history and reads the second run's `was` (`was 5/50
  succeed`, `was 50/50 succeed`) and `previous`, then a run of `walk`
  alone keeps another file and shows none.
- Docs: `docs/contract.md`, the usage text, `SH3004`'s page and the
  `shuyi-balance-bots` skill (the summary items, the history, `was` and
  `previous`, `--history`, `--no-history`, `--brief`, `SH3004`).

Deviations:

- History lives in `kept`, not in `playtest`, so the existing
  `playtest(game, input)` stays pure (no file, no clock) and the MCP
  tool keeps a history as the command does.
- The file name cleans the stage's and the entrants' names too, not only
  the options (a name with `/` would name another folder). An option is
  written without its dashes (`mode=casual`, `hard`).
- `was` reads the last run's own summary text (its first item), not
  the current item recomputed over its tries: what was printed then.
- `.gitignore` ignores `/examples/*/target/`, where an example's history
  goes by default.

Follow-ups: none new. 2111's clause now says it adds `made` to the
history's record; 2544's spread clause says what 2533's items give.

The game moves onto this in game#1180 (its PR will move the pin);
the engine's PR merges first.

Breaking: shuyi-cli: contract::playtest::Played, PlaytestOutput: new fields `summary`, `was` and `previous`, and now `#[non_exhaustive]`: build neither by literal (the engine does)
Breaking: shuyi-cli: contract::playtest::PlaytestArgs: new field `brief`, and now `#[non_exhaustive]`: read it from `parse_args`, don't build it by literal
