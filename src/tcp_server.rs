//! # `tcp_server_mocker`
//!
//! Mock a TCP server for testing application that connect to external TCP server.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::Instruction::{
    self, ReceiveMessageWithMaxSize, SendMessage, SendMessageDependingOnLastReceivedMessage,
};
use crate::ServerMocker;
use crate::ServerMockerError::{
    self, UnableToAcceptConnection, UnableToBindListener, UnableToGetLocalAddress,
    UnableToReadTcpStream, UnableToSendInstructions, UnableToSetReadTimeout,
    UnableToWriteTcpStream,
};

/// A TCP server mocker
///
/// Can be used to mock a TCP server if the application you want to test uses TCP sockets to connect to a server.
///
/// Only 1 client can be connected to the mocked server. When the connection is closed, the mocked server will stop.
pub struct TcpServerMocker {
    socket_addr: SocketAddr,
    instruction_tx: Sender<Vec<Instruction>>,
    message_rx: Receiver<Vec<u8>>,
    error_rx: Receiver<ServerMockerError>,
}

impl TcpServerMocker {
    /// Create a new instance of the TCP server mocker on a random free port.
    /// The port can be retrieved with the [`ServerMocker::port`] method.
    pub fn new() -> Result<Self, ServerMockerError> {
        Self::new_with_port(0)
    }

    /// Create a new instance of the TCP server mocker on the given port.
    /// If the port is already in use, the method will return an error.
    ///
    /// # Panics
    /// It is assumed that threads can use messages channels without panicking.
    pub fn new_with_port(port: u16) -> Result<Self, ServerMockerError> {
        let (instruction_tx, instruction_rx) = mpsc::channel();
        let (message_tx, message_rx) = mpsc::channel();
        let (error_tx, error_rx) = mpsc::channel();

        let addr: SocketAddr = ([127, 0, 0, 1], port).into();
        let listener = TcpListener::bind(addr).map_err(|e| UnableToBindListener(port, e))?;

        let socket_addr = listener.local_addr().map_err(UnableToGetLocalAddress)?;

        thread::spawn(move || {
            let Ok(tcp_stream) = listener.accept().map_err(|e| {
                error_tx
                    .send(UnableToAcceptConnection(socket_addr, e))
                    .unwrap();
            }) else {
                return;
            };
            // We need to manage only 1 client
            Self::handle_connection(tcp_stream.0, &instruction_rx, &message_tx, &error_tx);
        });

        Ok(Self {
            socket_addr,
            instruction_tx,
            message_rx,
            error_rx,
        })
    }
}

/// `TcpServerMocker` implementation
///
/// # Example
/// ```
/// use std::io::Write;
/// use std::net::TcpStream;
/// use socket_server_mocker::ServerMocker;
/// use socket_server_mocker::Instruction::{self, ReceiveMessage, StopExchange};
/// use socket_server_mocker::TcpServerMocker;
///
/// let tcp_server_mocker = TcpServerMocker::new_with_port(1234).unwrap();
/// let mut client = TcpStream::connect("127.0.0.1:1234").unwrap();
///
/// tcp_server_mocker.add_mock_instructions(vec![
///     ReceiveMessage,
///     StopExchange,
/// ]).unwrap();
/// client.write_all(&[1, 2, 3]).unwrap();
///
/// let mock_server_received_message = tcp_server_mocker.pop_received_message();
/// assert_eq!(Some(vec![1, 2, 3]), mock_server_received_message);
/// assert!(tcp_server_mocker.pop_server_error().is_none());
/// assert!(tcp_server_mocker.pop_server_error().is_none());
/// ```
impl ServerMocker for TcpServerMocker {
    fn socket_address(&self) -> SocketAddr {
        self.socket_addr
    }

    fn add_mock_instructions(
        &self,
        instructions: Vec<Instruction>,
    ) -> Result<(), ServerMockerError> {
        self.instruction_tx
            .send(instructions)
            .map_err(UnableToSendInstructions)
    }

