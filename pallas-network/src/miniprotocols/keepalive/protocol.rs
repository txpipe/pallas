/// Opaque token echoed in a keep-alive round trip to match request to response.
pub type Cookie = u16;

/// Keep-alive state machine state.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Client may send another ping (or close the protocol).
    Client,
    /// Server owes a response carrying the given cookie.
    Server(Cookie),
    /// Protocol is terminated.
    Done,
}

/// Keep-alive protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Client → server: ping carrying a cookie to echo back.
    KeepAlive(Cookie),
    /// Server → client: response echoing the cookie from the ping.
    ResponseKeepAlive(Cookie),
    /// Client → server: terminate the protocol.
    Done,
}
