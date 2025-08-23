use std::{net::Ipv4Addr, time::Duration};

use pallas_network2::{behavior::AnyMessage, emulation, PeerId};
use rand::Rng;

fn reply_handshake_ok(
    pid: PeerId,
    msg: pallas_network2::protocol::handshake::Message<
        pallas_network2::protocol::handshake::n2n::VersionData,
    >,
    jitter: Duration,
    queue: &mut emulation::ReplyQueue<AnyMessage>,
) {
    let pallas_network2::protocol::handshake::Message::Propose(version_table) = msg else {
        queue.push_jittered_disconnect(pid, jitter);
        return;
    };

    tracing::debug!("received handshake propose");

    let (version, mut data) = version_table.values.into_iter().next().unwrap();

    data.peer_sharing = Some(1);

    let msg = pallas_network2::protocol::handshake::Message::Accept(version, data);

    tracing::debug!(version, "replying handshake propose ok");
    queue.push_jittered_msg(pid, AnyMessage::Handshake(msg), jitter);
}

fn reply_keepalive_ok(
    pid: PeerId,
    msg: pallas_network2::protocol::keepalive::Message,
    jitter: Duration,
    queue: &mut emulation::ReplyQueue<AnyMessage>,
) {
    let pallas_network2::protocol::keepalive::Message::KeepAlive(token) = msg else {
        queue.push_jittered_disconnect(pid, jitter);
        return;
    };

    tracing::debug!("received keepalive");

    if rand::thread_rng().gen_ratio(1, 10) {
        tracing::debug!("randomly dropping connection (1/10)");
        queue.push_jittered_disconnect(pid, jitter);
        return;
    }

    let msg = pallas_network2::protocol::keepalive::Message::ResponseKeepAlive(token);

    tracing::debug!(token, "replying keepalive ok");
    queue.push_jittered_msg(pid, AnyMessage::KeepAlive(msg), jitter);
}

fn reply_peer_sharing_ok(
    pid: PeerId,
    msg: pallas_network2::protocol::peersharing::Message,
    jitter: Duration,
    queue: &mut emulation::ReplyQueue<AnyMessage>,
) {
    let pallas_network2::protocol::peersharing::Message::ShareRequest(amount) = msg else {
        queue.push_jittered_disconnect(pid, jitter);
        return;
    };

    tracing::debug!(amount, "received peer sharing request");

    let msg = pallas_network2::protocol::peersharing::Message::SharePeers(vec![
        pallas_network2::protocol::peersharing::PeerAddress::V4(
            Ipv4Addr::new(123, 123, 123, 123),
            9999,
        ),
    ]);

    tracing::debug!(amount, "replying peer sharing ok");
    queue.push_jittered_msg(pid, AnyMessage::PeerSharing(msg), jitter);
}

fn reply_block_fetch_ok(
    pid: PeerId,
    msg: pallas_network2::protocol::blockfetch::Message,
    queue: &mut emulation::ReplyQueue<AnyMessage>,
) {
    match msg {
        pallas_network2::protocol::blockfetch::Message::RequestRange(_range) => {
            tracing::debug!("received block fetch request");

            let msg = pallas_network2::protocol::blockfetch::Message::StartBatch;
            queue.push_jittered_msg(
                pid.clone(),
                AnyMessage::BlockFetch(msg),
                Duration::from_secs(0),
            );

            let msg2 = pallas_network2::protocol::blockfetch::Message::Block(b"abc".to_vec());
            queue.push_jittered_msg(
                pid.clone(),
                AnyMessage::BlockFetch(msg2),
                Duration::from_secs(1),
            );

            let msg3 = pallas_network2::protocol::blockfetch::Message::BatchDone;
            queue.push_jittered_msg(
                pid.clone(),
                AnyMessage::BlockFetch(msg3),
                Duration::from_secs(2),
            );
        }
        _ => queue.push_jittered_disconnect(pid, Duration::from_secs(0)),
    }
}

#[derive(Default)]
pub struct MyEmulatorRules;

impl emulation::Rules for MyEmulatorRules {
    type Message = AnyMessage;

    #[tracing::instrument(skip(self, msg, jitter, queue))]
    fn reply_to(
        &self,
        pid: PeerId,
        msg: Self::Message,
        jitter: Duration,
        queue: &mut emulation::ReplyQueue<Self::Message>,
    ) {
        match msg {
            AnyMessage::Handshake(msg) => reply_handshake_ok(pid, msg, jitter, queue),
            AnyMessage::KeepAlive(msg) => reply_keepalive_ok(pid, msg, jitter, queue),
            AnyMessage::PeerSharing(msg) => reply_peer_sharing_ok(pid, msg, jitter, queue),
            AnyMessage::BlockFetch(msg) => reply_block_fetch_ok(pid, msg, queue),
            _ => todo!(),
        };
    }

    fn should_connect(&self, pid: pallas_network2::PeerId) -> bool {
        tracing::debug!(%pid, "connection requested");
        true
    }
}

pub type MyEmulator = emulation::Emulator<AnyMessage, MyEmulatorRules>;
