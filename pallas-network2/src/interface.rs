use std::{
    collections::HashMap,
    future::ready,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{
    FutureExt, Stream, StreamExt,
    channel::oneshot,
    future::{Either, Shared, select},
    stream::{FusedStream, FuturesUnordered},
};

use tokio::{sync::Mutex, time::Instant};

use crate::{
    Channel, Interface, InterfaceCommand, InterfaceError, InterfaceEvent, Message, Payload, PeerId,
    bearer::{Bearer, BearerReadHalf, BearerWriteHalf, Timestamp},
};

/// The pool opening a connection (connect, accept), failing to, or closing
/// one. Produced by futures that belong to no connection, so never cancelled.
enum ConnectionStateEvent {
    Connected(PeerId, Bearer),
    Disconnected(PeerId),
    OpenFailed(PeerId, tokio::io::Error),
}

/// Messages sent or received on an open connection. Produced by futures that
/// belong to that connection, and cancelled with it.
enum MessagingEvent<M: Message> {
    Sent(PeerId, M),
    Recv(PeerId, Vec<M>, BearerReadHalf, ChunkBuffer),
    Error(PeerId, tokio::io::Error),
    /// The future was cancelled with its connection. Never reported.
    Cancelled,
}

/// What a future of the pool yields.
enum InternalEvent<M: Message> {
    ConnectionState(ConnectionStateEvent),
    Messaging(MessagingEvent<M>),
}

type ConnectionStateFuture = Pin<Box<dyn Future<Output = ConnectionStateEvent> + Send>>;

type InterfaceFuture<M> = Pin<Box<dyn Future<Output = InternalEvent<M>> + Send>>;

/// Resolves when the [`Connection`] that created it is dropped.
type CancelSignal = Shared<oneshot::Receiver<()>>;

/// Runs `future` until it completes or its connection is dropped, whichever
/// comes first. A dropped connection wins a tie, so no event of a
/// disconnected connection is ever reported. Once cancelled, `future` is
/// dropped with what it owns (read half, writer clone, writer lock).
fn cancellable<M: Message>(
    future: impl Future<Output = MessagingEvent<M>> + Send + 'static,
    cancelled: CancelSignal,
) -> InterfaceFuture<M> {
    Box::pin(
        select(cancelled, Box::pin(future)).map(|either| match either {
            Either::Left(_) => InternalEvent::Messaging(MessagingEvent::Cancelled),
            Either::Right((event, _)) => InternalEvent::Messaging(event),
        }),
    )
}

/// Everything the pool keeps about an open connection. Dropping it cancels
/// the connection's pending futures; once they are dropped, nothing owns the
/// socket anymore and it is closed.
struct Connection {
    writer: SharedWriter,
    cancelled: CancelSignal,
    _cancel: oneshot::Sender<()>,
}

impl Connection {
    fn new(writer: BearerWriteHalf) -> Self {
        let (cancel, cancelled) = oneshot::channel();

        Self {
            writer: Arc::new(Mutex::new(writer)),
            cancelled: cancelled.shared(),
            _cancel: cancel,
        }
    }
}

async fn connect(pid: PeerId) -> ConnectionStateEvent {
    let pid = pid.clone();

    tracing::debug!(%pid, "connecting bearer");
    let bearer = Bearer::connect_tcp((pid.host.clone(), pid.port)).await;

    match bearer {
        Ok(bearer) => ConnectionStateEvent::Connected(pid.clone(), bearer),
        Err(e) => ConnectionStateEvent::OpenFailed(pid.clone(), e),
    }
}

async fn send<M: Message>(
    pid: PeerId,
    writer: SharedWriter,
    msg: M,
    ts: Timestamp,
    mode: u16,
) -> MessagingEvent<M> {
    let pid = pid.clone();
    let copy = msg.clone();

    let mut writer = writer.lock().await;

    let result = writer.write_message(msg, ts, mode).await;

    match result {
        Ok(_) => MessagingEvent::Sent(pid.clone(), copy),
        Err(e) => MessagingEvent::Error(pid.clone(), e),
    }
}

/// Buffer of partial payload chunks keyed by channel, used to reassemble
/// multi-segment messages.
pub type ChunkBuffer = HashMap<Channel, Payload>;

async fn recv<M: Message>(
    pid: PeerId,
    mut reader: BearerReadHalf,
    mut partial_chunks: ChunkBuffer,
) -> MessagingEvent<M> {
    let pid = pid.clone();

    let result = reader.read_full_msgs(&mut partial_chunks).await;

    match result {
        Ok(msgs) => MessagingEvent::Recv(pid.clone(), msgs, reader, partial_chunks),
        Err(e) => MessagingEvent::Error(pid.clone(), e),
    }
}

/// A thread-safe handle to a shared [`BearerWriteHalf`].
pub type SharedWriter = Arc<Mutex<BearerWriteHalf>>;

// ---------------------------------------------------------------------------
// TcpConnectionPool — shared connection-management logic
// ---------------------------------------------------------------------------

struct TcpConnectionPool<M: Message> {
    futures: FuturesUnordered<InterfaceFuture<M>>,
    connections: HashMap<PeerId, Connection>,
    clock: Instant,
    /// The mode bit to set on outgoing segments (0 for initiator, PROTOCOL_SERVER for responder).
    mode: u16,
}

impl<M: Message> TcpConnectionPool<M> {
    fn new(mode: u16) -> Self {
        Self {
            futures: FuturesUnordered::new(),
            connections: HashMap::new(),
            clock: Instant::now(),
            mode,
        }
    }

    fn push_future(&mut self, f: ConnectionStateFuture) {
        self.futures
            .push(Box::pin(f.map(InternalEvent::ConnectionState)));
    }

    /// Pushes a future that belongs to the connection with `pid`, so that it
    /// is cancelled with it. Does nothing if there is no such connection.
    fn push_connection_future(
        &mut self,
        pid: &PeerId,
        f: impl Future<Output = MessagingEvent<M>> + Send + 'static,
    ) {
        if let Some(connection) = self.connections.get(pid) {
            self.futures
                .push(cancellable(f, connection.cancelled.clone()));
        }
    }

    fn take_writer(&mut self, pid: &PeerId) -> Option<SharedWriter> {
        self.connections.get(pid).map(|c| c.writer.clone())
    }

    fn on_connected(&mut self, pid: PeerId, bearer: Bearer) -> InterfaceEvent<M> {
        let (read, write) = bearer.into_split();

        self.connections.insert(pid.clone(), Connection::new(write));

        let future = recv(pid.clone(), read, HashMap::new());
        self.push_connection_future(&pid, future);

        InterfaceEvent::Connected(pid)
    }

    fn on_disconnected(&mut self, pid: PeerId) -> InterfaceEvent<M> {
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
        self.push_connection_future(&pid, future);

        InterfaceEvent::Recv(pid, msgs)
    }

    fn on_error(&mut self, pid: PeerId, error: tokio::io::Error) -> InterfaceEvent<M> {
        tracing::error!("error: {:?}", error);
        InterfaceEvent::Error(pid, InterfaceError::Other(error.to_string()))
    }

    /// Nothing to report, but other futures may already be ready: asks to be
    /// polled again.
    fn on_cancelled(&mut self, cx: &mut Context<'_>) -> Poll<Option<InterfaceEvent<M>>> {
        cx.waker().wake_by_ref();
        Poll::Pending
    }

    fn handle_connection_state_event(&mut self, event: ConnectionStateEvent) -> InterfaceEvent<M> {
        match event {
            ConnectionStateEvent::Connected(pid, stream) => self.on_connected(pid, stream),
            ConnectionStateEvent::Disconnected(pid) => self.on_disconnected(pid),
            ConnectionStateEvent::OpenFailed(pid, error) => self.on_error(pid, error),
        }
    }

    fn handle_messaging_event(
        &mut self,
        event: MessagingEvent<M>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<InterfaceEvent<M>>> {
        match event {
            MessagingEvent::Sent(pid, msg) => Poll::Ready(Some(self.on_sent(pid, msg))),
            MessagingEvent::Recv(pid, msgs, stream, buf) => {
                Poll::Ready(Some(self.on_recv(pid, msgs, stream, buf)))
            }
            MessagingEvent::Error(pid, error) => Poll::Ready(Some(self.on_error(pid, error))),
            MessagingEvent::Cancelled => self.on_cancelled(cx),
        }
    }

    fn dispatch_send(&mut self, pid: PeerId, msg: M) {
        let ts = self.clock.elapsed().as_micros() as u32;

        let Some(writer) = self.take_writer(&pid) else {
            tracing::error!(%pid, "trying to send to a peer not connected");
            return;
        };

        let future = send(pid.clone(), writer, msg, ts, self.mode);
        self.push_connection_future(&pid, future);
    }

    /// Closes the connection without waiting for the peer or for a pending
    /// write: dropping the [`Connection`] cancels its futures, which drops the
    /// read half and every writer clone, so the socket is closed.
    ///
    /// The cancelled futures are woken now, so `FuturesUnordered` (FIFO) polls
    /// and drops them before the `Disconnected` pushed after them is reported.
    fn dispatch_disconnect(&mut self, pid: PeerId) {
        if self.connections.remove(&pid).is_none() {
            tracing::warn!(%pid, "trying to disconnect a peer not connected");
        }

        self.push_future(Box::pin(ready(ConnectionStateEvent::Disconnected(pid))));
    }

    fn poll_next_event(&mut self, cx: &mut Context<'_>) -> Poll<Option<InterfaceEvent<M>>> {
        let event = self.futures.poll_next_unpin(cx);

        match event {
            Poll::Ready(Some(event)) => match event {
                InternalEvent::ConnectionState(event) => {
                    Poll::Ready(Some(self.handle_connection_state_event(event)))
                }
                InternalEvent::Messaging(event) => self.handle_messaging_event(event, cx),
            },
            Poll::Ready(None) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// TcpInterface — outbound connections
// ---------------------------------------------------------------------------

/// A network interface that initiates outbound TCP connections to peers.
///
/// Implements [`Interface`] by managing a pool of TCP connections and
/// dispatching connect/send/disconnect commands.
pub struct TcpInterface<M: Message> {
    pool: TcpConnectionPool<M>,
}

impl<M: Message> Default for TcpInterface<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Message> TcpInterface<M> {
    /// Creates a new TCP interface for initiating outbound connections.
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

async fn accept_tcp(listener: Arc<tokio::net::TcpListener>) -> ConnectionStateEvent {
    match Bearer::accept_tcp(&listener).await {
        Ok((bearer, addr)) => {
            let pid = PeerId {
                host: addr.ip().to_string(),
                port: addr.port(),
            };
            tracing::info!(%pid, "accepted inbound connection");
            ConnectionStateEvent::Connected(pid, bearer)
        }
        Err(e) => {
            tracing::error!("accept error: {:?}", e);
            // Use a sentinel peer id for accept errors
            let pid = PeerId {
                host: "accept-error".to_string(),
                port: 0,
            };
            ConnectionStateEvent::OpenFailed(pid, e)
        }
    }
}

/// A network interface that accepts inbound TCP connections from a bound
/// listener.
///
/// Implements [`Interface`] by continuously accepting new connections and
/// managing the resulting peer sessions. Outbound `Connect` commands are
/// ignored since connections are initiated by remote peers.
pub struct TcpListenerInterface<M: Message> {
    pool: TcpConnectionPool<M>,
    listener: Arc<tokio::net::TcpListener>,
    accept_fut: ConnectionStateFuture,
}

impl<M: Message> TcpListenerInterface<M> {
    /// Creates a new listener interface that will accept connections on the
    /// given [`TcpListener`](tokio::net::TcpListener).
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
            let ie = self.pool.handle_connection_state_event(event);

            // Re-arm the accept future for the next connection
            self.accept_fut = Box::pin(accept_tcp(Arc::clone(&self.listener)));

            return Poll::Ready(Some(ie));
        }

        // Then poll existing connections
        self.pool.poll_next_event(cx)
    }
}
