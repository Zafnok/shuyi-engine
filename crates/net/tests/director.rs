//! Quick play over the in-memory network: a server loop around a
//! `Director` of `Dots` rooms, 80 `RoomClient`s that greet it, play and
//! follow their rooms on a bad network, and 100,000 seeded operations of
//! churn with the director's invariants checked after each.

use std::collections::{BTreeMap, BTreeSet};

use shuyi_core::tick::{ActionId, SeatInput, TickSim};
use shuyi_core::{ContentHash, Made, SimRng};
use shuyi_net::director::{Director, DirectorConfig, RoomId};
use shuyi_net::room::testing::{DOWN, Dots, LEFT, OUT, RIGHT, UP};
use shuyi_net::room::wire::{ClientMsg, Frame, Refusal};
use shuyi_net::room::{Advance, Outbox, RoomClient, RoomGame, Seat};
use shuyi_net::{Channel, Faults, MemoryEndpoint, MemoryNetwork, NetEvent, PeerId, Transport};

/// Each room's sim's hash after each of its steps, by room, round and
/// step.
type Hashes = BTreeMap<(RoomId, u32, u32), u64>;

fn made() -> Made {
    Made::new("dots", ContentHash(0))
}

/// A network with delays of 1 to 4 steps, and loss, duplicates and
/// reordering of unreliable messages.
fn bad_network(seed: u64) -> MemoryNetwork {
    let faults = Faults::none()
        .with_seed(seed)
        .with_loss(100)
        .with_duplicate(50)
        .with_reorder(50)
        .with_delay_min(1)
        .with_delay_max(4);
    MemoryNetwork::with_faults(faults)
}

/// The server: a director and the loop around it.
struct Server {
    director: Director<Dots>,
    end: MemoryEndpoint,
    out: Outbox,
    /// The peers its end is linked to.
    linked: BTreeSet<PeerId>,
    hashes: Hashes,
}

impl Server {
    fn new(net: &mut MemoryNetwork, config: DirectorConfig, seed: u64) -> Self {
        Self {
            director: Director::new(config, Dots, made(), seed),
            end: net.endpoint(),
            out: Outbox::new(),
            linked: BTreeSet::new(),
            hashes: Hashes::new(),
        }
    }

    /// One step of the server: what its end saw handed to the director,
    /// the director's step, its frames sent and its peers closed.
    fn tick(&mut self) {
        for event in self.end.poll() {
            match &event {
                NetEvent::Connected(peer) => {
                    self.linked.insert(*peer);
                }
                NetEvent::Disconnected { peer, .. } => {
                    self.linked.remove(peer);
                }
                _ => {}
            }
            self.director.on_event(event);
        }
        self.out.clear();
        self.director.step(&mut self.out);
        for (peer, frame) in &self.out.frames {
            let _ = self.end.send(*peer, Channel::Reliable, &frame.encode());
        }
        for peer in &self.out.drop {
            self.end.disconnect(*peer);
        }
    }

    /// Each room's hash at the step it reached.
    fn record(&mut self) {
        for (id, room) in self.director.rooms() {
            let sim = room.sim();
            let round = room.stats().round;
            self.hashes.insert((id, round, sim.steps), sim.hash());
        }
    }
}

/// A player: its end, its client, and what it saw.
struct Player {
    end: MemoryEndpoint,
    server: PeerId,
    client: RoomClient<Dots>,
    rng: SimRng,
    refused: Vec<Refusal>,
    resyncs: u32,
    /// Each comparison of its sim with its room's, after an advance.
    equal: Vec<bool>,
}

