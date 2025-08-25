pub type Cookie = u16;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ClientState {
    Empty,
    Response(Cookie),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Client(ClientState),
    Server(Cookie),
    Done,
}

#[derive(Debug, Clone)]
pub enum Message {
    KeepAlive(Cookie),
    ResponseKeepAlive(Cookie),
    Done,
}
