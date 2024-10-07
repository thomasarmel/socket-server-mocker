//! # `server_mocker`
//!
//! Mock an IP server for testing application that connect to external server.

use std::net::SocketAddr;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use crate::tcp_server::TcpMocker;
use crate::udp_server::UdpMocker;
use crate::ServerMockerError::UnableToSendInstructions;
use crate::{Instruction, ServerMockerError};

/// Options for the mocker, implemented by the specific TCP/UDP backends
pub trait MockerOptions: Clone {
    /// Socket address on which the server will listen. Will be set to `127.0.0.1:0` by default.
    fn socket_address(&self) -> SocketAddr;

    /// Timeout for the server to wait for a message from the client.
    fn net_timeout(&self) -> Duration;

    /// Run the server mocker with the given instructions
    fn run(
        self,
        instruction_rx: Receiver<Vec<Instruction>>,
        message_tx: Sender<Vec<u8>>,
        error_tx: Sender<ServerMockerError>,
    ) -> Result<SocketAddr, ServerMockerError>;
}

/// A socket server mocker, able to mock a TCP or UDP server to help test socket connections in a user app.
///
/// # TCP Example
///
/// ```
/// use std::io::Write;
/// use std::net::TcpStream;
/// use socket_server_mocker::ServerMocker;
/// use socket_server_mocker::Instruction::{self, ReceiveMessage, StopExchange};
///
/// let server = ServerMocker::tcp().unwrap();
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
///
/// # UDP Example
///
/// ```
/// use std::net::{SocketAddr, UdpSocket};
/// use socket_server_mocker::ServerMocker;
/// use socket_server_mocker::Instruction::{ReceiveMessage, SendMessage, StopExchange};
///
/// let server = ServerMocker::udp().unwrap();
/// // 0 = random port
/// let mut client = UdpSocket::bind("127.0.0.1:0").unwrap();
/// server.add_mock_instructions(vec![
///    ReceiveMessage,
///    SendMessage(vec![4, 5, 6]),
///    StopExchange,
/// ]).unwrap();
///
/// client.send_to(&[1, 2, 3], server.socket_address()).unwrap();
/// let mut buffer = [0; 3];
/// client.recv_from(&mut buffer).unwrap();
/// assert_eq!([4, 5, 6], buffer);
/// assert_eq!(Some(vec![1, 2, 3]), server.pop_received_message());
/// assert!(server.pop_server_error().is_none());
/// ```
pub struct ServerMocker<T> {
    options: T,
    socket_addr: SocketAddr,
    instruction_tx: Sender<Vec<Instruction>>,
    message_rx: Receiver<Vec<u8>>,
    error_rx: Receiver<ServerMockerError>,
}

impl ServerMocker<TcpMocker> {
    /// Create a new instance of the UDP server mocker on a random free port.
    /// The port can be retrieved with the [`ServerMocker::port`] method.
    pub fn tcp() -> Result<Self, ServerMockerError> {
        Self::tcp_with_port(0)
    }

    /// Create a new instance of the UDP server mocker on the given port.
    /// If the port is already in use, the method will return an error.
    pub fn tcp_with_port(port: u16) -> Result<Self, ServerMockerError> {
        let mut opts = TcpMocker::default();
        opts.socket_addr.set_port(port);
        Self::new_with_opts(opts)
    }
}

impl ServerMocker<UdpMocker> {
    /// Create a new instance of the UDP server mocker on a random free port.
    /// The port can be retrieved with the [`ServerMocker::port`] method.
    pub fn udp() -> Result<Self, ServerMockerError> {
        Self::udp_with_port(0)
    }

    /// Create a new instance of the UDP server mocker on the given port.
    /// If the port is already in use, the method will return an error.
    pub fn udp_with_port(port: u16) -> Result<Self, ServerMockerError> {
        let mut opts = UdpMocker::default();
        opts.socket_addr.set_port(port);
        Self::new_with_opts(opts)
    }
}

impl<T: MockerOptions> ServerMocker<T> {
    /// Get the options used to create the server mocker
    pub fn options(&self) -> &T {
        &self.options
    }

    /// Get the socket address on which the server is listening
    pub fn socket_address(&self) -> SocketAddr {
        self.socket_addr
    }

    /// Get the port on which the server is listening
    pub fn port(&self) -> u16 {
        self.socket_addr.port()
    }

    /// Add instructions to the server mocker
    pub fn add_mock_instructions(
        &self,
        instructions: Vec<Instruction>,
    ) -> Result<(), ServerMockerError> {
        self.instruction_tx
            .send(instructions)
            .map_err(UnableToSendInstructions)
    }

    /// Pop the last received message from the server mocker
    pub fn pop_received_message(&self) -> Option<Vec<u8>> {
        self.message_rx
            .recv_timeout(self.options.net_timeout())
            .ok()
    }

    /// Pop the last server error from the server mocker
    pub fn pop_server_error(&self) -> Option<ServerMockerError> {
        self.error_rx.recv_timeout(self.options.net_timeout()).ok()
    }

    /// Create a new instance of the TCP server mocker with the given options.
    ///
    /// # Panics
    /// It is assumed that threads can use messages channels without panicking.
    pub fn new_with_opts(options: T) -> Result<Self, ServerMockerError> {
        let (instruction_tx, instruction_rx) = mpsc::channel();
        let (message_tx, message_rx) = mpsc::channel();
        let (error_tx, error_rx) = mpsc::channel();
        let socket_addr = options.clone().run(instruction_rx, message_tx, error_tx)?;

        Ok(Self {
            options,
            socket_addr,
            instruction_tx,
            message_rx,
            error_rx,
        })
    }
}
