//! Shuyi Engine's networking, pure: the [`Transport`] trait a game's online
//! play talks through, and [`MemoryNetwork`], an in-memory network with
//! seeded [`Faults`] for tests.
//!
//! A transport links this end to peers, each a [`PeerId`]. It sends bytes
//! to one on a [`Channel`] (`Reliable`, ordered and lossless as a WebSocket
//! is, or `Unreliable`), and [`Transport::poll`] hands back what happened
//! since: [`NetEvent`]s of peers connected, messages and disconnects. The
//! sockets live in the backends (ADR-0155): a server's in `shuyi-server`,
//! a native client's and a web client's in `shuyi-app`. This crate has no
//! I/O, no clock, no threads and no floats, so the same sends, and the
//! same faults' seed, give the same deliveries on every target.
//!
//! ```
//! use shuyi_net::{Channel, MemoryNetwork, NetEvent, Transport};
//!
//! let mut net = MemoryNetwork::new();
//! let mut server = net.endpoint();
//! let mut client = net.endpoint();
//! net.connect(server.id(), client.id());
//! net.step();
//! assert_eq!(client.poll(), [NetEvent::Connected(server.id())]);
//! assert_eq!(server.poll(), [NetEvent::Connected(client.id())]);
//!
//! client.send(server.id(), Channel::Reliable, b"hello")?;
//! net.step();
//! assert_eq!(
//!     server.poll(),
//!     [NetEvent::Message {
//!         peer: client.id(),
//!         channel: Channel::Reliable,
//!         bytes: b"hello".to_vec(),
//!     }]
//! );
//! # Ok::<(), shuyi_net::SendError>(())
//! ```
//!
//! [`MemoryNetwork::with_faults`] puts the same code under a bad network:
//! it loses, duplicates and reorders `Unreliable` messages at chances per
//! mille, and delays every delivery by `delay_min` to `delay_max` steps.
//! `Reliable` messages are delayed but stay ordered and lossless. The same
//! seed and the same sends give the same deliveries, on every target:
//!
//! ```
//! use shuyi_net::{Channel, Faults, MemoryNetwork, Transport};
//!
//! let faults = Faults::none()
//!     .with_seed(7)
//!     .with_loss(100) // 10 % of unreliable messages lost
//!     .with_duplicate(50)
//!     .with_reorder(50)
//!     .with_delay_min(1)
//!     .with_delay_max(4); // 1 to 4 steps
//! let mut net = MemoryNetwork::with_faults(faults);
//! let mut server = net.endpoint();
//! let mut client = net.endpoint();
//! net.connect(server.id(), client.id());
//! for step in 0..100u32 {
//!     let _ = client.send(server.id(), Channel::Unreliable, &step.to_le_bytes());
//!     net.step();
//! }
//! for _ in 0..4 {
//!     net.step();
//! }
//! let lost = client.stats().dropped();
//! assert!(lost > 0);
//! assert_eq!(server.stats().messages_in() + lost, 100 + client.stats().duplicated());
//! ```
//!
//! [`room`] runs a fixed-tick game online: a [`room::Room`] on the server
//! seats players and bots, steps the real sim, and sends every step's
//! inputs, encoded by [`room::wire`], so each client steps its own copy
//! with exactly those. A game gives it its rules through
//! [`room::RoomGame`].
#![forbid(unsafe_code)]
// No floats anywhere: delivery is in whole steps, chances in per mille.
#![cfg_attr(not(test), deny(clippy::float_arithmetic))]

mod faults;
mod memory;
pub mod room;
mod transport;

pub use faults::Faults;
pub use memory::{MemoryEndpoint, MemoryNetwork};
pub use transport::{Channel, DisconnectReason, NetEvent, NetStats, PeerId, SendError, Transport};
