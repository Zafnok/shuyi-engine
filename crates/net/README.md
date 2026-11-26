# shuyi-net

Online play for fixed-tick games, without the sockets. A game's online
code talks through the `Transport` trait: it sends bytes to a peer on a
reliable channel (ordered and lossless, as a WebSocket is) or an
unreliable one, and polls for peers connected, messages and disconnects.
`MemoryNetwork` is a whole network in memory, for tests: endpoints in one
process, moved on one step at a time, so a test of online code needs no
socket and runs the same on every target, wasm32 included (seeded loss,
duplicates, reordering and delay are 2620's). It is the base of the rooms
of a game server, where the server broadcasts each step's inputs and
every client steps the same rules
([ADR-0152](../../docs/adr/0152-real-time-rooms-a-server-broadcasts-each-steps-inputs.md)).
It serves fixed-tick games. The rooms (2614), the quick-play director
(2615) and the server (2616) are planned; lockstep, rollback and a
referee are parked (1140–1150).

## What it is not for

- **Sockets.** The crate is pure: no I/O, no clock, no threads and no
  floats. A server's sockets are `shuyi-server`'s (2616), a native
  client's `shuyi-app`'s behind its `net` feature (2617), and the web's
  our own JS plugin's (1142).
- **Rooms, bots and the wire's frames.** Those are 2614's, on this crate.
- **A turn game played by two peers.** That is lockstep (1140), parked.
- **Sharing a transport between threads.** Nothing here is `Send`: a
  game's transport lives on its main thread.

## What it costs

- **Size**, measured on 2026-11-26 with Hello, Grid making a
  `MemoryNetwork`, sending on both channels and disconnecting, release
  builds: **+21,159 bytes** of `.wasm` (3,915,937 → 3,937,096). A game
  that doesn't turn the front door's `net` feature on builds none of it:
  no default feature reaches the crate.
- **Dependencies:** none, of the engine or outside it.

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

Code written against `impl Transport` runs unchanged over a real
backend. `stats()` counts each end's bytes and messages both ways.

## Beside its alternatives

The other answer to *Online play* is none, the default. A game moves to
online play by keeping its rules deterministic, which every Shuyi game
already does, and sending inputs through a `Transport`; it writes no
state codec. Moving back is turning the feature off. Lockstep, rollback
and a referee (parked) will be other sync models over the same
transport.
