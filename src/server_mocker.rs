//! # `server_mocker`
//!
//! Mock an IP server for testing application that connect to external server.

use std::net::SocketAddr;
use std::time::Duration;

use crate::{Instruction, ServerMockerError};

/// Trait that define the behavior of a network server mocker over an IP layer.
///
/// The mocker is able to receive and send messages to the application that is tested.
///
/// The mocker can be configured to send messages to the tested application depending on the messages received.
///
/// You can later check that the messages sent by the tested application are the ones expected.
pub trait ServerMocker {
    /// Default timeout for the server to wait for a message from the client.
    const DEFAULT_NET_TIMEOUT: Duration = Duration::from_millis(100);

    /// Timeout if no more instruction is available and [`Instruction::StopExchange`] hasn't been sent
    const DEFAULT_THREAD_RECEIVER_TIMEOUT: Duration = Duration::from_secs(100);

    /// Returns the socket address on which the mock server is listening
    fn socket_address(&self) -> SocketAddr;

    /// Returns the port on which the mock server is listening
    ///
    /// Listen only on local interface
    ///
    /// Port should not be used by another listening process
    fn port(&self) -> u16 {
        self.socket_address().port()
    }

    /// Adds a slice of instructions to the server mocker
    ///
    /// The server mocker will execute the instructions in the order they are added
    ///
    /// This function could be called as many times as you want, until the connection is closed (event by the client or the server if received a [`Instruction::StopExchange`] instruction)
    ///
    /// If you push a [`Instruction::SendMessage`] instruction, you must ensure that there is a client connected to the server mocker
    ///
    /// If you push a [`Instruction::ReceiveMessage`] instruction, you must ensure that the client will send a message to the server mocker within the timeout defined in [`ServerMocker::DEFAULT_NET_TIMEOUT`]
    fn add_mock_instructions(
        &self,
        instructions: Vec<Instruction>,
    ) -> Result<(), ServerMockerError>;

    /// Return first message received by the mock server on the messages queue
    ///
    /// If no message is available, wait for [`ServerMocker::DEFAULT_NET_TIMEOUT`] and then return None
    ///
    /// If a message is available, will return the message and remove it from the queue
    fn pop_received_message(&self) -> Option<Vec<u8>>;

    /// Return first [error](ServerMockerError) received by the mock server on the errors queue
    ///
    /// If no error is available, wait for [`ServerMocker::DEFAULT_NET_TIMEOUT`] and then return None
    fn pop_server_error(&self) -> Option<ServerMockerError>;
}
