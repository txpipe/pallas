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

use tokio::{sync::Mutex, time::Instant};

use crate::{
    Channel, Interface, InterfaceCommand, InterfaceError, InterfaceEvent, Message, Payload, PeerId,
    bearer::{Bearer, BearerReadHalf, BearerWriteHalf, Timestamp},
};

enum InternalEvent<M: Message> {
    Connected(PeerId, Bearer),
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

async fn send<M: Message>(
    pid: PeerId,
    writer: SharedWriter,
    msg: M,
    ts: Timestamp,
    mode: u16,
) -> InternalEvent<M> {
    let pid = pid.clone();
    let copy = msg.clone();

    let mut writer = writer.lock().await;

    let result = writer.write_message(msg, ts, mode).await;

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

// ---------------------------------------------------------------------------
// TcpConnectionPool — shared connection-management logic
// ---------------------------------------------------------------------------

struct TcpConnectionPool<M: Message> {
    futures: FuturesUnordered<InterfaceFuture<M>>,
    writers: HashMap<PeerId, SharedWriter>,
    clock: Instant,
    /// The mode bit to set on outgoing segments (0 for initiator, PROTOCOL_SERVER for responder).
    mode: u16,
}

impl<M: Message> TcpConnectionPool<M> {
    fn new(mode: u16) -> Self {
        Self {
            futures: FuturesUnordered::new(),
            writers: HashMap::new(),
            clock: Instant::now(),
            mode,
        }
    }

    fn push_future(&mut self, f: InterfaceFuture<M>) {
        self.futures.push(f);
    }

    fn take_writer(&mut self, pid: &PeerId) -> Option<SharedWriter> {
        self.writers.get(pid).cloned()
    }

    fn on_connected(&mut self, pid: PeerId, bearer: Bearer) -> InterfaceEvent<M> {
        let (read, write) = bearer.into_split();

        self.writers
            .insert(pid.clone(), Arc::new(Mutex::new(write)));

        let future = recv(pid.clone(), read, HashMap::new());
        self.futures.push(Box::pin(future));

        InterfaceEvent::Connected(pid)
    }

    fn on_disconnected(&mut self, pid: PeerId) -> InterfaceEvent<M> {
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
            InternalEvent::Sent(pid, msg) => self.on_sent(pid, msg),
            InternalEvent::Recv(pid, msgs, stream, buf) => self.on_recv(pid, msgs, stream, buf),
            InternalEvent::Disconnected(pid) => self.on_disconnected(pid),
            InternalEvent::Error(pid, error) => self.on_error(pid, error),
        }
    }

    fn dispatch_send(&mut self, pid: PeerId, msg: M) {
        let ts = self.clock.elapsed().as_micros() as u32;

        let Some(writer) = self.take_writer(&pid) else {
            tracing::error!(%pid, "trying to send to a peer not connected");
            return;
        };

        let future = send(pid, writer, msg, ts, self.mode);
        self.futures.push(Box::pin(future));
    }

    fn dispatch_disconnect(&mut self, pid: PeerId) {
        let Some(stream) = self.take_writer(&pid) else {
            tracing::warn!(%pid, "trying to disconnect a peer not connected");
            self.futures
                .push(Box::pin(ready(InternalEvent::Disconnected(pid.clone()))));
            return;
        };

        let future = disconnect(pid, stream);
        self.futures.push(Box::pin(future));
    }

    fn poll_next_event(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<InterfaceEvent<M>>> {
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

// ---------------------------------------------------------------------------
// TcpInterface — outbound connections
// ---------------------------------------------------------------------------

pub struct TcpInterface<M: Message> {
    pool: TcpConnectionPool<M>,
}

impl<M: Message> Default for TcpInterface<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Message> TcpInterface<M> {
    pub fn new() -> Self {
        Self {
            pool: TcpConnectionPool::new(crate::protocol::PROTOCOL_CLIENT),
        }
    }
}

impl<M: Message> Interface<M> for TcpInterface<M> {
    fn dispatch(&mut self, cmd: InterfaceCommand<M>) {
        match cmd {
            InterfaceCommand::Connect(pid) => {
                let future = connect(pid.clone());
                self.pool.push_future(Box::pin(future));
            }
            InterfaceCommand::Send(pid, msg) => {
                self.pool.dispatch_send(pid, msg);
            }
            InterfaceCommand::Disconnect(pid) => {
                self.pool.dispatch_disconnect(pid);
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
        self.pool.poll_next_event(cx)
    }
}

// ---------------------------------------------------------------------------
// TcpListenerInterface — inbound connections via a bound TCP listener
// ---------------------------------------------------------------------------

async fn accept_tcp<M: Message>(
    listener: Arc<tokio::net::TcpListener>,
) -> InternalEvent<M> {
    match Bearer::accept_tcp(&listener).await {
        Ok((bearer, addr)) => {
            let pid = PeerId {
                host: addr.ip().to_string(),
                port: addr.port(),
            };
            tracing::info!(%pid, "accepted inbound connection");
            InternalEvent::Connected(pid, bearer)
        }
        Err(e) => {
            tracing::error!("accept error: {:?}", e);
            // Use a sentinel peer id for accept errors
            let pid = PeerId {
                host: "accept-error".to_string(),
                port: 0,
            };
            InternalEvent::Error(pid, e)
        }
    }
}

pub struct TcpListenerInterface<M: Message> {
    pool: TcpConnectionPool<M>,
    listener: Arc<tokio::net::TcpListener>,
    accept_fut: InterfaceFuture<M>,
}

impl<M: Message> TcpListenerInterface<M> {
    pub fn new(listener: tokio::net::TcpListener) -> Self {
        let listener = Arc::new(listener);
        let accept_fut = Box::pin(accept_tcp(Arc::clone(&listener)));

        Self {
            pool: TcpConnectionPool::new(crate::protocol::PROTOCOL_SERVER),
            listener,
            accept_fut,
        }
    }
}

impl<M: Message> Interface<M> for TcpListenerInterface<M> {
    fn dispatch(&mut self, cmd: InterfaceCommand<M>) {
        match cmd {
            InterfaceCommand::Connect(pid) => {
                tracing::warn!(%pid, "TcpListenerInterface does not support outbound Connect, ignoring");
            }
            InterfaceCommand::Send(pid, msg) => {
                self.pool.dispatch_send(pid, msg);
            }
            InterfaceCommand::Disconnect(pid) => {
                self.pool.dispatch_disconnect(pid);
            }
        }
    }
}

impl<M: Message> FusedStream for TcpListenerInterface<M> {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl<M: Message> Stream for TcpListenerInterface<M> {
    type Item = InterfaceEvent<M>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First, poll the accept future for new inbound connections
        if let Poll::Ready(event) = self.accept_fut.as_mut().poll(cx) {
            let ie = self.pool.handle_internal_event(event);

            // Re-arm the accept future for the next connection
            self.accept_fut = Box::pin(accept_tcp(Arc::clone(&self.listener)));

            return Poll::Ready(Some(ie));
        }

        // Then poll existing connections
        self.pool.poll_next_event(cx)
    }
}
