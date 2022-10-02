//! # server_mocker_instruction
//!
//! Instructions for the mocked server.

/// Represents socket message sent and received by the server mocker.
pub type BinaryMessage = Vec<u8>;

/// Type of network instruction executed by the server mocker.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerMockerInstruction {
    /// Send given message to the client
    SendMessage(BinaryMessage),
    /// Wait for a message to be received. The message could be recovered with [TcpServerMocker::pop_received_message()](../tcp_server_mocker/struct.TcpServerMocker.html#method.pop_received_message)
    ReceiveMessage,
    /// Stop the exchange with the client, close the connection
    StopExchange,
}

/// Represents a list of network instructions to be executed by the server mocker
///
/// The list is executed in order, one instruction at a time.
///
/// The list is executed in a loop, until the [StopExchange](enum.ServerMockerInstruction.html#variant.StopExchange) instruction is received.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerMockerInstructionsList {
    pub(crate) instructions: Vec<ServerMockerInstruction>,
}

/// Creates a new ServerMockerInstructionsList with the given instructions
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
    /// Creates a new ServerMockerInstructionsList without instruction
    pub fn new() -> ServerMockerInstructionsList {
        ServerMockerInstructionsList {
            instructions: Vec::new(),
        }
    }

    /// Creates a new ServerMockerInstructionsList with the given instructions
    ///
    /// Takes a slice of ServerMockerInstruction and clone it into the new ServerMockerInstructionsList
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
    /// Message is given as a [BinaryMessage](type.BinaryMessage.html)
    pub fn add_send_message(&mut self, message: BinaryMessage) {
        self.instructions
            .push(ServerMockerInstruction::SendMessage(message));
    }

    /// Add instruction for sending a message to the client
    ///
    /// Takes ownership of self and returns a new ServerMockerInstructionsList
    ///
    /// Message is given as a [BinaryMessage](type.BinaryMessage.html)
    pub fn with_added_send_message(mut self, message: BinaryMessage) -> Self {
        self.add_send_message(message);
        self
    }

    /// Add instruction for waiting for a message to be received from the client
    ///
    /// Takes self as a mutable reference
    ///
    /// The message could be recovered with [TcpServerMocker::pop_received_message()](../tcp_server_mocker/struct.TcpServerMocker.html#method.pop_received_message)
    pub fn add_receive_message(&mut self) {
        self.instructions
            .push(ServerMockerInstruction::ReceiveMessage);
    }

    /// Add instruction for waiting for a message to be received from the client
    ///
    /// Takes ownership of self and returns a new ServerMockerInstructionsList
    ///
    /// The message could be recovered with [TcpServerMocker::pop_received_message()](../tcp_server_mocker/struct.TcpServerMocker.html#method.pop_received_message)
    pub fn with_added_receive_message(mut self) -> Self {
        self.add_receive_message();
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
