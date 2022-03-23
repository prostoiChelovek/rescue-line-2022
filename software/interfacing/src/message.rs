use crate::commands::Command;

use bincode::{Decode, Encode, error::{EncodeError, DecodeError}};
use heapless::Vec;

pub const MAX_MESSAGE_LEN: usize = 40;  
pub const ECC_LEN: usize = 8;

pub type MessageBuffer = Vec<u8, MAX_MESSAGE_LEN>;
pub type IdType = u32;

#[derive(Encode, Decode, PartialEq, Debug)]
pub enum Message {
    Command(IdType, Command),
    Ack(IdType),
    Done(IdType),
}

impl Message {
    pub fn serialize(&self) -> Result<MessageBuffer, MessageSerializeErorr> {
        let mut buffer: MessageBuffer = Vec::new();
        buffer.resize(buffer.capacity(), 0).unwrap();

        let reed_solomon_encoder = reed_solomon::Encoder::new(ECC_LEN);

        let data_size = bincode::encode_into_slice(self, &mut buffer[..], Self::get_config())?;
        let reed_solomon_encoded = reed_solomon_encoder.encode(&buffer[..data_size]);

        buffer.resize(reed_solomon_encoded.len(), 0).unwrap();
        buffer.copy_from_slice(&reed_solomon_encoded);

        Ok(buffer)
    }

    pub fn deserialize(buff: &[u8]) -> Result<Self, MessageDeserializeErorr> {
        let reed_solomon_decoder = reed_solomon::Decoder::new(ECC_LEN);
        let decoded = reed_solomon_decoder.correct(buff, None)?;

        let (result, _): (Self, _) = bincode::decode_from_slice(&decoded[..], Self::get_config())?;
        Ok(result)
    }

    fn get_config() -> bincode::config::Configuration {
        bincode::config::standard()
    }
}

#[derive(Debug)]
pub enum MessageSerializeErorr {
    Encode(EncodeError)
}

impl From<EncodeError> for MessageSerializeErorr {
    fn from(err: EncodeError) -> Self {
        Self::Encode(err)
    }
}

#[derive(Debug)]
pub enum MessageDeserializeErorr {
    Ecc(reed_solomon::DecoderError),
    Decode(DecodeError)
}

impl From<reed_solomon::DecoderError> for MessageDeserializeErorr {
    fn from(err: reed_solomon::DecoderError) -> Self {
        Self::Ecc(err)
    }
}

impl From<DecodeError> for MessageDeserializeErorr {
    fn from(err: DecodeError) -> Self {
        Self::Decode(err)
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::SetSpeedParams;

    use super::*;

    #[test]
    fn deserialize_test() {
        let msg = Message::Command(42,
                                   Command::SetSpeed(SetSpeedParams {
                                       left: -100, right: 42
                                   }));
        let serialized = msg.serialize().unwrap();
        let deserialized = Message::deserialize(&serialized[..]).unwrap();
        assert_eq!(deserialized, msg)
    }

    #[test]
    fn deserialize_corrupted_test() {
        let msg = Message::Command(42,
                                   Command::SetSpeed(SetSpeedParams {
                                       left: -100, right: 42
                                   }));
        let mut serialized = msg.serialize().unwrap();
        for i in 0..5 {
            serialized[i] = 0x0;
        }

        let deserialized = Message::deserialize(&serialized[..]).unwrap();
        assert_eq!(deserialized, msg)
    }
}