impl Player {
    fn new(net: &mut MemoryNetwork, server: PeerId, seed: u64) -> Self {
        let end = net.endpoint();
        net.connect(server, end.id());
        Self {
            end,
            server,
            client: RoomClient::new(Dots),
            rng: SimRng::new(seed),
            refused: Vec::new(),
            resyncs: 0,
            equal: Vec::new(),
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        let _ = self.end.send(self.server, Channel::Reliable, bytes);
    }

    /// Greets the server once linked, asks to play once welcomed, and
    /// hands the client every other frame.
    fn receive(&mut self) {
        for event in self.end.poll() {
            let bytes = match event {
                NetEvent::Connected(_) => {
                    self.send(&ClientMsg::hello(made()).encode());
                    continue;
                }
                NetEvent::Message { bytes, .. } => bytes,
                _ => continue,
            };
            match Frame::decode::<Dots>(&bytes) {
                Ok(Frame::Welcome) => self.send(&ClientMsg::Play.encode()),
                Ok(Frame::Refused { why }) => self.refused.push(why),
                _ => assert_eq!(self.client.on_frame(&bytes), Ok(())),
            }
        }
    }

    /// The client's step: its advance, a resync sent, the player's input
    /// or an `Alive`, and its sim compared with its room's.
    fn advance(&mut self, server: &Server) {
        if let Advance::Resync(bytes) = self.client.advance() {
            self.resyncs += 1;
            self.send(&bytes);
        }
        let input = self.press();
        if let Some(bytes) = self.client.input(&input) {
            self.send(&bytes);
        }
        if let Some(bytes) = self.client.alive() {
            self.send(&bytes);
        }
        let room = server.director.room_of(self.end.id());
        if let (Some(room), Some(sim)) = (room, self.client.sim())
            && let Some(hash) = server.hashes.get(&(room, self.client.round(), sim.steps))
        {
            self.equal.push(*hash == sim.hash());
        }
    }

    /// A turn now and then, and going out now and then.
    fn press(&mut self) -> SeatInput {
        let mut pressed = Vec::new();
        if self.rng.roll_below(8) == 0 {
            let turns: [ActionId; 4] = [UP, RIGHT, DOWN, LEFT];
            let turn = usize::try_from(self.rng.roll_below(4)).unwrap_or(0);
            pressed.push(turns[turn]);
        }
        if self.rng.roll_below(60) == 0 {
            pressed.push(OUT);
        }
        SeatInput::new(pressed, Vec::new(), Vec::new())
    }
}

#[test]
fn eighty_clients_follow_their_ten_rooms_and_match_their_hashes_on_a_bad_network() {
    let mut net = bad_network(31);
    let mut server = Server::new(&mut net, DirectorConfig::new(), 41);
    let id = server.end.id();
    let mut players: Vec<Player> = (0..80).map(|n| Player::new(&mut net, id, n)).collect();
    for _ in 0..400 {
        server.tick();
        server.record();
        net.step();
        for player in &mut players {
            player.receive();
            player.advance(&server);
        }
    }
    let stats = server.director.stats();
    assert_eq!((stats.rooms, stats.humans + stats.watchers), (10, 80));
    let mut per_room = BTreeMap::<RoomId, usize>::new();
    for player in &players {
        assert_eq!(player.refused, [] as [shuyi_net::room::wire::Refusal; 0]);
        assert_eq!(player.resyncs, 0, "every check passed");
        assert!(player.equal.len() > 300, "{}", player.equal.len());
        assert!(player.equal.iter().all(|equal| *equal));
        let room = server.director.room_of(player.end.id());
        *per_room
            .entry(room.unwrap_or(RoomId(u16::MAX)))
            .or_default() += 1;
    }
    assert_eq!(per_room.len(), 10);
    assert!(per_room.values().all(|n| *n == Dots::SEATS));
}

/// The most clients the churn keeps linked, in its busy stretches: more
/// than three rooms hold, so some are told so.
const MOST_CLIENTS: usize = 40;
/// The most in its quiet stretches, so rooms empty and close.
const FEW_CLIENTS: usize = 6;
/// The operations of each stretch.
const STRETCH: u32 = 10_000;

/// The churn's own clients, as the test drives them.
struct Churn {
    net: MemoryNetwork,
    server: Server,
    clients: Vec<MemoryEndpoint>,
    rng: SimRng,
    config: DirectorConfig,
    /// The operations run.
    ops: u32,
}

impl Churn {
    fn new(seed: u64) -> Self {
        let config = DirectorConfig::new()
            .with_rooms_max(3)
            .with_rooms_warm(1)
            .with_idle_close_s(2)
            .with_hello_timeout_s(2);
        let mut net = bad_network(seed);
        let server = Server::new(&mut net, config, seed);
        Self {
            net,
            server,
            clients: Vec::new(),
            rng: SimRng::new(seed),
            config,
            ops: 0,
        }
    }

    /// Sends `msg` from a client the rng picks, on a channel it picks.
    fn say(&mut self, msg: &ClientMsg) {
        let Some(client) = self.pick() else {
            return;
        };
        let channel = if self.rng.roll_below(4) == 0 {
            Channel::Unreliable
        } else {
            Channel::Reliable
        };
        let server = self.server.end.id();
        let _ = self.clients[client].send(server, channel, &msg.encode());
    }

    fn pick(&mut self) -> Option<usize> {
        let len = u32::try_from(self.clients.len()).ok().filter(|n| *n > 0)?;
        usize::try_from(self.rng.roll_below(len)).ok()
    }

