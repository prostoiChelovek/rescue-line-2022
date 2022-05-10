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

pub const BAUD_RATE: u32 = 1_000_000;
pub const START_BYTE: u8 = 0b1010101;
pub const RETRY_TIMEOUT: u32 = 50; // ms

const INTERFACING_QUEUE_SIZE: usize = 5;
const REGISTRY_CAPACITY: usize = 4;

pub struct Interfacing {
    send: Queue<MessageBuffer, INTERFACING_QUEUE_SIZE>,

    next_id: u32,

    waiting_execute: Queue<CommandId, REGISTRY_CAPACITY>,
    commands: FnvIndexMap<IdType, CommandHandle, REGISTRY_CAPACITY>,

    receiving_status: ReceiveStatus,
    receiving_buffer: MessageBuffer
}

impl Interfacing {
    pub fn new() -> Self {
        Self { 
            send: Queue::new(),
            next_id: 0,
            waiting_execute: Queue::new(),
            commands: FnvIndexMap::new(),
            receiving_status: ReceiveStatus::NotStarted,
            receiving_buffer: MessageBuffer::new()
        }
    }

    // TODO: it is ugly that you need to pass time here,
    //       but i dunno how to do this properly right now
    pub fn execute(&mut self, command: Command, time: Option<u32>) -> Result<CommandId, MessageSerializeErorr> {
        let id = self.next_id;
        self.next_id += 1;

        self.send_message(&Message::Command(id, command.clone()))?;

        self.commands.insert(id, CommandHandle::new(command, time)).unwrap();

        Ok(CommandId::new(id))
    }

    pub fn handle_received_byte(&mut self, byte: u8) -> Result<(), UpdateErorr> {
        match self.receiving_status {
            ReceiveStatus::NotStarted => {
                if byte == START_BYTE {
                    self.receiving_status = ReceiveStatus::Started;
                }
            },
            ReceiveStatus::Started => {
                let size: usize = byte.into();
                self.receiving_status = ReceiveStatus::Receiving(size);
            },
            ReceiveStatus::Receiving(size) => {
                self.receiving_buffer.push(byte).unwrap();
                if self.receiving_buffer.len() == size {
                    let message = Message::deserialize(&self.receiving_buffer);

                    self.receiving_buffer.clear();
                    self.receiving_status = ReceiveStatus::NotStarted;

                    // need to reset the state despite any errors
                    let message = message?;

                    match message {
                        Message::Command(id, cmd) => {
                            // TODO: right now, commands are executed only on the embedded side
                            //       and we do not need to keep track of the time when the command
                            //       was started
                            self.commands.insert(id, CommandHandle::new(cmd, None)).unwrap();
                            self.waiting_execute.enqueue(CommandId::new(id)).unwrap();
                        },
                        Message::Ack(id) => {
                            let handle = self.commands.get_mut(&id).ok_or(UpdateErorr::BadId(id))?;
                            handle.status = CommandExecutionStatus::Started;
                        },
                        Message::Done(id) => {
                            let handle = self.commands.get_mut(&id).ok_or(UpdateErorr::BadId(id))?;
                            handle.status = CommandExecutionStatus::Finished;
                        }
                    };
                }
            }
        };
        Ok(())
    }

    pub fn retry_timed_out(&mut self, time: u32) -> Result<(), MessageSerializeErorr> {
        // TODO: this sucks but i cannot call send_message right in the loop
        //       because both iterator and the method mutably borrow self
        let mut commands: heapless::Vec<Message, REGISTRY_CAPACITY> = heapless::Vec::new();

        for (id, cmd) in self.commands.iter_mut() {
            if cmd.status == CommandExecutionStatus::NotStarted{
                if let Some(enqueue_time) = cmd.enqueue_time {
                    if time.wrapping_sub(enqueue_time) > RETRY_TIMEOUT {
                        commands.push(Message::Command(*id, cmd.command)).unwrap();
                        cmd.enqueue_time = Some(time);
                    }
                }
            }
        }

        for cmd in commands {
            self.send_message(&cmd)?;
        }

        Ok(())
    }

