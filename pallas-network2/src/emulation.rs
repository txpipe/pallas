//! Artifacts to emulate a network interface without any actual IO

use futures::{StreamExt as _, stream::FuturesUnordered};
use rand::Rng as _;
use std::{pin::Pin, time::Duration};

use crate::{Interface, InterfaceCommand, InterfaceError, InterfaceEvent, Message, PeerId};

#[derive(Debug)]
pub enum ReplyAction<M> {
    Message(M),
    Disconnect,
}

pub trait Rules {
    type Message: Message + Clone + 'static;

    fn reply_to(&self, msg: Self::Message) -> ReplyAction<Self::Message>;

    fn should_connect(&self, pid: PeerId) -> bool {
        true
    }

    fn jitter(&self) -> Duration {
        Duration::from_secs(rand::rng().random_range(0..3))
    }
}

pub struct Emulator<M, R>
where
    M: Message + Clone + Send + Sync + 'static,
    R: Rules<Message = M>,
{
    rules: R,
    pending: FuturesUnordered<Pin<Box<dyn Future<Output = InterfaceEvent<M>> + Send>>>,
}

impl<M, R> Default for Emulator<M, R>
where
    M: Message + Clone + Send + Sync + 'static,
    R: Rules<Message = M> + Default,
{
    fn default() -> Self {
        Self {
            rules: R::default(),
            pending: FuturesUnordered::new(),
        }
    }
}

impl<M, R> Interface<M> for Emulator<M, R>
where
    M: Message + Clone + Send + Sync + 'static,
    R: Rules<Message = M>,
{
    fn execute(&mut self, cmd: InterfaceCommand<M>) -> Result<(), InterfaceError> {
        match cmd {
            InterfaceCommand::Connect(peer_id) => {
                let jitter = self.rules.jitter();

                let future = Box::pin(async move {
                    tokio::time::sleep(jitter).await;
                    println!("emulation: connected to {}", peer_id);
                    InterfaceEvent::Connected(peer_id)
                });

                self.pending.push(future);
            }
            InterfaceCommand::Disconnect(peer_id) => {
                let jitter = self.rules.jitter();

                let future = Box::pin(async move {
                    tokio::time::sleep(jitter).await;
                    println!("emulation: disconnected to {}", peer_id);
                    InterfaceEvent::Disconnected(peer_id)
                });

                self.pending.push(future);
            }
            InterfaceCommand::Send(peer_id, msg) => {
                let pid2 = peer_id.clone();
                let msg2 = msg.clone();
                let future1 = Box::pin(async move { InterfaceEvent::Sent(pid2, msg2) });

                self.pending.push(future1);

                let jitter = self.rules.jitter();
                let reply = self.rules.reply_to(msg);

                match reply {
                    ReplyAction::Message(msg) => {
                        let future2 = Box::pin(async move {
                            tokio::time::sleep(jitter).await;
                            println!("emulation: recv from {}", peer_id);
                            InterfaceEvent::Recv(peer_id, msg)
                        });

                        self.pending.push(future2);
                    }
                    ReplyAction::Disconnect => {
                        let future2 = Box::pin(async move {
                            tokio::time::sleep(jitter).await;
                            println!("emulation: disconnect from {}", peer_id);
                            InterfaceEvent::Disconnected(peer_id)
                        });

                        self.pending.push(future2);
                    }
                };
            }
            _ => (),
        };

        Ok(())
    }

    async fn poll_next(&mut self) -> InterfaceEvent<M> {
        let next = self.pending.next().await;
        next.unwrap_or(InterfaceEvent::Idle)
    }
}
