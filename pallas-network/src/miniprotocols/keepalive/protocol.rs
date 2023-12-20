pub type KeepAliveCookie = u16;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Client,
    Server,
    Done,
}

#[derive(Debug, Clone)]
pub enum Message {
    KeepAlive(KeepAliveCookie),
    ResponseKeepAlive(KeepAliveCookie),
    Done,
}
