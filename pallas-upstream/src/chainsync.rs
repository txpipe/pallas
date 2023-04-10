use gasket::error::AsWorkError;
use tracing::{debug, info};

use pallas_miniprotocols::chainsync::{HeaderContent, NextResponse, Tip};
use pallas_miniprotocols::{chainsync, Point};
use pallas_traverse::MultiEraHeader;

use crate::framework::*;

fn to_traverse(header: &chainsync::HeaderContent) -> Result<MultiEraHeader<'_>, Error> {
    let out = match header.byron_prefix {
        Some((subtag, _)) => MultiEraHeader::decode(header.variant, Some(subtag), &header.cbor),
        None => MultiEraHeader::decode(header.variant, None, &header.cbor),
    };

    out.map_err(Error::parse)
}

pub type DownstreamPort = gasket::messaging::tokio::OutputPort<ChainSyncEvent>;

pub type OuroborosClient = chainsync::N2NClient<ProtocolChannel>;

pub struct Worker<C>
where
    C: Cursor,
{
    peer_address: String,
    network_magic: u64,
    chain_cursor: C,
    client: OuroborosClient,
    downstream: DownstreamPort,
    block_count: gasket::metrics::Counter,
    chain_tip: gasket::metrics::Gauge,
}

impl<C> Worker<C>
where
    C: Cursor,
{
    pub fn new(chain_cursor: C, plexer: ProtocolChannel, downstream: DownstreamPort) -> Self {
        let client = OuroborosClient::new(plexer);

        Self {
            chain_cursor,
            client,
            downstream,
            block_count: Default::default(),
            chain_tip: Default::default(),
        }
    }

    fn notify_tip(&self, tip: Tip) {
        self.chain_tip.set(tip.0.slot_or_default() as i64);
    }

    async fn intersect(&mut self) -> Result<(), gasket::error::Error> {
        let value = self.chain_cursor.intersection();

        let intersect = match value {
            Intersection::Origin => {
                info!("intersecting origin");
                self.client.intersect_origin().await.or_restart()?.into()
            }
            Intersection::Tip => {
                info!("intersecting tip");
                self.client.intersect_tip().await.or_restart()?.into()
            }
            Intersection::Breadcrumbs(points) => {
                info!("intersecting breadcrumbs");
                let (point, tip) = self
                    .client
                    .find_intersect(Vec::from(points))
                    .await
                    .or_restart()?;

                self.notify_tip(tip);

                point
            }
        };

        info!(?intersect, "intersected");

        Ok(())
    }

    async fn process_next(
        &mut self,
        next: NextResponse<HeaderContent>,
    ) -> Result<(), gasket::error::Error> {
        match next {
            chainsync::NextResponse::RollForward(header, tip) => {
                let header = to_traverse(&header).or_panic()?;

                debug!(slot = header.slot(), hash = %header.hash(), "chain sync roll forward");

                self.downstream
                    .send(ChainSyncEvent::RollForward(header.slot(), header.hash()).into())
                    .await?;

                self.notify_tip(tip);

                Ok(())
            }
            chainsync::NextResponse::RollBackward(point, tip) => {
                match &point {
                    Point::Origin => debug!("rollback to origin"),
                    Point::Specific(slot, _) => debug!(slot, "rollback"),
                };

                self.downstream
                    .send(ChainSyncEvent::Rollback(point).into())
                    .await?;

                self.notify_tip(tip);

                Ok(())
            }
            chainsync::NextResponse::Await => {
                info!("chain-sync reached the tip of the chain");
                Ok(())
            }
        }
    }

    async fn request_next(&mut self) -> Result<(), gasket::error::Error> {
        info!("requesting next block");
        let next = self.client.request_next().await.or_restart()?;
        self.process_next(next).await
    }

    async fn await_next(&mut self) -> Result<(), gasket::error::Error> {
        info!("awaiting next block (blocking)");
        let next = self.client.recv_while_must_reply().await.or_restart()?;
        self.process_next(next).await
    }
}

impl<C> gasket::runtime::Worker for Worker<C>
where
    C: Cursor + Sync + Send,
{
    type WorkUnit = ();

    fn metrics(&self) -> gasket::metrics::Registry {
        gasket::metrics::Builder::new()
            .with_counter("received_blocks", &self.block_count)
            .with_gauge("chain_tip", &self.chain_tip)
            .build()
    }

    async fn bootstrap(&mut self) -> Result<(), gasket::error::Error> {
        debug!("connecting");
        let bearer = AsyncBearer::connect_tcp(&self.peer_address)
            .await
            .or_retry()?;

        let mut plexer = bearer.into();

        let channel0 = plexer.subscribe_client(0);
        let channel2 = plexer.subscribe_client(2);
        let channel3 = plexer.subscribe_client(3);

        let plexer_handle = tokio::spawn(plexer.run());

        let versions = handshake::n2n::VersionTable::v7_and_above(network_magic);
        let mut client = handshake::Client::new(channel0);

        let confirmation = client.handshake(versions).await.or_restart()?;

        match confirmation {
            handshake::Confirmation::Accepted(version, _) => {
                info!(version, "connected to upstream peer");
                Ok(())
            }
            _ => {
                error!("couldn't agree on handshake version");
                Err(gasket::error::Error::WorkPanic)
            }
        }

        self.intersect().await?;

        Ok(())
    }

    async fn teardown(&mut self) -> Result<(), gasket::error::Error> {
        self.plexer_handle.abort();
    }

    async fn schedule(&mut self) -> gasket::runtime::ScheduleResult<Self::WorkUnit> {
        Ok(gasket::runtime::WorkSchedule::Unit(()))
    }

    async fn execute(&mut self, unit: &Self::WorkUnit) -> Result<(), gasket::error::Error> {
        match self.client.has_agency() {
            true => self.request_next().await?,
            false => self.await_next().await?,
        }

        Ok(())
    }
}
