mod codec;
pub mod queries;

use std::fmt::Debug;

use pallas_codec::Fragment;

use crate::machines::{Agent, MachineError, Transition};

use crate::common::Point;

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Idle,
    Acquiring,
    Acquired,
    Querying,
    Done,
}

#[derive(Debug)]
pub enum AcquireFailure {
    PointTooOld,
    PointNotInChain,
}
pub trait Query: Debug {
    type Request: Clone + Debug;
    type Response: Clone + Debug;
}

#[derive(Debug)]
pub enum Message<Q: Query> {
    Acquire(Option<Point>),
    Failure(AcquireFailure),
    Acquired,
    Query(Q::Request),
    Result(Q::Response),
    ReAcquire(Option<Point>),
    Release,
    Done,
}

pub type Output<QR> = Result<QR, AcquireFailure>;

#[derive(Debug)]
pub struct OneShotClient<Q: Query> {
    pub state: State,
    pub check_point: Option<Point>,
    pub request: Q::Request,
    pub output: Option<Output<Q::Response>>,
}

impl<Q> OneShotClient<Q>
where
    Q: Query,
    Message<Q>: Fragment,
{
    pub fn initial(check_point: Option<Point>, request: Q::Request) -> Self {
        Self {
            state: State::Idle,
            output: None,
            check_point,
            request,
        }
    }

    fn on_acquired(self) -> Transition<Self> {
        log::debug!("acquired check point for chain state");

        Ok(Self {
            state: State::Acquired,
            ..self
        })
    }

    fn on_result(self, response: Q::Response) -> Transition<Self> {
        log::debug!("query result received: {:?}", response);

        Ok(Self {
            // once we get a result, since this is a one-shot client, we mutate into Done
            state: State::Done,
            output: Some(Ok(response)),
            ..self
        })
    }

    fn on_failure(self, failure: AcquireFailure) -> Transition<Self> {
        log::debug!("acquire failure: {:?}", failure);

        Ok(Self {
            state: State::Idle,
            output: Some(Err(failure)),
            ..self
        })
    }
}

impl<Q> Agent for OneShotClient<Q>
where
    Q: Query + 'static,
    Message<Q>: Fragment,
{
    type Message = Message<Q>;

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    #[allow(clippy::match_like_matches_macro)]
    fn has_agency(&self) -> bool {
        match self.state {
            State::Idle => true,
            State::Acquired => true,
            _ => false,
        }
    }

    fn build_next(&self) -> Self::Message {
        match (&self.state, &self.output) {
            // if we're idle and without a result, assume start of flow
            (State::Idle, None) => Message::<Q>::Acquire(self.check_point.clone()),
            // if we don't have an output, assume start of query
            (State::Acquired, None) => Message::<Q>::Query(self.request.clone()),
            // if we have an output but still acquired, release the server
            (State::Acquired, Some(_)) => Message::<Q>::Release,
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn apply_start(self) -> Transition<Self> {
        Ok(self)
    }

    fn apply_outbound(self, msg: Self::Message) -> Transition<Self> {
        match (self.state, msg) {
            (State::Idle, Message::Acquire(_)) => Ok(Self {
                state: State::Acquiring,
                ..self
            }),
            (State::Acquired, Message::Query(_)) => Ok(Self {
                state: State::Querying,
                ..self
            }),
            (State::Acquired, Message::Release) => Ok(Self {
                state: State::Idle,
                ..self
            }),
            (State::Idle, Message::Done) => Ok(Self {
                state: State::Done,
                ..self
            }),
            _ => panic!(""),
        }
    }

    fn apply_inbound(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Acquiring, Message::Acquired) => self.on_acquired(),
            (State::Acquiring, Message::Failure(failure)) => self.on_failure(failure),
            (State::Querying, Message::Result(result)) => self.on_result(result),
            (_, msg) => Err(MachineError::InvalidMsgForState(self.state, msg).into()),
        }
    }
}
