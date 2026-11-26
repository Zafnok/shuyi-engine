use super::*;

/// A network with two endpoints linked and told so.
fn linked() -> (MemoryNetwork, MemoryEndpoint, MemoryEndpoint) {
    let mut net = MemoryNetwork::new();
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
    let mut net = MemoryNetwork::new();
    let ids: Vec<PeerId> = (0..3).map(|_| net.endpoint().id()).collect();
    assert_eq!(ids, [PeerId(0), PeerId(1), PeerId(2)]);
}

#[test]
fn connect_gives_each_end_connected_at_the_next_step() {
    let mut net = MemoryNetwork::new();
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
    let mut net = MemoryNetwork::new();
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
    let mut net = MemoryNetwork::new();
    let a = net.endpoint();
    net.connect(a.id(), PeerId(7));
}

#[test]
#[should_panic(expected = "not linked to itself")]
fn connect_to_itself_panics() {
    let mut net = MemoryNetwork::new();
    let a = net.endpoint();
    net.connect(a.id(), a.id());
}

#[test]
fn a_send_to_an_unknown_peer_is_unknown_peer() {
    let mut net = MemoryNetwork::new();
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
fn nothing_happens_between_steps() {
    let (mut net, mut a, mut b) = linked();
    a.send(b.id(), Channel::Reliable, b"x").unwrap();
    assert_eq!(b.poll(), []);
    net.step();
    assert_eq!(b.poll().len(), 1);
    // A step with nothing sent delivers nothing.
    net.step();
    assert_eq!(b.poll(), []);
}

#[test]
fn messages_arrive_in_the_order_sent_on_both_channels() {
    let (mut net, mut a, mut b) = linked();
    let channels = [Channel::Unreliable, Channel::Reliable, Channel::Unreliable];
    for (n, channel) in (0u8..).zip(channels) {
        a.send(b.id(), channel, &[n]).unwrap();
    }
    net.step();
    let got: Vec<NetEvent> = (0u8..)
        .zip(channels)
        .map(|(n, channel)| NetEvent::Message {
            peer: a.id(),
            channel,
            bytes: vec![n],
        })
        .collect();
    assert_eq!(b.poll(), got);
}

#[test]
fn a_clone_of_an_endpoint_shares_its_inbox() {
    let (mut net, mut a, b) = linked();
    let mut twin = b.clone();
    a.send(b.id(), Channel::Reliable, b"x").unwrap();
    net.step();
    assert_eq!(twin.poll().len(), 1);
    assert_eq!(twin.stats(), b.stats());
    assert_eq!(MemoryNetwork::default().steps(), 0);
}

#[test]
fn steps_count_the_steps_run() {
    let mut net = MemoryNetwork::new();
    assert_eq!(net.steps(), 0);
    net.step();
    net.step();
    assert_eq!(net.steps(), 2);
}

#[test]
fn new_is_with_faults_none() {
    assert_eq!(MemoryNetwork::new().faults(), Faults::none());
    assert_eq!(Faults::default(), Faults::none());
    assert_eq!(Faults::none().delays(), (1, 1));
}

#[test]
fn stats_count_what_the_faults_did_at_chances_of_a_thousand() {
    let (mut net, mut a, mut b) = linked();
    net.set_faults(Faults::none().with_loss(1_000));
    a.send(b.id(), Channel::Unreliable, b"lost").unwrap();
    // Faults hit Unreliable messages only.
    a.send(b.id(), Channel::Reliable, b"sure").unwrap();
    net.set_faults(Faults::none().with_duplicate(1_000));
    a.send(b.id(), Channel::Unreliable, b"twice").unwrap();
    net.step();
    let twice = NetEvent::Message {
        peer: a.id(),
        channel: Channel::Unreliable,
        bytes: b"twice".to_vec(),
    };
    let sure = NetEvent::Message {
        peer: a.id(),
        channel: Channel::Reliable,
        bytes: b"sure".to_vec(),
    };
    assert_eq!(b.poll(), [sure, twice.clone(), twice]);
    let stats = a.stats();
    assert_eq!(
        (stats.messages_out(), stats.dropped(), stats.duplicated()),
        (3, 1, 1)
    );
    assert_eq!(b.stats().messages_in(), 3);
}

#[test]
fn a_reorder_of_a_thousand_holds_a_message_behind_its_step() {
    let (mut net, mut a, mut b) = linked();
    net.set_faults(Faults::none().with_reorder(1_000));
    a.send(b.id(), Channel::Unreliable, b"held").unwrap();
    a.send(b.id(), Channel::Reliable, b"first").unwrap();
    net.step();
    let bytes: Vec<Vec<u8>> = b
        .poll()
        .into_iter()
        .map(|event| match event {
            NetEvent::Message { bytes, .. } => bytes,
            other => panic!("only messages: {other:?}"),
        })
        .collect();
    assert_eq!(bytes, [b"first".to_vec(), b"held".to_vec()]);
}

#[test]
fn chances_over_a_thousand_count_as_a_thousand() {
    let faults = Faults::none()
        .with_loss(5_000)
        .with_duplicate(1_001)
        .with_reorder(1_000);
    assert_eq!(
        (faults.loss(), faults.duplicate(), faults.reorder()),
        (1_000, 1_000, 1_000)
    );
    assert_eq!(Faults::none().with_loss(999).loss(), 999);
}

#[test]
fn a_delay_of_zero_counts_as_one() {
    let zero = Faults::none().with_delay_min(0).with_delay_max(0);
    assert_eq!((zero.delay_min(), zero.delay_max()), (0, 0), "kept as set");
    assert_eq!(zero.delays(), (1, 1));
    let (mut net, mut a, mut b) = linked();
    net.set_faults(zero);
    a.send(b.id(), Channel::Reliable, b"x").unwrap();
    a.send(b.id(), Channel::Unreliable, b"y").unwrap();
    assert_eq!(b.poll(), [], "nothing happens between steps");
    net.step();
    assert_eq!(b.poll().len(), 2);
}

#[test]
fn a_delay_max_below_the_min_counts_as_the_min() {
    let faults = Faults::none().with_delay_min(4).with_delay_max(2);
    assert_eq!(
        (faults.delay_min(), faults.delay_max()),
        (4, 2),
        "kept as set"
    );
    assert_eq!(faults.delays(), (4, 4));
    assert_eq!(Faults::none().with_delay_max(3).delays(), (1, 3));
    let (mut net, mut a, mut b) = linked();
    net.set_faults(faults);
    a.send(b.id(), Channel::Unreliable, b"x").unwrap();
    for _ in 0..3 {
        net.step();
        assert_eq!(b.poll(), []);
    }
    net.step();
    assert_eq!(b.poll().len(), 1, "at four steps, not two");
}

#[test]
fn set_faults_with_the_same_seed_keeps_drawing_and_a_new_seed_starts_afresh() {
    let faults = Faults::none().with_seed(3).with_delay_max(9);
    let mut net = MemoryNetwork::with_faults(faults);
    assert_eq!(net.shared.borrow().rng, SimRng::new(3));
    net.shared.borrow_mut().rng.next_u32();
    let drawn = net.shared.borrow().rng.clone();
    assert_ne!(drawn, SimRng::new(3));
    net.set_faults(faults.with_loss(10));
    assert_eq!(net.shared.borrow().rng, drawn, "the same seed keeps going");
    assert_eq!(net.faults(), faults.with_loss(10));
    net.set_faults(faults.with_seed(4));
    assert_eq!(net.shared.borrow().rng, SimRng::new(4), "a new seed afresh");
}

#[test]
fn an_unreliable_send_draws_five_and_a_reliable_one_draws_one() {
    let faults = Faults::none().with_seed(11);
    let mut net = MemoryNetwork::with_faults(faults);
    let mut a = net.endpoint();
    let b = net.endpoint();
    net.connect(a.id(), b.id());
    let mut want = SimRng::new(11);
    assert_eq!(net.shared.borrow().rng, want, "connect draws nothing");
    a.send(b.id(), Channel::Unreliable, b"x").unwrap();
    for _ in 0..5 {
        want.next_u32();
    }
    assert_eq!(net.shared.borrow().rng, want);
    a.send(b.id(), Channel::Reliable, b"x").unwrap();
    want.next_u32();
    assert_eq!(net.shared.borrow().rng, want);
    a.disconnect(b.id());
    want.next_u32();
    assert_eq!(net.shared.borrow().rng, want, "a disconnect draws one");
}

#[test]
fn a_chance_of_zero_never_comes_up() {
    // 5,000 sends draw 15,000 chances: a draw of 0 comes up about 15
    // times, and counts for nothing at a chance of 0.
    let (mut net, mut a, b) = linked();
    net.set_faults(Faults::none().with_seed(5).with_delay_max(4));
    for _ in 0..5_000 {
        a.send(b.id(), Channel::Unreliable, b"x").unwrap();
    }
    assert_eq!((a.stats().dropped(), a.stats().duplicated()), (0, 0));
}

#[test]
fn a_step_delivers_in_the_order_things_happened_across_links() {
    let (mut net, mut a, mut b) = linked();
    let c = net.endpoint();
    b.send(a.id(), Channel::Reliable, b"first").unwrap();
    net.connect(a.id(), c.id());
    net.step();
    assert_eq!(
        a.poll(),
        [
            NetEvent::Message {
                peer: b.id(),
                channel: Channel::Reliable,
                bytes: b"first".to_vec(),
            },
            NetEvent::Connected(c.id()),
        ]
    );
    assert_eq!(b.poll(), []);
}
