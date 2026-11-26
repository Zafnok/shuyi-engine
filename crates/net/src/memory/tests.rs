use super::*;

/// A network with no faults, and two endpoints linked and told so.
fn linked() -> (MemoryNetwork, MemoryEndpoint, MemoryEndpoint) {
    let mut net = MemoryNetwork::new(Faults::none());
    let mut a = net.endpoint();
    let mut b = net.endpoint();
    net.connect(a.id(), b.id());
    net.step();
    a.poll();
    b.poll();
    (net, a, b)
}

#[test]
fn endpoints_are_numbered_from_zero() {
    let mut net = MemoryNetwork::new(Faults::none());
    let ids: Vec<PeerId> = (0..3).map(|_| net.endpoint().id()).collect();
    assert_eq!(ids, [PeerId(0), PeerId(1), PeerId(2)]);
}

#[test]
fn connect_gives_each_end_connected_at_the_next_step() {
    let mut net = MemoryNetwork::new(Faults::none().with_delay_min(5));
    let mut a = net.endpoint();
    let mut b = net.endpoint();
    net.connect(a.id(), b.id());
    assert_eq!(a.poll(), []);
    net.step();
    assert_eq!(a.poll(), [NetEvent::Connected(b.id())]);
    assert_eq!(b.poll(), [NetEvent::Connected(a.id())]);
    // Linked once: again does nothing.
    net.connect(b.id(), a.id());
    net.step();
    assert_eq!(a.poll(), []);
}

#[test]
fn connected_comes_before_a_message_sent_at_once() {
    let mut net = MemoryNetwork::new(Faults::none());
    let a = net.endpoint();
    let mut b = net.endpoint();
    net.connect(a.id(), b.id());
    b.send(a.id(), Channel::Unreliable, b"hi").unwrap();
    net.step();
    let mut a = a;
    assert_eq!(
        a.poll(),
        [
            NetEvent::Connected(b.id()),
            NetEvent::Message {
                peer: b.id(),
                channel: Channel::Unreliable,
                bytes: b"hi".to_vec()
            }
        ]
    );
}

#[test]
#[should_panic(expected = "not an endpoint of this network")]
fn connect_to_an_unknown_endpoint_panics() {
    let mut net = MemoryNetwork::new(Faults::none());
    let a = net.endpoint();
    net.connect(a.id(), PeerId(7));
}

#[test]
#[should_panic(expected = "not linked to itself")]
fn connect_to_itself_panics() {
    let mut net = MemoryNetwork::new(Faults::none());
    let a = net.endpoint();
    net.connect(a.id(), a.id());
}

#[test]
fn a_send_to_an_unknown_peer_is_unknown_peer() {
    let mut net = MemoryNetwork::new(Faults::none());
    let mut a = net.endpoint();
    let b = net.endpoint();
    // `b` exists but isn't linked; `PeerId(9)` doesn't exist.
    for peer in [b.id(), PeerId(9)] {
        assert_eq!(
            a.send(peer, Channel::Reliable, b"x"),
            Err(SendError::UnknownPeer(peer))
        );
    }
    assert_eq!(a.stats(), NetStats::new());
}

#[test]
fn a_send_after_a_disconnect_is_closed_from_either_end() {
    let (mut net, mut a, mut b) = linked();
    a.disconnect(b.id());
    assert_eq!(
        a.send(b.id(), Channel::Reliable, b"x"),
        Err(SendError::Closed(b.id()))
    );
    assert_eq!(
        b.send(a.id(), Channel::Unreliable, b"x"),
        Err(SendError::Closed(a.id()))
    );
    net.step();
    // Closed twice, or a peer not linked: nothing more.
    a.disconnect(b.id());
    a.disconnect(PeerId(9));
    net.step();
    assert_eq!(
        a.poll(),
        [NetEvent::Disconnected {
            peer: b.id(),
            reason: DisconnectReason::Closed
        }]
    );
    assert_eq!(
        b.poll(),
        [NetEvent::Disconnected {
            peer: a.id(),
            reason: DisconnectReason::Closed
        }]
    );
}

