use std::future::ready;

use byteorder::{ByteOrder, NetworkEndian};
use gasket::error::AsWorkError;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::select;
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

use pallas_miniprotocols::handshake;

use crate::framework::*;

impl AsyncPlexer {
    pub async fn run(&mut self) -> tokio::io::Result<()> {
        loop {
            select! {
                Ok(_) = self.bearer.readable() => {
                    if let Ok(true) = self.bearer.has_segment().await {
                        trace!("demux selected");
                        self.demux().await?
                    }
                },
                Some(x) = self.ingress.1.recv() => {
                    trace!("mux selected");
                    self.mux(x).await?
                },
            }
        }
    }
}

pub struct Worker {
    peer_address: String,
    network_magic: u64,
    bearer: Option<AsyncBearer>,
    mux_input: MuxInputPort,
    channel2_out: Option<DemuxOutputPort>,
    channel3_out: Option<DemuxOutputPort>,
    ops_count: gasket::metrics::Counter,
}

impl Worker {
    pub fn new(
        peer_address: String,
        network_magic: u64,
        mux_input: MuxInputPort,
        channel2_out: Option<DemuxOutputPort>,
        channel3_out: Option<DemuxOutputPort>,
    ) -> Self {
        Self {
            peer_address,
            network_magic,
            channel2_out,
            channel3_out,
            mux_input,
            bearer: None,
            ops_count: Default::default(),
        }
    }
}

pub enum WorkUnit {
    Mux((u16, Vec<u8>)),
    Demux,
}

impl gasket::runtime::Worker for Worker {
    type WorkUnit = WorkUnit;

    fn metrics(&self) -> gasket::metrics::Registry {
        // TODO: define networking metrics (bytes in / out, etc)
        gasket::metrics::Builder::new()
            .with_counter("ops_count", &self.ops_count)
            .build()
    }

    async fn bootstrap(&mut self) -> Result<(), gasket::error::Error> {
        debug!("connecting");
        let bearer = AsyncBearer::connect_tcp(&self.peer_address)
            .await
            .or_retry()?;

        let mut plexer = bearer.into();

        handshake(&mut plexer, self.network_magic).await?;

        self.bearer = Some(plexer.into());

        Ok(())
    }

    async fn schedule(&mut self) -> gasket::runtime::ScheduleResult<Self::WorkUnit> {
        let bearer = self.bearer.as_mut().unwrap();
        trace!("selecting");
        select! {
            Ok(msg) = self.mux_input.recv() => { Ok(gasket::runtime::WorkSchedule::Unit(WorkUnit::Mux(msg.payload))) }
            x = bearer.has_segment() => match x {
                Ok(_) =>  Ok(gasket::runtime::WorkSchedule::Unit(WorkUnit::Demux)),
                Err(err) => {
                    warn!(?err, "bearer error");
                    Err(gasket::error::Error::ShouldRestart)
                },
            },
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => Ok(gasket::runtime::WorkSchedule::Idle),
        }
    }

    async fn execute(&mut self, unit: &Self::WorkUnit) -> Result<(), gasket::error::Error> {
        match unit {
            WorkUnit::Mux(x) => {
                trace!("muxing");
                self.bearer
                    .as_mut()
                    .unwrap()
                    .write_segment(x.0, &x.1)
                    .await
                    .or_restart()?;
            }
            WorkUnit::Demux => {
                trace!("demuxing");

                let (protocol, payload) = self
                    .bearer
                    .as_mut()
                    .unwrap()
                    .read_segment()
                    .await
                    .or_restart()?;

                match protocol {
                    2 => {
                        if let Some(channel) = &mut self.channel2_out {
                            channel.send(payload.into()).await?;
                            trace!("sent protocol 2 msg");
                        }
                    }
                    3 => {
                        if let Some(channel) = &mut self.channel3_out {
                            channel.send(payload.into()).await?;
                            trace!("sent protocol 3 msg");
                        }
                    }
                    x => warn!("trying to demux unexpected protocol {x}"),
                }
            }
        };

        Ok(())
    }
}
