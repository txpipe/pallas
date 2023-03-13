pub use crate::cursor;

pub mod n2n {
    use crate::{blockfetch, chainsync, cursor::Cursor, framework::*, plexer};
    use gasket::runtime::Tether;

    pub struct Runtime {
        pub plexer_tether: Tether,
        pub chainsync_tether: Tether,
        pub blockfetch_tether: Tether,
    }

    pub struct Bootstrapper {
        cursor: Cursor,
        peer_address: String,
        network_magic: u64,
        output: blockfetch::DownstreamPort,
    }

    impl Bootstrapper {
        pub fn new(cursor: Cursor, peer_address: String, network_magic: u64) -> Self {
            Bootstrapper {
                cursor,
                peer_address,
                network_magic,
                output: blockfetch::DownstreamPort::default(),
            }
        }

        pub fn borrow_output_port(&mut self) -> &mut blockfetch::DownstreamPort {
            &mut self.output
        }

        pub fn spawn(self) -> Result<Runtime, Error> {
            /*
            TODO: this is how we envision the setup of complex pipelines leveraging Rust macros:

            pipeline!(
                plexer = plexer::Worker::new(xx),
                chainsync = chainsync::Worker::new(yy),
                blockfetch = blockfetch::Worker::new(yy),
                reducer = reducer::Worker::new(yy),
                plexer.demux2 => chainsync.demux2,
                plexer.demux3 => blockfetch.demux3,
                chainsync.mux2 + blockfetch.mux3 => plexer.mux,
                chainsync.downstream => blockfetch.upstream,
                blockfetch.downstream => reducer.upstream,
            );

            The above snippet would replace the rest of the code in this function, which is just a more verbose, manual way of saying the same thing.
            */

            let mut mux_input = MuxInputPort::default();

            let mut demux2_out = DemuxOutputPort::default();
            let mut demux2_in = DemuxInputPort::default();
            gasket::messaging::connect_ports(&mut demux2_out, &mut demux2_in, 1000);

            let mut demux3_out = DemuxOutputPort::default();
            let mut demux3_in = DemuxInputPort::default();
            gasket::messaging::connect_ports(&mut demux3_out, &mut demux3_in, 1000);

            let mut mux2_out = MuxOutputPort::default();
            let mut mux3_out = MuxOutputPort::default();
            gasket::messaging::funnel_ports(
                vec![&mut mux2_out, &mut mux3_out],
                &mut mux_input,
                1000,
            );

            let mut chainsync_downstream = chainsync::DownstreamPort::default();
            let mut blockfetch_upstream = blockfetch::UpstreamPort::default();
            gasket::messaging::connect_ports(
                &mut chainsync_downstream,
                &mut blockfetch_upstream,
                20,
            );

            let plexer_tether = gasket::runtime::spawn_stage(
                plexer::Worker::new(
                    self.peer_address,
                    self.network_magic,
                    mux_input,
                    Some(demux2_out),
                    Some(demux3_out),
                ),
                gasket::runtime::Policy::default(),
                Some("plexer"),
            );

            let channel2 = ProtocolChannel(2, mux2_out, demux2_in);

            let chainsync_tether = gasket::runtime::spawn_stage(
                chainsync::Worker::new(self.cursor, channel2, chainsync_downstream),
                gasket::runtime::Policy::default(),
                Some("chainsync"),
            );

            let channel3 = ProtocolChannel(3, mux3_out, demux3_in);

            let blockfetch_tether = gasket::runtime::spawn_stage(
                blockfetch::Worker::new(channel3, blockfetch_upstream, self.output),
                gasket::runtime::Policy::default(),
                Some("blockfetch"),
            );

            Ok(Runtime {
                plexer_tether,
                chainsync_tether,
                blockfetch_tether,
            })
        }
    }
}
