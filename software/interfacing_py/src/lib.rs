use pyo3::exceptions::PyException;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use derive_more::Display;

#[pyclass]
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
}

#[pymodule]
fn interfacing_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyCommand>()?;
    m.add_class::<SetSpeedParams>()?;
    m.add_class::<Command>()?;
    m.add_class::<Interfacing>()?;

    Ok(())
}
