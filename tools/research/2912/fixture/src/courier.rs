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
