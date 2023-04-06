#![feature(async_fn_in_trait)]

use std::time::Duration;

use gasket::{
    messaging::{
        tokio::{InputPort, OutputPort},
        RecvPort, SendPort,
    },
    runtime::{ScheduleResult, WorkSchedule, Worker},
};
use pallas_miniprotocols::Point;
use pallas_upstream::{BlockFetchEvent, Cursor};
use tracing::{error, info};

struct Witness {
    input: InputPort<pallas_upstream::BlockFetchEvent>,
}

impl Worker for Witness {
    type WorkUnit = BlockFetchEvent;

    fn metrics(&self) -> gasket::metrics::Registry {
        gasket::metrics::Registry::new()
    }

    async fn schedule(&mut self) -> gasket::runtime::ScheduleResult<Self::WorkUnit> {
        error!("dequeing form witness");
        let msg = self.input.recv().await?;
        Ok(WorkSchedule::Unit(msg.payload))
    }

    async fn execute(&mut self, unit: &Self::WorkUnit) -> Result<(), gasket::error::Error> {
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
fn test_mainnet_upstream() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish(),
    )
    .unwrap();

    let mut b = pallas_upstream::n2n::Bootstrapper::new(
        StaticCursor,
        "relays-new.cardano-mainnet.iohk.io:3001".into(),
        764824073,
    );

    let (send, receive) = gasket::messaging::tokio::channel(200);

    // let mut f = Faker {
    //     output: Default::default(),
    // };

    //f.output.connect(send);

    b.connect_output(send);

    let b = b.spawn().unwrap();

    let mut w = Witness {
        input: Default::default(),
    };

    w.input.connect(receive);

    //let f = gasket::runtime::spawn_stage(f, Default::default(), Some("faker"));
    let w = gasket::runtime::spawn_stage(w, Default::default(), Some("witness"));

    let d = gasket::daemon::Daemon(vec![w]);

    d.block();
}