#[test]
fn a_message_over_the_limit_is_too_large() {
    let (mut net, mut a, b) = linked();
    assert_eq!(net.max_message(), 65_536);
    let big = vec![0; 65_537];
    assert_eq!(
        a.send(b.id(), Channel::Reliable, &big),
        Err(SendError::TooLarge {
            len: 65_537,
            max: 65_536
        })
    );
    assert_eq!(a.send(b.id(), Channel::Unreliable, &big[1..]), Ok(()));
    net.set_max_message(4);
    assert_eq!(
        a.send(b.id(), Channel::Unreliable, b"hello"),
        Err(SendError::TooLarge { len: 5, max: 4 })
    );
    assert_eq!(a.stats().messages_out(), 1);
}

#[test]
fn stats_count_bytes_and_messages_both_ways() {
    let (mut net, mut a, mut b) = linked();
    a.send(b.id(), Channel::Reliable, b"abc").unwrap();
    a.send(b.id(), Channel::Unreliable, b"de").unwrap();
    b.send(a.id(), Channel::Reliable, b"fghij").unwrap();
    net.step();
    assert_eq!(a.poll().len(), 1);
    assert_eq!(b.poll().len(), 2);
    let (sa, sb) = (a.stats(), b.stats());
    assert_eq!((sa.messages_out(), sa.bytes_out()), (2, 5));
    assert_eq!((sa.messages_in(), sa.bytes_in()), (1, 5));
    assert_eq!((sb.messages_out(), sb.bytes_out()), (1, 5));
    assert_eq!((sb.messages_in(), sb.bytes_in()), (2, 5));
    assert_eq!((sa.dropped(), sa.duplicated()), (0, 0));
}

#[test]
fn stats_count_what_the_faults_did() {
    let mut net = MemoryNetwork::new(Faults::none());
    let mut a = net.endpoint();
    let mut b = net.endpoint();
    net.connect(a.id(), b.id());
    net.set_faults(Faults::none().with_loss(1_000));
    a.send(b.id(), Channel::Unreliable, b"lost").unwrap();
    net.set_faults(Faults::none().with_duplicate(1_000));
    a.send(b.id(), Channel::Unreliable, b"twice").unwrap();
    net.step();
    assert_eq!(b.poll().len(), 3, "connected and two copies");
    let stats = a.stats();
    assert_eq!(
        (stats.messages_out(), stats.dropped(), stats.duplicated()),
        (2, 1, 1)
    );
    assert_eq!(b.stats().messages_in(), 2);
}

#[test]
fn chances_over_a_thousand_count_as_a_thousand() {
    let faults = Faults::none()
        .with_loss(5_000)
        .with_duplicate(1_001)
        .with_reorder(999);
    assert_eq!(
        (faults.loss(), faults.duplicate(), faults.reorder()),
        (1_000, 1_000, 999)
    );
    assert_eq!(Faults::default(), Faults::none());
}

#[test]
fn a_delay_of_zero_counts_as_one_and_a_max_below_the_min_as_the_min() {
    assert_eq!(Faults::none().with_delay_min(0).delays(), (1, 1));
    assert_eq!(
        Faults::none().with_delay_min(4).with_delay_max(2).delays(),
        (4, 4)
    );
    let (mut net, mut a, mut b) = linked();
    net.set_faults(Faults::none().with_delay_min(0).with_delay_max(0));
    a.send(b.id(), Channel::Reliable, b"x").unwrap();
    assert_eq!(b.poll(), [], "nothing happens between steps");
    net.step();
    assert_eq!(b.poll().len(), 1);
}

#[test]
fn set_faults_with_the_same_seed_keeps_drawing_and_a_new_seed_starts_afresh() {
    let faults = Faults::none().with_seed(3).with_delay_max(9);
    let mut net = MemoryNetwork::new(faults);
    net.shared.borrow_mut().rng.next_u32();
    let drawn = net.shared.borrow().rng.clone();
    net.set_faults(faults.with_loss(10));
    assert_eq!(net.shared.borrow().rng, drawn);
    assert_eq!(net.faults().loss(), 10);
    net.set_faults(faults.with_seed(4));
    assert_eq!(net.shared.borrow().rng, SimRng::new(4));
}

#[test]
fn steps_count_the_steps_run() {
    let mut net = MemoryNetwork::new(Faults::none());
    assert_eq!(net.steps(), 0);
    net.step();
    net.step();
    assert_eq!(net.steps(), 2);
}