    pub fn get_message_to_send(&mut self) -> Option<MessageBuffer> {
        self.send.dequeue()
    }

    pub fn is_finished(&self, id: CommandId) -> bool {
        self.commands[&id].status == CommandExecutionStatus::Finished
    }

    pub fn get_command(&self, id: CommandId) -> Command {
        self.commands[&id].command
    }

    pub fn start_executing(&mut self, id: CommandId) {
        self.commands[&id].status = CommandExecutionStatus::Started;
        self.send_message(&Message::Ack(id.into())).unwrap();
    }

    pub fn finish_executing(&mut self, id: CommandId) -> Result<(), MessageSerializeErorr> {
        self.send_message(&Message::Done(id.into()))?;

        self.commands.remove(&id);

        Ok(())
    }

    pub fn ack_finish(&mut self, id: CommandId) {
        self.commands.remove(&id);
    }

    pub fn get_command_to_execute(&mut self) -> Option<CommandId> {
        self.waiting_execute.dequeue()
    }

    fn add_message_preamble(msg: &mut MessageBuffer) {
        let mut tmp = MessageBuffer::new();
        tmp.push(START_BYTE).unwrap();
        tmp.push(msg.len().try_into().unwrap()).unwrap();
        tmp.extend_from_slice(&msg[..]).unwrap();
        *msg = take(&mut tmp);
    }

    fn send_message(&mut self, msg: &Message) -> Result<(), MessageSerializeErorr> {
        let mut encoded = msg.serialize()?;
        Self::add_message_preamble(&mut encoded);
        let encoded = encoded;
        self.send.enqueue(encoded).unwrap();

        Ok(())
    }
}

#[derive(Debug)]
enum ReceiveStatus {
    NotStarted,
    Started,
    Receiving(usize),
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
    pub(crate) status: CommandExecutionStatus,
    pub(crate) command: Command,
    pub(crate) enqueue_time: Option<u32>
}

impl CommandHandle {
    pub fn new(command: Command, enqueue_time: Option<u32>) -> Self {
        Self {
            status: CommandExecutionStatus::NotStarted,
            command,
            enqueue_time
        }
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

    fn consume_message(i: &mut Interfacing, msg: &Message) {
        let mut msg = msg.serialize().unwrap();
        Interfacing::add_message_preamble(&mut msg);
        for byte in msg {
            i.handle_received_byte(byte).unwrap();
        }
    }

    #[test]
    fn execute_handle_test() {
        let mut i = Interfacing::new();
        let id = i.execute(Command::Stop, None).unwrap();
        assert!(!i.is_finished(id));

        i.start_executing(id);
        let cmd = i.get_command(id);
        assert_eq!(cmd, Command::Stop);

        i.finish_executing(id).unwrap();
        assert!(i.get_message_to_send().is_some());
    }

    #[test]
    fn execute_send_test() {
        let mut i = Interfacing::new();
        i.execute(Command::Stop, None).unwrap();
        assert!(i.get_message_to_send().is_some());
        assert!(i.get_message_to_send().is_none());
    }

    #[test]
    fn done_test() {
        let mut i = Interfacing::new();
        let id = i.execute(Command::Stop, None).unwrap();

        let msg = Message::Done(*id);
        consume_message(&mut i, &msg);
        assert!(i.get_command_to_execute().is_none());

        assert!(i.is_finished(id));
    }

    #[test]
    fn many_commands_test() {
        let mut i = Interfacing::new();
        for _ in 0..50 {
            let mut ids: Vec<CommandId> = Vec::new();

            for _ in 0..REGISTRY_CAPACITY {
                let id = i.execute(Command::Stop, None).unwrap();
                assert!(i.get_message_to_send().is_some());
                ids.push(id);
            }
            for id in &ids {
                let msg = Message::Done(**id);
                consume_message(&mut i, &msg);
                assert!(i.get_command_to_execute().is_none());
            }
            for id in ids {
                assert!(i.is_finished(id));       
                i.ack_finish(id);
            }
        }
    }
}

