//! A small turn game: collect a parcel, cross a timed bridge, and deliver it.

use serde::Serialize;
use shuyi_core::{SimRng, TurnRules};

const WIDTH: u8 = 7;
const HEIGHT: u8 = 5;
const BRIDGE: (u8, u8) = (3, 2);
const DEPOT: (u8, u8) = (6, 2);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// One action in the courier's turn.
pub enum Action {
    /// Move one cell north.
    Up,
    /// Move one cell south.
    Down,
    /// Move one cell west.
    Left,
    /// Move one cell east.
    Right,
    /// Stay in place for one turn.
    Wait,
}

impl Action {
    /// Parse a lowercase action word from the oracle protocol.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "wait" => Some(Self::Wait),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// A consequence of a courier action.
pub enum Event {
    /// The courier changed cells.
    Moved,
    /// The courier stayed in place.
    Waited,
    /// The parcel entered the courier's inventory.
    PickedUp,
    /// The bridge was live when crossed.
    Shocked,
    /// The parcel reached the depot.
    Delivered,
    /// Time or shield ran out.
    Exhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// The courier's terminal result.
pub enum Outcome {
    /// The parcel was delivered.
    Won,
    /// Energy or shield reached zero.
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Why a courier action was refused.
pub enum Error {
    /// The game is already over.
    Over,
    /// The destination is a wall or beyond the board.
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
/// The complete deterministic courier state.
pub struct Courier {
    /// Courier column.
    pub x: u8,
    /// Courier row.
    pub y: u8,
    /// Parcel column.
    pub parcel_x: u8,
    /// Parcel row.
    pub parcel_y: u8,
    /// Whether the courier has collected the parcel.
    pub carrying: bool,
    /// Remaining bridge shocks allowed.
    pub shield: u8,
    /// Remaining actions allowed.
    pub energy: u8,
    /// Actions accepted so far.
    pub turn: u8,
    bridge_phase: u8,
    result: Option<Outcome>,
}

impl Courier {
    /// Create a game with seed-dependent parcel position and bridge phase.
    pub fn new(seed: u64) -> Self {
        let mut rng = SimRng::new(seed);
        let parcel_y = if rng.roll_below(2) == 0 { 1 } else { 3 };
        let bridge_phase = u8::try_from(rng.roll_below(3)).unwrap_or_default();
        Self {
            x: 0,
            y: 2,
            parcel_x: 1,
            parcel_y,
            carrying: false,
            shield: 2,
            energy: 18,
            turn: 0,
            bridge_phase,
            result: None,
        }
    }

    /// Whether the bridge will shock a courier entering it this turn.
    pub fn bridge_live(&self) -> bool {
        (self.turn + self.bridge_phase) % 3 == 1
    }

    /// State the oracle reveals, excluding the private bridge phase.
    pub fn visible(&self) -> serde_json::Value {
        serde_json::json!({
            "x": self.x, "y": self.y, "parcel": [self.parcel_x, self.parcel_y],
            "carrying": self.carrying, "shield": self.shield,
            "energy": self.energy, "turn": self.turn,
            "bridge_live": self.bridge_live(), "outcome": self.result,
        })
    }

    /// Render an ASCII frame suitable for capture and human inspection.
    pub fn frame(&self) -> String {
        let mut lines = vec![format!(
            "COURIER  turn {}  energy {}  shield {}  parcel {}",
            self.turn,
            self.energy,
            self.shield,
            if self.carrying { "held" } else { "ground" }
        )];
        for y in 0..HEIGHT {
            let mut row = String::new();
            for x in 0..WIDTH {
                let tile = if (x, y) == (self.x, self.y) {
                    '@'
                } else if (x, y) == (self.parcel_x, self.parcel_y) && !self.carrying {
                    'p'
                } else if (x, y) == DEPOT {
                    'D'
                } else if (x, y) == BRIDGE {
                    if self.bridge_live() { '!' } else { '=' }
                } else if x == 3 {
                    '#'
                } else {
                    '.'
                };
                row.push(tile);
            }
            lines.push(row);
        }
        if let Some(result) = self.result {
            lines.push(format!("{result:?}"));
        }
        lines.join("\n")
    }

    fn next(&self, action: Action) -> Option<(u8, u8)> {
        let (dx, dy) = match action {
            Action::Up => (0, -1),
            Action::Down => (0, 1),
            Action::Left => (-1, 0),
            Action::Right => (1, 0),
            Action::Wait => (0, 0),
        };
        let x = i16::from(self.x) + dx;
        let y = i16::from(self.y) + dy;
        if x < 0 || x >= i16::from(WIDTH) || y < 0 || y >= i16::from(HEIGHT) {
            return None;
        }
        let x = u8::try_from(x).ok()?;
        let y = u8::try_from(y).ok()?;
        if x == 3 && y != BRIDGE.1 {
            return None;
        }
        Some((x, y))
    }
}

impl TurnRules for Courier {
    type Command = Action;
    type Event = Event;
    type Error = Error;
    type Outcome = Outcome;

