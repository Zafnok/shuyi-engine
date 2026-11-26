//! Shuyi Engine's networking, pure: the [`Transport`] trait a game's online
//! play talks through, and [`MemoryNetwork`], an in-memory network for
//! tests.
//!
//! A transport links this end to peers, each a [`PeerId`]. It sends bytes
//! to one on a [`Channel`] (`Reliable`, ordered and lossless as a WebSocket
//! is, or `Unreliable`), and [`Transport::poll`] hands back what happened
//! since: [`NetEvent`]s of peers connected, messages and disconnects. The
//! sockets live in the backends (ADR-0153): a server's in `shuyi-server`,
//! a native client's and a web client's in `shuyi-app`. This crate has no
//! I/O, no clock, no threads and no floats.
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
#![forbid(unsafe_code)]
// No floats anywhere: delivery is in whole steps.
#![cfg_attr(not(test), deny(clippy::float_arithmetic))]

mod memory;
mod transport;

pub use memory::{MemoryEndpoint, MemoryNetwork};
pub use transport::{Channel, DisconnectReason, NetEvent, NetStats, PeerId, SendError, Transport};
