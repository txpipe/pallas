use gasket::{framework::*, messaging::*, runtime::Policy};

use pallas_upstream::{Cursor, UpstreamEvent};
use tracing::{error, info};

struct WitnessStage {
    input: gasket::messaging::tokio::InputPort<UpstreamEvent>,
}

impl gasket::framework::Stage for WitnessStage {
    type Unit = UpstreamEvent;
    type Worker = WitnessWorker;

    fn name(&self) -> &str {
        "witness"
    }
}

struct WitnessWorker;

#[async_trait::async_trait(?Send)]
impl Worker<WitnessStage> for WitnessWorker {
    async fn bootstrap(_: &WitnessStage) -> Result<Self, WorkerError> {
        Ok(Self)
    }

    async fn schedule(
        &mut self,
        stage: &mut WitnessStage,
    ) -> Result<WorkSchedule<UpstreamEvent>, WorkerError> {
        error!("dequeing form witness");
        let msg = stage.input.recv().await.or_panic()?;
        Ok(WorkSchedule::Unit(msg.payload))
    }

    async fn execute(
        &mut self,
        _: &UpstreamEvent,
        _: &mut WitnessStage,
    ) -> Result<(), WorkerError> {
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
    );

    upstream.downstream_port().connect(send);

    let mut witness = WitnessStage {
        input: Default::default(),
    };

    witness.input.connect(receive);

    let upstream = gasket::runtime::spawn_stage(upstream, Policy::default());
    let witness = gasket::runtime::spawn_stage(witness, Policy::default());

    let daemon = gasket::daemon::Daemon(vec![upstream, witness]);

    daemon.block();
}
