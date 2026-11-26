//! The in-memory network, for tests: endpoints in one process, moved on
//! one step at a time.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::transport::{
    Channel, DisconnectReason, NetEvent, NetStats, PeerId, SendError, Transport,
};

/// The default [`MemoryNetwork::max_message`]: 64 KiB.
const DEFAULT_MAX_MESSAGE: usize = 64 * 1024;

/// An in-memory network of endpoints, for tests. It moves on only by
/// [`step`](Self::step): one step delivers what was sent since the last,
/// in the order it was sent, and nothing happens between steps. Both
/// channels arrive whole, once and in order.
///
/// ```
/// use shuyi_net::{Channel, MemoryNetwork, NetEvent, Transport};
///
/// let mut net = MemoryNetwork::new();
/// let mut server = net.endpoint();
/// let mut clients: Vec<_> = (0..3).map(|_| net.endpoint()).collect();
/// for client in &clients {
///     net.connect(server.id(), client.id());
/// }
/// net.step();
/// assert_eq!(server.poll().len(), 3);
/// for client in &clients {
///     server.send(client.id(), Channel::Reliable, b"round 1")?;
/// }
/// net.step();
/// for client in &mut clients {
///     assert_eq!(
///         client.poll(),
///         [
///             NetEvent::Connected(server.id()),
///             NetEvent::Message {
///                 peer: server.id(),
///                 channel: Channel::Reliable,
///                 bytes: b"round 1".to_vec(),
///             },
///         ]
///     );
/// }
/// # Ok::<(), shuyi_net::SendError>(())
/// ```
#[derive(Debug)]
pub struct MemoryNetwork {
    shared: Rc<RefCell<Shared>>,
}

/// One end of a [`MemoryNetwork`], with its own [`PeerId`]. It implements
/// [`Transport`]. A clone is the same end: it shares the inbox.
#[derive(Debug, Clone)]
pub struct MemoryEndpoint {
    shared: Rc<RefCell<Shared>>,
    id: PeerId,
}

/// What the network and its endpoints share.
#[derive(Debug)]
struct Shared {
    /// How many steps have run.
    now: u64,
    max_message: usize,
    /// Each endpoint's inbox and counts, by its id.
    ends: Vec<End>,
    /// Each linked pair, the lower id first: whether it is closed.
    links: BTreeMap<(PeerId, PeerId), bool>,
    /// What is on its way, in the order it was sent: all of it arrives at
    /// the next step.
    flight: Vec<Item>,
}

/// One endpoint's state.
#[derive(Debug, Default)]
struct End {
    inbox: Vec<NetEvent>,
    stats: NetStats,
}

/// An item on its way to `to`, from `from`.
#[derive(Debug)]
struct Item {
    from: PeerId,
    to: PeerId,
    what: What,
}

/// What an item delivers.
#[derive(Debug)]
enum What {
    Connected,
    Message(Channel, Vec<u8>),
    Closed,
}

impl Default for MemoryNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryNetwork {
    /// An empty network.
    pub fn new() -> Self {
        Self {
            shared: Rc::new(RefCell::new(Shared {
                now: 0,
                max_message: DEFAULT_MAX_MESSAGE,
                ends: Vec::new(),
                links: BTreeMap::new(),
                flight: Vec::new(),
            })),
        }
    }

    /// A new endpoint, with the next [`PeerId`] (from 0), linked to
    /// nothing yet.
    pub fn endpoint(&mut self) -> MemoryEndpoint {
        let mut shared = self.shared.borrow_mut();
        let id = PeerId(u32::try_from(shared.ends.len()).unwrap_or(u32::MAX));
        shared.ends.push(End::default());
        MemoryEndpoint {
            shared: Rc::clone(&self.shared),
            id,
        }
    }

    /// Links endpoints `a` and `b`: each may send to the other from now
    /// on, and each sees [`NetEvent::Connected`] of the other at the next
    /// step, before anything the other sends. One endpoint may be linked
    /// to many. A pair is linked once: linking it again, even after a
    /// disconnect, does nothing (a peer that comes back is a new
    /// endpoint).
    ///
    /// # Panics
    ///
    /// If `a` or `b` isn't an endpoint of this network, or `a` is `b`.
    pub fn connect(&mut self, a: PeerId, b: PeerId) {
        let mut shared = self.shared.borrow_mut();
        let count = shared.ends.len();
        assert!(
            index(a) < count && index(b) < count,
            "{a} or {b} is not an endpoint of this network"
        );
        assert_ne!(a, b, "an endpoint is not linked to itself");
        let pair = (a.min(b), a.max(b));
        if shared.links.contains_key(&pair) {
            return;
        }
        shared.links.insert(pair, false);
        shared.push(b, a, What::Connected);
        shared.push(a, b, What::Connected);
    }

