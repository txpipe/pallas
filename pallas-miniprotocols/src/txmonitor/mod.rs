mod codec;

use std::{fmt::{Debug}};
use pallas_codec::Fragment;
use crate::machines::{Agent, MachineError, Transition};

type Slot = u64;
type TxId = String;
type Tx = String;

#[derive(Debug, PartialEq, Clone)]
pub enum StBusyKind {
    NextTx,
    HasTx,
    GetSizes,
}

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    StIdle,
    StAcquiring,
    StAcquired,
    StBusy(StBusyKind),
    StDone,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MempoolSizeAndCapacity {
    pub capacity_in_bytes : u32,
    pub size_in_bytes     : u32,
    pub number_of_txs     : u32,
}

#[derive(Debug, Clone)]
pub enum Message {
    MsgAcquire,
    MsgAcquired(Slot),
    MsgQuery(MsgRequest),
    MsgResponse(MsgResponse),
    MsgDone,
}


#[derive(Debug, Clone)]
pub enum MsgRequest {
    MsgAwaitAcquire,
    MsgNextTx,
    MsgHasTx(TxId), 
    MsgGetSizes,
    MsgRelease,
}
#[derive(Debug, Clone)]
pub enum MsgResponse {
    MsgReplyNextTx(Option<Tx>),
    MsgReplyHasTx(bool),
    MsgReplyGetSizes(MempoolSizeAndCapacity),  
}

#[derive(Debug, Clone)]
pub struct LocalTxMonitor {
    pub state       : State,
    pub snapshot    : Option<Slot>,
    pub request     : Option<MsgRequest>,
    pub output      : Option<MsgResponse>, 
}

impl LocalTxMonitor
    where 
        Message : Fragment,
{
    
    pub fn initial(state : State) -> Self {
        Self {
            state : state,
            snapshot : None,
            request :  None,
            output : None,
        }
    }

    fn on_acquired(self, s: Slot) -> Transition<Self> {
        log::debug!("acquired Slot: '{:?}' ",s);

        Ok(Self {
            state : State::StAcquired,
            snapshot : Some(s),
            output : None,
            ..self
        })
    }    

    fn on_reply_next_tx(self, tx: Option<Tx>) ->  Transition<Self> {
        log::debug!("Next Transaction: {:?}", tx);
        Ok(Self {
            output: Some(MsgResponse::MsgReplyNextTx(tx)),
            ..self
        })
       
    }
 
    fn on_reply_has_tx(self, arg: bool) ->  Transition<Self> {
        log::debug!("Mempool has transaction:   {:?}", arg);
        Ok(Self {
            output: Some(MsgResponse::MsgReplyHasTx(arg)),
            ..self
        })
    }

    fn on_reply_get_size(self, msc: MempoolSizeAndCapacity) ->  Transition<Self> {
        log::debug!("Mempool Status: {:?}", msc);

        Ok(Self {
            output: Some(MsgResponse::MsgReplyGetSizes(msc)),
            ..self
        })
    }
}

impl Agent for LocalTxMonitor
    where 
        Message: Fragment, {
    type Message = Message;
    type State = State;

    fn state(&self) -> &Self::State {
        log::debug!("State: {:?}", &self.state);
        &self.state
    }

    fn is_done(&self) -> bool {
        
        let done = self.state == State::StDone;
        log::debug!("is_done: {:?}",done);
        done
    }

    fn has_agency(&self) -> bool{
        log::trace!("Hase Agency:  State: {:?}, Request: {:?}, Response: {:?}",self.state,self.request,self.output);
        match &self.state {
            State::StIdle => true,
            State::StAcquiring => false,
            State::StAcquired => true,
            State::StBusy(..) => false,
            State::StDone => false,
        }
    }

    fn build_next(&self) -> Self::Message {
        log::debug!("build next; State: {:?}, request: {:?}, output: {:?}",&self.state, &self.request, &self.output);
        match (&self.state, &self.request, &self.output) {
            (State::StIdle, None ,None)                                     => Message::MsgAcquire,
            (State::StAcquired, None , None)                                => Message::MsgAcquire,
            (State::StAcquired, Some(MsgRequest::MsgAwaitAcquire), None)    => Message::MsgAcquire,
            (State::StAcquired, Some(MsgRequest::MsgNextTx),None)           => Message::MsgQuery(MsgRequest::MsgNextTx),
            (State::StAcquired, Some(MsgRequest::MsgHasTx(tx)),None) => Message::MsgQuery(MsgRequest::MsgHasTx(tx.clone())),
            (State::StAcquired, Some(MsgRequest::MsgGetSizes),None)         => Message::MsgQuery(MsgRequest::MsgGetSizes),
            (State::StAcquired, None, Some(_))                              => Message::MsgAcquire,
            (State::StAcquired, Some(req), Some(_))            => Message::MsgQuery(req.to_owned()),
            _ => panic!("I do not have agency, don't know what to do")
        }
    }

    fn apply_start(self) -> Transition<Self> {
        log::debug!("apply start");
        Ok(self)
    }

    fn apply_outbound(self, msg: Self::Message) -> Transition<Self> {
        log::debug!("apply outbound");
        match (self.state, msg) {
            
            (State::StIdle, Message::MsgAcquire) => {
                log::debug!("apply outbound : MsgAcquire");
                Ok(Self {
                state: State::StAcquiring,
                ..self
            })},
            (State::StAcquired, Message::MsgQuery(MsgRequest::MsgNextTx)) => Ok(Self {
                state: State::StBusy(StBusyKind::NextTx),
                ..self
            }),
            (State::StAcquired, Message::MsgQuery(MsgRequest::MsgHasTx(_))) => Ok(Self {
                state: State::StBusy(StBusyKind::HasTx),
                ..self
            }),
            
            (State::StAcquired, Message::MsgQuery(MsgRequest::MsgGetSizes)) => Ok(Self {
                state: State::StBusy(StBusyKind::GetSizes),
                ..self
            }),
            (State::StAcquired, Message::MsgAcquire) => Ok(Self {
                state: State::StAcquiring,
                ..self
            }),
            (State::StAcquired, Message::MsgQuery(MsgRequest::MsgRelease)) => Ok(Self {
                state: State::StIdle,
                ..self
            }),
            (State::StIdle, Message::MsgDone) => Ok(Self {
                state: State::StDone,
                ..self
            }),

            _ => panic!("PANIC! Cannot match outbound")
        }
    }

    fn apply_inbound(self, msg: Self::Message) -> Transition<Self> {
        log::debug!("apply inbound");
        match (&self.state , msg) {
            (State::StAcquiring, Message::MsgAcquired(s))                                                                       => self.on_acquired(s),
            (State::StBusy(StBusyKind::NextTx), Message::MsgResponse(MsgResponse::MsgReplyNextTx(tx)))               => self.on_reply_next_tx(tx),
            (State::StBusy(StBusyKind::HasTx), Message::MsgResponse(MsgResponse::MsgReplyHasTx(arg)))                          => self.on_reply_has_tx(arg),
            (State::StBusy(StBusyKind::GetSizes), Message::MsgResponse(MsgResponse::MsgReplyGetSizes(msc)))  => self.on_reply_get_size(msc),
            (state, msg) => Err(MachineError::invalid_msg::<Self>(&state, &msg)),

        }
    }

}