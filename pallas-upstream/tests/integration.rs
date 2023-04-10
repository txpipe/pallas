use gasket::{
    messaging::{
        tokio::{InputPort, OutputPort},
        RecvPort, SendPort,
    },
    runtime::{WorkSchedule, Worker},
};

use pallas_upstream::{Cursor, UpstreamEvent};
use tracing::error;

struct Witness {
    input: InputPort<UpstreamEvent>,
}

#[async_trait::async_trait]
impl Worker for Witness {
    type WorkUnit = UpstreamEvent;

    fn metrics(&self) -> gasket::metrics::Registry {
        gasket::metrics::Registry::new()
    }

    async fn schedule(&mut self) -> gasket::runtime::ScheduleResult<Self::WorkUnit> {
        error!("dequeing form witness");
        let msg = self.input.recv().await?;
        Ok(WorkSchedule::Unit(msg.payload))
    }

    async fn execute(&mut self, _: &Self::WorkUnit) -> Result<(), gasket::error::Error> {
        error!("witnessing block event");

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

    let mut output_port = OutputPort::default();
    output_port.connect(send);

    let upstream = pallas_upstream::n2n::Worker::new(
        "relays-new.cardano-mainnet.iohk.io:3001".into(),
        764824073,
        StaticCursor,
        output_port,
    );

    let mut witness = Witness {
        input: Default::default(),
    };

    witness.input.connect(receive);

    let upstream = gasket::runtime::spawn_stage(upstream, Default::default(), Some("upstream"));
    let witness = gasket::runtime::spawn_stage(witness, Default::default(), Some("witness"));

    let daemon = gasket::daemon::Daemon(vec![upstream, witness]);

    daemon.block();
}
