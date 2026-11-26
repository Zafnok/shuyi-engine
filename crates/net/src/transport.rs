//! The transport trait and the values it passes.

use core::fmt;

/// One end a transport is linked to: a connection, numbered by the
/// transport. A peer that drops out and comes back is a new connection
/// with a new id.
///
/// ```
/// use shuyi_net::PeerId;
///
/// let peer = PeerId(3);
/// assert_eq!(peer.to_string(), "peer 3");
/// assert!(PeerId(2) < peer);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub u32);

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "peer {}", self.0)
    }
}

/// How a message travels.
///
/// ```
/// use shuyi_net::Channel;
///
/// // Inputs and the server's steps go reliably; a hint that the next one
/// // replaces may go unreliably.
/// let channel = Channel::Reliable;
/// assert_ne!(channel, Channel::Unreliable);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Channel {
    /// Ordered and lossless, as a WebSocket over TCP: each message arrives
    /// once, after every one sent before it to the same peer.
    Reliable,
    /// May be lost, duplicated or reordered. Over a WebSocket a backend
    /// sends these reliably too; the in-memory network's faults hit only
    /// these.
    Unreliable,
}

/// What a transport saw since the last [`Transport::poll`].
///
/// ```
/// use shuyi_net::{Channel, DisconnectReason, NetEvent, PeerId};
///
/// fn describe(event: &NetEvent) -> String {
///     match event {
///         NetEvent::Connected(peer) => format!("{peer} joined"),
///         NetEvent::Message { peer, bytes, .. } => format!("{peer} sent {} bytes", bytes.len()),
///         NetEvent::Disconnected { peer, .. } => format!("{peer} left"),
///         _ => "something new".to_owned(),
///     }
/// }
/// let event = NetEvent::Message { peer: PeerId(1), channel: Channel::Reliable, bytes: vec![7] };
/// assert_eq!(describe(&event), "peer 1 sent 1 bytes");
/// let left = NetEvent::Disconnected { peer: PeerId(1), reason: DisconnectReason::Closed };
/// assert_eq!(describe(&left), "peer 1 left");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NetEvent {
    /// A peer is linked: messages to it may be sent, and its messages
    /// follow this event.
    Connected(PeerId),
    /// A message from a peer.
    Message {
        /// Who sent it.
        peer: PeerId,
        /// How it travelled.
        channel: Channel,
        /// Its bytes, as sent.
        bytes: Vec<u8>,
    },
    /// A peer is gone: nothing more comes from it, and a send to it is
    /// [`SendError::Closed`].
    Disconnected {
        /// Who is gone.
        peer: PeerId,
        /// Why.
        reason: DisconnectReason,
    },
}

/// Why a peer is gone.
///
/// ```
/// use shuyi_net::DisconnectReason;
///
/// let reason = DisconnectReason::Error("connection reset".to_owned());
/// assert_ne!(reason, DisconnectReason::Closed);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DisconnectReason {
    /// Either end asked, with [`Transport::disconnect`] or its own close.
    Closed,
    /// Nothing came from the peer for too long.
    TimedOut,
    /// The other end refused the connection.
    Refused,
    /// The backend failed, in its own words.
    Error(String),
}

/// Why a [`Transport::send`] sent nothing.
///
/// ```
/// use shuyi_net::{PeerId, SendError};
///
/// let error = SendError::TooLarge { len: 70_000, max: 65_536 };
/// assert_eq!(
///     error.to_string(),
///     "a message of 70000 bytes is over the limit of 65536 bytes"
/// );
/// assert_eq!(SendError::UnknownPeer(PeerId(9)).to_string(), "no link to peer 9");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SendError {
    /// This end has never been linked to that peer.
    UnknownPeer(PeerId),
    /// The link to that peer is closed, by either end.
    Closed(PeerId),
    /// The message is longer than the transport carries.
    TooLarge {
        /// The message's length, in bytes.
        len: usize,
        /// The most the transport carries, in bytes.
        max: usize,
    },
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPeer(peer) => write!(f, "no link to {peer}"),
            Self::Closed(peer) => write!(f, "the link to {peer} is closed"),
            Self::TooLarge { len, max } => {
                write!(
                    f,
                    "a message of {len} bytes is over the limit of {max} bytes"
                )
            }
        }
    }
}

impl core::error::Error for SendError {}

