//! A resource puzzle with a deadline and a seeded gather bonus.

use serde::Serialize;
use shuyi_core::{SimRng, TurnRules};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// One action in the workshop's turn.
pub enum Action {
    /// Collect ore and coal, with occasional bonus ore.
    Gather,
    /// Spend ore and coal to make an ingot.
    Refine,
    /// Spend an ingot and heat to make a gear.
    Craft,
    /// Send a gear to fill one order.
    Deliver,
    /// Clear fatigue.
    Rest,
}

impl Action {
    /// Parse a lowercase action word from the oracle protocol.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "gather" => Some(Self::Gather),
            "refine" => Some(Self::Refine),
            "craft" => Some(Self::Craft),
            "deliver" => Some(Self::Deliver),
            "rest" => Some(Self::Rest),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// A consequence of a workshop action.
pub enum Event {
    /// Basic ore and coal were gathered.
    Gathered,
    /// A seeded cycle awarded one extra ore.
    BonusOre,
    /// One ingot was made.
    Refined,
    /// One gear was made.
    Crafted,
    /// One gear filled an order.
    Delivered,
    /// Fatigue was cleared.
    Rested,
    /// The last turn passed before both orders were filled.
    Deadline,
    /// Both orders were filled.
    Won,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// The workshop's terminal result.
pub enum Outcome {
    /// Both orders were filled.
    Won,
    /// The deadline passed.
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Why a workshop action was refused.
pub enum Error {
    /// The game is already over.
    Over,
    /// Gathering requires a rest first.
    Fatigued,
    /// Refining requires more ore.
    NoOre,
    /// Refining requires more coal.
    NoCoal,
    /// Crafting requires an ingot.
    NoIngot,
    /// Crafting requires heat.
    NoHeat,
    /// Delivery requires a gear.
    NoGear,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
/// The complete deterministic workshop state.
pub struct Workshop {
    /// Ore pieces held.
    pub ore: u8,
    /// Coal pieces held.
    pub coal: u8,
    /// Refined ingots held.
    pub ingots: u8,
    /// Crafted gears held.
    pub gears: u8,
    /// Heat available for crafting.
    pub heat: u8,
    /// Gathering fatigue, capped at three.
    pub fatigue: u8,
    /// Filled orders.
    pub orders: u8,
    /// Gather actions accepted so far.
    pub gathers: u8,
    /// Actions accepted so far.
    pub turn: u8,
    bonus_phase: u8,
    result: Option<Outcome>,
}

impl Workshop {
    /// Create a game with a seed-dependent ore bonus phase.
    pub fn new(seed: u64) -> Self {
        let mut rng = SimRng::new(seed);
        Self {
            ore: 0,
            coal: 0,
            ingots: 0,
            gears: 0,
            heat: 0,
            fatigue: 0,
            orders: 0,
            gathers: 0,
            turn: 0,
            bonus_phase: u8::try_from(rng.roll_below(3)).unwrap_or_default(),
            result: None,
        }
    }

    /// State the oracle reveals, excluding the private bonus phase.
    pub fn visible(&self) -> serde_json::Value {
        serde_json::json!({
            "ore": self.ore, "coal": self.coal, "ingots": self.ingots,
            "gears": self.gears, "heat": self.heat, "fatigue": self.fatigue,
            "orders": self.orders, "gathers": self.gathers, "turn": self.turn,
            "outcome": self.result,
        })
    }

    /// Render an ASCII frame suitable for capture and human inspection.
    pub fn frame(&self) -> String {
        let mut lines = vec![
            format!("WORKSHOP  turn {}/16  orders {}/2", self.turn, self.orders),
            format!(
                "ore {}  coal {}  ingots {}  gears {}",
                self.ore, self.coal, self.ingots, self.gears
            ),
            format!(
                "heat {}  fatigue {}  gathers {}",
                self.heat, self.fatigue, self.gathers
            ),
            "gather -> refine -> craft -> deliver; rest when tired".to_owned(),
        ];
        if let Some(result) = self.result {
            lines.push(format!("{result:?}"));
        }
        lines.join("\n")
    }
}

impl TurnRules for Workshop {
    type Command = Action;
    type Event = Event;
    type Error = Error;
    type Outcome = Outcome;

    fn check(&self, action: &Action) -> Result<(), Error> {
        if self.result.is_some() {
            return Err(Error::Over);
        }
        match action {
            Action::Gather if self.fatigue >= 3 => Err(Error::Fatigued),
            Action::Refine if self.ore < 2 => Err(Error::NoOre),
            Action::Refine if self.coal < 1 => Err(Error::NoCoal),
            Action::Craft if self.ingots < 1 => Err(Error::NoIngot),
            Action::Craft if self.heat < 1 => Err(Error::NoHeat),
            Action::Deliver if self.gears < 1 => Err(Error::NoGear),
            _ => Ok(()),
        }
    }

    fn apply(&mut self, action: &Action) -> Result<Vec<Event>, Error> {
        self.check(action)?;
        let mut events = Vec::new();
        match action {
            Action::Gather => {
                self.ore += 1;
                self.coal += 1;
                self.fatigue += 1;
                self.gathers += 1;
                events.push(Event::Gathered);
                if (self.gathers + self.bonus_phase).is_multiple_of(3) {
                    self.ore += 1;
                    events.push(Event::BonusOre);
                }
            }
            Action::Refine => {
                self.ore -= 2;
                self.coal -= 1;
                self.ingots += 1;
                self.heat += 1;
                events.push(Event::Refined);
            }
            Action::Craft => {
                self.ingots -= 1;
                self.heat -= 1;
                self.gears += 1;
                events.push(Event::Crafted);
            }
            Action::Deliver => {
                self.gears -= 1;
                self.orders += 1;
                events.push(Event::Delivered);
            }
            Action::Rest => {
                self.fatigue = 0;
                events.push(Event::Rested);
            }
        }
        self.turn += 1;
        if self.orders >= 2 {
            self.result = Some(Outcome::Won);
            events.push(Event::Won);
        } else if self.turn >= 16 {
            self.result = Some(Outcome::Lost);
            events.push(Event::Deadline);
        }
        Ok(events)
    }

    fn legal_commands(&self) -> Vec<Action> {
        [
            Action::Gather,
            Action::Refine,
            Action::Craft,
            Action::Deliver,
            Action::Rest,
        ]
        .into_iter()
        .filter(|action| self.check(action).is_ok())
        .collect()
    }

    fn outcome(&self) -> Option<Outcome> {
        self.result
    }
}
