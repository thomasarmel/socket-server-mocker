//! # `server_mocker_instruction`
//!
//! Instructions sent by the testing code to the mocked server.

/// Represents socket message sent and received by the server mocker.
pub type BinaryMessage = Vec<u8>;

/// Type of network instruction executed by the server mocker.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerMockerInstruction {
    /// Send given message to the client
    SendMessage(BinaryMessage),
    /// Send a message to the client depending on the last received message
    ///
    /// If the given function returns None, no message is sent
    ///
    /// # Example
    /// ```
    /// use socket_server_mocker::server_mocker_instruction::{BinaryMessage, ServerMockerInstruction};
    /// use socket_server_mocker::server_mocker_instruction::ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage;
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

/// Represents a list of network instructions to be executed by the server mocker
///
/// The list is executed in order, one instruction at a time.
///
/// The list is executed in a loop, until the [StopExchange](ServerMockerInstruction::StopExchange) instruction is received.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerMockerInstructionsList {
    pub(crate) instructions: Vec<ServerMockerInstruction>,
}

/// Creates a new `ServerMockerInstructionsList` with the given instructions
///
/// # Example
/// ```
/// use socket_server_mocker::server_mocker_instruction::{ServerMockerInstruction, ServerMockerInstructionsList};
/// let mut instructions_list = ServerMockerInstructionsList::new_with_instructions([
///     ServerMockerInstruction::ReceiveMessage,
///     ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()),
/// ].as_slice()).with_added_receive_message();
/// instructions_list.add_stop_exchange();
/// ```
impl ServerMockerInstructionsList {
    /// Creates a new `ServerMockerInstructionsList` without instruction
    pub fn new() -> ServerMockerInstructionsList {
        ServerMockerInstructionsList {
            instructions: Vec::new(),
        }
    }

    /// Creates a new `ServerMockerInstructionsList` with the given instructions
    ///
    /// Takes a slice of `ServerMockerInstruction` and clone it into the new `ServerMockerInstructionsList`
    pub fn new_with_instructions(
        instructions: &[ServerMockerInstruction],
    ) -> ServerMockerInstructionsList {
        ServerMockerInstructionsList {
            instructions: instructions.to_vec(),
        }
    }

    /// Add instruction for sending a message to the client
    ///
    /// Takes self as a mutable reference
    ///
    /// Message is given as a [`BinaryMessage`]
    pub fn add_send_message(&mut self, message: BinaryMessage) {
        self.instructions
            .push(ServerMockerInstruction::SendMessage(message));
    }

    /// Add instruction for sending a message to the client
    ///
    /// Takes ownership of self and returns a new `ServerMockerInstructionsList`
    ///
    /// Message is given as a [`BinaryMessage`]
    pub fn with_added_send_message(mut self, message: BinaryMessage) -> Self {
        self.add_send_message(message);
        self
    }

    /// Add instruction for sending a message to the client depending on the last received message
    ///
    /// Takes self as a mutable reference
    ///
    /// If the given function returns None, no message is sent
    pub fn add_send_message_depending_on_last_received_message(
        &mut self,
        message: fn(Option<BinaryMessage>) -> Option<BinaryMessage>,
    ) {
        self.instructions
            .push(ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(message));
    }

    /// Add instruction for sending a message to the client depending on the last received message
    ///
    /// Takes ownership of self and returns a new ServerMockerInstructionsList
    ///
    /// If the given function returns None, no message is sent
    pub fn with_added_send_message_depending_on_last_received_message(
        mut self,
        message: fn(Option<BinaryMessage>) -> Option<BinaryMessage>,
    ) -> Self {
        self.add_send_message_depending_on_last_received_message(message);
        self
    }

    /// Add instruction for waiting for a message to be received from the client
    ///
    /// Takes self as a mutable reference
    ///
    /// The message could be recovered with [TcpServerMocker::pop_received_message()](crate::server_mocker::ServerMocker::pop_received_message)
    pub fn add_receive_message(&mut self) {
        self.instructions
            .push(ServerMockerInstruction::ReceiveMessage);
    }

    /// Add instruction for waiting for a message to be received from the client
    ///
    /// Takes ownership of self and returns a new ServerMockerInstructionsList
    ///
    /// The message could be recovered with [TcpServerMocker::pop_received_message()](crate::server_mocker::ServerMocker::pop_received_message)
    pub fn with_added_receive_message(mut self) -> Self {
        self.add_receive_message();
        self
    }

    /// Add instruction for waiting for a message to be received from the client with a maximum size
    ///
    /// Takes self as a mutable reference
    ///
    /// If the message is bigger than the given size, the message is truncated.
    pub fn add_receive_message_with_max_size(&mut self, max_size: usize) {
        self.instructions
            .push(ServerMockerInstruction::ReceiveMessageWithMaxSize(max_size));
    }

