use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::Instruction::{
    self, ReceiveMessageWithMaxSize, SendMessage, SendMessageDependingOnLastReceivedMessage,
};
use crate::ServerMocker;
use crate::ServerMockerError::{
    self, FailedToSendUdpMessage, GotSendMessageBeforeReceiveMessage, UnableToBindListener,
    UnableToGetLocalAddress, UnableToReadUdpStream, UnableToSendInstructions,
    UnableToSetReadTimeout,
};

// FIXME: consider combining Udp and Tcp options into a single struct
/// Options for the TCP server mocker
#[derive(Debug, Clone)]
pub struct UdpMockerOptions {
    /// Socket address on which the server will listen. Will be set to `127.0.0.1:0` by default.
    pub socket_addr: SocketAddr,
    /// Timeout for the server to wait for a message from the client.
    pub net_timeout: Duration,
    /// Timeout if no more instruction is available and [`Instruction::StopExchange`] hasn't been sent
    pub rx_timeout: Duration,
    /// Maximum size of a UDP packet in bytes, specified in RFC 768
    pub max_packet_size: usize,
}

impl Default for UdpMockerOptions {
    fn default() -> Self {
        Self {
            socket_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            net_timeout: Duration::from_millis(100),
            rx_timeout: Duration::from_secs(100),
            max_packet_size: 65507,
        }
    }
}

/// A UDP server mocker
///
/// Can be used to mock a UDP server if the application you want to test uses UDP sockets to connect to a server.
///
/// It's preferable that only 1 client sends messages to the mocked server.
/// When the object is dropped or a [stop instruction](Instruction::StopExchange) is received, the mocked server will stop.
/// The server will also stop in case no more instructions are available.
pub struct UdpServerMocker {
    options: UdpMockerOptions,
    socket_addr: SocketAddr,
    instruction_tx: Sender<Vec<Instruction>>,
    message_rx: Receiver<Vec<u8>>,
    error_rx: Receiver<ServerMockerError>,
}

impl UdpServerMocker {
    /// Create a new instance of the UDP server mocker on a random free port.
    /// The port can be retrieved with the [`ServerMocker::port`] method.
    pub fn new() -> Result<Self, ServerMockerError> {
        Self::new_with_port(0)
    }

    /// Create a new instance of the UDP server mocker on the given port.
    /// If the port is already in use, the method will return an error.
    pub fn new_with_port(port: u16) -> Result<Self, ServerMockerError> {
        let mut opts = UdpMockerOptions::default();
        opts.socket_addr.set_port(port);
        Self::new_with_opts(opts)
    }

