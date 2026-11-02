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
            Action::Refine if self.coal == 0 => Err(Error::NoCoal),
            Action::Craft if self.ingots == 0 => Err(Error::NoIngot),
            Action::Craft if self.heat == 0 => Err(Error::NoHeat),
            Action::Deliver if self.gears == 0 => Err(Error::NoGear),
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
                if crate::kernel::gather_bonus(self.gathers, self.bonus_phase) {
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

#[cfg(test)]
mod tests {
    use super::{Action, Error, Event, Outcome, Workshop};
    use shuyi_core::TurnRules;

    #[test]
    fn protocol_actions_and_initial_observation() {
        for (word, action) in [
            ("gather", Action::Gather),
            ("refine", Action::Refine),
            ("craft", Action::Craft),
            ("deliver", Action::Deliver),
            ("rest", Action::Rest),
        ] {
            assert_eq!(Action::parse(word), Some(action));
        }
        assert_eq!(Action::parse("Gather"), None);
        assert_eq!(Action::parse("unknown"), None);

        let workshop = Workshop::new(0);
        assert_eq!(
            workshop.visible(),
            serde_json::json!({
                "ore": 0, "coal": 0, "ingots": 0, "gears": 0,
                "heat": 0, "fatigue": 0, "orders": 0,
                "gathers": 0, "turn": 0, "outcome": null,
            })
        );
        assert_eq!(
            workshop.legal_commands(),
            vec![Action::Gather, Action::Rest]
        );
        assert_eq!(workshop.outcome(), None);
        assert_eq!(
            workshop.frame(),
            "WORKSHOP  turn 0/16  orders 0/2\n\
             ore 0  coal 0  ingots 0  gears 0\n\
             heat 0  fatigue 0  gathers 0\n\
             gather -> refine -> craft -> deliver; rest when tired"
        );
    }

    #[test]
    fn checks_require_each_resource_and_rest_before_another_gather() {
        let mut workshop = Workshop::new(0);
        assert_eq!(workshop.check(&Action::Refine), Err(Error::NoOre));
        assert_eq!(workshop.check(&Action::Craft), Err(Error::NoIngot));
        assert_eq!(workshop.check(&Action::Deliver), Err(Error::NoGear));

        workshop.ore = 1;
        workshop.coal = 2;
        assert_eq!(workshop.check(&Action::Refine), Err(Error::NoOre));
        workshop.ore = 2;
        workshop.coal = 0;
        assert_eq!(workshop.check(&Action::Refine), Err(Error::NoCoal));
        workshop.coal = 1;
        assert_eq!(workshop.check(&Action::Refine), Ok(()));

        workshop.ingots = 1;
        assert_eq!(workshop.check(&Action::Craft), Err(Error::NoHeat));
        workshop.heat = 1;
        assert_eq!(workshop.check(&Action::Craft), Ok(()));

        workshop.gears = 1;
        workshop.fatigue = 2;
        assert_eq!(
            workshop.legal_commands(),
            vec![
                Action::Gather,
                Action::Refine,
                Action::Craft,
                Action::Deliver,
                Action::Rest,
            ]
        );
        workshop.fatigue = 3;
        assert_eq!(workshop.check(&Action::Gather), Err(Error::Fatigued));
        assert_eq!(
            workshop.legal_commands(),
            vec![Action::Refine, Action::Craft, Action::Deliver, Action::Rest]
        );
        assert_eq!(workshop.apply(&Action::Rest), Ok(vec![Event::Rested]));
        assert_eq!(workshop.fatigue, 0);

        workshop.result = Some(Outcome::Lost);
        assert_eq!(workshop.legal_commands(), Vec::<Action>::new());
        assert_eq!(workshop.check(&Action::Rest), Err(Error::Over));
    }

    #[test]
    fn gathering_bonus_and_resource_flow() {
        let mut workshop = Workshop::new(0);
        workshop.bonus_phase = 1;
        workshop.ore = 2;
        workshop.coal = 2;

        assert_eq!(workshop.apply(&Action::Gather), Ok(vec![Event::Gathered]));
        assert_eq!(
            (
                workshop.ore,
                workshop.coal,
                workshop.gathers,
                workshop.fatigue
            ),
            (3, 3, 1, 1)
        );
        assert_eq!(
            workshop.apply(&Action::Gather),
            Ok(vec![Event::Gathered, Event::BonusOre])
        );
        assert_eq!(
            (
                workshop.ore,
                workshop.coal,
                workshop.gathers,
                workshop.fatigue
            ),
            (5, 4, 2, 2)
        );

        workshop.ingots = 2;
        workshop.gears = 2;
        workshop.heat = 3;
        assert_eq!(workshop.apply(&Action::Refine), Ok(vec![Event::Refined]));
        assert_eq!(
            (workshop.ore, workshop.coal, workshop.ingots, workshop.heat),
            (3, 3, 3, 4)
        );
        assert_eq!(workshop.apply(&Action::Craft), Ok(vec![Event::Crafted]));
        assert_eq!((workshop.ingots, workshop.heat, workshop.gears), (2, 3, 3));
        assert_eq!(workshop.apply(&Action::Deliver), Ok(vec![Event::Delivered]));
        assert_eq!((workshop.gears, workshop.orders, workshop.turn), (2, 1, 5));
        assert_eq!(workshop.outcome(), None);
        assert_eq!(
            workshop.visible(),
            serde_json::json!({
                "ore": 3, "coal": 3, "ingots": 2, "gears": 2,
                "heat": 3, "fatigue": 2, "orders": 1,
                "gathers": 2, "turn": 5, "outcome": null,
            })
        );
    }

    #[test]
    fn deadline_and_delivery_have_distinct_terminal_results() {
        let mut lost = Workshop::new(0);
        lost.turn = 15;
        assert_eq!(
            lost.apply(&Action::Rest),
            Ok(vec![Event::Rested, Event::Deadline])
        );
        assert_eq!(lost.outcome(), Some(Outcome::Lost));
        assert_eq!(lost.apply(&Action::Rest), Err(Error::Over));
        assert_eq!(lost.turn, 16);
        assert!(lost.frame().ends_with("Lost"));

        let mut won = Workshop::new(0);
        won.turn = 15;
        won.orders = 1;
        won.gears = 1;
        assert_eq!(
            won.apply(&Action::Deliver),
            Ok(vec![Event::Delivered, Event::Won])
        );
        assert_eq!(won.outcome(), Some(Outcome::Won));
        assert_eq!(won.turn, 16);
        assert_eq!(won.gears, 0);
        assert_eq!(won.visible()["outcome"], "Won");
        assert!(won.frame().ends_with("Won"));
    }
}