    fn check(&self, action: &Action) -> Result<(), Error> {
        if self.result.is_some() {
            return Err(Error::Over);
        }
        self.next(*action).ok_or(Error::Blocked).map(|_| ())
    }

    fn apply(&mut self, action: &Action) -> Result<Vec<Event>, Error> {
        self.check(action)?;
        let was_live = self.bridge_live();
        let (x, y) = self.next(*action).ok_or(Error::Blocked)?;
        let mut events = vec![if *action == Action::Wait {
            Event::Waited
        } else {
            Event::Moved
        }];
        self.x = x;
        self.y = y;
        self.energy -= 1;
        if (x, y) == (self.parcel_x, self.parcel_y) && !self.carrying {
            self.carrying = true;
            events.push(Event::PickedUp);
        }
        if (x, y) == BRIDGE && was_live {
            self.shield -= 1;
            events.push(Event::Shocked);
        }
        self.turn += 1;
        if (x, y) == DEPOT && self.carrying {
            self.result = Some(Outcome::Won);
            events.push(Event::Delivered);
        } else if self.energy == 0 || self.shield == 0 {
            self.result = Some(Outcome::Lost);
            events.push(Event::Exhausted);
        }
        Ok(events)
    }

    fn legal_commands(&self) -> Vec<Action> {
        [
            Action::Up,
            Action::Down,
            Action::Left,
            Action::Right,
            Action::Wait,
        ]
        .into_iter()
        .filter(|action| self.check(action).is_ok())
        .collect()
    }

    fn outcome(&self) -> Option<Outcome> {
        self.result
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Courier, Error, Event, Outcome};
    use shuyi_core::TurnRules;

    fn game() -> Courier {
        Courier {
            x: 0,
            y: 2,
            parcel_x: 1,
            parcel_y: 1,
            carrying: false,
            shield: 2,
            energy: 18,
            turn: 0,
            bridge_phase: 0,
            result: None,
        }
    }

    #[test]
    fn action_words_are_the_oracle_contract() {
        for (word, action) in [
            ("up", Action::Up),
            ("down", Action::Down),
            ("left", Action::Left),
            ("right", Action::Right),
            ("wait", Action::Wait),
        ] {
            assert_eq!(Action::parse(word), Some(action));
        }
        assert_eq!(Action::parse("UP"), None);
        assert_eq!(Action::parse("jump"), None);
    }

    #[test]
    fn bridge_cycle_and_visible_state_agree() {
        let mut courier = game();
        for (turn, live) in [false, true, false, false, true, false]
            .into_iter()
            .enumerate()
        {
            courier.turn = u8::try_from(turn).expect("small turn");
            assert_eq!(courier.bridge_live(), live);
            assert_eq!(courier.visible()["bridge_live"], live);
        }
        courier.turn = 0;
        courier.bridge_phase = 2;
        assert!(!courier.bridge_live());
        courier.turn = 2;
        assert!(courier.bridge_live());
        assert_eq!(
            game().visible(),
            serde_json::json!({
                "x": 0, "y": 2, "parcel": [1, 1], "carrying": false,
                "shield": 2, "energy": 18, "turn": 0,
                "bridge_live": false, "outcome": null,
            })
        );
    }

    #[test]
    fn frame_shows_walls_bridge_parcel_depot_and_result() {
        let courier = game();
        assert_eq!(
            courier.frame(),
            "COURIER  turn 0  energy 18  shield 2  parcel ground\n...#...\n.p.#...\n@..=..D\n...#...\n...#..."
        );
        let courier = Courier {
            x: 2,
            y: 1,
            carrying: true,
            bridge_phase: 1,
            result: Some(Outcome::Won),
            ..courier
        };
        assert_eq!(
            courier.frame(),
            "COURIER  turn 0  energy 18  shield 2  parcel held\n...#...\n..@#...\n...!..D\n...#...\n...#...\nWon"
        );
    }

