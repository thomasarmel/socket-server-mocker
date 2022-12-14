//! # tcp_server_mocker
//!
//! Mock a TCP server for testing application that connect to external TCP server.

use crate::server_mocker::ServerMocker;
use crate::server_mocker_error::{ServerMockerError, ServerMockerErrorFatality};
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
    error_receiver: Receiver<ServerMockerError>,
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
/// let tcp_server_mocker = TcpServerMocker::new(1234).unwrap();
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
/// assert!(tcp_server_mocker.pop_server_error().is_none());
/// assert!(tcp_server_mocker.pop_server_error().is_none());
/// ```
impl ServerMocker for TcpServerMocker {
    fn new(port: u16) -> Result<Self, ServerMockerError> {
        let (instruction_tx, instruction_rx): (
            Sender<ServerMockerInstructionsList>,
            Receiver<ServerMockerInstructionsList>,
        ) = mpsc::channel();
        let (message_tx, message_rx): (Sender<BinaryMessage>, Receiver<BinaryMessage>) =
            mpsc::channel();
        let (error_tx, error_rx): (Sender<ServerMockerError>, Receiver<ServerMockerError>) =
            mpsc::channel();

        let tcp_listener = TcpListener::bind(format!("127.0.0.1:{}", port)).map_err(|e| {
            ServerMockerError::new(
                &format!("Failed to bind TCP listener on port {}: {}", port, e),
                ServerMockerErrorFatality::Fatal,
            )
        })?;
        let port = tcp_listener
            .local_addr()
            .map_err(|e| {
                ServerMockerError::new(
                    &format!("Failed to get local address of TCP listener: {}", e),
                    ServerMockerErrorFatality::Fatal,
                )
            })?
            .port();

        thread::spawn(move || {
            let tcp_stream = match tcp_listener.accept() {
                Ok(incoming_client) => incoming_client.0,
                Err(_) => {
                    error_tx
                        .send(ServerMockerError::new(
                            &format!("Failed to accept incoming client on port {}", port),
                            ServerMockerErrorFatality::Fatal,
                        ))
                        .unwrap();
                    return;
                }
            }; // We need to manage only 1 client
            Self::handle_connection(tcp_stream, instruction_rx, message_tx, error_tx);
        });

        Ok(Self {
            listening_port: port,
            instructions_sender: instruction_tx,
            message_receiver: message_rx,
            error_receiver: error_rx,
        })
    }

    fn listening_port(&self) -> u16 {
        self.listening_port
    }

    fn add_mock_instructions_list(
        &self,
        instructions_list: ServerMockerInstructionsList,
    ) -> Result<(), ServerMockerError> {
        self.instructions_sender
            .send(instructions_list)
            .map_err(|e| {
                ServerMockerError::new(
                    &format!(
                        "Failed to send instructions list to TCP server mocker: {}",
                        e
                    ),
                    ServerMockerErrorFatality::NonFatal,
                )
            })
    }

    fn pop_received_message(&self) -> Option<BinaryMessage> {
        self.message_receiver
            .recv_timeout(std::time::Duration::from_millis(
                Self::DEFAULT_NET_TIMEOUT_MS,
            ))
            .ok()
    }

    fn pop_server_error(&self) -> Option<ServerMockerError> {
        self.error_receiver
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
        error_sender: Sender<ServerMockerError>,
    ) {
        if tcp_stream
            .set_read_timeout(Some(std::time::Duration::from_millis(
                Self::DEFAULT_NET_TIMEOUT_MS,
            )))
            .is_err()
        {
            error_sender
                .send(ServerMockerError::new(
                    "Failed to set read timeout on TCP stream",
                    ServerMockerErrorFatality::Fatal,
                ))
                .unwrap();
            return;
        }
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
                        if let Err(e) = Self::send_packet(&mut tcp_stream, &binary_message) {
                            error_sender.send(e).unwrap();
                        }
                    }
                    ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(
                        sent_message_calculator,
                    ) => {
                        // Call the closure to get the message to send
                        let message_to_send =
                            sent_message_calculator(last_received_message.clone());
                        // Send the message or skip if the closure returned None
                        if let Some(message_to_send) = message_to_send {
                            if let Err(e) = Self::send_packet(&mut tcp_stream, &message_to_send) {
                                error_sender.send(e).unwrap();
                            }
                        }
                    }
                    ServerMockerInstruction::ReceiveMessage => {
                        let whole_received_packet = match Self::read_packet(&mut tcp_stream) {
                            Ok(whole_received_packet) => whole_received_packet,
                            Err(e) => {
                                error_sender.send(e).unwrap();
                                continue;
                            }
                        };
                        last_received_message = Some(whole_received_packet.clone());
                        message_sender.send(whole_received_packet).unwrap();
                    }
                    ServerMockerInstruction::ReceiveMessageWithMaxSize(max_message_size) => {
                        let mut whole_received_packet = match Self::read_packet(&mut tcp_stream) {
                            Ok(whole_received_packet) => whole_received_packet,
                            Err(e) => {
                                error_sender.send(e).unwrap();
                                continue;
                            }
                        };
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

    // Read a TCP packet from the client, using temporary buffer of size [DEFAULT_SOCKET_READER_BUFFER_SIZE](Self::DEFAULT_SOCKET_READER_BUFFER_SIZE)
    fn read_packet(tcp_stream: &mut TcpStream) -> Result<BinaryMessage, ServerMockerError> {
        let mut whole_received_packet: Vec<u8> = Vec::new();
        let mut buffer = [0; Self::DEFAULT_SOCKET_READER_BUFFER_SIZE];

        loop {
            let bytes_read = tcp_stream.read(&mut buffer).map_err(|e| {
                ServerMockerError::new(
                    &format!("Failed to read from TCP stream: {}", e),
                    ServerMockerErrorFatality::NonFatal,
                )
            })?;
            whole_received_packet.extend_from_slice(&buffer[..bytes_read]);
            if bytes_read < Self::DEFAULT_SOCKET_READER_BUFFER_SIZE {
                break;
            }
        }
        Ok(whole_received_packet)
    }

    fn send_packet(
        tcp_stream: &mut TcpStream,
        packet: &BinaryMessage,
    ) -> Result<(), ServerMockerError> {
        tcp_stream.write_all(&packet).map_err(|e| {
            ServerMockerError::new(
                &format!("Failed to write to TCP stream: {}", e),
                ServerMockerErrorFatality::NonFatal,
            )
        })
    }
}
