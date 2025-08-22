use std::{
    collections::HashMap,
    future::ready,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    Stream, StreamExt,
    stream::{FusedStream, FuturesUnordered},
};
use tokio::time::Instant;

use crate::{
    Channel, Interface, InterfaceCommand, InterfaceError, InterfaceEvent, Message, Payload, PeerId,
    bearer::{Bearer, BearerReadHalf, BearerWriteHalf, Timestamp},
};

enum InternalEvent<M: Message> {
    Connected(PeerId, Bearer),
    Disconnected(PeerId),
    Sent(PeerId, M, BearerWriteHalf),
    Recv(PeerId, M, BearerReadHalf, ChunkBuffer),
    Error(PeerId, tokio::io::Error),
}

pub type InterfaceFuture<M: Message> = Pin<Box<dyn Future<Output = InternalEvent<M>> + Send>>;

async fn connect<M: Message>(pid: PeerId) -> InternalEvent<M> {
    let pid = pid.clone();

    tracing::debug!(%pid, "connecting bearer");
    let bearer = Bearer::connect_tcp((pid.host.clone(), pid.port)).await;

    match bearer {
        Ok(bearer) => InternalEvent::Connected(pid.clone(), bearer),
        Err(e) => InternalEvent::Error(pid.clone(), e),
    }
}

async fn send<M: Message>(
    pid: PeerId,
    mut writer: BearerWriteHalf,
    msg: M,
    ts: Timestamp,
) -> InternalEvent<M> {
    let pid = pid.clone();
    let copy = msg.clone();

    let result = writer.write_message(msg, ts).await;

    match result {
        Ok(_) => InternalEvent::Sent(pid.clone(), copy, writer),
        Err(e) => InternalEvent::Error(pid.clone(), e),
    }
}

pub type ChunkBuffer = HashMap<Channel, Payload>;

async fn recv<M: Message>(
    pid: PeerId,
    mut reader: BearerReadHalf,
    mut partial_chunks: ChunkBuffer,
) -> InternalEvent<M> {
    let pid = pid.clone();

    let result = reader.recv_full_msg(&mut partial_chunks).await;

    match result {
        Ok(msg) => InternalEvent::Recv(pid.clone(), msg, reader, partial_chunks),
        Err(e) => InternalEvent::Error(pid.clone(), e),
    }
}

async fn disconnect<M: Message>(pid: PeerId, mut writer: BearerWriteHalf) -> InternalEvent<M> {
    let pid = pid.clone();

    writer.shutdown().await.unwrap();

    InternalEvent::Disconnected(pid.clone())
}

pub struct TokioInterface<M: Message> {
    futures: FuturesUnordered<InterfaceFuture<M>>,
    writers: HashMap<PeerId, BearerWriteHalf>,
    clock: Instant,
}

impl<M: Message> TokioInterface<M> {
    pub fn new() -> Self {
        Self {
            futures: FuturesUnordered::new(),
            writers: HashMap::new(),
            clock: Instant::now(),
        }
    }

    fn take_writer(&mut self, pid: &PeerId) -> Option<BearerWriteHalf> {
        self.writers.remove(pid)
    }

    fn on_connected(&mut self, pid: PeerId, bearer: Bearer) -> InterfaceEvent<M> {
        let (read, write) = bearer.into_split();

        // we store the writer for this peer so we can send messages to it when
        // requested
        self.writers.insert(pid.clone(), write);

        // we immediately schedule a new recv for this peer to handle the incoming
        // messages
        let future = recv(pid.clone(), read, HashMap::new());
        self.futures.push(Box::pin(future));

        InterfaceEvent::Connected(pid)
    }

    fn on_disconnected(&mut self, pid: PeerId) -> InterfaceEvent<M> {
        // we remove the writer for this peer so we can't send messages to it anymore
        self.writers.remove(&pid);

        InterfaceEvent::Disconnected(pid)
    }

    fn on_sent(&mut self, pid: PeerId, msg: M, writer: BearerWriteHalf) -> InterfaceEvent<M> {
        self.writers.insert(pid.clone(), writer);

        InterfaceEvent::Sent(pid, msg)
    }

    fn on_recv(
        &mut self,
        pid: PeerId,
        msg: M,
        reader: BearerReadHalf,
        partial_chunks: ChunkBuffer,
    ) -> InterfaceEvent<M> {
        // we immediately schedule a new recv for this peer
        let future = recv(pid.clone(), reader, partial_chunks);
        self.futures.push(Box::pin(future));

        InterfaceEvent::Recv(pid, msg)
    }

    fn on_error(&mut self, pid: PeerId, error: tokio::io::Error) -> InterfaceEvent<M> {
        tracing::error!("error: {:?}", error);

        InterfaceEvent::Error(pid, InterfaceError::Other(error.to_string()))
    }

    fn handle_internal_event(&mut self, event: InternalEvent<M>) -> InterfaceEvent<M> {
        match event {
            InternalEvent::Connected(pid, stream) => self.on_connected(pid, stream),
            InternalEvent::Sent(pid, msg, stream) => self.on_sent(pid, msg, stream),
            InternalEvent::Recv(pid, msg, stream, buf) => self.on_recv(pid, msg, stream, buf),
            InternalEvent::Disconnected(pid) => self.on_disconnected(pid),
            InternalEvent::Error(pid, error) => self.on_error(pid, error),
        }
    }
}

impl<M: Message> Interface<M> for TokioInterface<M> {
    fn dispatch(&mut self, cmd: InterfaceCommand<M>) {
        match cmd {
            InterfaceCommand::Connect(pid) => {
                let future = connect(pid.clone());
                self.futures.push(Box::pin(future));
            }
            InterfaceCommand::Send(pid, msg) => {
                let ts = self.clock.elapsed().as_micros() as u32;

                let Some(writer) = self.take_writer(&pid) else {
                    tracing::error!(%pid, "trying to send to a peer not connected");
                    return;
                };

                let future = send(pid, writer, msg, ts);
                self.futures.push(Box::pin(future));
            }
            InterfaceCommand::Disconnect(pid) => {
                let Some(stream) = self.take_writer(&pid) else {
                    tracing::warn!(%pid, "trying to disconnect a peer not connected");

                    // trying to disconnect a peer is expected, it's easier for behaviors to trigger
                    // preventive disconnects than checking constantly for a state that might not be
                    // up-to-date. So, if we can't find a connected peer, we just go with it and
                    // assume it's ok.
                    self.futures
                        .push(Box::pin(ready(InternalEvent::Disconnected(pid.clone()))));

                    return;
                };

                let future = disconnect(pid, stream);
                self.futures.push(Box::pin(future));
            }
        }
    }
}

impl<M: Message> FusedStream for TokioInterface<M> {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl<M: Message> Stream for TokioInterface<M> {
    type Item = InterfaceEvent<M>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let event = self.futures.poll_next_unpin(cx);

        match event {
            Poll::Ready(Some(event)) => {
                let event = self.handle_internal_event(event);
                Poll::Ready(Some(event))
            }
            Poll::Ready(None) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
    }
}
