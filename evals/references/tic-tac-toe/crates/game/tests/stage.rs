//! The board played through the engine's Harness, and through the game's
//! tools (the tool contract). Key names here are the Harness's test-script
//! names, input to a test, not game code.

use std::fs;
use std::path::Path;

use game::stages::top_down_grid::{StageRecord, StageScreen, Tools, crash_record};
use rules::{Board, Command, Mark, Outcome};
use shuyi::core::{Pos, Sides, TurnRules};
use shuyi::platform::harness::Harness;
use shuyi::prelude::{Cell, GlyphBuffer, Rgb, Screen};
use shuyi::ui::frame::color::KEY;
use shuyi::ui::text::TextRole;
use shuyi_cli::GameTools;
use shuyi_cli::contract::check::{CheckInput, check, targets};
use shuyi_cli::contract::playtest::{PlaytestInput, playtest};
use shuyi_cli::contract::replay::{ReplayInput, replay};
use shuyi_cli::contract::run::{RunInput, keys, run, scene};
use shuyi_cli::contract::target::engine_profiles;
use shuyi_cli::project::Project;

/// The game's tools.
fn tools() -> Tools {
    game::stages::tools()
}

/// The game in the Harness, on a new game seeded `seed`.
fn on_stage(seed: u64) -> Harness<game::Data> {
    let mut h = tools().harness(seed).unwrap_or_else(|e| panic!("{e}"));
    h.keys("Enter");
    h
}

fn stage(h: &Harness<game::Data>) -> &StageScreen {
    h.screen::<StageScreen>()
        .unwrap_or_else(|| panic!("no stage: {:?}", h.screens()))
}

/// X's win: the middle, the top-left and the bottom-right, O between.
const WIN: &str = "Enter Up Enter Left Enter Right Right Enter Down Down Enter";

#[test]
fn confirm_on_the_title_opens_the_board_with_the_cursor_in_the_middle() {
    let mut h = on_stage(0);
    assert_eq!(h.screens(), ["title", "stage"]);
    assert_eq!(stage(&h).cursor, Pos::new(1, 1));
    // The cursor stops at the edges.
    h.keys("Up Up Up Left Left");
    assert_eq!(stage(&h).cursor, Pos::new(0, 0));
    h.keys("Enter");
    let screen = stage(&h);
    assert_eq!(screen.board.at(Pos::new(0, 0)), Some(Mark::X));
    assert_eq!(screen.board.side_to_move(), Mark::O);
    assert_eq!(screen.cursor, Pos::new(0, 0), "it stays after a mark");
    // A marked cell is refused, and nothing changes.
    h.keys("Enter");
    assert_eq!(stage(&h).commands.len(), 1);
    assert!(h.snapshot().contains("O to move"), "{}", h.snapshot());
}

