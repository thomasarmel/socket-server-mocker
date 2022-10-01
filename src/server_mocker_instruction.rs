pub type BinaryMessage = Vec<u8>; // &[u8] ?

#[derive(Debug, Clone, PartialEq)]
pub enum ServerMockerInstruction {
    SendMessage(BinaryMessage),
    ReceiveMessage,
    StopExchange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerMockerInstructionsList {
    pub(crate) instructions: Vec<ServerMockerInstruction>,
}

impl ServerMockerInstructionsList {
    pub fn new() -> ServerMockerInstructionsList {
        ServerMockerInstructionsList {
            instructions: Vec::new(),
        }
    }

    pub fn new_with_instructions(instructions: &[ServerMockerInstruction]) -> ServerMockerInstructionsList {
        ServerMockerInstructionsList {
            instructions: instructions.to_vec(),
        }
    }

    pub fn add_send_message(&mut self, message: BinaryMessage) {
        self.instructions.push(ServerMockerInstruction::SendMessage(message));
    }

    pub fn with_added_send_message(mut self, message: BinaryMessage) -> Self {
        self.add_send_message(message);
        self
    }

    pub fn add_receive_message(&mut self) {
        self.instructions.push(ServerMockerInstruction::ReceiveMessage);
    }

    pub fn with_added_receive_message(mut self) -> Self {
        self.add_receive_message();
        self
    }

    pub fn add_stop_exchange(&mut self) {
        self.instructions.push(ServerMockerInstruction::StopExchange);
    }

    pub fn with_added_stop_exchange(mut self) -> Self {
        self.add_stop_exchange();
        self
    }
}