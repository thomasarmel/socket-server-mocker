pub mod tcp_server_mocker;
pub mod udp_server_mocker;

use crate::server_mocker_instruction::{BinaryMessage, ServerMockerInstruction, ServerMockerInstructionsList};

pub trait ServerMocker {
    fn new(port: u16) -> Self;

    fn listening_port(&self) -> u16;

    fn add_mock_instructions_list(&self, instructions_list: ServerMockerInstructionsList);

    fn add_mock_instructions(&self, instructions: &[ServerMockerInstruction]) {
        self.add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions(
            instructions,
        ));
    }

    fn pop_received_message(&self) -> Option<BinaryMessage>;
}