//! Rooms over the in-memory network: a host loop around a `Room` of
//! `Dots`, and `RoomClient`s that follow it under delays, loss,
//! duplicates and reordering, a joiner mid-round and a client whose
//! catch-up was spoiled on the way.

use std::collections::BTreeMap;

use shuyi_core::SimRng;
use shuyi_core::tick::{ActionId, SeatInput, TickSim};
use shuyi_net::room::testing::{DOWN, Dots, LEFT, OUT, RIGHT, UP};
use shuyi_net::room::wire::{ClientMsg, Frame};
use shuyi_net::room::{Advance, HASH_EVERY, Outbox, Room, RoomClient};
use shuyi_net::{Channel, Faults, MemoryEndpoint, MemoryNetwork, NetEvent, PeerId, Transport};

/// The room's sim's hash after each of its steps, by round and step.
type Hashes = BTreeMap<(u32, u32), u64>;

/// The server: a room and the loop around it.
struct Host {
    room: Room<Dots>,
    end: MemoryEndpoint,
    out: Outbox,
    hashes: Hashes,
}

impl Host {
    fn new(net: &mut MemoryNetwork, seed: u64) -> Self {
        Self {
            room: Room::new(Dots, seed),
            end: net.endpoint(),
            out: Outbox::new(),
            hashes: Hashes::new(),
        }
    }

    /// One step of the server: what peers did, the room's step, and what
    /// it wrote sent on the reliable channel.
    fn tick(&mut self) {
        for event in self.end.poll() {
            match event {
                NetEvent::Connected(peer) => {
                    let _ = self.room.join(peer);
                }
                NetEvent::Message { peer, bytes, .. } => {
                    if let Ok(msg) = ClientMsg::decode(&bytes) {
                        self.room.receive(peer, msg);
                    }
                }
                NetEvent::Disconnected { peer, .. } => self.room.leave(peer),
                _ => {}
            }
        }
        self.out.clear();
        self.room.step(&mut self.out);
        for (peer, frame) in &self.out.frames {
            // A peer gone meanwhile leaves at the next poll.
            let _ = self.end.send(*peer, Channel::Reliable, &frame.encode());
        }
        for peer in &self.out.drop {
            self.end.disconnect(*peer);
        }
        let sim = self.room.sim();
        self.hashes
            .insert((self.room.stats().round, sim.steps), sim.hash());
    }
}

