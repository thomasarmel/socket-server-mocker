//! # tcp_server_mocker
//!
//! Mock a TCP server for testing application that connect to external TCP server.

use crate::server_mocker::ServerMocker;
use crate::server_mocker_instruction::{
    BinaryMessage, ServerMockerInstruction, ServerMockerInstructionsList,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

/// A TCP server mocker
///
/// Can be used to mock a TCP server if the application you want to test uses TCP sockets to connect to a server.
///
/// Only 1 client can be connected to the mocked server. When the connection is closed, the mocked server will stop.
pub struct TcpServerMocker {
    listening_port: u16,
    instructions_sender: Sender<ServerMockerInstructionsList>,
    message_receiver: Receiver<BinaryMessage>,
}

/// TcpServerMocker implementation
///
/// # Example
/// ```
/// use std::io::Write;
/// use std::net::TcpStream;
/// use socket_server_mocker::server_mocker::ServerMocker;
/// use socket_server_mocker::server_mocker_instruction::{ServerMockerInstructionsList, ServerMockerInstruction};
/// use socket_server_mocker::tcp_server_mocker::TcpServerMocker;
///
/// let tcp_server_mocker = TcpServerMocker::new(1234);
/// let mut client = TcpStream::connect("127.0.0.1:1234").unwrap();
///
/// tcp_server_mocker.add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions([
///     ServerMockerInstruction::ReceiveMessage,
///     ServerMockerInstruction::StopExchange,
/// ].as_slice()));
/// client.write_all(&[1, 2, 3]).unwrap();
///
/// let mock_server_received_message = tcp_server_mocker.pop_received_message();
/// assert_eq!(Some(vec![1, 2, 3]), mock_server_received_message);
/// ```
impl ServerMocker for TcpServerMocker {
    fn new(port: u16) -> Self {
        let (instruction_tx, instruction_rx): (
            Sender<ServerMockerInstructionsList>,
            Receiver<ServerMockerInstructionsList>,
        ) = mpsc::channel();
        let (message_tx, message_rx): (Sender<BinaryMessage>, Receiver<BinaryMessage>) =
            mpsc::channel();

        let tcp_listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
        let port = match port {
            0 => tcp_listener.local_addr().unwrap().port(),
            _ => port,
        };

        thread::spawn(move || {
            let tcp_stream = tcp_listener.accept().unwrap().0; // We need to manage only 1 client
            Self::handle_connection(tcp_stream, instruction_rx, message_tx);
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
                Self::DEFAULT_NET_TIMEOUT_MS,
            ))
            .ok()
    }
}

/// Specific implementation methods and constants for TCP server mocker
impl TcpServerMocker {
    // Default buffer size for TCP socket
    const DEFAULT_SOCKET_READER_BUFFER_SIZE: usize = 1024;

    fn handle_connection(
        mut tcp_stream: TcpStream,
        instructions_receiver: Receiver<ServerMockerInstructionsList>,
        message_sender: Sender<BinaryMessage>,
    ) {
        tcp_stream
            .set_read_timeout(Some(std::time::Duration::from_millis(
                Self::DEFAULT_NET_TIMEOUT_MS,
            )))
            .unwrap();
        let mut last_received_message: Option<BinaryMessage> = None;

        loop {
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
                        tcp_stream.write_all(&binary_message).unwrap();
                    }
                    ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(
                        sent_message_calculator,
                    ) => {
                        // Call the closure to get the message to send
                        let message_to_send =
                            sent_message_calculator(last_received_message.clone());
                        // Send the message or skip if the closure returned None
                        if let Some(message_to_send) = message_to_send {
                            tcp_stream.write_all(&message_to_send).unwrap();
                        }
                    }
                    ServerMockerInstruction::ReceiveMessage => {
                        let whole_received_packet = Self::read_packet(&mut tcp_stream);
                        last_received_message = Some(whole_received_packet.clone());
                        message_sender.send(whole_received_packet).unwrap();
                    }
                    ServerMockerInstruction::ReceiveMessageWithMaxSize(max_message_size) => {
                        let mut whole_received_packet = Self::read_packet(&mut tcp_stream);
                        whole_received_packet.truncate(max_message_size);
                        last_received_message = Some(whole_received_packet.clone());
                        message_sender.send(whole_received_packet).unwrap();
                    }
                    ServerMockerInstruction::StopExchange => {
                        return;
                    }
                }
            }
        }
    }

    // Read a TCP packet from the client, using temporary buffer of size [DEFAULT_SOCKET_READER_BUFFER_SIZE](#associatedconstant.DEFAULT_SOCKET_READER_BUFFER_SIZE)
    fn read_packet(tcp_stream: &mut TcpStream) -> BinaryMessage {
        let mut whole_received_packet: Vec<u8> = Vec::new();
        let mut buffer = [0; Self::DEFAULT_SOCKET_READER_BUFFER_SIZE];

        loop {
            let bytes_read = tcp_stream.read(&mut buffer).unwrap();
            whole_received_packet.extend_from_slice(&buffer[..bytes_read]);
            if bytes_read < Self::DEFAULT_SOCKET_READER_BUFFER_SIZE {
                break;
            }
        }
        whole_received_packet
    }
}
