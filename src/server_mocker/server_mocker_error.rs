//! # `server_mocker_error`
//!
//! `server_mocker_error` is a type representing an error raised by a server mocker. It's mainly errors raised by the underlying socket server.
//!
//! The error is raised directly during call to [`ServerMocker`](crate::server_mocker::ServerMocker) methods, or when the server mocker is running asynchronously and an error occurs.
//!
//! If so, errors can be retrieved with [`ServerMocker::pop_server_error`](crate::server_mocker::ServerMocker::pop_server_error) method.

use std::io;
use std::net::SocketAddr;
use std::sync::mpsc::SendError;

use crate::server_mocker_instruction::Instruction;

/// Represents an error raised by a server mocker.
///
/// The error is raised directly during call to [`ServerMocker`](crate::server_mocker::ServerMocker) methods, or when the server mocker is running asynchronously and an error occurs.
///
/// If so, errors can be retrieved with [`ServerMocker::pop_server_error`](crate::server_mocker::ServerMocker::pop_server_error) method.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum ServerMockerError {
    #[error("{}: Failed to bind TCP listener on port {0}: {1}", self.fatal_str())]
    UnableToBindListener(u16, io::Error),
    #[error("{}: Failed to get local address of a listener: {0}", self.fatal_str())]
    UnableToGetLocalAddress(io::Error),
    #[error("{}: Failed to accept incoming connection on {0}: {1}", self.fatal_str())]
    UnableToAcceptConnection(SocketAddr, io::Error),
    #[error("{}: Failed to send instructions list to TCP server mocker: {0}", self.fatal_str())]
    UnableToSendInstructions(SendError<Vec<Instruction>>),
    #[error("{}: Failed to set read timeout on TCP stream: {0}", self.fatal_str())]
    UnableToSetReadTimeout(io::Error),
    #[error("{}: Failed to read from TCP stream: {0}", self.fatal_str())]
    UnableToReadTcpStream(io::Error),
    #[error("{}: Failed to write to TCP stream: {0}", self.fatal_str())]
    UnableToWriteTcpStream(io::Error),
    #[error("{}: Failed to receive message from client: {0}", self.fatal_str())]
    UnableToReadUdpStream(io::Error),
    #[error("{}: SendMessage instruction received before a ReceiveMessage", self.fatal_str())]
    GotSendMessageBeforeReceiveMessage,
    #[error("{}: Failed to send message to client: {0}", self.fatal_str())]
    FailedToSendUdpMessage(io::Error),
}

impl ServerMockerError {
    /// Indicate if this is a fatal error
    pub fn is_fatal(&self) -> bool {
        match self {
            ServerMockerError::UnableToBindListener(_, _)
            | ServerMockerError::UnableToGetLocalAddress(_)
            | ServerMockerError::UnableToAcceptConnection(_, _)
            | ServerMockerError::UnableToSetReadTimeout(_) => true,

            ServerMockerError::UnableToSendInstructions(_)
            | ServerMockerError::UnableToReadTcpStream(_)
            | ServerMockerError::UnableToWriteTcpStream(_)
            | ServerMockerError::UnableToReadUdpStream(_)
            | ServerMockerError::GotSendMessageBeforeReceiveMessage
            | ServerMockerError::FailedToSendUdpMessage(_) => false,
        }
    }

    fn fatal_str(&self) -> &'static str {
        if self.is_fatal() {
            "Fatal"
        } else {
            "Non fatal"
        }
    }
}
