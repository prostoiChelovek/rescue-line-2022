#![feature(arbitrary_self_types)]

use pyo3::exceptions::PyException;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use derive_more::Display;

#[pyclass(subclass)]
pub struct Interfacing(interfacing::Interfacing);

#[derive(Clone, Debug, FromPyObject)]
pub struct Empty(PyObject);

// TODO: i hate to write this boilerplate,
//       but i don't have enough time to write a proc macro for that
#[pyclass]
#[derive(Clone, Debug)]
pub enum Command {
    Stop,
    SetSpeed,
    OpenGripper,
    CloseGripper,
    LiftGripper,
    LowerGripper
}

#[pyclass]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SetSpeedParams(interfacing::commands::SetSpeedParams);

#[pymethods]
impl SetSpeedParams {
    #[new]
    pub fn new(left: i32, right: i32) -> Self {
        Self(interfacing::commands::SetSpeedParams { left, right })
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyCommand(Command, Option<Py<PyAny>>);

#[pymethods]
impl PyCommand {
    #[new]
    pub fn new(cmd: Command, params: Option<Py<PyAny>>) -> Self {
        Self(cmd, params)
    }
}

impl TryFrom<PyCommand> for interfacing::commands::Command {
    type Error = PyErr;

    fn try_from(cmd: PyCommand) -> Result<Self, Self::Error> {
        Python::with_gil(|py| -> PyResult<Self> {
            use interfacing::commands::Command as iCommand;
            Ok(match cmd.0 {
                Command::Stop => iCommand::Stop,
                Command::SetSpeed => {
                    let params: SetSpeedParams = cmd.1.ok_or(PyValueError::new_err("Expected parameters for the command"))?.extract(py)?;
                    iCommand::SetSpeed(params.0)
                },
                Command::OpenGripper => iCommand::OpenGripper,
                Command::CloseGripper => iCommand::CloseGripper,
                Command::LiftGripper => iCommand::LiftGripper,
                Command::LowerGripper => iCommand::LowerGripper
            })
        })
    }
}

#[pyclass]
#[derive(Clone, Copy)]
pub struct CommandId(interfacing::CommandId);

#[pymethods]
impl CommandId {
    pub fn __str__(&self) -> String {
        self.0.to_string()
    }
}

#[pyclass]
pub struct CommandHandle(interfacing::CommandHandle);

#[pyclass]
#[derive(Clone, PartialEq, Debug)]
pub struct MessageBuffer(interfacing::message::MessageBuffer);

#[pymethods]
impl MessageBuffer {
    #[new]
    pub fn new(obj: &PyAny) -> PyResult<Self> {
        let vec: Vec<u8> = obj.extract()?;
        Ok(Self(interfacing::message::MessageBuffer::from_iter(vec.into_iter())))
    }

    fn __iter__(self: PyRef<Self>) -> PyResult<Py<MessageBufferIterator>> {
        // TODO: clone is kinda inefficient, but i dunno how to get rid of it
        //       and it's not that big of a deal anyway
        let iter = MessageBufferIterator(self.0.clone().into_iter());
        Py::new(self.py(), iter)
    }
}

#[pyclass]
pub struct MessageBufferIterator(<interfacing::message::MessageBuffer as IntoIterator>::IntoIter);

#[pymethods]
impl MessageBufferIterator {
    fn __iter__<'a>(self: PyRef<'a, Self>) -> PyRef<'a, Self> {
        self
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<<interfacing::message::MessageBuffer as IntoIterator>::Item> {
        slf.0.next()
    }
}

#[pyclass]
#[derive(Debug, Display)]
#[display(fmt = "Cannot deserialize a message: {}", "0")]
pub struct MessageDeserializeErorr(interfacing::message::MessageDeserializeErorr);

impl std::error::Error for MessageDeserializeErorr {}

impl From<MessageDeserializeErorr> for PyErr {
    fn from(err: MessageDeserializeErorr) -> PyErr {
        PyException::new_err(err.to_string())
    }
}

#[pyclass]
#[derive(Debug, Display)]
#[display(fmt = "Cannot serialize a message: {}", "0")]
pub struct MessageSerializeErorr(interfacing::message::MessageSerializeErorr);

impl std::error::Error for MessageSerializeErorr {}

impl From<MessageSerializeErorr> for PyErr {
    fn from(err: MessageSerializeErorr) -> PyErr {
        PyException::new_err(err.to_string())
    }
}

#[pyclass]
#[derive(Debug, Display)]
#[display(fmt = "Update failed: {:?}", "self.0")]
pub struct UpdateErorr(interfacing::UpdateErorr);

impl std::error::Error for UpdateErorr {}

impl From<UpdateErorr> for PyErr {
    fn from(err: UpdateErorr) -> PyErr {
        PyException::new_err(err.to_string())
    }
}

#[pymethods]
impl Interfacing {
    #[new]
    pub fn new() -> Self {
        Interfacing(interfacing::Interfacing::new())
    }

    pub fn execute(&mut self, command: PyCommand) -> PyResult<CommandId> {
        let result = self.0.execute(command.try_into()?)
            .map_err(|e| MessageSerializeErorr(e))?;
        Ok(CommandId(result))
    }

    pub fn update(&mut self) -> PyResult<Option<CommandId>> {
        Ok(self.0.update().map(|r| r.map(|s| CommandId(s))).map_err(|e| UpdateErorr(e))?)
    }

    // TODO: figure out how to do this properly
    /*
    pub fn get_handle(&mut self, id: CommandId) -> Py<CommandHandle> {
        let handle = self.0.get_handle(id.0);
        Python::with_gil(|py| -> PyResult<Py<CommandHandle>> {
            let foo: Py<CommandHandle> = Py::new(py, CommandHandle(*handle))?;
            Ok(foo)
        }).unwrap()
    }
    */

    pub fn check_finished(&mut self, id: CommandId) -> bool {
        self.0.get_handle(id.0).is_finished()
    }

    pub fn get_message_to_send(&mut self) -> Option<MessageBuffer> {
        self.0.get_message_to_send().map(|m| MessageBuffer(m))
    }

    pub fn set_received_message(&mut self, message: MessageBuffer) {
        self.0.set_received_message(message.0)
    }

    pub fn ack_finish(&mut self, id: CommandId) {
        self.0.ack_finish(id.0)
    }

    #[classattr]
    #[allow(non_snake_case)]
    pub fn BAUD_RATE() -> usize {
        interfacing::BAUD_RATE
    }

    #[classattr]
    #[allow(non_snake_case)]
    pub fn START_BYTE() -> u8 {
        interfacing::START_BYTE
    }
}

#[pymodule]
fn interfacing_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyCommand>()?;
    m.add_class::<SetSpeedParams>()?;
    m.add_class::<Command>()?;
    m.add_class::<CommandId>()?;
    m.add_class::<MessageBuffer>()?;
    m.add_class::<Interfacing>()?;

    Ok(())
}
