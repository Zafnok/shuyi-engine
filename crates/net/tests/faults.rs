//! The in-memory network's faults: `Reliable` untouched, `Unreliable` hit
//! at the rates asked, delays within bounds, and the same deliveries for
//! the same seed on every target (CI runs this file on wasm32 too).

use proptest::prelude::*;
use shuyi_net::{Channel, Faults, MemoryEndpoint, MemoryNetwork, NetEvent, PeerId, Transport};

/// A message's bytes: its number and the step it was sent at.
fn stamp(number: u32, step: u64) -> Vec<u8> {
    let mut bytes = number.to_le_bytes().to_vec();
    bytes.extend_from_slice(&step.to_le_bytes());
    bytes
}

/// What [`stamp`] wrote.
fn read(bytes: &[u8]) -> (u32, u64) {
    let mut number = [0; 4];
    number.copy_from_slice(&bytes[..4]);
    let mut step = [0; 8];
    step.copy_from_slice(&bytes[4..12]);
    (u32::from_le_bytes(number), u64::from_le_bytes(step))
}

/// One message as it arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Arrival {
    channel: Channel,
    number: u32,
    sent: u64,
    arrived: u64,
}

/// A network with `faults` and two endpoints linked and told so.
fn pair(faults: Faults) -> (MemoryNetwork, MemoryEndpoint, MemoryEndpoint) {
    let mut net = MemoryNetwork::new(faults);
    let mut a = net.endpoint();
    let mut b = net.endpoint();
    net.connect(a.id(), b.id());
    net.step();
    assert_eq!(a.poll(), [NetEvent::Connected(b.id())]);
    assert_eq!(b.poll(), [NetEvent::Connected(a.id())]);
    (net, a, b)
}

/// The messages `to` holds from `from`, marked as arriving at `step`.
fn collect(to: &mut MemoryEndpoint, from: PeerId, step: u64, into: &mut Vec<Arrival>) {
    for event in to.poll() {
        let NetEvent::Message {
            peer,
            channel,
            bytes,
        } = event
        else {
            panic!("only messages: {event:?}");
        };
        assert_eq!(peer, from);
        let (number, sent) = read(&bytes);
        into.push(Arrival {
            channel,
            number,
            sent,
            arrived: step,
        });
    }
}

/// Sends `script` from `a` to `b`, one step per entry: each entry is the
/// channels of that step's messages, numbered in order. Then steps until
/// everything is delivered. What `b` received, in order.
fn run(faults: Faults, script: &[Vec<Channel>]) -> Vec<Arrival> {
    let (mut net, mut a, mut b) = pair(faults);
    let mut number = 0;
    let mut got = Vec::new();
    for channels in script {
        for &channel in channels {
            let sent = a.send(b.id(), channel, &stamp(number, net.steps()));
            assert_eq!(sent, Ok(()));
            number += 1;
        }
        net.step();
        collect(&mut b, a.id(), net.steps(), &mut got);
    }
    let total = u64::from(number);
    for _ in 0..=u64::from(faults.delay_max().max(faults.delay_min())) + total {
        net.step();
        collect(&mut b, a.id(), net.steps(), &mut got);
    }
    got
}

/// A send script: up to 30 steps of up to 6 messages, each on either
/// channel.
fn script() -> impl Strategy<Value = Vec<Vec<Channel>>> {
    let channel = prop_oneof![Just(Channel::Reliable), Just(Channel::Unreliable)];
    prop::collection::vec(prop::collection::vec(channel, 0..6), 1..30)
}

/// Faults at 500 per mille each, with delays of 1 to 12 steps.
fn heavy() -> impl Strategy<Value = Faults> {
    (any::<u64>(), 0u32..6, 0u32..6).prop_map(|(seed, min, spread)| {
        Faults::none()
            .with_seed(seed)
            .with_loss(500)
            .with_duplicate(500)
            .with_reorder(500)
            .with_delay_min(min)
            .with_delay_max(min + spread)
    })
}

/// The numbers of the messages sent on `channel`, in order.
fn sent_on(script: &[Vec<Channel>], channel: Channel) -> Vec<u32> {
    script
        .iter()
        .flatten()
        .zip(0..)
        .filter(|(c, _)| **c == channel)
        .map(|(_, n)| n)
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Over 256 seeds with loss, duplicates and reordering each at 500
    /// per mille: every `Reliable` message arrives once, in order.
    #[test]
    fn reliable_is_never_lost_duplicated_or_reordered(faults in heavy(), script in script()) {
        let got = run(faults, &script);
        let reliable: Vec<u32> = got
            .iter()
            .filter(|a| a.channel == Channel::Reliable)
            .map(|a| a.number)
            .collect();
        prop_assert_eq!(reliable, sent_on(&script, Channel::Reliable));
    }

    /// Every delivery lands between the least and the most delay after its
    /// send; a `Reliable` one later only behind the one before it.
    #[test]
    fn delays_are_within_bounds(faults in heavy(), script in script()) {
        let min = u64::from(faults.delay_min().max(1));
        let max = u64::from(faults.delay_max()).max(min);
        let mut before = 0;
        for arrival in run(faults, &script) {
            let delay = arrival.arrived - arrival.sent;
            prop_assert!(delay >= min, "{arrival:?} under {min}");
            if arrival.channel == Channel::Unreliable {
                prop_assert!(delay <= max, "{arrival:?} over {max}");
            } else {
                prop_assert!(
                    delay <= max || arrival.arrived == before,
                    "{arrival:?} over {max}, not behind one at {before}"
                );
                before = arrival.arrived;
            }
        }
    }
}

