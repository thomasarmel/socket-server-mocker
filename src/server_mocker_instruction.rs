//! # `server_mocker_instruction`
//!
//! Instructions sent by the testing code to the mocked server.

/// Represents socket message sent and received by the server mocker.
pub type BinaryMessage = Vec<u8>;

/// Type of network instruction executed by the server mocker.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Send given message to the client
    SendMessage(BinaryMessage),
    /// Send a message to the client depending on the last received message
    ///
    /// If the given function returns None, no message is sent
    ///
    /// # Example
    /// ```
    /// use socket_server_mocker::server_mocker_instruction::{BinaryMessage, Instruction};
    /// use socket_server_mocker::server_mocker_instruction::Instruction::SendMessageDependingOnLastReceivedMessage;
    /// SendMessageDependingOnLastReceivedMessage(|last_received_message: Option<BinaryMessage>| {
    ///    if let Some(last_received_message) = last_received_message {
    ///       if last_received_message == vec![0x01, 0x02, 0x03] {
    ///         Some(vec![0x04, 0x05, 0x06])
    ///      } else {
    ///        None
    ///     }
    ///   } else {
    ///    None
    /// }
    /// });
    /// ```
    SendMessageDependingOnLastReceivedMessage(fn(Option<BinaryMessage>) -> Option<BinaryMessage>),
    /// Wait for a message to be received.
    ///
    /// The message could be recovered with [`ServerMocker::pop_received_message`](crate::server_mocker::ServerMocker::pop_received_message)
    ReceiveMessage,
    /// Wait for a message to be received with a maximum size (useful in UDP).
    ///
    /// If the message is bigger than the given size, the message is truncated.
    ///
    /// The message could be recovered with [`ServerMocker::pop_received_message`](crate::server_mocker::ServerMocker::pop_received_message)
    ReceiveMessageWithMaxSize(usize),
    /// Stop the exchange with the client, close the connection in case of TCP
    StopExchange,
}
