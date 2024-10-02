//! # `udp_server_mocker`
//!
//! Mock a UDP server for testing application that connect to external UDP server.

use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::server_mocker::ServerMocker;
use crate::server_mocker_error::ServerMockerError;
use crate::server_mocker_error::ServerMockerError::{
    FailedToSendUdpMessage, GotSendMessageBeforeReceiveMessage, UnableToBindListener,
    UnableToGetLocalAddress, UnableToReadUdpStream, UnableToSendInstructions,
    UnableToSetReadTimeout,
};
use crate::server_mocker_instruction::Instruction::{
    ReceiveMessageWithMaxSize, SendMessage, SendMessageDependingOnLastReceivedMessage,
};
use crate::server_mocker_instruction::{BinaryMessage, Instruction};

/// A UDP server mocker
///
/// Can be used to mock a UDP server if the application you want to test uses UDP sockets to connect to a server.
///
/// It's preferable that only 1 client sends messages to the mocked server.
/// When the object is dropped or a [stop instruction](Instruction::StopExchange) is received, the mocked server will stop.
/// The server will also stop in case no more instructions are available.
pub struct UdpServerMocker {
    socket_addr: SocketAddr,
    instruction_tx: Sender<Vec<Instruction>>,
    message_rx: Receiver<BinaryMessage>,
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
        let (instruction_tx, instruction_rx): (
            Sender<Vec<Instruction>>,
            Receiver<Vec<Instruction>>,
        ) = mpsc::channel();
        let (message_tx, message_rx) = mpsc::channel();
        let (error_tx, error_rx) = mpsc::channel();

        let addr: SocketAddr = ([127, 0, 0, 1], port).into();
        let listener = UdpSocket::bind(addr).map_err(|e| UnableToBindListener(port, e))?;

        let socket_addr = listener.local_addr().map_err(UnableToGetLocalAddress)?;

        thread::spawn(move || {
            Self::handle_dgram_stream(&listener, &instruction_rx, &message_tx, &error_tx);
        });

        Ok(Self {
            socket_addr,
            instruction_tx,
            message_rx,
            error_rx,
        })
    }
}

/// `UdpServerMocker` implementation
///
/// # Example
/// ```
/// use std::net::{SocketAddr, UdpSocket};
/// use socket_server_mocker::server_mocker::ServerMocker;
/// use socket_server_mocker::server_mocker_instruction::Instruction::{ReceiveMessage, SendMessage, StopExchange};
/// use socket_server_mocker::udp_server_mocker::UdpServerMocker;
///
/// let udp_server_mocker = UdpServerMocker::new().unwrap();
/// // 0 = random port
/// let mut client = UdpSocket::bind("127.0.0.1:0").unwrap();
/// let server_addr = SocketAddr::from(([127, 0, 0, 1], udp_server_mocker.port()));
///
/// udp_server_mocker.add_mock_instructions(vec![
///    ReceiveMessage,
///    SendMessage(vec![4, 5, 6]),
///    StopExchange,
/// ]).unwrap();
///
/// client.send_to(&[1, 2, 3], server_addr).unwrap();
/// let mut buffer = [0; 3];
/// client.recv_from(&mut buffer).unwrap();
/// assert_eq!([4, 5, 6], buffer);
/// assert_eq!(Some(vec![1, 2, 3]), udp_server_mocker.pop_received_message());
/// assert!(udp_server_mocker.pop_server_error().is_none());
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

    fn pop_received_message(&self) -> Option<BinaryMessage> {
        self.message_rx
            .recv_timeout(Duration::from_millis(Self::DEFAULT_NET_TIMEOUT_MS))
            .ok()
    }

    fn pop_server_error(&self) -> Option<ServerMockerError> {
        self.error_rx
            .recv_timeout(Duration::from_millis(Self::DEFAULT_NET_TIMEOUT_MS))
            .ok()
    }
}

/// Specific implementation methods and constants for UDP server mocker
impl UdpServerMocker {
    // Maximum size of a UDP packet in bytes, specified in RFC 768
    const MAX_UDP_PACKET_SIZE: usize = 65507;

    fn handle_dgram_stream(
        connection: &UdpSocket,
        instructions_receiver: &Receiver<Vec<Instruction>>,
        message_sender: &Sender<BinaryMessage>,
        error_sender: &Sender<ServerMockerError>,
    ) {
        let timeout = Some(Duration::from_millis(Self::DEFAULT_NET_TIMEOUT_MS));
        if let Err(e) = connection.set_read_timeout(timeout) {
            error_sender.send(UnableToSetReadTimeout(e)).unwrap();
            return;
        }

        // Last message received with the address of the client, used to send the response
        let mut last_received_packed_with_addr: Option<(SocketAddr, BinaryMessage)> = None;

        // Timeout: if no more instruction is available and StopExchange hasn't been sent
        // Stop server if no more instruction is available and StopExchange hasn't been sent
        while let Ok(instructions_list) = instructions_receiver.recv_timeout(Duration::from_millis(
            Self::DEFAULT_THREAD_RECEIVER_TIMEOUT_MS,
        )) {
            for instruction in instructions_list {
                match instruction {
                    SendMessage(binary_message) => {
                        if let Err(e) = Self::send_packet_to_last_client(
                            connection,
                            &binary_message,
                            &last_received_packed_with_addr,
                        ) {
                            error_sender.send(e).unwrap();
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
                            if let Err(e) = Self::send_packet_to_last_client(
                                connection,
                                &message_to_send,
                                &last_received_packed_with_addr,
                            ) {
                                error_sender.send(e).unwrap();
                            }
                        }
                    }
                    Instruction::ReceiveMessage => {
                        let received_packet_with_addr =
                            match Self::receive_packet(connection, Self::MAX_UDP_PACKET_SIZE) {
                                Ok(received) => received,
                                Err(e) => {
                                    error_sender.send(e).unwrap();
                                    continue;
                                }
                            };

                        last_received_packed_with_addr = Some((
                            received_packet_with_addr.0,
                            received_packet_with_addr.1.clone(),
                        ));
                        message_sender.send(received_packet_with_addr.1).unwrap();
                    }
                    ReceiveMessageWithMaxSize(max_message_size) => {
                        match Self::receive_packet(connection, max_message_size) {
                            Ok(received) => {
                                last_received_packed_with_addr =
                                    Some((received.0, received.1.clone()));
                                message_sender.send(received.1).unwrap();
                            }
                            Err(e) => error_sender.send(e).unwrap(),
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
        udp_socket: &UdpSocket,
        max_packet_size: usize,
    ) -> Result<(SocketAddr, BinaryMessage), ServerMockerError> {
        let mut whole_received_packet: Vec<u8> = vec![0; max_packet_size];

        let (bytes_read, packet_sender_addr) = udp_socket
            .recv_from(&mut whole_received_packet)
            .map_err(UnableToReadUdpStream)?;

        // Remove the extra bytes
        whole_received_packet.truncate(bytes_read);

        Ok((packet_sender_addr, whole_received_packet))
    }

    fn send_packet_to_last_client(
        udp_socket: &UdpSocket,
        message_to_send: &BinaryMessage,
        last_received_packed_with_addr: &Option<(SocketAddr, BinaryMessage)>,
    ) -> Result<(), ServerMockerError> {
        // Last message received with the address of the client, used to send the response
        last_received_packed_with_addr
            .as_ref()
            .ok_or(GotSendMessageBeforeReceiveMessage)?;

        udp_socket
            .send_to(
                message_to_send,
                last_received_packed_with_addr.as_ref().unwrap().0,
            )
            .map_err(FailedToSendUdpMessage)?;
        Ok(())
    }
}
