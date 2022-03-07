use std::fmt::Debug;

use crate::machines::{Agent, DecodePayload, EncodePayload, MachineOutput, Transition};

use super::protocol::{Message, State};

pub type SubmitResult<E> = Result<(), E>;

#[derive(Debug)]
pub struct BatchClient<T, E> {
    pub state: State,
    pub pending: Vec<T>,
    pub inflight: Option<T>,
    pub results: Vec<SubmitResult<E>>,
}

impl<T, E> BatchClient<T, E>
where
    T: EncodePayload + DecodePayload + Clone,
    E: EncodePayload + DecodePayload + Debug,
{
    pub fn initial(mut fifo_requests: Vec<T>) -> Self {
        // reverse the fifo vec to treat it as a stack of pending requests
        fifo_requests.reverse();

        Self {
            state: State::Idle,
            inflight: None,
            results: Vec::with_capacity(fifo_requests.len()),
            pending: fifo_requests,
        }
    }

    fn pop_pending(self, output: &impl MachineOutput) -> Transition<Self> {
        log::debug!("popping next pending tx");

        let Self { mut pending, .. } = self;

        match pending.pop() {
            Some(next) => {
                output.send_msg(&Message::<T, E>::SubmitTx(next.clone()))?;

                Ok(Self {
                    state: State::Busy,
                    inflight: Some(next),
                    pending,
                    ..self
                })
            }
            None => Ok(Self {
                state: State::Done,
                pending,
                ..self
            }),
        }
    }

    fn on_accept(self) -> Transition<Self> {
        log::debug!("tx accepted");

        let Self {
            mut results,
            pending,
            ..
        } = self;

        results.push(Ok(()));

        Ok(Self {
            state: State::Idle,
            inflight: None,
            results,
            pending,
        })
    }

    fn on_reject(self, reason: E) -> Transition<Self> {
        log::debug!("tx rejected with reason {:?}", reason);

        let Self {
            mut results,
            pending,
            ..
        } = self;

        results.push(Err(reason));

        Ok(Self {
            state: State::Idle,
            inflight: None,
            results,
            pending,
        })
    }
}

impl<T, E> Agent for BatchClient<T, E>
where
    T: EncodePayload + DecodePayload + Debug + Clone,
    E: EncodePayload + DecodePayload + Debug,
{
    type Message = Message<T, E>;

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state {
            State::Idle => true,
            State::Busy => false,
            State::Done => false,
        }
    }

    fn send_next(self, output: &impl MachineOutput) -> Transition<Self> {
        match self.state {
            State::Idle => self.pop_pending(output),
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn receive_next(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Busy, Message::AcceptTx) => self.on_accept(),
            (State::Busy, Message::RejectTx(reason)) => self.on_reject(reason),
            (State::Busy, _) => panic!("Invalid message for busy state"),
            (State::Idle, _) => panic!("I have agency, I don't expect messages"),
            (State::Done, _) => panic!("I'm done, I don't expect messages"),
        }
    }
}