    /// Delivers what was sent since the last step into the endpoints'
    /// inboxes, in the order it was sent.
    pub fn step(&mut self) {
        let mut shared = self.shared.borrow_mut();
        shared.now += 1;
        for item in std::mem::take(&mut shared.flight) {
            shared.deliver(item);
        }
    }

    /// Sets the longest message a send takes, in bytes: a longer one is
    /// [`SendError::TooLarge`], so a test catches a message no real
    /// backend would carry. 64 KiB unless set.
    pub fn set_max_message(&mut self, bytes: usize) {
        self.shared.borrow_mut().max_message = bytes;
    }

    /// The longest message a send takes, in bytes.
    pub fn max_message(&self) -> usize {
        self.shared.borrow().max_message
    }

    /// How many steps have run.
    pub fn steps(&self) -> u64 {
        self.shared.borrow().now
    }
}

impl Shared {
    /// Puts an item on its way, due at the next step.
    fn push(&mut self, from: PeerId, to: PeerId, what: What) {
        self.flight.push(Item { from, to, what });
    }

    /// Hands `item` to its endpoint. Nothing is sent after a close, so
    /// nothing arrives after one.
    fn deliver(&mut self, item: Item) {
        let Item { from, to, what } = item;
        let event = match what {
            What::Connected => NetEvent::Connected(from),
            What::Message(channel, bytes) => {
                self.ends[index(to)].stats.record_received(bytes.len());
                NetEvent::Message {
                    peer: from,
                    channel,
                    bytes,
                }
            }
            What::Closed => NetEvent::Disconnected {
                peer: from,
                reason: DisconnectReason::Closed,
            },
        };
        self.ends[index(to)].inbox.push(event);
    }

    /// Sends `bytes` from `from` to `to` on `channel`.
    fn send(
        &mut self,
        from: PeerId,
        to: PeerId,
        channel: Channel,
        bytes: &[u8],
    ) -> Result<(), SendError> {
        match self.links.get(&(from.min(to), from.max(to))) {
            None => return Err(SendError::UnknownPeer(to)),
            Some(true) => return Err(SendError::Closed(to)),
            Some(false) => {}
        }
        if bytes.len() > self.max_message {
            return Err(SendError::TooLarge {
                len: bytes.len(),
                max: self.max_message,
            });
        }
        self.ends[index(from)].stats.record_sent(bytes.len());
        self.push(from, to, What::Message(channel, bytes.to_vec()));
        Ok(())
    }

    /// Closes the link of `from` to `to`, if it is open. Both ends are
    /// told at the next step, each after the messages sent to it before
    /// the close, as a TCP close follows its data.
    fn disconnect(&mut self, from: PeerId, to: PeerId) {
        match self.links.get_mut(&(from.min(to), from.max(to))) {
            Some(closed @ false) => *closed = true,
            _ => return,
        }
        self.push(to, from, What::Closed);
        self.push(from, to, What::Closed);
    }
}

/// `peer`'s place in [`Shared::ends`].
fn index(peer: PeerId) -> usize {
    usize::try_from(peer.0).unwrap_or(usize::MAX)
}

impl MemoryEndpoint {
    /// This end's id, as its peers see it.
    pub fn id(&self) -> PeerId {
        self.id
    }
}

impl Transport for MemoryEndpoint {
    fn send(&mut self, peer: PeerId, channel: Channel, bytes: &[u8]) -> Result<(), SendError> {
        self.shared.borrow_mut().send(self.id, peer, channel, bytes)
    }

    fn poll(&mut self) -> Vec<NetEvent> {
        std::mem::take(&mut self.shared.borrow_mut().ends[index(self.id)].inbox)
    }

    fn disconnect(&mut self, peer: PeerId) {
        self.shared.borrow_mut().disconnect(self.id, peer);
    }

    fn stats(&self) -> NetStats {
        self.shared.borrow().ends[index(self.id)].stats
    }
}

#[cfg(test)]
mod tests;
