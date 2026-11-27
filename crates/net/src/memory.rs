//! The in-memory network, for tests: endpoints in one process, moved on
//! one step at a time, with seeded faults.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use shuyi_core::SimRng;

use crate::faults::{Faults, PER_MILLE};
use crate::transport::{
    Channel, DisconnectReason, NetEvent, NetStats, PeerId, SendError, Transport,
};

/// The default [`MemoryNetwork::max_message`]: 64 KiB.
const DEFAULT_MAX_MESSAGE: usize = 64 * 1024;

/// An in-memory network of endpoints, for tests. It moves on only by
/// [`step`](Self::step): one step delivers what is due, and nothing
/// happens between steps. With no [`Faults`] ([`new`](Self::new)), one
/// step delivers what was sent since the last, in the order it was sent,
/// on both channels.
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
///
/// # Faults
///
/// [`with_faults`](Self::with_faults) makes a network that loses,
/// duplicates and reorders `Unreliable` messages and delays every
/// delivery. Its draws come from one `SimRng`, at each send in a fixed
/// order: an `Unreliable` message draws its loss, its duplicate, its
/// reordering, its delay and its copy's delay, five draws whatever comes
/// up; a `Reliable` message and a disconnect draw one delay. So the same
/// seed and the same sends give the same deliveries on every target.
///
/// - **`Unreliable`** messages may be lost (counted as
///   [`NetStats::dropped`] on the sender), arrive twice (the copy with its
///   own delay; [`NetStats::duplicated`]), or be held behind the other
///   deliveries of their step, so later messages that arrive at that step
///   come first. A delay that varies reorders them on its own too.
/// - **`Reliable`** messages stay ordered and lossless, as a WebSocket
///   over TCP: each is delayed, but no earlier than the one before it on
///   the same link and direction, and never lost, duplicated or reordered.
/// - **A disconnect:** the end that closes is told at the next step; the
///   other end after the `Reliable` messages sent before the close, at a
///   drawn delay. Once an end is told, what else arrives on that link is
///   thrown away: messages still in flight from a closed peer.
/// - [`connect`](Self::connect) draws nothing: each end is told at the
///   next step.
///
/// ```
/// use shuyi_net::{Channel, Faults, MemoryNetwork, NetEvent, Transport};
///
/// let faults = Faults::none()
///     .with_seed(1)
///     .with_loss(200)
///     .with_delay_max(3);
/// let mut net = MemoryNetwork::with_faults(faults);
/// let mut server = net.endpoint();
/// let mut client = net.endpoint();
/// net.connect(server.id(), client.id());
/// net.step();
/// client.poll();
/// for n in 0..10u8 {
///     server.send(client.id(), Channel::Reliable, &[n])?;
/// }
/// // Within the most delay, every Reliable message is there, in order.
/// let mut got = Vec::new();
/// for _ in 0..3 {
///     net.step();
///     got.extend(client.poll());
/// }
/// let sent: Vec<_> = (0..10u8)
///     .map(|n| NetEvent::Message { peer: server.id(), channel: Channel::Reliable, bytes: vec![n] })
///     .collect();
/// assert_eq!(got, sent);
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
    faults: Faults,
    /// The faults' draws, from [`Faults::seed`].
    rng: SimRng,
    /// How many steps have run.
    now: u64,
    max_message: usize,
    /// Each endpoint's inbox and counts, by its id.
    ends: Vec<End>,
    /// Each linked pair, the lower id first: whether it is closed.
    links: BTreeMap<(PeerId, PeerId), bool>,
    /// Each direction of a link, sender first.
    ways: BTreeMap<(PeerId, PeerId), Way>,
    /// What is on its way, in delivery order.
    flight: BTreeMap<Slot, Item>,
    /// The next item's number: the order of the sends.
    next_seq: u64,
}

/// One endpoint's state.
#[derive(Debug, Default)]
struct End {
    inbox: Vec<NetEvent>,
    stats: NetStats,
}

/// One direction of a link.
#[derive(Debug, Default)]
struct Way {
    /// The step the last `Reliable` item on it is due, which the next is
    /// no earlier than.
    reliable_due: u64,
    /// The receiver has been told the sender is gone: what else arrives is
    /// thrown away.
    gone: bool,
}