/// Measured over 10,000 `Unreliable` messages at 100 per mille each, in
/// steps of 500: the share lost of those sent, the share that arrived
/// twice and the share that arrived after a later one, of those that
/// arrived. Per mille.
fn rates(seed: u64) -> (usize, usize, usize) {
    let faults = Faults::none()
        .with_seed(seed)
        .with_loss(100)
        .with_duplicate(100)
        .with_reorder(100);
    let script = vec![vec![Channel::Unreliable; 500]; 20];
    let got = run(faults, &script);
    let mut copies = vec![0usize; 10_000];
    let mut latest = None;
    let mut reordered = 0;
    for arrival in &got {
        let n = usize::try_from(arrival.number).unwrap_or(usize::MAX);
        copies[n] += 1;
        if copies[n] == 1 {
            if latest.is_some_and(|l| l > arrival.number) {
                reordered += 1;
            }
            latest = latest.max(Some(arrival.number));
        }
    }
    let arrived = copies.iter().filter(|c| **c > 0).count();
    let twice = copies.iter().filter(|c| **c == 2).count();
    (
        (10_000 - arrived) * 1_000 / 10_000,
        twice * 1_000 / arrived,
        reordered * 1_000 / arrived,
    )
}

#[test]
fn unreliable_faults_come_at_the_rates_asked() {
    for seed in [0, 1, 0x5EED] {
        let (loss, duplicate, reorder) = rates(seed);
        for (what, rate) in [
            ("loss", loss),
            ("duplicate", duplicate),
            ("reorder", reorder),
        ] {
            assert!(
                (70..=130).contains(&rate),
                "seed {seed}: {what} at {rate} per mille"
            );
        }
    }
}

/// A run of three endpoints with every fault, a disconnect in the middle
/// and faults changed half way: every event of every endpoint, with the
/// step it was polled at.
fn session(seed: u64) -> Vec<(u64, PeerId, NetEvent)> {
    let faults = Faults::none()
        .with_seed(seed)
        .with_loss(200)
        .with_duplicate(150)
        .with_reorder(150)
        .with_delay_min(1)
        .with_delay_max(4);
    let mut net = MemoryNetwork::new(faults);
    let mut ends: Vec<MemoryEndpoint> = (0..3).map(|_| net.endpoint()).collect();
    net.connect(ends[0].id(), ends[1].id());
    net.connect(ends[0].id(), ends[2].id());
    let mut log = Vec::new();
    for step in 0..60u32 {
        if step == 30 {
            net.set_faults(faults.with_loss(0).with_delay_max(2));
        }
        if step == 40 {
            let gone = ends[2].id();
            ends[0].disconnect(gone);
        }
        for i in 1..3u8 {
            let server = ends[0].id();
            let peer = ends[usize::from(i)].id();
            let channel = if step % 3 == 0 {
                Channel::Reliable
            } else {
                Channel::Unreliable
            };
            // After the disconnect, the closed link's sends fail.
            let _ = ends[usize::from(i)].send(server, channel, &step.to_le_bytes());
            let _ = ends[0].send(peer, Channel::Reliable, &[i]);
        }
        net.step();
        for end in &mut ends {
            let id = end.id();
            log.extend(end.poll().into_iter().map(|e| (net.steps(), id, e)));
        }
    }
    log
}

/// FNV-1a, 64-bit, over `bytes`.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn the_same_seed_and_sends_give_the_same_deliveries() {
    assert_eq!(session(42), session(42));
    assert_ne!(session(42), session(43));
}

/// The deliveries of `session(42)`, pinned: the same on every target, so
/// CI's wasm32 run of this file checks delivery order there.
#[test]
fn the_deliveries_are_pinned_on_every_target() {
    let log = session(42);
    // The session holds both ends' disconnects, not only messages.
    let gone = log
        .iter()
        .filter(|(_, _, e)| matches!(e, NetEvent::Disconnected { .. }))
        .count();
    assert_eq!(gone, 2);
    let text = format!("{log:?}");
    assert_eq!(
        (log.len(), fnv1a(text.as_bytes())),
        (206, 7_821_411_635_039_160_361)
    );
}
