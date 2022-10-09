//! # server_mocker
//!
//! Mock an IP server for testing application that connect to external server.

pub mod tcp_server_mocker;
pub mod udp_server_mocker;

use crate::server_mocker_instruction::{
    BinaryMessage, ServerMockerInstruction, ServerMockerInstructionsList,
};

/// Trait that define the behavior of a network server mocker over an IP layer.
///
/// The mocker is able to receive and send messages to the application that is tested.
///
/// The mocker can be configured to send messages to the tested application depending on the messages received.
///
/// You can afterwards check that the messages sent by the tested application are the ones expected.
pub trait ServerMocker {
    /// Default timeout in milliseconds for the server to wait for a message from the client.
    const DEFAULT_NET_TIMEOUT_MS: u64 = 100;

    /// Timeout if no more instruction is available and [ServerMockerInstruction::StopExchange](crate::server_mocker_instruction::ServerMockerInstruction::StopExchange) hasn't been sent
    const DEFAULT_THREAD_RECEIVER_TIMEOUT_MS: u64 = 100;

    /// Creates a new server mocker
    ///
    /// # Arguments
    /// port - the port to listen on, should be the same as the port the application you want to test uses to connect to the server
    ///
    /// Will listen on the local interface, port should not be used by another listening application
    ///
    /// Note that only 1 client will be able to connect to the server in case you use TCP, and the messages that the server send back to the client will be sent to the last client that sent to the server.
    ///
    /// If port is set to 0, the OS will choose a free port. Then you can get the port with [listening_port](Self::listening_port)
    ///
    /// # Panics
    /// Will panic if the port is already used by another application, or in case of any other error with sockets
    ///
    /// Will panic in case of error with thread channel
    fn new(port: u16) -> Self;

    /// Returns the port on which the mock server is listening
    ///
    /// Listen only on local interface
    ///
    /// Port should not be used by another listening process
    fn listening_port(&self) -> u16;

    /// Adds a list of instructions to the server mocker
    ///
    /// The server mocker will execute the instructions in the order they are added
    ///
    /// This function could be called as many times as you want, until the connection is closed (event by the client or the server if received a [ServerMockerInstruction::StopExchange](crate::server_mocker_instruction::ServerMockerInstruction::StopExchange) instruction)
    ///
    /// If you push a [ServerMockerInstruction::SendMessage](crate::server_mocker_instruction::ServerMockerInstruction::SendMessage) instruction, you must ensure that there is a client connected to the server mocker
    ///
    /// If you push a [ServerMockerInstruction::ReceiveMessage](crate::server_mocker_instruction::ServerMockerInstruction::ReceiveMessage) instruction, you must ensure that the client will send a message to the server mocker within the timeout defined in [ServerMocker::DEFAULT_NET_TIMEOUT_MS](Self::DEFAULT_NET_TIMEOUT_MS)
    ///
    /// # Panics
    /// Will panic in case of error with thread channel
    fn add_mock_instructions_list(&self, instructions_list: ServerMockerInstructionsList);

    /// Adds a slice of instructions to the server mocker
    ///
    /// The server mocker will execute the instructions in the order they are added
    ///
    /// This function could be called as many times as you want, until the connection is closed (event by the client or the server if received a [ServerMockerInstruction::StopExchange](crate::server_mocker_instruction::ServerMockerInstruction::StopExchange) instruction)
    ///
    /// If you push a [ServerMockerInstruction::SendMessage](crate::server_mocker_instruction::ServerMockerInstruction::SendMessage) instruction, you must ensure that there is a client connected to the server mocker
    ///
    /// If you push a [ServerMockerInstruction::ReceiveMessage](crate::server_mocker_instruction::ServerMockerInstruction::ReceiveMessage) instruction, you must ensure that the client will send a message to the server mocker within the timeout defined in [ServerMocker::DEFAULT_NET_TIMEOUT_MS](Self::DEFAULT_NET_TIMEOUT_MS)
    ///
    /// # Panics
    /// Will panic in case of error with thread channel
    fn add_mock_instructions(&self, instructions: &[ServerMockerInstruction]) {
        self.add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions(
            instructions,
        ));
    }

    /// Return first message received by the mock server on the messages queue
    ///
    /// If no message is available, wait during [ServerMocker::DEFAULT_NET_TIMEOUT_MS](Self::DEFAULT_NET_TIMEOUT_MS) and then return None
    ///
    /// If a message is available, will return the message and remove it from the queue
    fn pop_received_message(&self) -> Option<BinaryMessage>;
}
