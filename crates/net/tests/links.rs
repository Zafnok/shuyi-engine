//! Links of the in-memory network: a server linked to many clients, and
//! disconnects.

use shuyi_net::{
    Channel, DisconnectReason, Faults, MemoryEndpoint, MemoryNetwork, NetEvent, PeerId, SendError,
    Transport,
};

/// A message event from `peer`.
fn message(peer: PeerId, channel: Channel, bytes: &[u8]) -> NetEvent {
    NetEvent::Message {
        peer,
        channel,
        bytes: bytes.to_vec(),
    }
}

/// A disconnect of `peer`, closed by either end.
fn closed(peer: PeerId) -> NetEvent {
    NetEvent::Disconnected {
        peer,
        reason: DisconnectReason::Closed,
    }
}

/// Steps `net` `steps` times, and what `end` received meanwhile.
fn after(net: &mut MemoryNetwork, steps: u32, end: &mut MemoryEndpoint) -> Vec<NetEvent> {
    let mut got = Vec::new();
    for _ in 0..steps {
        net.step();
        got.extend(end.poll());
    }
    got
}

#[test]
fn a_server_linked_to_three_keeps_talking_to_two_when_one_leaves() {
    let mut net = MemoryNetwork::new(Faults::none().with_seed(9).with_delay_max(3));
    let mut server = net.endpoint();
    let mut clients: Vec<MemoryEndpoint> = (0..3).map(|_| net.endpoint()).collect();
    for client in &clients {
        net.connect(server.id(), client.id());
    }
    net.step();
    let ids: Vec<PeerId> = clients.iter().map(MemoryEndpoint::id).collect();
    assert_eq!(
        server.poll(),
        ids.iter()
            .map(|id| NetEvent::Connected(*id))
            .collect::<Vec<_>>()
    );
    for client in &mut clients {
        assert_eq!(client.poll(), [NetEvent::Connected(server.id())]);
    }

    // The middle client leaves.
    clients[1].disconnect(server.id());
    for (client, id) in clients.iter_mut().zip(&ids) {
        let sent = client.send(server.id(), Channel::Reliable, b"input");
        if *id == ids[1] {
            assert_eq!(sent, Err(SendError::Closed(server.id())));
        } else {
            assert_eq!(sent, Ok(()));
        }
    }
    let mut got = after(&mut net, 3, &mut server);
    got.sort_by_key(|event| format!("{event:?}"));
    let mut want = vec![
        closed(ids[1]),
        message(ids[0], Channel::Reliable, b"input"),
        message(ids[2], Channel::Reliable, b"input"),
    ];
    want.sort_by_key(|event| format!("{event:?}"));
    assert_eq!(got, want);

    // The others keep talking, both ways.
    for id in [ids[0], ids[2]] {
        server.send(id, Channel::Reliable, b"step").unwrap();
    }
    assert_eq!(
        server.send(ids[1], Channel::Reliable, b"step"),
        Err(SendError::Closed(ids[1]))
    );
    for (client, id) in clients.iter_mut().zip(&ids) {
        let got = after(&mut net, 3, client);
        if *id == ids[1] {
            assert_eq!(got, [closed(server.id())]);
        } else {
            assert_eq!(got, [message(server.id(), Channel::Reliable, b"step")]);
        }
    }
    assert_eq!(server.stats().messages_out(), 2);
    assert_eq!(server.stats().messages_in(), 2);
}

#[test]
fn a_disconnect_follows_the_reliable_messages_sent_before_it() {
    for seed in 0..50 {
        let faults = Faults::none()
            .with_seed(seed)
            .with_loss(500)
            .with_delay_min(1)
            .with_delay_max(6);
        let mut net = MemoryNetwork::new(faults);
        let mut a = net.endpoint();
        let mut b = net.endpoint();
        net.connect(a.id(), b.id());
        net.step();
        a.poll();
        b.poll();
        for n in 0..5u8 {
            a.send(b.id(), Channel::Reliable, &[n]).unwrap();
        }
        // `b`'s message on its way to `a` when `a` closes.
        b.send(a.id(), Channel::Reliable, b"late").unwrap();
        a.disconnect(b.id());
        assert_eq!(
            a.send(b.id(), Channel::Reliable, b"x"),
            Err(SendError::Closed(b.id()))
        );

        let at_b = after(&mut net, 20, &mut b);
        let mut want: Vec<NetEvent> = (0..5u8)
            .map(|n| message(a.id(), Channel::Reliable, &[n]))
            .collect();
        want.push(closed(a.id()));
        assert_eq!(at_b, want, "seed {seed}");

        // The end that closed is told, and gets nothing after: `late`
        // arrives only if it was due before the close was.
        let at_a = a.poll();
        assert_eq!(at_a.last(), Some(&closed(b.id())), "seed {seed}");
        assert!(
            at_a.len() == 1 || at_a[0] == message(b.id(), Channel::Reliable, b"late"),
            "seed {seed}: {at_a:?}"
        );
        assert_eq!(
            b.send(a.id(), Channel::Unreliable, b"x"),
            Err(SendError::Closed(a.id()))
        );
    }
}

#[test]
fn the_end_that_closes_is_told_at_the_next_step() {
    let mut net = MemoryNetwork::new(Faults::none().with_delay_min(5));
    let mut a = net.endpoint();
    let b = net.endpoint();
    net.connect(a.id(), b.id());
    for _ in 0..5 {
        net.step();
    }
    a.poll();
    a.disconnect(b.id());
    net.step();
    assert_eq!(a.poll(), [closed(b.id())]);
}
