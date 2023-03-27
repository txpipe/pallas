use std::borrow::Cow;
use std::time::Duration;

use gasket::error::AsWorkError;
use tracing::{debug, error, info};

use pallas_miniprotocols::handshake;

use crate::newplexer::MioPlexer;
use crate::{framework::*, newplexer};

struct GasketPlexerQueue {
    input: MuxInputPort,
    channel2_out: Option<DemuxOutputPort>,
    channel3_out: Option<DemuxOutputPort>,
}

impl newplexer::PlexerQueue for GasketPlexerQueue {
    fn demux_dispatch(
        &mut self,
        protocol: newplexer::Protocol,
        payload: newplexer::Payload,
    ) -> Result<(), newplexer::Error> {
        match protocol {
            2 => match &mut self.channel2_out {
                Some(output) => output
                    .send(payload.into_owned().into())
                    .map_err(|_| newplexer::Error::InvalidProtocol(2)),
                None => Err(newplexer::Error::InvalidProtocol(2)),
            },
            3 => match &mut self.channel3_out {
                Some(output) => output
                    .send(payload.into_owned().into())
                    .map_err(|_| newplexer::Error::InvalidProtocol(2)),
                None => Err(newplexer::Error::InvalidProtocol(2)),
            },
            x => Err(newplexer::Error::InvalidProtocol(x)),
        }
    }

    fn mux_peek(&mut self) -> Option<(newplexer::Protocol, newplexer::Payload)> {
        match self.input.try_recv() {
            Ok(x) => {
                let (protocol, payload) = x.payload;
                debug!(protocol, "mux request");
                Some((protocol, Cow::Owned(payload)))
            }
            Err(x) => match x {
                gasket::error::Error::RecvIdle => None,
                _ => todo!(),
            },
        }
    }

    fn mux_commit(&mut self) {
        // TODO
    }
}

pub struct Worker {
    peer_address: String,
    network_magic: u64,
    plexer: MioPlexer,
    queue: GasketPlexerQueue,
    bearer: Option<mio::Token>,
    ops_count: gasket::metrics::Counter,
}

impl Worker {
    pub fn new(
        peer_address: String,
        network_magic: u64,
        input: MuxInputPort,
        channel2_out: Option<DemuxOutputPort>,
        channel3_out: Option<DemuxOutputPort>,
    ) -> Self {
        Self {
            peer_address,
            network_magic,
            queue: GasketPlexerQueue {
                input,
                channel2_out,
                channel3_out,
            },
            bearer: None,
            plexer: MioPlexer::new(),
            ops_count: Default::default(),
        }
    }

    fn handshake(&mut self) -> Result<(), gasket::error::Error> {
        info!("executing handshake");

        let (channel_to_plexer, plexer_from_channel) = std::sync::mpsc::channel();
        let (plexer_to_channel, channel_from_plexer) = std::sync::mpsc::channel();
        let channel = newplexer::SimpleChannel(0, channel_to_plexer, channel_from_plexer);

        let mut queue = newplexer::SimplePlexerQueue::new(plexer_from_channel);
        queue.register_channel(0, plexer_to_channel);

        let versions = handshake::n2n::VersionTable::v7_and_above(self.network_magic);
        let mut client = handshake::Client::new(channel);

        let thread = std::thread::spawn(move || client.handshake(versions));

        while !thread.is_finished() {
            self.plexer
                .poll(&mut queue, Duration::from_millis(50))
                .or_panic()?;
        }

        let output = thread.join().unwrap().or_panic()?;
        debug!("handshake output: {:?}", output);

        match output {
            handshake::Confirmation::Accepted(version, _) => {
                info!(version, "connected to upstream peer");
                Ok(())
            }
            _ => {
                error!("couldn't agree on handshake version");
                Err(gasket::error::Error::WorkPanic)
            }
        }
    }
}

impl gasket::runtime::Worker for Worker {
    fn metrics(&self) -> gasket::metrics::Registry {
        // TODO: define networking metrics (bytes in / out, etc)
        gasket::metrics::Builder::new()
            .with_counter("ops_count", &self.ops_count)
            .build()
    }

    fn bootstrap(&mut self) -> Result<(), gasket::error::Error> {
        debug!("connecting muxer");

        self.bearer = self
            .plexer
            .connect_tcp_bearer(&self.peer_address)
            .or_panic()?
            .into();

        self.handshake()?;

        Ok(())
    }

    fn work(&mut self) -> gasket::runtime::WorkResult {
        self.plexer
            .poll(&mut self.queue, Duration::from_millis(50))
            .or_restart()?;

        Ok(gasket::runtime::WorkOutcome::Partial)
    }
}
