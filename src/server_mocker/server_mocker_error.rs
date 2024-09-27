//! # server_mocker_error
//!
//! `server_mocker_error` is a type representing an error raised by a server mocker. It's mainly errors raised by the underlying socket server.
//!
//! The error is raised directly during call to [`ServerMocker`](crate::server_mocker::ServerMocker) methods, or when the server mocker is running asynchronously and an error occurs.
//!
//! If so, errors can be retrieved with [`ServerMocker::pop_server_error`](crate::server_mocker::ServerMocker::pop_server_error) method.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Represents the fatalities of a server mocker error.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ServerMockerErrorFatality {
    /// The error is fatal, the server mocker will stop.
    Fatal,
    /// The error is not fatal, the server mocker will continue.
    NonFatal,
}

impl ServerMockerErrorFatality {
    /// Returns true if the error is fatal, false otherwise.
    pub fn is_fatal(&self) -> bool {
        match self {
            Self::Fatal => true,
            Self::NonFatal => false,
        }
    }
}

/// Will display "Fatal" or "Non-fatal" depending on the error fatality.
impl Display for ServerMockerErrorFatality {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fatal => write!(f, "Fatal"),
            Self::NonFatal => write!(f, "Non fatal"),
        }
    }
}

/// Represents an error raised by a server mocker.
///
/// The error is raised directly during call to [`ServerMocker`](crate::server_mocker::ServerMocker) methods, or when the server mocker is running asynchronously and an error occurs.
///
/// If so, errors can be retrieved with [`ServerMocker::pop_server_error`](crate::server_mocker::ServerMocker::pop_server_error) method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerMockerError {
    /// The error message.
    pub message: String,
    /// The error [fatality](ServerMockerErrorFatality) - fatal if the mocked server stopped.
    pub fatality: ServerMockerErrorFatality,
}

impl ServerMockerError {
    pub(crate) fn new(message: &str, fatality: ServerMockerErrorFatality) -> Self {
        Self {
            message: message.to_string(),
            fatality,
        }
    }

    /// Returns true if the error is fatal, false otherwise.
    pub fn is_fatal(&self) -> bool {
        self.fatality.is_fatal()
    }
}

/// Will display:
///
/// "{Fatal | Non fatal}: {error message}"
impl Display for ServerMockerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.fatality, self.message)
    }
}

/// Ensure that `std::error::Error` is implemented for `ServerMockerError`
impl Error for ServerMockerError {}
