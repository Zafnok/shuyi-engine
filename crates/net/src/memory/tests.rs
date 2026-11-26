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
