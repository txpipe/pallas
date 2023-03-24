use gasket::messaging::SendAdapter;
use tracing::{error, info, instrument};

use pallas_crypto::hash::Hash;
use pallas_miniprotocols::blockfetch;
use pallas_miniprotocols::Point;

use crate::framework::*;

pub type UpstreamPort = gasket::messaging::crossbeam::TwoPhaseInputPort<ChainSyncEvent>;
pub type OuroborosClient = blockfetch::Client<ProtocolChannel>;

pub struct Worker<T>
where
    T: Send + Sync,
{
    client: OuroborosClient,
    upstream: UpstreamPort,
    downstream: DownstreamPort<T>,
    block_count: gasket::metrics::Counter,
}

impl<T> Worker<T>
where
    T: Send + Sync,
{
    pub fn new(
        plexer: ProtocolChannel,
        upstream: UpstreamPort,
        downstream: DownstreamPort<T>,
    ) -> Self {
        let client = OuroborosClient::new(plexer);

        Self {
            client,
            upstream,
            downstream,
            block_count: Default::default(),
        }
    }

    #[instrument(skip(self), fields(slot, %hash))]
    fn fetch_block(&mut self, slot: u64, hash: Hash<32>) -> Result<Vec<u8>, gasket::error::Error> {
        info!("fetching block");

        match self
            .client
            .fetch_single(Point::Specific(slot, hash.to_vec()))
        {
            Ok(x) => {
                info!("block fetch succeeded");
                Ok(x)
            }
            Err(blockfetch::Error::ChannelError(x)) => {
                error!("plexer channel error: {}", x);
                Err(gasket::error::Error::RetryableError)
            }
            Err(x) => {
                error!("unrecoverable block fetch error: {}", x);
                Err(gasket::error::Error::WorkPanic)
            }
        }
    }
}

impl<A> gasket::runtime::Worker for Worker<A>
where
    A: SendAdapter<BlockFetchEvent>,
{
    fn metrics(&self) -> gasket::metrics::Registry {
        gasket::metrics::Builder::new()
            .with_counter("fetched_blocks", &self.block_count)
            .build()
    }

    fn work(&mut self) -> gasket::runtime::WorkResult {
        let msg = self.upstream.recv_or_idle()?;

        let msg = match msg.payload {
            ChainSyncEvent::RollForward(s, h) => {
                let body = self.fetch_block(s, h)?;
                self.block_count.inc(1);
                BlockFetchEvent::RollForward(s, h, body)
            }
            ChainSyncEvent::Rollback(x) => BlockFetchEvent::Rollback(x),
        };

        self.downstream.send(msg.into())?;

        // remove the processed event from the queue
        self.upstream.commit();

        Ok(gasket::runtime::WorkOutcome::Partial)
    }
}