#[test]
fn a_won_game_stays_on_screen_and_confirm_starts_the_next() {
    let mut h = on_stage(5);
    h.keys(WIN);
    let screen = stage(&h);
    assert_eq!(
        screen.board.outcome(),
        Some(Outcome::Won {
            mark: Mark::X,
            turn: 5
        })
    );
    assert!(h.snapshot().contains("X wins!"), "{}", h.snapshot());
    h.keys("Enter");
    assert_eq!(stage(&h).seed, 6);
    let found = &stage(&h).commands;
    assert!(found.is_empty(), "{found:?}");
    // Cancel goes back to the title, and there leaves the game.
    h.keys("Backspace");
    assert_eq!(h.screens(), ["title"]);
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
fn the_stages_help_line_paints_its_keys_in_the_palettes_key_colour() {
    let h = on_stage(0);
    let ctx = h.ctx();
    let key = ctx.palette.lookup(KEY).expect("the palette names `key`");
    let view = stage(&h).view(ctx);
    let keys: String = view
        .help
        .spans()
        .iter()
        .filter(|span| matches!(span.role, Some(TextRole::Key | TextRole::Arrows)))
        .map(|span| span.text.as_str())
        .collect();
    assert!(!keys.is_empty(), "{:?}", view.help);
    let black = Rgb::new(0, 0, 0);
    let mut buf = GlyphBuffer::new(60, 24, Cell::new(' ', black, black));
    stage(&h).draw(ctx, &mut buf);
    let cells: String = (0..60)
        .filter_map(|x| buf.get(x, 23))
        .filter(|cell| cell.fg == key)
        .map(|cell| cell.glyph)
        .collect();
    assert_eq!(cells, keys);
}

#[test]
fn check_finds_no_error_in_the_files() {
    let answer = check(&tools(), &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
}

#[test]
fn run_and_scene_show_the_board_and_its_data() {
    let mut input = RunInput::default();
    input.script = keys("Enter Enter Up Enter");
    let answer = run(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    let shown = answer.result.unwrap().scene.unwrap();
    assert_eq!(shown.data["turn"], "x");
    assert_eq!(shown.data["winner"], "");
    assert_eq!(shown.data["board"][0], ".o.");
    assert_eq!(shown.data["board"][1], ".x.");
    assert_eq!(shown.data["commands"], 2);
    // `scene` sees the same, and the end of a game.
    let answer = scene(&tools(), &input);
    assert_eq!(answer.result.unwrap().scene.unwrap().text, shown.text);
    input.script = keys(&format!("Enter {WIN}"));
    input.path = Some("/winner".to_owned());
    let answer = scene(&tools(), &input);
    assert_eq!(answer.result.unwrap().value, Some("x".into()));
    input.path = None;
    input.shape = true;
    let shape = scene(&tools(), &input).result.unwrap().shape.unwrap();
    for key in ["turn", "winner", "board", "cursor", "seed", "commands"] {
        assert!(
            shape["properties"].get(key).is_some(),
            "no {key} in {shape}"
        );
    }
}

#[test]
fn a_draw_is_named_in_the_scene() {
    let mut input = RunInput::default();
    // X: (0,0) (2,2) (1,2) (2,0) (0,1); O: (1,1) (1,0) (0,2) (2,1).
    input.script = keys(
        "Enter Up Left Enter Right Down Enter Right Down Enter Left Up Up Enter \
         Down Down Enter Left Enter Right Right Up Up Enter Down Enter Left Left Enter",
    );
    input.path = Some("/winner".to_owned());
    let answer = scene(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.result.unwrap().value, Some("draw".into()));
}

#[test]
fn the_bots_play_to_an_end_and_their_records_replay() {
    let mut input = PlaytestInput::default();
    input.tries = Some(20);
    let answer = playtest(&tools(), &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap().remove(0);
    let bot = &out.entrants[0];
    assert_eq!(bot.tries.len(), 20);
    assert_eq!(bot.outcomes.values().sum::<u32>(), 20);
    // A plain bot run: no checks, nothing judged.
    assert_eq!(out.report.checks, Vec::new());
    let dir = std::env::temp_dir().join(format!("game-replay-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let record = bot.tries[0].record.clone().unwrap();
    fs::write(dir.join("play.ron"), &record).unwrap();
    let answer = replay(&tools(), &dir, &ReplayInput::new("play.ron"));
    assert!(answer.ok, "{answer:?}");
    let run = &answer.result.unwrap().runs[0];
    let parsed: StageRecord = ron::from_str(&record).unwrap();
    // The bots stamp their records with this build, and replay runs them.
    assert_eq!(parsed.made, Some(game::made()));
    assert_eq!(run.made, parsed.made);
    assert_eq!(run.steps.len(), parsed.commands.len() + 1);
    // The record replays to the outcome the bots saw.
    let mut board = Board::new();
    for command in &parsed.commands {
        board.apply(command).unwrap();
    }
    assert_eq!(run.outcome, board.outcome().map(|o| format!("{o:?}")));
    assert_eq!(run.outcome, out.report.entrants[0].tries[0].outcome);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_crash_record_holds_the_commands_taken_and_the_pending_one_and_replays() {
    let title = tools().harness(0).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(crash_record(title.game()), None, "no game is open");
    assert!(game::stages::crash_record().is_some());
    let mut h = on_stage(3);
    h.keys("Enter Up Enter");
    let third = Command::Place(Pos::new(0, 0));
    let screen = h.screen_mut::<StageScreen>().unwrap();
    screen.pending = Some(third);
    let taken = screen.commands.clone();
    assert_eq!(taken.len(), 2);
    let text = crash_record(h.game()).unwrap_or_else(|| panic!("no record"));
    let parsed: StageRecord = ron::from_str(&text).unwrap();
    let mut all = taken;
    all.push(third);
    assert_eq!((parsed.seed, parsed.commands), (3, all));
    // The tools' replay reads it.
    let dir = std::env::temp_dir().join(format!("game-crash-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("crash.ron"), &text).unwrap();
    let answer = replay(&tools(), &dir, &ReplayInput::new("crash.ron"));
    fs::remove_dir_all(&dir).unwrap();
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.result.unwrap().runs[0].steps.len(), 4);
}

#[test]
fn the_title_and_the_stage_declare_their_actions_and_they_fit_every_target() {
    let controls = tools()
        .controls()
        .unwrap()
        .unwrap_or_else(|e| panic!("{e}"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = Project::load(&root).unwrap_or_else(|e| panic!("{e}"));
    let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    for id in project.config.targets.keys() {
        assert_eq!(controls.fit(&profiles[id].profile), [], "{id}");
    }
    let game = game::game();
    let mut ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<&str> = game
        .every_screen(&mut ctx)
        .iter()
        .map(|s| s.name())
        .collect();
    assert_eq!(names, ["title", "stage"]);
    let mut input = CheckInput::default();
    input.target = Some("all".to_owned());
    let answer = targets(&project.config, &project.root, &input, Some(&tools()));
    assert!(answer.ok, "{answer:?}");
}
