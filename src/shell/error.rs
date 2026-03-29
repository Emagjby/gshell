use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ShellError {
    Io(std::io::Error),
    Message(String),
}

pub type ShellResult<T> = Result<T, ShellError>;

impl ShellError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl Display for ShellError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellError::Io(err) => write!(f, "io error: {err}"),
            ShellError::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ShellError {}

impl From<std::io::Error> for ShellError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
