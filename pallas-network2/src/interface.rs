use std::{
    collections::HashMap,
    future::ready,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{
    Stream, StreamExt,
    stream::{FusedStream, FuturesUnordered},
};

use tokio::{net::TcpListener, sync::Mutex, time::Instant};

use crate::{
    Channel, Interface, InterfaceCommand, InterfaceError, InterfaceEvent, Message, Payload, PeerId,
    bearer::{Bearer, BearerReadHalf, BearerWriteHalf, Timestamp},
};

enum InternalEvent<M: Message> {
    Connected(PeerId, Bearer),
    Accepted(PeerId, Bearer),
    Disconnected(PeerId),
    Sent(PeerId, M),
    Recv(PeerId, Vec<M>, BearerReadHalf, ChunkBuffer),
    Error(PeerId, tokio::io::Error),
}

type InterfaceFuture<M> = Pin<Box<dyn Future<Output = InternalEvent<M>> + Send>>;

async fn connect<M: Message>(pid: PeerId) -> InternalEvent<M> {
    let pid = pid.clone();

    tracing::debug!(%pid, "connecting bearer");
    let bearer = Bearer::connect_tcp((pid.host.clone(), pid.port)).await;

    match bearer {
        Ok(bearer) => InternalEvent::Connected(pid.clone(), bearer),
        Err(e) => InternalEvent::Error(pid.clone(), e),
    }
}

async fn accept<M: Message>(listener: SharedListener) -> InternalEvent<M> {
    let listener = listener.lock().await;

    tracing::debug!("waiting for incoming connection");
    let connection = Bearer::accept_tcp(&listener).await;

    match connection {
        Ok((bearer, addr)) => {
            let pid = PeerId::from(addr);
            InternalEvent::Accepted(pid, bearer)
        }
        Err(e) => {
            dbg!(&e);
            todo!("handle error accepting connection")
        }
    }
}

async fn send<M: Message>(
    pid: PeerId,
    writer: SharedWriter,
    msg: M,
    ts: Timestamp,
) -> InternalEvent<M> {
    let pid = pid.clone();
    let copy = msg.clone();

    let mut writer = writer.lock().await;

    let result = writer.write_message(msg, ts).await;

    match result {
        Ok(_) => InternalEvent::Sent(pid.clone(), copy),
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

    let result = reader.read_full_msgs(&mut partial_chunks).await;

    match result {
        Ok(msgs) => InternalEvent::Recv(pid.clone(), msgs, reader, partial_chunks),
        Err(e) => InternalEvent::Error(pid.clone(), e),
    }
}

async fn disconnect<M: Message>(pid: PeerId, writer: SharedWriter) -> InternalEvent<M> {
    let pid = pid.clone();

    let mut writer = writer.lock().await;

    writer.shutdown().await.unwrap();

    InternalEvent::Disconnected(pid.clone())
}

pub type SharedWriter = Arc<Mutex<BearerWriteHalf>>;

pub type SharedListener = Arc<Mutex<TcpListener>>;

pub struct TcpInterface<M: Message> {
    futures: FuturesUnordered<InterfaceFuture<M>>,
    writers: HashMap<PeerId, SharedWriter>,
    listener: SharedListener,
    clock: Instant,
}

impl<M: Message> TcpInterface<M> {
    pub fn new(listener: TcpListener) -> Self {
        Self {
            listener: Arc::new(Mutex::new(listener)),
            futures: FuturesUnordered::new(),
            writers: HashMap::new(),
            clock: Instant::now(),
        }
    }

    fn take_writer(&mut self, pid: &PeerId) -> Option<SharedWriter> {
        self.writers.get(pid).cloned()
    }

    fn take_listener(&mut self) -> SharedListener {
        self.listener.clone()
    }

    fn setup_peer(&mut self, pid: &PeerId, bearer: Bearer) {
        let (read, write) = bearer.into_split();

        // we store the writer for this peer so we can send messages to it when
        // requested
        self.writers
            .insert(pid.clone(), Arc::new(Mutex::new(write)));

        // we immediately schedule a new recv for this peer to handle the incoming
        // messages
        let future = recv(pid.clone(), read, HashMap::new());
        self.futures.push(Box::pin(future));
    }

    fn on_connected(&mut self, pid: PeerId, bearer: Bearer) -> InterfaceEvent<M> {
        self.setup_peer(&pid, bearer);
        InterfaceEvent::Connected(pid)
    }

    fn on_accepted(&mut self, pid: PeerId, bearer: Bearer) -> InterfaceEvent<M> {
        self.setup_peer(&pid, bearer);
        InterfaceEvent::Accepted(pid)
    }

    fn on_disconnected(&mut self, pid: PeerId) -> InterfaceEvent<M> {
        // we remove the writer for this peer so we can't send messages to it anymore
        self.writers.remove(&pid);

        InterfaceEvent::Disconnected(pid)
    }

    fn on_sent(&mut self, pid: PeerId, msg: M) -> InterfaceEvent<M> {
        InterfaceEvent::Sent(pid, msg)
    }

    fn on_recv(
        &mut self,
        pid: PeerId,
        msgs: Vec<M>,
        reader: BearerReadHalf,
        partial_chunks: ChunkBuffer,
    ) -> InterfaceEvent<M> {
        // we immediately schedule a new recv for this peer
        let future = recv(pid.clone(), reader, partial_chunks);
        self.futures.push(Box::pin(future));

        InterfaceEvent::Recv(pid, msgs)
    }

    fn on_error(&mut self, pid: PeerId, error: tokio::io::Error) -> InterfaceEvent<M> {
        tracing::error!("error: {:?}", error);

        InterfaceEvent::Error(pid, InterfaceError::Other(error.to_string()))
    }

    fn handle_internal_event(&mut self, event: InternalEvent<M>) -> InterfaceEvent<M> {
        match event {
            InternalEvent::Connected(pid, stream) => self.on_connected(pid, stream),
            InternalEvent::Accepted(pid, stream) => self.on_accepted(pid, stream),
            InternalEvent::Sent(pid, msg) => self.on_sent(pid, msg),
            InternalEvent::Recv(pid, msgs, stream, buf) => self.on_recv(pid, msgs, stream, buf),
            InternalEvent::Disconnected(pid) => self.on_disconnected(pid),
            InternalEvent::Error(pid, error) => self.on_error(pid, error),
        }
    }
}

impl<M: Message> Interface<M> for TcpInterface<M> {
    fn dispatch(&mut self, cmd: InterfaceCommand<M>) {
        match cmd {
            InterfaceCommand::Connect(pid) => {
                let future = connect(pid.clone());
                self.futures.push(Box::pin(future));
            }
            InterfaceCommand::Accept => {
                let listener = self.take_listener();
                let future = accept(listener);
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

                    // trying to disconnect a missing peer is expected, it's easier for behaviors to trigger
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

impl<M: Message> FusedStream for TcpInterface<M> {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl<M: Message> Stream for TcpInterface<M> {
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