    fn pop_received_message(&self) -> Option<Vec<u8>> {
        self.message_rx.recv_timeout(Self::DEFAULT_NET_TIMEOUT).ok()
    }

    fn pop_server_error(&self) -> Option<ServerMockerError> {
        self.error_rx.recv_timeout(Self::DEFAULT_NET_TIMEOUT).ok()
    }
}

/// Specific implementation methods and constants for TCP server mocker
impl TcpServerMocker {
    // Default buffer size for TCP socket
    const DEFAULT_SOCKET_READER_BUFFER_SIZE: usize = 1024;

    fn handle_connection(
        mut connection: TcpStream,
        instruction_rx: &Receiver<Vec<Instruction>>,
        message_tx: &Sender<Vec<u8>>,
        error_tx: &Sender<ServerMockerError>,
    ) {
        let timeout = Some(Self::DEFAULT_NET_TIMEOUT);
        if let Err(e) = connection.set_read_timeout(timeout) {
            error_tx.send(UnableToSetReadTimeout(e)).unwrap();
            return;
        }
        let mut last_received_message: Option<Vec<u8>> = None;

        // Timeout: if no more instruction is available and StopExchange hasn't been sent
        // Stop server if no more instruction is available and StopExchange hasn't been sent
        while let Ok(instructions) =
            instruction_rx.recv_timeout(Self::DEFAULT_THREAD_RECEIVER_TIMEOUT)
        {
            for instruction in instructions {
                match instruction {
                    SendMessage(binary_message) => {
                        if let Err(e) = Self::send_packet(&mut connection, &binary_message) {
                            error_tx.send(e).unwrap();
                        }
                    }
                    SendMessageDependingOnLastReceivedMessage(sent_message_calculator) => {
                        // Call the closure to get the message to send
                        let message_to_send =
                            sent_message_calculator(last_received_message.clone());
                        // Send the message or skip if the closure returned None
                        if let Some(message_to_send) = message_to_send {
                            if let Err(e) = Self::send_packet(&mut connection, &message_to_send) {
                                error_tx.send(e).unwrap();
                            }
                        }
                    }
                    Instruction::ReceiveMessage => {
                        match Self::read_packet(&mut connection) {
                            Ok(whole_received_packet) => {
                                last_received_message = Some(whole_received_packet.clone());
                                message_tx.send(whole_received_packet).unwrap();
                            }
                            Err(e) => error_tx.send(e).unwrap(),
                        };
                    }
                    ReceiveMessageWithMaxSize(max_message_size) => {
                        match Self::read_packet(&mut connection) {
                            Ok(mut whole_received_packet) => {
                                whole_received_packet.truncate(max_message_size);
                                last_received_message = Some(whole_received_packet.clone());
                                message_tx.send(whole_received_packet).unwrap();
                            }
                            Err(e) => error_tx.send(e).unwrap(),
                        };
                    }
                    Instruction::StopExchange => {
                        return;
                    }
                }
            }
        }
    }

    // Read a TCP packet from the client, using temporary buffer of size [DEFAULT_SOCKET_READER_BUFFER_SIZE](Self::DEFAULT_SOCKET_READER_BUFFER_SIZE)
    fn read_packet(tcp_stream: &mut TcpStream) -> Result<Vec<u8>, ServerMockerError> {
        let mut whole_received_packet: Vec<u8> = Vec::new();
        let mut buffer = [0; Self::DEFAULT_SOCKET_READER_BUFFER_SIZE];

        loop {
            let bytes_read = tcp_stream
                .read(&mut buffer)
                .map_err(UnableToReadTcpStream)?;
            whole_received_packet.extend_from_slice(&buffer[..bytes_read]);
            if bytes_read < Self::DEFAULT_SOCKET_READER_BUFFER_SIZE {
                break;
            }
        }
        Ok(whole_received_packet)
    }

    fn send_packet(tcp_stream: &mut TcpStream, packet: &[u8]) -> Result<(), ServerMockerError> {
        tcp_stream.write_all(packet).map_err(UnableToWriteTcpStream)
    }
}