    /// Create a new instance of the TCP server mocker with the given options.
    ///
    /// # Panics
    /// It is assumed that threads can use messages channels without panicking.
    pub fn new_with_opts(options: UdpMockerOptions) -> Result<Self, ServerMockerError> {
        let (instruction_tx, instruction_rx) = mpsc::channel();
        let (message_tx, message_rx) = mpsc::channel();
        let (error_tx, error_rx) = mpsc::channel();

        let listener = UdpSocket::bind(options.socket_addr)
            .map_err(|e| UnableToBindListener(options.socket_addr, e))?;
        let socket_addr = listener.local_addr().map_err(UnableToGetLocalAddress)?;

        let options_copy = options.clone();
        thread::spawn(move || {
            UdpServerImpl {
                options: options_copy,
                connection: listener,
                instruction_rx,
                message_tx,
                error_tx,
            }
            .handle_dgram_stream();
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
    pub fn options(&self) -> &UdpMockerOptions {
        &self.options
    }
}

/// `UdpServerMocker` implementation
///
/// # Example
/// ```
/// use std::net::{SocketAddr, UdpSocket};
/// use socket_server_mocker::ServerMocker;
/// use socket_server_mocker::Instruction::{ReceiveMessage, SendMessage, StopExchange};
/// use socket_server_mocker::UdpServerMocker;
///
/// let server = UdpServerMocker::new().unwrap();
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
impl ServerMocker for UdpServerMocker {
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
struct UdpServerImpl {
    options: UdpMockerOptions,
    connection: UdpSocket,
    instruction_rx: Receiver<Vec<Instruction>>,
    message_tx: Sender<Vec<u8>>,
    error_tx: Sender<ServerMockerError>,
}

/// Specific implementation methods and constants for UDP server mocker
impl UdpServerImpl {
    fn handle_dgram_stream(&self) {
        let timeout = Some(self.options.net_timeout);
        if let Err(e) = self.connection.set_read_timeout(timeout) {
            self.error_tx.send(UnableToSetReadTimeout(e)).unwrap();
            return;
        }

        // Last message received with the address of the client, used to send the response
        let mut last_received_packed_with_addr: Option<(SocketAddr, Vec<u8>)> = None;

        // Timeout: if no more instruction is available and StopExchange hasn't been sent
        // Stop server if no more instruction is available and StopExchange hasn't been sent
        while let Ok(instructions) = self.instruction_rx.recv_timeout(self.options.rx_timeout) {
            for instruction in instructions {
                match instruction {
                    SendMessage(binary_message) => {
                        if let Err(e) = self.send_packet_to_last_client(
                            &binary_message,
                            &last_received_packed_with_addr,
                        ) {
                            self.error_tx.send(e).unwrap();
                        }
                    }
                    SendMessageDependingOnLastReceivedMessage(sent_message_calculator) => {
                        // Pass None if no message has been received yet
                        let message_to_send =
                            sent_message_calculator(match last_received_packed_with_addr {
                                Some((_, ref message)) => Some(message.clone()),
                                None => None,
                            });
                        if let Some(message_to_send) = message_to_send {
                            if let Err(e) = self.send_packet_to_last_client(
                                &message_to_send,
                                &last_received_packed_with_addr,
                            ) {
                                self.error_tx.send(e).unwrap();
                            }
                        }
                    }
                    Instruction::ReceiveMessage => {
                        let received_packet_with_addr =
                            match self.receive_packet(self.options.max_packet_size) {
                                Ok(received) => received,
                                Err(e) => {
                                    self.error_tx.send(e).unwrap();
                                    continue;
                                }
                            };

                        last_received_packed_with_addr = Some((
                            received_packet_with_addr.0,
                            received_packet_with_addr.1.clone(),
                        ));
                        self.message_tx.send(received_packet_with_addr.1).unwrap();
                    }
                    ReceiveMessageWithMaxSize(max_message_size) => {
                        match self.receive_packet(max_message_size) {
                            Ok(received) => {
                                last_received_packed_with_addr =
                                    Some((received.0, received.1.clone()));
                                self.message_tx.send(received.1).unwrap();
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

    fn receive_packet(
        &self,
        max_packet_size: usize,
    ) -> Result<(SocketAddr, Vec<u8>), ServerMockerError> {
        let mut whole_received_packet: Vec<u8> = vec![0; max_packet_size];

        let (bytes_read, packet_sender_addr) = self
            .connection
            .recv_from(&mut whole_received_packet)
            .map_err(UnableToReadUdpStream)?;

        // Remove the extra bytes
        whole_received_packet.truncate(bytes_read);

        Ok((packet_sender_addr, whole_received_packet))
    }

    fn send_packet_to_last_client(
        &self,
        message_to_send: &[u8],
        last_received_packed_with_addr: &Option<(SocketAddr, Vec<u8>)>,
    ) -> Result<(), ServerMockerError> {
        // Last message received with the address of the client, used to send the response
        last_received_packed_with_addr
            .as_ref()
            .ok_or(GotSendMessageBeforeReceiveMessage)?;

        self.connection
            .send_to(
                message_to_send,
                last_received_packed_with_addr.as_ref().unwrap().0,
            )
            .map_err(FailedToSendUdpMessage)?;
        Ok(())
    }
}