/// When an item is delivered: by its step, then the held back last, then
/// in the order it was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Slot {
    due: u64,
    held: bool,
    seq: u64,
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
    /// An empty network with no faults: the same as
    /// [`with_faults`](Self::with_faults)`(`[`Faults::none`]`())`.
    pub fn new() -> Self {
        Self::with_faults(Faults::none())
    }

    /// An empty network that does `faults` to what is sent on it.
    pub fn with_faults(faults: Faults) -> Self {
        Self {
            shared: Rc::new(RefCell::new(Shared {
                faults,
                rng: SimRng::new(faults.seed()),
                now: 0,
                max_message: DEFAULT_MAX_MESSAGE,
                ends: Vec::new(),
                links: BTreeMap::new(),
                ways: BTreeMap::new(),
                flight: BTreeMap::new(),
                next_seq: 0,
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
    /// step, whatever the faults, before anything the other sends. One
    /// endpoint may be linked to many. A pair is linked once: linking it
    /// again, even after a disconnect, does nothing (a peer that comes
    /// back is a new endpoint).
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
        let due = shared.now + 1;
        for (from, to) in [(b, a), (a, b)] {
            shared.ways.insert(
                (from, to),
                Way {
                    reliable_due: due,
                    gone: false,
                },
            );
            shared.push(due, false, from, to, What::Connected);
        }
    }

    /// Delivers what is due at this step into the endpoints' inboxes: in
    /// the order sent, the held back last.
    pub fn step(&mut self) {
        let mut shared = self.shared.borrow_mut();
        shared.now += 1;
        let now = shared.now;
        while let Some(entry) = shared.flight.first_entry() {
            if entry.key().due > now {
                break;
            }
            let item = entry.remove();
            shared.deliver(item);
        }
    }

    /// Replaces the faults, between steps, for the sends from now on;
    /// what is on its way keeps its delays. A new seed starts the draws
    /// afresh; the same seed keeps drawing where it was.
    pub fn set_faults(&mut self, faults: Faults) {
        let mut shared = self.shared.borrow_mut();
        if faults.seed() != shared.faults.seed() {
            shared.rng = SimRng::new(faults.seed());
        }
        shared.faults = faults;
    }

    /// The faults in use.
    pub fn faults(&self) -> Faults {
        self.shared.borrow().faults
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
    /// Puts an item on its way, due at step `due`.
    fn push(&mut self, due: u64, held: bool, from: PeerId, to: PeerId, what: What) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.flight
            .insert(Slot { due, held, seq }, Item { from, to, what });
    }

    /// The step a send made now is delivered at: one draw.
    fn draw_due(&mut self) -> u64 {
        let (min, max) = self.faults.delays();
        let spread = u32::try_from(max - min + 1).unwrap_or(u32::MAX);
        self.now + min + u64::from(self.rng.roll_below(spread))
    }

    /// Whether a chance of `per_mille` comes up: one draw.
    fn chance(&mut self, per_mille: u16) -> bool {
        self.rng.roll_below(u32::from(PER_MILLE)) < u32::from(per_mille)
    }

    /// Hands `item` to its endpoint, unless the endpoint was told its
    /// sender is gone.
    fn deliver(&mut self, item: Item) {
        let Item { from, to, what } = item;
        let way = self.ways.entry((from, to)).or_default();
        if way.gone {
            return;
        }
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
            What::Closed => {
                way.gone = true;
                NetEvent::Disconnected {
                    peer: from,
                    reason: DisconnectReason::Closed,
                }
            }
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
        match channel {
            Channel::Reliable => {
                let drawn = self.draw_due();
                let way = self.ways.entry((from, to)).or_default();
                let due = drawn.max(way.reliable_due);
                way.reliable_due = due;
                self.push(due, false, from, to, What::Message(channel, bytes.to_vec()));
            }
            Channel::Unreliable => self.send_unreliable(from, to, bytes),
        }
        Ok(())
    }

    /// Sends `bytes` from `from` to `to` unreliably: always the same five
    /// draws, whatever comes up.
    fn send_unreliable(&mut self, from: PeerId, to: PeerId, bytes: &[u8]) {
        let faults = self.faults;
        let lost = self.chance(faults.loss());
        let twice = self.chance(faults.duplicate());
        let held = self.chance(faults.reorder());
        let due = self.draw_due();
        let copy_due = self.draw_due();
        let stats = &mut self.ends[index(from)].stats;
        if lost {
            stats.record_dropped();
            return;
        }
        if twice {
            stats.record_duplicated();
        }
        let message = || What::Message(Channel::Unreliable, bytes.to_vec());
        self.push(due, held, from, to, message());
        if twice {
            self.push(copy_due, held, from, to, message());
        }
    }

    /// Closes the link of `from` to `to`, if it is open.
    fn disconnect(&mut self, from: PeerId, to: PeerId) {
        match self.links.get_mut(&(from.min(to), from.max(to))) {
            Some(closed @ false) => *closed = true,
            _ => return,
        }
        // The end that closed knows at the next step; the other end after
        // the `Reliable` messages sent before the close, as a TCP close
        // follows its data.
        let drawn = self.draw_due();
        let after = self
            .ways
            .get(&(from, to))
            .map_or(drawn, |way| drawn.max(way.reliable_due));
        let now = self.now;
        self.push(now + 1, false, to, from, What::Closed);
        self.push(after, false, from, to, What::Closed);
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