/// What a transport has carried, counted from its start.
///
/// A backend counts with the `record_*` methods; a game or a tool reads
/// the getters.
///
/// ```
/// use shuyi_net::NetStats;
///
/// let mut stats = NetStats::new();
/// stats.record_sent(12);
/// stats.record_received(36);
/// assert_eq!((stats.bytes_out(), stats.messages_out()), (12, 1));
/// assert_eq!((stats.bytes_in(), stats.messages_in()), (36, 1));
/// assert_eq!(stats.dropped(), 0);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NetStats {
    bytes_out: u64,
    bytes_in: u64,
    messages_out: u64,
    messages_in: u64,
    dropped: u64,
    duplicated: u64,
}

impl NetStats {
    /// Nothing carried yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes_out: 0,
            bytes_in: 0,
            messages_out: 0,
            messages_in: 0,
            dropped: 0,
            duplicated: 0,
        }
    }

    /// The bytes of the messages sent.
    pub const fn bytes_out(&self) -> u64 {
        self.bytes_out
    }

    /// The bytes of the messages received.
    pub const fn bytes_in(&self) -> u64 {
        self.bytes_in
    }

    /// How many messages were sent.
    pub const fn messages_out(&self) -> u64 {
        self.messages_out
    }

    /// How many messages were received, each copy of a duplicate counted.
    pub const fn messages_in(&self) -> u64 {
        self.messages_in
    }

    /// How many messages sent were lost by a fault.
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    /// How many messages sent were delivered twice by a fault.
    pub const fn duplicated(&self) -> u64 {
        self.duplicated
    }

    /// Counts one message of `bytes` bytes sent.
    pub fn record_sent(&mut self, bytes: usize) {
        self.messages_out += 1;
        self.bytes_out += widen(bytes);
    }

    /// Counts one message of `bytes` bytes received.
    pub fn record_received(&mut self, bytes: usize) {
        self.messages_in += 1;
        self.bytes_in += widen(bytes);
    }

    /// Counts one message sent and lost.
    pub fn record_dropped(&mut self) {
        self.dropped += 1;
    }

    /// Counts one message sent and delivered twice.
    pub fn record_duplicated(&mut self) {
        self.duplicated += 1;
    }
}

/// A length as a count: `usize` is at most 64 bits on every target.
fn widen(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

/// A link to peers that carries bytes: the network side of a game's
/// online play.
///
/// Nothing is `Send`: a game's transport lives on its main thread, and a
/// backend with a thread of its own keeps the thread inside.
///
/// ```
/// use shuyi_net::{Channel, NetEvent, PeerId, Transport};
///
/// /// Sends `bytes` to every peer that has connected.
/// fn broadcast(transport: &mut impl Transport, peers: &mut Vec<PeerId>, bytes: &[u8]) {
///     for event in transport.poll() {
///         match event {
///             NetEvent::Connected(peer) => peers.push(peer),
///             NetEvent::Disconnected { peer, .. } => peers.retain(|p| *p != peer),
///             _ => {}
///         }
///     }
///     for peer in peers.iter() {
///         // A peer that closed since the poll is told next poll.
///         let _ = transport.send(*peer, Channel::Reliable, bytes);
///     }
/// }
///
/// let mut net = shuyi_net::MemoryNetwork::new(shuyi_net::Faults::none());
/// let mut server = net.endpoint();
/// let mut client = net.endpoint();
/// net.connect(server.id(), client.id());
/// net.step();
/// let mut peers = Vec::new();
/// broadcast(&mut server, &mut peers, b"step 1");
/// net.step();
/// assert_eq!(client.poll().len(), 2);
/// assert_eq!(server.stats().messages_out(), 1);
/// ```
pub trait Transport {
    /// Sends `bytes` to `peer` on `channel`. Nothing is sent on an error.
    fn send(&mut self, peer: PeerId, channel: Channel, bytes: &[u8]) -> Result<(), SendError>;

    /// What happened since the last poll, oldest first.
    fn poll(&mut self) -> Vec<NetEvent>;

    /// Closes the link to `peer`: both ends get
    /// [`NetEvent::Disconnected`] with [`DisconnectReason::Closed`]. A
    /// peer not linked, or closed already, is left as it is.
    fn disconnect(&mut self, peer: PeerId);

    /// What this end has carried.
    fn stats(&self) -> NetStats;
}
