use std::fmt::Debug;
use thiserror::*;

use super::protocol::*;
use crate::multiplexer;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("attempted to send message after protocol is done")]
    ProtocolDone,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClientRequest {
    BlockRequest(Slot, Hash),
    BlockTxsRequest(Slot, Hash, TxMap),
    VoteRequest(Vec<(Slot, VoteIssuerId)>),
    RangeRequest {
        first: (Slot, Hash),
        last: (Slot, Hash),
    },
    Done,
}

pub struct Server(State, multiplexer::ChannelBuffer);

impl Server {
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(State::Idle, multiplexer::ChannelBuffer::new(channel))
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        use State::*;

        matches!(&self.0, Block | BlockTxs | Votes | BlockRange)
    }

    fn assert_agency_is_ours(&self) -> Result<(), ServerError> {
        if self.is_done() {
            Err(ServerError::ProtocolDone)
        } else if !self.has_agency() {
            Err(ServerError::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), ServerError> {
        if self.has_agency() {
            Err(ServerError::AgencyIsOurs)
        } else if self.is_done() {
            Err(ServerError::ProtocolDone)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message) -> Result<(), ServerError> {
        use Message::*;
        match (self.state(), msg) {
            (State::Block, Block(_)) => Ok(()),
            (State::BlockTxs, BlockTxs(_)) => Ok(()),
            (State::Votes, VoteDelivery(..)) => Ok(()),
            (State::BlockRange, NextBlockAndTxs(..)) => Ok(()),
            (State::BlockRange, LastBlockAndTxs(..)) => Ok(()),
            _ => Err(ServerError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ServerError> {
        use Message::*;

        if self.state() == &State::Idle
            && matches!(
                msg,
                BlockRequest(..)
                    | BlockTxsRequest(..)
                    | VoteRequest(..)
                    | RangeRequest { .. }
                    | Done
            )
        {
            Ok(())
        } else {
            Err(ServerError::InvalidInbound)
        }
    }

    pub async fn send_message(&mut self, msg: &Message) -> Result<(), ServerError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ServerError::Plexer)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message, ServerError> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(ServerError::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub async fn recv_while_idle(&mut self) -> Result<Option<ClientRequest>, ServerError> {
        use ClientRequest::*;

        match self.recv_message().await? {
            Message::BlockRequest(slot, hash) => {
                self.0 = State::Block;
                Ok(Some(BlockRequest(slot, hash)))
            }
            Message::BlockTxsRequest(slot, hash, tx_map) => {
                self.0 = State::BlockTxs;
                Ok(Some(BlockTxsRequest(slot, hash, tx_map)))
            }
            Message::VoteRequest(req) => {
                self.0 = State::Votes;
                Ok(Some(VoteRequest(req)))
            }
            Message::RangeRequest { first, last } => {
                self.0 = State::BlockRange;
                Ok(Some(RangeRequest { first, last }))
            }
            Message::Done => {
                self.0 = State::Done;

                Ok(None)
            }
            _ => Err(ServerError::InvalidInbound),
        }
    }

    pub async fn send_block(&mut self, response: EndorserBlock) -> Result<(), ServerError> {
        let msg = Message::Block(response);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn send_block_txs(&mut self, response: Vec<AnyCbor>) -> Result<(), ServerError> {
        let msg = Message::BlockTxs(response);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn send_vote_delivery(
        &mut self,
        response: Vec<LeiosVote>,
    ) -> Result<(), ServerError> {
        let msg = Message::VoteDelivery(response);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn send_next_block_and_txs(
        &mut self,
        block: EndorserBlock,
        txs: Vec<AnyCbor>,
    ) -> Result<(), ServerError> {
        let msg = Message::NextBlockAndTxs(block, txs);
        self.send_message(&msg).await?;
        self.0 = State::BlockRange;

        Ok(())
    }

    pub async fn send_last_block_and_txs(
        &mut self,
        block: EndorserBlock,
        txs: Vec<AnyCbor>,
    ) -> Result<(), ServerError> {
        let msg = Message::NextBlockAndTxs(block, txs);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }
}

pub fn bitmap_to_indices(bitmap: u64) -> Vec<usize> {
    (0..64)
        .rev()
        .enumerate()
        .filter(|(_, y)| (bitmap >> y) & 1 == 1)
        .map(|(x, _)| x)
        .collect()
}

pub fn bitmap_selection<TMap: IntoIterator<Item = (u16, u64)>, Tx: Clone>(
    tx_map: TMap,
    data: &[Tx],
) -> Vec<Tx> {
    tx_map
        .into_iter()
        .map(|(index, bitmap)| {
            bitmap_to_indices(bitmap)
                .into_iter()
                .map(move |i| data[64 * index as usize + i].clone())
        })
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_bitmap_indices() {
        assert_eq!(bitmap_to_indices(0xd000000000000002), vec![0, 1, 3, 62]);
        assert_eq!(bitmap_to_indices(0xe000000000000000), vec![0, 1, 2]);
    }

    #[test]
    fn test_bitmap_selection() {
        let map = BTreeMap::<u16, u64>::from([(0, 0x5000000000000000), (1, 0x8000000000000000)]);
        assert_eq!(
            bitmap_selection(map, &eb_tx()),
            [eb_tx()[1].clone(), eb_tx()[3].clone(), eb_tx()[64].clone()]
        );
    }

    fn eb_tx() -> Vec<Vec<u8>> {
        let mut list = vec![
            hex::decode(
                "58359719B92F47E7ABC8436813A42C1A5780C4ADDBF008E58E6CB8A4A3142067\
                 E2BD47E713EBDB3672446C8DD5697D6F29477DA5ABD6F9",
            )
            .unwrap(),
            hex::decode(
                "583551C27E9FD7D03351C243B98F6E33E9D29AD62CE9061580358B9CD4754505\
                 7B54A726322F849C5D73C01AE9881AA458F3A5F9DEA664",
            )
            .unwrap(),
            hex::decode(
                "58356764A66870461BD63041BF1028FF898BDC58E95DA9EA6E684EBCC225F97A\
                 ECF647BC7EA72BAC069D1FF9E3E9CB59C72181585FD4F0",
            )
            .unwrap(),
            hex::decode(
                "5903E584035557626AE726D5BCE067C798B43B3DE035C3618F86CA1CF31969EB\
                 B6711D354C445650D52E34F9E9A2057ECB363FE04FD3D5CE76B05E7C0CE7C563",
            )
            .unwrap(),
        ];
        list.append(&mut vec![vec![]; 60]);
        list.append(&mut vec![vec![10]]);

        list
    }
}
