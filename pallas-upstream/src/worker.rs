use std::marker::PhantomData;

use gasket::framework::*;
use gasket::messaging::*;
use tracing::{debug, info};

use pallas_network::facades::PeerClient;
use pallas_network::miniprotocols::chainsync::{self, HeaderContent, NextResponse};
use pallas_network::miniprotocols::Point;
use pallas_traverse::MultiEraHeader;

use crate::framework::*;

fn to_traverse(header: &HeaderContent) -> Result<MultiEraHeader<'_>, WorkerError> {
    let out = match header.byron_prefix {
        Some((subtag, _)) => MultiEraHeader::decode(header.variant, Some(subtag), &header.cbor),
        None => MultiEraHeader::decode(header.variant, None, &header.cbor),
    };

    out.or_panic()
}

pub type DownstreamPort<A> = gasket::messaging::OutputPort<A, UpstreamEvent>;

async fn intersect(peer: &mut PeerClient, intersection: Intersection) -> Result<(), WorkerError> {
    let chainsync = peer.chainsync();

    let intersect = match intersection {
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
            let (point, _) = chainsync.find_intersect(points).await.or_restart()?;
            point
        }
    };

    info!(?intersect, "intersected");

    Ok(())
}

pub struct Worker<C, A>
where
    C: Cursor,
    A: SendAdapter<UpstreamEvent>,
{
    peer_session: PeerClient,
    _panthom_c: PhantomData<C>,
    _panthom_a: PhantomData<A>,
}

impl<C, A> Worker<C, A>
where
    C: Cursor,
    A: SendAdapter<UpstreamEvent>,
{
    async fn process_next(
        &mut self,
        stage: &mut Stage<C, A>,
        next: &NextResponse<HeaderContent>,
    ) -> Result<(), WorkerError> {
        match next {
            NextResponse::RollForward(header, tip) => {
                let header = to_traverse(header).or_panic()?;
                let slot = header.slot();
                let hash = header.hash();

                debug!(slot, %hash, "chain sync roll forward");

                let block = self
                    .peer_session
                    .blockfetch()
                    .fetch_single(pallas_network::miniprotocols::Point::Specific(
                        slot,
                        hash.to_vec(),
                    ))
                    .await
                    .or_retry()?;

                stage
                    .downstream
                    .send(UpstreamEvent::RollForward(slot, hash, block).into())
                    .await
                    .or_panic()?;

                stage.chain_tip.set(tip.0.slot_or_default() as i64);

                Ok(())
            }
            chainsync::NextResponse::RollBackward(point, tip) => {
                match &point {
                    Point::Origin => debug!("rollback to origin"),
                    Point::Specific(slot, _) => debug!(slot, "rollback"),
                };

                stage
                    .downstream
                    .send(UpstreamEvent::Rollback(point.clone()).into())
                    .await
                    .or_panic()?;

                stage.chain_tip.set(tip.0.slot_or_default() as i64);

                Ok(())
            }
            chainsync::NextResponse::Await => {
                info!("chain-sync reached the tip of the chain");
                Ok(())
            }
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<C, A> gasket::framework::Worker<Stage<C, A>> for Worker<C, A>
where
    C: Cursor + Sync + Send,
    A: SendAdapter<UpstreamEvent>,
{
    async fn bootstrap(stage: &Stage<C, A>) -> Result<Self, WorkerError> {
        debug!("connecting");

        let intersection = stage.chain_cursor.intersection();

        let mut peer_session = PeerClient::connect(&stage.peer_address, stage.network_magic)
            .await
            .or_retry()?;

        intersect(&mut peer_session, intersection).await?;

        let worker = Self {
            peer_session,
            _panthom_a: Default::default(),
            _panthom_c: Default::default(),
        };

        Ok(worker)
    }

    async fn schedule(
        &mut self,
        _stage: &mut Stage<C, A>,
    ) -> Result<WorkSchedule<NextResponse<HeaderContent>>, WorkerError> {
        let client = self.peer_session.chainsync();

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

        Ok(WorkSchedule::Unit(next))
    }

    async fn execute(
        &mut self,
        unit: &NextResponse<HeaderContent>,
        stage: &mut Stage<C, A>,
    ) -> Result<(), WorkerError> {
        self.process_next(stage, unit).await
    }

    async fn teardown(&mut self) -> Result<(), WorkerError> {
        self.peer_session.abort();

        Ok(())
    }
}

pub struct Stage<C, A>
where
    C: Cursor,
    A: SendAdapter<UpstreamEvent>,
{
    peer_address: String,
    network_magic: u64,
    chain_cursor: C,
    downstream: DownstreamPort<A>,
    block_count: gasket::metrics::Counter,
    chain_tip: gasket::metrics::Gauge,
}

impl<C, A> gasket::framework::Stage for Stage<C, A>
where
    C: Cursor,
    A: SendAdapter<UpstreamEvent>,
{
    type Unit = NextResponse<HeaderContent>;
    type Worker = Worker<C, A>;

    fn name(&self) -> &str {
        "upstream"
    }

    fn metrics(&self) -> gasket::metrics::Registry {
        let mut registry = gasket::metrics::Registry::default();

        registry.track_counter("received_blocks", &self.block_count);
        registry.track_gauge("chain_tip", &self.chain_tip);

        registry
    }
}

impl<C, A> Stage<C, A>
where
    C: Cursor,
    A: SendAdapter<UpstreamEvent>,
{
    pub fn new(peer_address: String, network_magic: u64, chain_cursor: C) -> Self {
        Self {
            peer_address,
            network_magic,
            chain_cursor,
            downstream: Default::default(),
            block_count: Default::default(),
            chain_tip: Default::default(),
        }
    }

    pub fn downstream_port(&mut self) -> &mut DownstreamPort<A> {
        &mut self.downstream
    }
}