    #[test]
    fn boundaries_and_wall_filter_legal_actions() {
        let courier = game();
        assert_eq!(
            courier.legal_commands(),
            vec![Action::Up, Action::Down, Action::Right, Action::Wait]
        );
        assert_eq!(courier.check(&Action::Left), Err(Error::Blocked));
        assert_eq!(
            Courier {
                y: 0,
                ..courier.clone()
            }
            .check(&Action::Wait),
            Ok(())
        );
        assert_eq!(
            Courier {
                x: 2,
                y: 1,
                ..courier.clone()
            }
            .check(&Action::Right),
            Err(Error::Blocked)
        );
        assert_eq!(
            Courier {
                x: 2,
                ..courier.clone()
            }
            .check(&Action::Right),
            Ok(())
        );
        for (x, y, action) in [
            (0, 0, Action::Up),
            (0, 4, Action::Down),
            (6, 2, Action::Right),
            (8, 2, Action::Wait),
            (0, 6, Action::Wait),
        ] {
            assert_eq!(
                Courier {
                    x,
                    y,
                    ..courier.clone()
                }
                .check(&action),
                Err(Error::Blocked),
                "boundary at ({x}, {y})"
            );
        }
    }

    #[test]
    fn movement_waiting_and_pickup_change_state_once() {
        let mut courier = game();
        assert_eq!(courier.apply(&Action::Right), Ok(vec![Event::Moved]));
        assert_eq!((courier.x, courier.y), (1, 2));
        assert!(!courier.carrying);
        assert_eq!((courier.energy, courier.turn), (17, 1));
        assert_eq!(courier.apply(&Action::Wait), Ok(vec![Event::Waited]));
        assert_eq!(
            (courier.x, courier.y, courier.energy, courier.turn),
            (1, 2, 16, 2)
        );
        assert_eq!(
            courier.apply(&Action::Up),
            Ok(vec![Event::Moved, Event::PickedUp])
        );
        assert!(courier.carrying);
        assert_eq!(
            (courier.x, courier.y, courier.energy, courier.turn),
            (1, 1, 15, 3)
        );
        assert_eq!(courier.apply(&Action::Wait), Ok(vec![Event::Waited]));
        assert_eq!(courier.energy, 14);
    }

    #[test]
    fn bridge_shocks_only_when_entered_live() {
        let mut courier = Courier {
            x: 2,
            bridge_phase: 1,
            ..game()
        };
        assert_eq!(
            courier.apply(&Action::Right),
            Ok(vec![Event::Moved, Event::Shocked])
        );
        assert_eq!((courier.x, courier.y, courier.shield), (3, 2, 1));
        assert_eq!(courier.apply(&Action::Wait), Ok(vec![Event::Waited]));
        assert_eq!(courier.shield, 1);

        let mut safe_bridge = Courier { x: 2, ..game() };
        assert_eq!(safe_bridge.apply(&Action::Right), Ok(vec![Event::Moved]));
        assert_eq!(safe_bridge.shield, 2);

        let mut away = Courier {
            bridge_phase: 1,
            ..game()
        };
        assert_eq!(away.apply(&Action::Right), Ok(vec![Event::Moved]));
        assert_eq!(away.shield, 2);
    }

    #[test]
    fn exhaustion_and_delivery_are_terminal() {
        let mut shocked = Courier {
            x: 2,
            shield: 1,
            bridge_phase: 1,
            ..game()
        };
        assert_eq!(
            shocked.apply(&Action::Right),
            Ok(vec![Event::Moved, Event::Shocked, Event::Exhausted])
        );
        assert_eq!(shocked.shield, 0);
        assert_eq!(shocked.outcome(), Some(Outcome::Lost));
        assert_eq!(shocked.legal_commands(), Vec::<Action>::new());
        assert_eq!(shocked.apply(&Action::Wait), Err(Error::Over));

        let mut tired = Courier {
            energy: 1,
            ..game()
        };
        assert_eq!(
            tired.apply(&Action::Wait),
            Ok(vec![Event::Waited, Event::Exhausted])
        );
        assert_eq!((tired.energy, tired.turn), (0, 1));
        assert_eq!(tired.outcome(), Some(Outcome::Lost));

        let mut empty_handed = Courier { x: 5, ..game() };
        assert_eq!(empty_handed.apply(&Action::Right), Ok(vec![Event::Moved]));
        assert_eq!(empty_handed.outcome(), None);

        let mut delivering = Courier {
            x: 5,
            carrying: true,
            energy: 1,
            turn: 2,
            ..game()
        };
        assert_eq!(
            delivering.apply(&Action::Right),
            Ok(vec![Event::Moved, Event::Delivered])
        );
        assert_eq!((delivering.energy, delivering.turn), (0, 3));
        assert_eq!(delivering.outcome(), Some(Outcome::Won));
        assert_eq!(delivering.check(&Action::Wait), Err(Error::Over));
    }
}