    /// One operation the rng picks.
    fn operate(&mut self) {
        let server = self.server.end.id();
        self.ops += 1;
        let most = if (self.ops / STRETCH).is_multiple_of(2) {
            MOST_CLIENTS
        } else {
            FEW_CLIENTS
        };
        match self.rng.roll_below(100) {
            // At the most clients, one goes instead.
            0..10 if self.clients.len() >= most => self.disconnect(),
            0..10 => {
                // A client connects and, mostly, greets the server first:
                // now and then with another build, a `Play` first, or
                // nothing.
                let mut end = self.net.endpoint();
                self.net.connect(server, end.id());
                let first = match self.rng.roll_below(20) {
                    0 => Some(ClientMsg::hello(Made::new("another build", ContentHash(1)))),
                    1 => Some(ClientMsg::Play),
                    2 => None,
                    _ => Some(ClientMsg::hello(made())),
                };
                if let Some(first) = first {
                    let _ = end.send(server, Channel::Reliable, &first.encode());
                }
                self.clients.push(end);
            }
            10..14 => self.say(&ClientMsg::hello(made())),

            14..38 => self.say(&ClientMsg::Play),
            38..41 => self.say(&ClientMsg::Leave),
            41..44 => self.disconnect(),
            44..45 => {
                let budget = self.config.msgs_per_second();
                if let Some(client) = self.pick() {
                    for _ in 0..=budget {
                        let _ = self.clients[client].send(
                            server,
                            Channel::Reliable,
                            &ClientMsg::Alive.encode(),
                        );
                    }
                }
            }
            45..52 => {
                // Every client still there says it is.
                for end in &mut self.clients {
                    let _ = end.send(server, Channel::Reliable, &ClientMsg::Alive.encode());
                }
            }
            52..60 => self.say(&ClientMsg::Alive),
            60..68 => {
                let input = SeatInput::new(
                    vec![ActionId(self.rng.roll_below(5).try_into().unwrap_or(0))],
                    Vec::new(),
                    Vec::new(),
                );
                self.say(&ClientMsg::Input(input));
            }
            _ => self.step(),
        }
    }

    /// A client the rng picks closes its link.
    fn disconnect(&mut self) {
        if let Some(client) = self.pick() {
            let mut end = self.clients.swap_remove(client);
            end.disconnect(self.server.end.id());
        }
    }

    /// The server's step, the network's, and every client reading what
    /// came; a client its server closed is dropped.
    fn step(&mut self) {
        self.server.tick();
        self.net.step();
        self.clients.retain_mut(|end| {
            end.poll()
                .iter()
                .all(|event| !matches!(event, NetEvent::Disconnected { .. }))
        });
    }

    /// The director's invariants.
    fn check(&self) {
        let d = &self.server.director;
        let stats = d.stats();
        let rooms: Vec<_> = d.rooms().collect();
        assert!(rooms.len() <= usize::from(self.config.rooms_max()));
        assert!(rooms.len() >= usize::from(self.config.rooms_warm()));
        let (mut humans, mut bots, mut watchers) = (0, 0, 0);
        for (id, room) in &rooms {
            let room_stats = room.stats();
            assert_eq!(room_stats.humans + room_stats.bots, Dots::SEATS);
            humans += room_stats.humans;
            bots += room_stats.bots;
            watchers += room_stats.watchers;
            for seat in room.seats() {
                if let Seat::Human(peer) = seat {
                    assert_eq!(d.room_of(*peer), Some(*id), "{peer} sits in {id}");
                }
            }
        }
        assert_eq!(
            (stats.rooms, stats.humans, stats.bots, stats.watchers),
            (rooms.len(), humans, bots, watchers)
        );
        // Each peer the director places is in that room, and the rooms
        // hold no one else: so no peer is in two.
        let mut placed = 0;
        for peer in &self.server.linked {
            if let Some(id) = d.room_of(*peer) {
                let place = d.room(id).and_then(|room| room.place(*peer));
                assert!(place.is_some(), "{peer} is in {id}");
                placed += 1;
            }
        }
        assert_eq!(placed, humans + watchers);
        assert!(stats.peers <= self.server.linked.len());
    }
}

#[test]
fn a_hundred_thousand_seeded_operations_keep_the_directors_invariants() {
    let mut churn = Churn::new(2615);
    let mut most = 0;
    let mut seen = BTreeSet::new();
    for _ in 0..100_000 {
        churn.operate();
        churn.check();
        most = most.max(churn.server.director.stats().rooms);
        seen.extend(churn.server.director.rooms().map(|(id, _)| id));
    }
    let refused = churn.server.director.stats().refused;
    // The churn reached every rule: the cap, rooms closed and opened
    // anew, and every refusal.
    assert_eq!(most, 3);
    assert!(seen.len() > 6, "{seen:?}");
    assert!(refused.busy > 0 && refused.other_build > 0);
    assert!(refused.protocol > 0 && refused.flood > 0);
}
