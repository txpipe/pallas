mod codec;

use std::fmt::Debug;

use log::debug;

use pallas_machines::{Agent, DecodePayload, EncodePayload, MachineError, MachineOutput, Transition, primitives::Point};

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
    type Request: EncodePayload + DecodePayload + Clone + Debug;
    type Response: EncodePayload + DecodePayload + Clone + Debug;
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

impl<Q: Query> OneShotClient<Q> {
    pub fn initial(check_point: Option<Point>, request: Q::Request) -> Self {
        Self {
            state: State::Idle,
            output: None,
            check_point,
            request,
        }
    }

    fn send_acquire(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::<Q>::Acquire(self.check_point.clone());

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Acquiring,
            ..self
        })
    }

    fn send_query(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::<Q>::Query(self.request.clone());

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Querying,
            ..self
        })
    }

    fn send_release(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::<Q>::Release;

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Idle,
            ..self
        })
    }

    fn on_acquired(self) -> Transition<Self> {
        debug!("acquired check point for chain state");

        Ok(Self {
            state: State::Acquired,
            ..self
        })
    }

    fn on_result(self, response: Q::Response) -> Transition<Self> {
        debug!("query result received: {:?}", response);

        Ok(Self {
            state: State::Acquired,
            output: Some(Ok(response)),
            ..self
        })
    }

    fn on_failure(self, failure: AcquireFailure) -> Transition<Self> {
        debug!("acquire failure: {:?}", failure);

        Ok(Self {
            state: State::Idle,
            output: Some(Err(failure)),
            ..self
        })
    }

    fn done(self) -> Transition<Self> {
        Ok(Self {
            state: State::Done,
            ..self
        })
    }
}

impl<Q: Query + 'static> Agent for OneShotClient<Q> {
    type Message = Message<Q>;

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state {
            State::Idle => true,
            State::Acquired => true,
            _ => false,
        }
    }

    fn send_next(self, tx: &impl MachineOutput) -> Transition<Self> {
        match (&self.state, &self.output) {
            // if we're idle and without a result, assume start of flow
            (State::Idle, None) => self.send_acquire(tx),
            // if we're idle and with a result, assume end of flow
            (State::Idle, Some(_)) => self.done(),
            // if we don't have an output, assume start of query
            (State::Acquired, None) => self.send_query(tx),
            // if we have an output but still acquired, release the server
            (State::Acquired, Some(_)) => self.send_release(tx),
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn receive_next(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Acquiring, Message::Acquired) => self.on_acquired(),
            (State::Acquiring, Message::Failure(failure)) => self.on_failure(failure),
            (State::Querying, Message::Result(result)) => self.on_result(result),
            (_, msg) => Err(MachineError::InvalidMsgForState(self.state, msg).into()),
        }
    }
}
