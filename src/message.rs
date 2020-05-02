use std::convert::From;
use serde_repr::*;
use serde::{Serialize, Deserialize};

pub enum Message {
    Received(String),
    Broadcast(String),
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum MessageType {
    ServerChoice, //0
    Message, //1
    ServersList,
    ServersStatus, //setup for fetching status
    ForcedMove,

    Unknown,
}

impl From<u64> for MessageType {
    fn from(v: u64) -> Self {
        match v {
            0 => MessageType::ServerChoice,
            1 => MessageType::Message,
            2 => MessageType::ServersList,
            3 => MessageType::ServersStatus,
            4 => MessageType::ForcedMove,
            _ => MessageType::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerMessageText {
    pub name: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerMessage<T> {
    pub message_type: MessageType,
    pub message: T,
}
