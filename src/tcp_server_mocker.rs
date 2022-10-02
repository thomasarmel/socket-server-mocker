//! # tcp_server_mocker
//!
//! Mock a TCP server for testing application that connect to external TCP server.

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
impl TcpServerMocker {
    /// Default timeout in milliseconds for the server to wait for a message from the client
    pub const DEFAULT_TCP_TIMEOUT_MS: u64 = 100;
    /// Timeout if no more instruction is available and [ServerMockerInstruction::StopExchange](../server_mocker_instruction/enum.ServerMockerInstruction.html#variant.StopExchange) hasn't been sent
    pub const DEFAULT_THREAD_RECEIVER_TIMEOUT_MS: u64 = 100;

    const DEFAULT_SOCKET_READER_BUFFER_SIZE: usize = 1024;

    /// Creates a new TCP server mocker
    ///
    /// # Arguments
    /// port - the port to listen on, should be the same as the port the application you want to test uses to connect to the server
    ///
    /// Will listen on the local interface, port should not be used by another listening application
    ///
    /// Note that only 1 client will be able to connect to the server
    ///
    /// If port is set to 0, the OS will choose a free port. Then you can get the port with [listening_port](#method.listening_port)
    ///
    /// # Panics
    /// Will panic if the port is already used by another application, or in case of any other error with TCP sockets
    ///
    /// Will panic in case of error with thread channel
    pub fn new(port: u16) -> Self {
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

    fn handle_connection(
        mut tcp_stream: TcpStream,
        instructions_receiver: Receiver<ServerMockerInstructionsList>,
        message_sender: Sender<BinaryMessage>,
    ) {
        tcp_stream
            .set_read_timeout(Some(std::time::Duration::from_millis(
                Self::DEFAULT_TCP_TIMEOUT_MS,
            )))
            .unwrap();
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
                    ServerMockerInstruction::ReceiveMessage => {
                        let mut whole_received_packet: Vec<u8> = Vec::new();
                        let mut buffer = [0; Self::DEFAULT_SOCKET_READER_BUFFER_SIZE];

                        loop {
                            let bytes_read = tcp_stream.read(&mut buffer).unwrap();
                            whole_received_packet.extend_from_slice(&buffer[..bytes_read]);
                            if bytes_read < Self::DEFAULT_SOCKET_READER_BUFFER_SIZE {
                                break;
                            }
                        }
                        message_sender.send(whole_received_packet).unwrap();
                    }
                    ServerMockerInstruction::StopExchange => {
                        return;
                    }
                }
            }
        }
    }

    /// Adds a list of instructions to the server mocker
    ///
    /// The server mocker will execute the instructions in the order they are added
    ///
    /// This function could be called as many times as you want, until the connection is closed (event by the client or the server if received a [ServerMockerInstruction::StopExchange](../server_mocker_instruction/enum.ServerMockerInstruction.html#variant.StopExchange) instruction)
    ///
    /// If you push a [ServerMockerInstruction::SendMessage](../server_mocker_instruction/enum.ServerMockerInstruction.html#variant.SendMessage) instruction, you must ensure that there is a client connected to the server mocker
    ///
    /// If you push a [ServerMockerInstruction::ReceiveMessage](../server_mocker_instruction/enum.ServerMockerInstruction.html#variant.ReceiveMessage) instruction, you must ensure that the client will send a message to the server mocker within the timeout defined in [TcpServerMocker::DEFAULT_TCP_TIMEOUT_MS](#associatedconstant.DEFAULT_TCP_TIMEOUT_MS)
    ///
    /// # Panics
    /// Will panic in case of error with thread channel
    pub fn add_mock_instructions_list(&self, instructions_list: ServerMockerInstructionsList) {
        self.instructions_sender.send(instructions_list).unwrap();
    }

    /// Adds a slice of instructions to the server mocker
    ///
    /// The server mocker will execute the instructions in the order they are added
    ///
    /// This function could be called as many times as you want, until the connection is closed (event by the client or the server if received a [ServerMockerInstruction::StopExchange](../server_mocker_instruction/enum.ServerMockerInstruction.html#variant.StopExchange) instruction)
    ///
    /// If you push a [ServerMockerInstruction::SendMessage](../server_mocker_instruction/enum.ServerMockerInstruction.html#variant.SendMessage) instruction, you must ensure that there is a client connected to the server mocker
    ///
    /// If you push a [ServerMockerInstruction::ReceiveMessage](../server_mocker_instruction/enum.ServerMockerInstruction.html#variant.ReceiveMessage) instruction, you must ensure that the client will send a message to the server mocker within the timeout defined in [TcpServerMocker::DEFAULT_TCP_TIMEOUT_MS](#associatedconstant.DEFAULT_TCP_TIMEOUT_MS)
    ///
    /// # Panics
    /// Will panic in case of error with thread channel
    pub fn add_mock_instructions(&self, instructions: &[ServerMockerInstruction]) {
        self.add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions(
            instructions,
        ));
    }

    /// Return first message received by the mock server on the messages queue
    ///
    /// If no message is available, wait during [TcpServerMocker::DEFAULT_TCP_TIMEOUT_MS](#associatedconstant.DEFAULT_TCP_TIMEOUT_MS) and then return None
    ///
    /// If a message is available, will return the message and remove it from the queue
    pub fn pop_received_message(&self) -> Option<BinaryMessage> {
        self.message_receiver
            .recv_timeout(std::time::Duration::from_millis(
                Self::DEFAULT_TCP_TIMEOUT_MS,
            ))
            .ok()
    }

    /// Returns the port on which the mock server is listening
    ///
    /// Listen only on local interface
    ///
    /// Port should not be used by another listening process
    pub fn listening_port(&self) -> u16 {
        self.listening_port
    }
}
