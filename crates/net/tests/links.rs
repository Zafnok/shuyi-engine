//! Links of the in-memory network: a server linked to many clients, and
//! disconnects, with no faults and under delays and loss.

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
    let mut net = MemoryNetwork::new();
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
fn a_disconnect_follows_the_messages_sent_before_it_on_both_ends() {
    let mut net = MemoryNetwork::new();
    let mut a = net.endpoint();
    let mut b = net.endpoint();
    net.connect(a.id(), b.id());
    net.step();
    a.poll();
    b.poll();
    for n in 0..5u8 {
        a.send(b.id(), Channel::Reliable, &[n]).unwrap();
    }
    // `b`'s message, on its way to `a` when `a` closes.
    b.send(a.id(), Channel::Unreliable, b"late").unwrap();
    a.disconnect(b.id());
    assert_eq!(
        a.send(b.id(), Channel::Reliable, b"x"),
        Err(SendError::Closed(b.id()))
    );
    assert_eq!(
        b.send(a.id(), Channel::Unreliable, b"x"),
        Err(SendError::Closed(a.id()))
    );
    assert_eq!(a.poll(), [], "told at the next step");
    net.step();
    let mut want: Vec<NetEvent> = (0..5u8)
        .map(|n| message(a.id(), Channel::Reliable, &[n]))
        .collect();
    want.push(closed(a.id()));
    assert_eq!(b.poll(), want);
    assert_eq!(
        a.poll(),
        [
            message(b.id(), Channel::Unreliable, b"late"),
            closed(b.id())
        ]
    );
    // Closed once: nothing more, however long.
    a.disconnect(b.id());
    b.disconnect(a.id());
    assert_eq!(after(&mut net, 5, &mut b), []);
    assert_eq!(a.poll(), []);
}

#[test]
fn a_disconnect_under_delays_and_loss_follows_the_reliable_messages_on_both_ends() {
    for seed in 0..200 {
        let faults = Faults::none()
            .with_seed(seed)
            .with_loss(500)
            .with_duplicate(300)
            .with_reorder(300)
            .with_delay_min(1)
            .with_delay_max(6);
        let mut net = MemoryNetwork::with_faults(faults);
        let mut a = net.endpoint();
        let mut b = net.endpoint();
        net.connect(a.id(), b.id());
        net.step();
        a.poll();
        b.poll();
        for n in 0..5u8 {
            a.send(b.id(), Channel::Reliable, &[n]).unwrap();
            a.send(b.id(), Channel::Unreliable, &[100 + n]).unwrap();
        }
        // Messages of `b`'s on their way to `a` when `a` closes.
        b.send(a.id(), Channel::Reliable, b"late").unwrap();
        b.send(a.id(), Channel::Unreliable, b"later").unwrap();
        a.disconnect(b.id());
        assert_eq!(
            a.send(b.id(), Channel::Reliable, b"x"),
            Err(SendError::Closed(b.id()))
        );
        assert_eq!(
            b.send(a.id(), Channel::Unreliable, b"x"),
            Err(SendError::Closed(a.id()))
        );

        // The end that closed is told at the next step, and gets nothing
        // after: `b`'s messages arrive only if they were due by then.
        net.step();
        let at_a = a.poll();
        assert_eq!(at_a.last(), Some(&closed(b.id())), "seed {seed}");
        assert!(
            at_a[..at_a.len() - 1].iter().all(|event| matches!(
                event,
                NetEvent::Message { peer, .. } if *peer == b.id()
            )),
            "seed {seed}: {at_a:?}"
        );

        // The other end gets every Reliable message, in order, then the
        // close, then nothing: the Unreliable ones in flight after it are
        // thrown away.
        let mut at_b = b.poll();
        at_b.extend(after(&mut net, 20, &mut b));
        let reliable: Vec<NetEvent> = at_b
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    NetEvent::Message {
                        channel: Channel::Reliable,
                        ..
                    }
                )
            })
            .cloned()
            .collect();
        let want: Vec<NetEvent> = (0..5u8)
            .map(|n| message(a.id(), Channel::Reliable, &[n]))
            .collect();
        assert_eq!(reliable, want, "seed {seed}");
        assert_eq!(at_b.last(), Some(&closed(a.id())), "seed {seed}: {at_b:?}");
        let closes = at_b
            .iter()
            .filter(|event| matches!(event, NetEvent::Disconnected { .. }))
            .count();
        assert_eq!(closes, 1, "seed {seed}");
        assert_eq!(a.poll(), [], "seed {seed}");
    }
}

#[test]
fn the_end_that_closes_is_told_at_the_next_step_however_long_the_delays() {
    let mut net = MemoryNetwork::with_faults(Faults::none().with_delay_min(5));
    let mut a = net.endpoint();
    let mut b = net.endpoint();
    net.connect(a.id(), b.id());
    net.step();
    assert_eq!(
        a.poll(),
        [NetEvent::Connected(b.id())],
        "connect draws no delay"
    );
    b.poll();
    a.disconnect(b.id());
    net.step();
    assert_eq!(a.poll(), [closed(b.id())]);
    // The other end at five steps after the close.
    assert_eq!(b.poll(), []);
    assert_eq!(after(&mut net, 3, &mut b), []);
    net.step();
    assert_eq!(b.poll(), [closed(a.id())]);
}
