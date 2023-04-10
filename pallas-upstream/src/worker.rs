use gasket::error::AsWorkError;
use tracing::{debug, info};

use pallas_network::facades::PeerClient;
use pallas_network::miniprotocols::chainsync::{self, HeaderContent, NextResponse, Tip};
use pallas_network::miniprotocols::Point;
use pallas_traverse::MultiEraHeader;

use crate::framework::*;

fn to_traverse(header: &HeaderContent) -> Result<MultiEraHeader<'_>, gasket::error::Error> {
    let out = match header.byron_prefix {
        Some((subtag, _)) => MultiEraHeader::decode(header.variant, Some(subtag), &header.cbor),
        None => MultiEraHeader::decode(header.variant, None, &header.cbor),
    };

    out.or_panic()
}

pub type DownstreamPort = gasket::messaging::tokio::OutputPort<UpstreamEvent>;

pub struct Worker<C>
where
    C: Cursor,
{
    peer_address: String,
    network_magic: u64,
    chain_cursor: C,
    peer_session: Option<PeerClient>,
    downstream: DownstreamPort,
    block_count: gasket::metrics::Counter,
    chain_tip: gasket::metrics::Gauge,
}

impl<C> Worker<C>
where
    C: Cursor,
{
    pub fn new(
        peer_address: String,
        network_magic: u64,
        chain_cursor: C,
        downstream: DownstreamPort,
    ) -> Self {
        Self {
            peer_address,
            network_magic,
            chain_cursor,
            downstream,
            peer_session: None,
            block_count: Default::default(),
            chain_tip: Default::default(),
        }
    }

    fn notify_tip(&self, tip: &Tip) {
        self.chain_tip.set(tip.0.slot_or_default() as i64);
    }

    async fn intersect(&mut self) -> Result<(), gasket::error::Error> {
        let value = self.chain_cursor.intersection();

        let chainsync = self.peer_session.as_mut().unwrap().chainsync();

        let intersect = match value {
            Intersection::Origin => {
                info!("intersecting origin");
                chainsync.intersect_origin().await.or_restart()?.into()
            }
            Intersection::Tip => {
                info!("intersecting tip");
                chainsync.intersect_tip().await.or_restart()?.into()
            }
            Intersection::Breadcrumbs(points) => {
                info!("intersecting breadcrumbs");
                let (point, tip) = chainsync.find_intersect(points).await.or_restart()?;

                self.notify_tip(&tip);

                point
            }
        };

        info!(?intersect, "intersected");

        Ok(())
    }

    async fn process_next(
        &mut self,
        next: &NextResponse<HeaderContent>,
    ) -> Result<(), gasket::error::Error> {
        match next {
            NextResponse::RollForward(header, tip) => {
                let header = to_traverse(header).or_panic()?;
                let slot = header.slot();
                let hash = header.hash();

                debug!(slot, %hash, "chain sync roll forward");

                let block = self
                    .peer_session
                    .as_mut()
                    .unwrap()
                    .blockfetch()
                    .fetch_single(pallas_network::miniprotocols::Point::Specific(
                        slot,
                        hash.to_vec(),
                    ))
                    .await
                    .or_retry()?;

                self.downstream
                    .send(UpstreamEvent::RollForward(slot, hash, block).into())
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
                    .send(UpstreamEvent::Rollback(point.clone()).into())
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
}

#[async_trait::async_trait]
impl<C> gasket::runtime::Worker for Worker<C>
where
    C: Cursor + Sync + Send,
{
    type WorkUnit = NextResponse<HeaderContent>;

    fn metrics(&self) -> gasket::metrics::Registry {
        gasket::metrics::Builder::new()
            .with_counter("received_blocks", &self.block_count)
            .with_gauge("chain_tip", &self.chain_tip)
            .build()
    }

    async fn bootstrap(&mut self) -> Result<(), gasket::error::Error> {
        debug!("connecting");

        let peer = PeerClient::connect(&self.peer_address, self.network_magic)
            .await
            .or_restart()?;

        self.peer_session = Some(peer);

        self.intersect().await?;

        Ok(())
    }

    async fn teardown(&mut self) -> Result<(), gasket::error::Error> {
        self.peer_session.as_mut().unwrap().abort();

        Ok(())
    }

    async fn schedule(&mut self) -> gasket::runtime::ScheduleResult<Self::WorkUnit> {
        let client = self.peer_session.as_mut().unwrap().chainsync();

        let next = match client.has_agency() {
            true => {
                info!("requesting next block");
                client.request_next().await.or_restart()?
            }
            false => {
                info!("awaiting next block (blocking)");
                client.recv_while_must_reply().await.or_restart()?
            }
        };

        Ok(gasket::runtime::WorkSchedule::Unit(next))
    }

    async fn execute(&mut self, unit: &Self::WorkUnit) -> Result<(), gasket::error::Error> {
        self.process_next(unit).await
    }
}
