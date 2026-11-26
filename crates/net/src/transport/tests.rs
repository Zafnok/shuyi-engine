use super::*;

#[test]
fn a_peer_reads_as_its_number() {
    assert_eq!(PeerId(0).to_string(), "peer 0");
    assert_eq!(PeerId(41).to_string(), "peer 41");
}

#[test]
fn each_send_error_says_what_went_wrong() {
    assert_eq!(
        SendError::UnknownPeer(PeerId(2)).to_string(),
        "no link to peer 2"
    );
    assert_eq!(
        SendError::Closed(PeerId(5)).to_string(),
        "the link to peer 5 is closed"
    );
    assert_eq!(
        SendError::TooLarge { len: 9, max: 4 }.to_string(),
        "a message of 9 bytes is over the limit of 4 bytes"
    );
}

#[test]
fn stats_start_at_nothing() {
    let stats = NetStats::new();
    assert_eq!(stats, NetStats::default());
    assert_eq!(
        [
            stats.bytes_out(),
            stats.bytes_in(),
            stats.messages_out(),
            stats.messages_in(),
            stats.dropped(),
            stats.duplicated(),
        ],
        [0; 6]
    );
}

#[test]
fn each_record_adds_to_its_own_counts_alone() {
    let mut stats = NetStats::new();
    stats.record_sent(3);
    stats.record_sent(4);
    assert_eq!((stats.messages_out(), stats.bytes_out()), (2, 7));
    stats.record_received(5);
    stats.record_received(6);
    assert_eq!((stats.messages_in(), stats.bytes_in()), (2, 11));
    stats.record_dropped();
    stats.record_dropped();
    stats.record_dropped();
    stats.record_duplicated();
    stats.record_duplicated();
    assert_eq!((stats.dropped(), stats.duplicated()), (3, 2));
    // Nothing leaked into another count.
    assert_eq!((stats.messages_out(), stats.bytes_out()), (2, 7));
    assert_eq!((stats.messages_in(), stats.bytes_in()), (2, 11));
}
