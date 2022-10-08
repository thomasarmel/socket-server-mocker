use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::server_mocker::ServerMocker;
use crate::server_mocker_instruction::{BinaryMessage, ServerMockerInstruction, ServerMockerInstructionsList};

pub struct UdpServerMocker {
    listening_port: u16,
    instructions_sender: Sender<ServerMockerInstructionsList>,
    message_receiver: Receiver<BinaryMessage>,
}

impl ServerMocker for UdpServerMocker {
    fn new(port: u16) -> Self {
        let (instruction_tx, instruction_rx): (
            Sender<ServerMockerInstructionsList>,
            Receiver<ServerMockerInstructionsList>,
        ) = mpsc::channel();
        let (message_tx, message_rx): (Sender<BinaryMessage>, Receiver<BinaryMessage>) =
            mpsc::channel();

        let socket = UdpSocket::bind(format!("127.0.0.1:{}", port)).unwrap();

        let port = match port {
            0 => socket.local_addr().unwrap().port(),
            _ => port,
        };

        thread::spawn(move || {
            Self::handle_dgram_stream(socket, instruction_rx, message_tx);
        });

        Self {
            listening_port: port,
            instructions_sender: instruction_tx,
            message_receiver: message_rx,
        }
    }

    fn listening_port(&self) -> u16 {
        self.listening_port
    }

    fn add_mock_instructions_list(&self, instructions_list: ServerMockerInstructionsList) {
        self.instructions_sender.send(instructions_list).unwrap();
    }

    fn pop_received_message(&self) -> Option<BinaryMessage> {
        self.message_receiver
            .recv_timeout(std::time::Duration::from_millis(
                Self::DEFAULT_UDP_TIMEOUT_MS,
            ))
            .ok()
    }
}

impl UdpServerMocker {
    pub const DEFAULT_UDP_TIMEOUT_MS: u64 = 100;

    pub const DEFAULT_THREAD_RECEIVER_TIMEOUT_MS: u64 = 100;

    const MAX_UDP_PACKET_SIZE: usize = 65507;

    fn handle_dgram_stream(udp_socket: UdpSocket,
                           instructions_receiver: Receiver<ServerMockerInstructionsList>,
                           message_sender: Sender<BinaryMessage>) {
        udp_socket.set_read_timeout(Some(std::time::Duration::from_millis(
            Self::DEFAULT_UDP_TIMEOUT_MS,
        ))).unwrap();
        loop {
            let mut dgram_pair_addr : Option<SocketAddr> = None;
            let mut last_received_packed : Option<BinaryMessage> = None;
            // Timeout: if no more instruction is available and StopExchange hasn't been sent
            let instructions_list = match instructions_receiver.recv_timeout(
                std::time::Duration::from_millis(Self::DEFAULT_THREAD_RECEIVER_TIMEOUT_MS),
            ) {
                Ok(instructions_list) => instructions_list.instructions,
                Err(_) => {
                    break; // Stop server if no more instruction is available and StopExchange hasn't been sent
                }
            };
            for instruction in instructions_list {
                match instruction {
                    ServerMockerInstruction::SendMessage(binary_message) => {
                        udp_socket.send_to(&binary_message, dgram_pair_addr.unwrap()).unwrap();
                    }
                    ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(sent_message_calculator) => {
                        let message_to_send = sent_message_calculator(last_received_packed.clone());
                        if let Some(message_to_send) = message_to_send {
                            udp_socket.send_to(&message_to_send, dgram_pair_addr.unwrap()).unwrap();
                        }
                    }
                    ServerMockerInstruction::ReceiveMessage => {
                        let mut whole_received_packet: Vec<u8> = vec![0; Self::MAX_UDP_PACKET_SIZE];

                        let (bytes_read, packet_sender_addr) = udp_socket.recv_from(&mut whole_received_packet).unwrap();
                        dgram_pair_addr = Some(packet_sender_addr);

                        whole_received_packet.truncate(bytes_read);
                        last_received_packed = Some(whole_received_packet.clone());

                        message_sender.send(whole_received_packet).unwrap();
                    }
                    ServerMockerInstruction::ReceiveMessageWithMaxSize(max_message_size) => {
                        let mut whole_received_packet: Vec<u8> = vec![0; max_message_size];

                        let (bytes_read, packet_sender_addr) = udp_socket.recv_from(&mut whole_received_packet).unwrap();
                        dgram_pair_addr = Some(packet_sender_addr);

                        whole_received_packet.truncate(bytes_read);
                        last_received_packed = Some(whole_received_packet.clone());

                        message_sender.send(whole_received_packet).unwrap();
                    }
                    ServerMockerInstruction::StopExchange => {
                        return;
                    }
                }
            }
        }
    }
}