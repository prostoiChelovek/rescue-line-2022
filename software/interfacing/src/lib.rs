#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

pub mod commands;
pub mod message;

use crate::{
    commands::Command,
    message::{
        Message, MessageBuffer, IdType,
        MessageSerializeErorr, MessageDeserializeErorr
    },
};

use core::{ops::Deref, mem::take};

use heapless::{spsc::Queue, FnvIndexMap};

pub const BAUD_RATE: usize = 115200;
pub const START_BYTE: u8 = 0b1010101;

const INTERFACING_QUEUE_SIZE: usize = 2;
const REGISTRY_CAPACITY: usize = 4;

pub struct Interfacing {
    received: Queue<MessageBuffer, INTERFACING_QUEUE_SIZE>,
    send: Queue<MessageBuffer, INTERFACING_QUEUE_SIZE>,

    next_id: u32,

    commands: FnvIndexMap<IdType, CommandHandle, REGISTRY_CAPACITY>
}

impl Interfacing {
    pub fn new() -> Self {
        Self { 
            received: Queue::new(),
            send: Queue::new(),
            next_id: 0,
            commands: FnvIndexMap::new()
        }
    }

    pub fn execute(&mut self, command: Command) -> Result<CommandId, MessageSerializeErorr> {
        let id = self.next_id;
        self.next_id += 1;

        let msg = Message::Command(id, command.clone());

        let mut encoded = msg.serialize()?;
        Self::add_message_preamble(&mut encoded);
        let encoded = encoded;

        self.commands.insert(id, CommandHandle::new(command)).unwrap();
        self.send.enqueue(encoded).unwrap();

        Ok(CommandId::new(id))
    }

    pub fn update(&mut self) -> Result<Option<CommandId>, UpdateErorr> {
        if let Some(received) = self.received.dequeue() {
            let message = Message::deserialize(&received)?;
            match message {
                Message::Command(id, cmd) => {
                    self.commands.insert(id, CommandHandle::new(cmd)).unwrap();
                    Ok(Some(CommandId::new(id)))
                },
                Message::Ack(id) => {
                    let handle = self.commands.get_mut(&id).ok_or(UpdateErorr::BadId(id))?;
                    handle.start_executing();
                    Ok(None)
                },
                Message::Done(id) => {
                    let handle = self.commands.get_mut(&id).ok_or(UpdateErorr::BadId(id))?;
                    handle.finish_executing();
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    pub fn get_handle(&mut self, id: CommandId) -> &mut CommandHandle {
        &mut self.commands[&id]
    }

    pub fn get_message_to_send(&mut self) -> Option<MessageBuffer> {
        self.send.dequeue()
    }

    /// message should not contain preamble
    pub fn set_received_message(&mut self, message: MessageBuffer) {
        self.received.enqueue(message).unwrap();
    }

    pub fn ack_finish(&mut self, id: CommandId) {
        self.commands.remove(&id);
    }

    fn add_message_preamble(msg: &mut MessageBuffer) {
        let mut tmp = MessageBuffer::new();
        tmp.push(START_BYTE).unwrap();
        tmp.push(msg.len().try_into().unwrap()).unwrap();
        tmp.extend_from_slice(&msg[..]).unwrap();
        *msg = take(&mut tmp);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CommandId(IdType);

impl CommandId {
    pub(crate) fn new(id: IdType) -> Self { Self { 0: id } }
}

impl Deref for CommandId {
    type Target = IdType;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl From<CommandId> for IdType {
    fn from(id: CommandId) -> Self { *id }
}

#[derive(Debug, PartialEq)]
enum CommandExecutionStatus {
    NotStarted,
    Started,
    Finished
}

#[derive(Debug)]
pub struct CommandHandle {
    status: CommandExecutionStatus,
    command: Command
}

impl CommandHandle {
    pub fn new(command: Command) -> Self {
        Self {
            status: CommandExecutionStatus::NotStarted,
            command
        }
    }

    pub fn start_executing(&mut self) -> &Command {
        self.status = CommandExecutionStatus::Started;
        &self.command
    }

    pub fn finish_executing(&mut self) {
        self.status = CommandExecutionStatus::Finished;
    }

    pub fn is_finished(&self) -> bool {
        self.status == CommandExecutionStatus::Finished
    }
}

#[derive(Debug)]
pub enum UpdateErorr {
    Decode(MessageDeserializeErorr),
    BadId(IdType)
}

impl From<MessageDeserializeErorr> for UpdateErorr {
    fn from(err: MessageDeserializeErorr) -> Self {
        Self::Decode(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn execute_handle_test() {
        let mut i = Interfacing::new();
        let id = i.execute(Command::Stop).unwrap();
        let handle = i.get_handle(id);
        assert!(!handle.is_finished());

        let cmd = handle.start_executing();
        assert_eq!(*cmd, Command::Stop);

        handle.finish_executing();
        assert!(handle.is_finished());
    }

    #[test]
    fn execute_send_test() {
        let mut i = Interfacing::new();
        i.execute(Command::Stop).unwrap();
        assert!(i.get_message_to_send().is_some());
        assert!(i.get_message_to_send().is_none());
    }

    #[test]
    fn done_test() {
        let mut i = Interfacing::new();
        let id = i.execute(Command::Stop).unwrap();

        let msg = Message::Done(*id);
        i.set_received_message(msg.serialize().unwrap());
        assert!(i.update().unwrap().is_none());

        let handle = i.get_handle(id);
        assert!(handle.is_finished());
    }

    #[test]
    fn many_commands_test() {
        let mut i = Interfacing::new();
        for _ in 0..50 {
            let mut ids: Vec<CommandId> = Vec::new();

            for _ in 0..REGISTRY_CAPACITY {
                let id = i.execute(Command::Stop).unwrap();
                assert!(i.get_message_to_send().is_some());
                ids.push(id);
            }
            for id in &ids {
                let msg = Message::Done(**id);
                i.set_received_message(msg.serialize().unwrap());
                assert!(i.update().unwrap().is_none());
            }
            for id in ids {
                let handle = i.get_handle(id);
                assert!(handle.is_finished());       
                i.ack_finish(id);
            }
        }
    }
}