/// A player: its end of the network, its client, and what it saw.
struct Player {
    end: MemoryEndpoint,
    server: PeerId,
    client: RoomClient<Dots>,
    /// Its presses.
    rng: SimRng,
    /// Whether to spoil the seed of the next catch-up it receives.
    spoil: bool,
    resyncs: u32,
    /// Each comparison of its sim with the room's, after an advance.
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
            spoil: false,
            resyncs: 0,
            equal: Vec::new(),
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        let _ = self.end.send(self.server, Channel::Reliable, bytes);
    }

    /// Hands the client each frame that arrived.
    fn receive(&mut self) {
        for event in self.end.poll() {
            let NetEvent::Message { mut bytes, .. } = event else {
                continue;
            };
            let catch_up = matches!(Frame::decode::<Dots>(&bytes), Ok(Frame::Joined { .. }));
            if catch_up && self.spoil {
                // The seed's first byte: after the tag, the place, the
                // roster's two bytes and the round.
                bytes[8] ^= 0xFF;
                self.spoil = false;
            }
            assert_eq!(self.client.on_frame(&bytes), Ok(()));
        }
    }

    /// The client's step: its advance, a resync sent, the player's input
    /// or an `Alive`, and its sim compared with the room's.
    fn advance(&mut self, hashes: &Hashes) {
        match self.client.advance() {
            Advance::Resync(bytes) => {
                self.resyncs += 1;
                self.send(&bytes);
            }
            Advance::Closed => panic!("no server closes here"),
            _ => {}
        }
        let input = self.press();
        if let Some(bytes) = self.client.input(&input) {
            self.send(&bytes);
        }
        if let Some(bytes) = self.client.alive() {
            self.send(&bytes);
        }
        if let Some(equal) = self.matches(hashes) {
            self.equal.push(equal);
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

    /// Whether the client's sim is the room's at the same step, when the
    /// room passed that step.
    fn matches(&self, hashes: &Hashes) -> Option<bool> {
        let sim = self.client.sim()?;
        let room = hashes.get(&(self.client.round(), sim.steps))?;
        Some(*room == sim.hash())
    }
}

/// A network with delays of 1 to 4 steps, and loss, duplicates and
/// reordering on: they spare the reliable channel, which the frames take.
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

/// One step of everything: the server, the network, then each player.
fn tick(host: &mut Host, net: &mut MemoryNetwork, players: &mut [Player]) {
    host.tick();
    net.step();
    for player in players {
        player.receive();
        player.advance(&host.hashes);
    }
}

#[test]
fn three_clients_keep_the_rooms_hash_over_ten_rounds_on_a_bad_network() {
    let mut net = bad_network(11);
    let mut host = Host::new(&mut net, 21);
    let server = host.end.id();
    let mut players: Vec<Player> = (0..3).map(|n| Player::new(&mut net, server, n)).collect();
    let mut steps = 0;
    while host.room.stats().round <= 10 {
        tick(&mut host, &mut net, &mut players);
        steps += 1;
        assert!(steps < 20_000, "ten rounds end well before");
    }
    // A few steps more, so the clients see the eleventh round's start.
    for _ in 0..8 {
        tick(&mut host, &mut net, &mut players);
    }
    for player in &players {
        assert_eq!(player.resyncs, 0, "every check passed");
        assert!(player.equal.iter().all(|equal| *equal));
        assert!(player.equal.len() > 500);
        assert_eq!(player.client.round(), 11);
        assert_eq!(player.client.wins(), host.room.wins());
        assert!(player.client.my_seat().is_some());
    }
}

#[test]
fn a_client_that_joins_mid_round_reaches_the_rooms_state_and_passes_its_next_check() {
    let mut net = bad_network(12);
    let mut host = Host::new(&mut net, 22);
    let server = host.end.id();
    let mut players: Vec<Player> = (0..2).map(|n| Player::new(&mut net, server, n)).collect();
    while host.room.stats().step < 2 * HASH_EVERY + 7 {
        tick(&mut host, &mut net, &mut players);
    }
    assert_eq!(host.room.stats().round, 1);
    let mut joiner = Player::new(&mut net, server, 9);
    // Until its catch-up arrives.
    while joiner.client.sim().is_none() {
        tick(&mut host, &mut net, &mut players);
        joiner.receive();
    }
    let round = joiner.client.round();
    let caught_up = joiner.client.sim().map_or(0, |sim| sim.steps);
    assert!(caught_up > 2 * HASH_EVERY);
    assert_eq!(joiner.matches(&host.hashes), Some(true));
    joiner.advance(&host.hashes);
    // Past its next check.
    let check = (caught_up / HASH_EVERY + 1) * HASH_EVERY;
    let past = |joiner: &Player| {
        joiner.client.round() > round || joiner.client.sim().is_some_and(|sim| sim.steps > check)
    };
    for _ in 0..200 {
        if past(&joiner) {
            break;
        }
        tick(&mut host, &mut net, &mut players);
        joiner.receive();
        joiner.advance(&host.hashes);
    }
    assert!(past(&joiner));
    assert_eq!(joiner.resyncs, 0);
    assert!(joiner.equal.iter().all(|equal| *equal));
    assert!(players.iter().all(|player| player.resyncs == 0));
}

#[test]
fn a_client_whose_catch_up_was_spoiled_resyncs_and_is_equal_after() {
    let mut net = bad_network(13);
    let mut host = Host::new(&mut net, 23);
    let server = host.end.id();
    let mut players: Vec<Player> = (0..3).map(|n| Player::new(&mut net, server, n)).collect();
    players[2].spoil = true;
    for _ in 0..6 * HASH_EVERY {
        tick(&mut host, &mut net, &mut players);
    }
    let [honest @ .., spoiled] = players.as_slice() else {
        unreachable!("three players");
    };
    for player in honest {
        assert_eq!(player.resyncs, 0);
        assert!(player.equal.iter().all(|equal| *equal));
    }
    // Its sim differed until the check at step 15 and the catch-up it
    // asked for; then it was the room's at every step.
    assert_eq!(spoiled.resyncs, 1);
    let differed = spoiled.equal.iter().take_while(|equal| !**equal).count();
    assert!(differed >= 10);
    let after = &spoiled.equal[differed..];
    assert!(after.len() > 3 * usize::try_from(HASH_EVERY).unwrap_or(0));
    assert!(after.iter().all(|equal| *equal));
}
