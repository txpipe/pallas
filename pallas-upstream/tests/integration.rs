use gasket::{framework::*, messaging::*, runtime::Policy};

use pallas_upstream::{Cursor, UpstreamEvent};
use tracing::{error, info};

struct WitnessStage {
    input: gasket::messaging::tokio::InputPort<UpstreamEvent>,
}

impl gasket::framework::Stage for WitnessStage {
    fn name(&self) -> &str {
        "witness"
    }

    fn policy(&self) -> gasket::runtime::Policy {
        Policy::default()
    }

    fn register_metrics(&self, _: &mut gasket::metrics::Registry) {}
}

struct WitnessWorker;

#[async_trait::async_trait(?Send)]
impl Worker for WitnessWorker {
    type Unit = UpstreamEvent;
    type Stage = WitnessStage;

    async fn bootstrap(_: &Self::Stage) -> Result<Self, WorkerError> {
        Ok(Self)
    }

    async fn schedule(
        &mut self,
        stage: &mut Self::Stage,
    ) -> Result<WorkSchedule<Self::Unit>, WorkerError> {
        error!("dequeing form witness");
        let msg = stage.input.recv().await.or_panic()?;
        Ok(WorkSchedule::Unit(msg.payload))
    }

    async fn execute(&mut self, _: &Self::Unit, _: &mut Self::Stage) -> Result<(), WorkerError> {
        info!("witnessing block event");

        Ok(())
    }
}

struct StaticCursor;

impl Cursor for StaticCursor {
    fn intersection(&self) -> pallas_upstream::Intersection {
        pallas_upstream::Intersection::Origin
    }
}

#[test]
#[ignore]
fn test_mainnet_upstream() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .unwrap();

    let (send, receive) = gasket::messaging::tokio::channel(200);

    let mut upstream = pallas_upstream::n2n::Stage::new(
        "relays-new.cardano-mainnet.iohk.io:3001".into(),
        764824073,
        StaticCursor,
        Policy::default(),
    );

    upstream.downstream_port().connect(send);

    let mut witness = WitnessStage {
        input: Default::default(),
    };

    witness.input.connect(receive);

    let upstream = gasket::runtime::spawn_stage::<pallas_upstream::n2n::Worker<_, _>>(upstream);
    let witness = gasket::runtime::spawn_stage::<WitnessWorker>(witness);

    let daemon = gasket::daemon::Daemon(vec![upstream, witness]);

    daemon.block();
}
