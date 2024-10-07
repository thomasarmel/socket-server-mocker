use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::Instruction::{
    self, ReceiveMessageWithMaxSize, SendMessage, SendMessageDependingOnLastReceivedMessage,
};
use crate::ServerMocker;
use crate::ServerMockerError::{
    self, UnableToAcceptConnection, UnableToBindListener, UnableToGetLocalAddress,
    UnableToReadTcpStream, UnableToSendInstructions, UnableToSetReadTimeout,
    UnableToWriteTcpStream,
};

// FIXME: consider consolidating options for both TCP & UDP
/// Options for the TCP server mocker
#[derive(Debug, Clone)]
pub struct TcpMockerOptions {
    /// Socket address on which the server will listen. Will be set to `127.0.0.1:0` by default.
    pub socket_addr: SocketAddr,
    /// Timeout for the server to wait for a message from the client.
    pub net_timeout: Duration,
    /// Timeout if no more instruction is available and [`Instruction::StopExchange`] hasn't been sent
    pub rx_timeout: Duration,
    /// Buffer size for TCP socket
    pub reader_buffer_size: usize,
}

impl Default for TcpMockerOptions {
    fn default() -> Self {
        Self {
            socket_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            net_timeout: Duration::from_millis(100),
            rx_timeout: Duration::from_secs(100),
            reader_buffer_size: 1024,
        }
    }
}

/// A TCP server mocker
///
/// Can be used to mock a TCP server if the application you want to test uses TCP sockets to connect to a server.
///
/// Only 1 client can be connected to the mocked server. When the connection is closed, the mocked server will stop.
pub struct TcpServerMocker {
    options: TcpMockerOptions,
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
    pub fn new_with_port(port: u16) -> Result<Self, ServerMockerError> {
        let mut opts = TcpMockerOptions::default();
        opts.socket_addr.set_port(port);
        Self::new_with_opts(opts)
    }

    /// Create a new instance of the TCP server mocker with the given options.
    ///
    /// # Panics
    /// It is assumed that threads can use messages channels without panicking.
    pub fn new_with_opts(options: TcpMockerOptions) -> Result<Self, ServerMockerError> {
        let (instruction_tx, instruction_rx) = mpsc::channel();
        let (message_tx, message_rx) = mpsc::channel();
        let (error_tx, error_rx) = mpsc::channel();

        let listener = TcpListener::bind(options.socket_addr)
            .map_err(|e| UnableToBindListener(options.socket_addr, e))?;
        let socket_addr = listener.local_addr().map_err(UnableToGetLocalAddress)?;

        let options_copy = options.clone();
        thread::spawn(move || match listener.accept() {
            Ok((stream, _addr)) => {
                TcpServerImpl {
                    options: options_copy,
                    stream,
                    instruction_rx,
                    message_tx,
                    error_tx,
                }
                .handle_connection();
            }
            Err(err) => {
                error_tx
                    .send(UnableToAcceptConnection(socket_addr, err))
                    .unwrap();
            }
        });

        Ok(Self {
            options,
            socket_addr,
            instruction_tx,
            message_rx,
            error_rx,
        })
    }

    /// Get the options used to create the server mocker
    pub fn options(&self) -> &TcpMockerOptions {
        &self.options
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
/// let server = TcpServerMocker::new().unwrap();
/// let mut client = TcpStream::connect(server.socket_address()).unwrap();
///
/// server.add_mock_instructions(vec![
///     ReceiveMessage,
///     StopExchange,
/// ]).unwrap();
/// client.write_all(&[1, 2, 3]).unwrap();
///
/// let mock_server_received_message = server.pop_received_message();
/// assert_eq!(Some(vec![1, 2, 3]), mock_server_received_message);
/// assert!(server.pop_server_error().is_none());
/// assert!(server.pop_server_error().is_none());
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
        self.message_rx.recv_timeout(self.options.net_timeout).ok()
    }

    fn pop_server_error(&self) -> Option<ServerMockerError> {
        self.error_rx.recv_timeout(self.options.net_timeout).ok()
    }
}

/// TCP server mocker thread implementation
struct TcpServerImpl {
    options: TcpMockerOptions,
    stream: TcpStream,
    instruction_rx: Receiver<Vec<Instruction>>,
    message_tx: Sender<Vec<u8>>,
    error_tx: Sender<ServerMockerError>,
}

/// TCP server mocker thread implementation
impl TcpServerImpl {
    fn handle_connection(&mut self) {
        let timeout = Some(self.options.net_timeout);
        if let Err(e) = self.stream.set_read_timeout(timeout) {
            self.error_tx.send(UnableToSetReadTimeout(e)).unwrap();
            return;
        }
        let mut last_received_message: Option<Vec<u8>> = None;

        // Timeout: if no more instruction is available and StopExchange hasn't been sent
        // Stop server if no more instruction is available and StopExchange hasn't been sent
        while let Ok(instructions) = self.instruction_rx.recv_timeout(self.options.rx_timeout) {
            for instruction in instructions {
                match instruction {
                    SendMessage(binary_message) => {
                        if let Err(e) = self.send_packet(&binary_message) {
                            self.error_tx.send(e).unwrap();
                        }
                    }
                    SendMessageDependingOnLastReceivedMessage(sent_message_calculator) => {
                        // Call the closure to get the message to send
                        let message_to_send =
                            sent_message_calculator(last_received_message.clone());
                        // Send the message or skip if the closure returned None
                        if let Some(message_to_send) = message_to_send {
                            if let Err(e) = self.send_packet(&message_to_send) {
                                self.error_tx.send(e).unwrap();
                            }
                        }
                    }
                    Instruction::ReceiveMessage => {
                        match self.read_packet() {
                            Ok(whole_received_packet) => {
                                last_received_message = Some(whole_received_packet.clone());
                                self.message_tx.send(whole_received_packet).unwrap();
                            }
                            Err(e) => self.error_tx.send(e).unwrap(),
                        };
                    }
                    ReceiveMessageWithMaxSize(max_message_size) => {
                        match self.read_packet() {
                            Ok(mut whole_received_packet) => {
                                whole_received_packet.truncate(max_message_size);
                                last_received_message = Some(whole_received_packet.clone());
                                self.message_tx.send(whole_received_packet).unwrap();
                            }
                            Err(e) => self.error_tx.send(e).unwrap(),
                        };
                    }
                    Instruction::StopExchange => {
                        return;
                    }
                }
            }
        }
    }

    /// Read a TCP packet from the client, using temporary buffer
    fn read_packet(&mut self) -> Result<Vec<u8>, ServerMockerError> {
        let mut whole_received_packet: Vec<u8> = Vec::new();
        // FIXME: not much point in reading into a buffer and copying, perhaps need to consolidate
        let mut buffer = vec![0; self.options.reader_buffer_size];

        loop {
            let bytes_read = self
                .stream
                .read(&mut buffer)
                .map_err(UnableToReadTcpStream)?;
            whole_received_packet.extend_from_slice(&buffer[..bytes_read]);
            if bytes_read < self.options.reader_buffer_size {
                break;
            }
        }
        Ok(whole_received_packet)
    }

    fn send_packet(&mut self, packet: &[u8]) -> Result<(), ServerMockerError> {
        self.stream
            .write_all(packet)
            .map_err(UnableToWriteTcpStream)
    }
}