    /// Add instruction for waiting for a message to be received from the client with a maximum size
    ///
    /// Takes ownership of self and returns a new ServerMockerInstructionsList
    ///
    /// If the message is bigger than the given size, the message is truncated.
    pub fn with_added_receive_message_with_max_size(mut self, max_size: usize) -> Self {
        self.add_receive_message_with_max_size(max_size);
        self
    }

    /// Add instruction for stopping the exchange with the client, closing the connection.
    ///
    /// Once the connection is closed, the client will receive an error when trying to send a message.
    ///
    /// Takes self as a mutable reference
    pub fn add_stop_exchange(&mut self) {
        self.instructions
            .push(ServerMockerInstruction::StopExchange);
    }

    /// Add instruction for stopping the exchange with the client, closing the connection.
    ///
    /// Once the connection is closed, the client will receive an error when trying to send a message.
    ///
    /// Takes ownership of self and returns a new ServerMockerInstructionsList
    pub fn with_added_stop_exchange(mut self) -> Self {
        self.add_stop_exchange();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_mocker_instructions_list() {
        let mut instructions_list = ServerMockerInstructionsList::new_with_instructions(
            [
                ServerMockerInstruction::ReceiveMessage,
                ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()),
            ]
            .as_slice(),
        )
        .with_added_receive_message();
        instructions_list.add_stop_exchange();

        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![
                    ServerMockerInstruction::ReceiveMessage,
                    ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()),
                    ServerMockerInstruction::ReceiveMessage,
                    ServerMockerInstruction::StopExchange,
                ]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_new() {
        let instructions_list = ServerMockerInstructionsList::new();
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_new_with_instructions() {
        let instructions_list = ServerMockerInstructionsList::new_with_instructions(
            [
                ServerMockerInstruction::ReceiveMessage,
                ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()),
            ]
            .as_slice(),
        );
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![
                    ServerMockerInstruction::ReceiveMessage,
                    ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()),
                ]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_add_send_message() {
        let mut instructions_list = ServerMockerInstructionsList::new();
        instructions_list.add_send_message("hello from server".as_bytes().to_vec());
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![ServerMockerInstruction::SendMessage(
                    "hello from server".as_bytes().to_vec()
                ),]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_with_added_send_message() {
        let instructions_list = ServerMockerInstructionsList::new()
            .with_added_send_message("hello from server".as_bytes().to_vec());
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![ServerMockerInstruction::SendMessage(
                    "hello from server".as_bytes().to_vec()
                ),]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_add_send_message_depending_on_last_received_message() {
        let mut instructions_list = ServerMockerInstructionsList::new();

        let message_generator = |message| {
            if let Some(message) = message {
                if message == "hello from client".as_bytes().to_vec() {
                    Some("hello from server".as_bytes().to_vec())
                } else {
                    None
                }
            } else {
                None
            }
        };

        instructions_list.add_send_message_depending_on_last_received_message(message_generator);
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![
                    ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(
                        message_generator
                    ),
                ]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_with_added_send_message_depending_on_last_received_message(
    ) {
        let message_generator = |message| {
            if let Some(message) = message {
                if message == "hello from client".as_bytes().to_vec() {
                    Some("hello from server".as_bytes().to_vec())
                } else {
                    None
                }
            } else {
                None
            }
        };

        let instructions_list = ServerMockerInstructionsList::new()
            .with_added_send_message_depending_on_last_received_message(message_generator);
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![
                    ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(
                        message_generator
                    ),
                ]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_add_receive_message() {
        let mut instructions_list = ServerMockerInstructionsList::new();
        instructions_list.add_receive_message();
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![ServerMockerInstruction::ReceiveMessage,]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_with_added_receive_message() {
        let instructions_list = ServerMockerInstructionsList::new().with_added_receive_message();
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![ServerMockerInstruction::ReceiveMessage,]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_add_receive_message_with_max_size() {
        let mut instructions_list = ServerMockerInstructionsList::new();
        instructions_list.add_receive_message_with_max_size(100);
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![ServerMockerInstruction::ReceiveMessageWithMaxSize(100),]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_with_added_receive_message_with_max_size() {
        let instructions_list =
            ServerMockerInstructionsList::new().with_added_receive_message_with_max_size(100);
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![ServerMockerInstruction::ReceiveMessageWithMaxSize(100),]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_add_stop_exchange() {
        let mut instructions_list = ServerMockerInstructionsList::new();
        instructions_list.add_stop_exchange();
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![ServerMockerInstruction::StopExchange,]
            }
        );
    }

    #[test]
    fn test_server_mocker_instructions_list_with_added_stop_exchange() {
        let instructions_list = ServerMockerInstructionsList::new().with_added_stop_exchange();
        assert_eq!(
            instructions_list,
            ServerMockerInstructionsList {
                instructions: vec![ServerMockerInstruction::StopExchange,]
            }
        );
    }
}
