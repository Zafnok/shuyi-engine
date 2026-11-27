# shuyi-net

Online play for fixed-tick games, without the sockets. A game's online
code talks through the `Transport` trait: it sends bytes to a peer on a
reliable channel (ordered and lossless, as a WebSocket is) or an
unreliable one, and polls for peers connected, messages and disconnects.
`MemoryNetwork` is a whole network in memory, for tests: endpoints in one
process, moved on one step at a time, so a test of online code needs no
socket and runs the same on every target, wasm32 included. Its `Faults`
lose, duplicate and reorder unreliable messages and delay every delivery,
drawn from one seed, so a test of online code under a bad network gives
the same deliveries in every run. It is the base of the rooms
of a game server, where the server broadcasts each step's inputs and
every client steps the same rules
([ADR-0155](../../docs/adr/0155-real-time-rooms-a-server-broadcasts-each-steps-inputs.md)).
It serves fixed-tick games. Its `room` module is that room's server
side: a `Room` seats players in a game's seats, rides the empty ones with
the game's bot, steps the real sim and writes each step's inputs for
every player, and `room::wire` is their bytes. The client that follows a
room (2621), the quick-play director (2615) and the server (2616) are
planned; lockstep, rollback and a referee are parked (1140–1150).

## What it is not for

- **Sockets.** The crate is pure: no I/O, no clock, no threads and no
  floats. A server's sockets are `shuyi-server`'s (2616), a native
  client's `shuyi-app`'s behind its `net` feature (2617), and the web's
  our own JS plugin's (1142).
- **A turn game played by two peers.** That is lockstep (1140), parked.
- **Watching by choice, names and chat.** A room's only watchers are
  players waiting for the next round's seat, and it carries no text
  from players (ADR-0155 §6).
- **Sharing a transport between threads.** Nothing here is `Send`: a
  game's transport lives on its main thread.

## What it costs

- **Size**, measured on 2026-11-26 with Hello, Grid making a
  `MemoryNetwork` with faults, sending on both channels and
  disconnecting, release builds: **+29,301 bytes** of `.wasm` (3,928,020
  → 3,957,321), of which the faults are 18,064 (the same use of the
  fault-free network of 2613: 3,939,257). `MemoryNetwork::new()` builds
  the faults too: it is `with_faults(Faults::none())`. A game that
  doesn't turn the front door's `net` feature on builds none of it: no
  default feature reaches the crate.
- **Dependencies:** `shuyi-core`, for the faults' `SimRng`, which every
  game has already; nothing outside the engine.

- **The rooms** cost a game nothing until it builds one: `Room` is
  generic over the game's `RoomGame`, and runs on the server. The
  client's size in a game is measured with the client (2621).
  `room::testing::Dots`, a test game, is behind the `testing` feature,
  off by default.

## How to start

Turn it on with the `shuyi` crate's `net` feature, or depend on
`shuyi-net` directly. Then test a game's online code on the in-memory
network:

```rust
use shuyi::net::{Channel, MemoryNetwork, NetEvent, Transport};

let mut net = MemoryNetwork::new();
let mut server = net.endpoint();
let mut client = net.endpoint();
net.connect(server.id(), client.id());
net.step();
assert_eq!(client.poll(), [NetEvent::Connected(server.id())]);

server
    .send(client.id(), Channel::Reliable, b"step 1")
    .expect("the client is linked");
net.step();
assert_eq!(
    client.poll(),
    [NetEvent::Message {
        peer: server.id(),
        channel: Channel::Reliable,
        bytes: b"step 1".to_vec(),
    }]
);
```

Then put it under a bad network: `MemoryNetwork::with_faults` loses,
duplicates and reorders `Unreliable` messages at chances per mille, and
delays every delivery by `delay_min` to `delay_max` steps. `Reliable`
messages are delayed but stay ordered and lossless, as over a WebSocket.
The same seed and the same sends give the same deliveries, on every
target:

```rust
use shuyi::net::{Channel, Faults, MemoryNetwork, Transport};

let faults = Faults::none()
    .with_seed(7)
    .with_loss(100) // 10 % of unreliable messages lost
    .with_duplicate(50)
    .with_reorder(50)
    .with_delay_min(1)
    .with_delay_max(4); // 1 to 4 steps
let mut net = MemoryNetwork::with_faults(faults);
let mut server = net.endpoint();
let mut client = net.endpoint();
net.connect(server.id(), client.id());
for step in 0..100u32 {
    let _ = client.send(server.id(), Channel::Unreliable, &step.to_le_bytes());
    net.step();
}
for _ in 0..4 {
    net.step();
}
let lost = client.stats().dropped();
assert!(lost > 0);
assert_eq!(server.stats().messages_in() + lost, 100 + client.stats().duplicated());
```

`set_faults` changes them between steps: a new seed starts the draws
afresh, the same seed keeps drawing where it was.

Code written against `impl Transport` runs unchanged over a real
backend. `stats()` counts each end's bytes and messages both ways, and
the messages the faults lost (`dropped`) and sent twice (`duplicated`).

To put a tick game in a room, implement `room::RoomGame` for it: its
`TickSim`, its seats (at most 8), its steps a second, its longest round
and its pause between rounds, and its bot, a pure function of the sim
and the room's rng. A host steps the room at the game's rate and sends
each frame it writes to its peer on the `Reliable` channel:

```rust
use shuyi_net::PeerId;
use shuyi_net::room::testing::Dots; // the `testing` feature's game
use shuyi_net::room::wire::{ClientMsg, Frame};
use shuyi_net::room::{Joined, Outbox, Room};

let mut room = Room::new(Dots, 7);
assert_eq!(room.join(PeerId(1)), Ok(Joined::Seat(0)));
// What a peer sends, read with `ClientMsg::decode`.
room.receive(PeerId(1), ClientMsg::Alive);
let mut out = Outbox::new();
room.step(&mut out);
for (peer, frame) in &out.frames {
    let bytes = frame.encode(); // sent to `peer`
    assert_eq!(Frame::decode::<Dots>(&bytes).as_ref(), Ok(frame));
    assert_eq!(*peer, PeerId(1));
}
```

A peer the room lets go, silent for five seconds of steps, is in
`out.drop`, for the host to close.

## Beside its alternatives

The other answer to *Online play* is none, the default. A game moves to
online play by keeping its rules deterministic, which every Shuyi game
already does, and sending inputs through a `Transport`; it writes no
state codec. Moving back is turning the feature off. Lockstep, rollback
and a referee (parked) will be other sync models over the same
transport.
