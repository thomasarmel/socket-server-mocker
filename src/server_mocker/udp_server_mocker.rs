//! # udp_server_mocker
//!
//! Mock a UDP server for testing application that connect to external UDP server.

use crate::server_mocker::ServerMocker;
use crate::server_mocker_error;
use crate::server_mocker_error::{ServerMockerError, ServerMockerErrorFatality};
use crate::server_mocker_instruction::{
    BinaryMessage, ServerMockerInstruction, ServerMockerInstructionsList,
};
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

/// A UDP server mocker
///
/// Can be used to mock a UDP server if the application you want to test uses UDP sockets to connect to a server.
///
/// It's preferable that only 1 client sends messages to the mocked server.
/// When the object is dropped or a [stop instruction](crate::server_mocker_instruction::ServerMockerInstruction::StopExchange) is received, the mocked server will stop.
/// The server will also stop in case no more instructions are available.
pub struct UdpServerMocker {
    listening_port: u16,
    instructions_sender: Sender<ServerMockerInstructionsList>,
    message_receiver: Receiver<BinaryMessage>,
    error_receiver: Receiver<ServerMockerError>,
}

/// UdpServerMocker implementation
///
/// # Example
/// ```
/// use std::net::{SocketAddr, UdpSocket};
/// use socket_server_mocker::server_mocker::ServerMocker;
/// use socket_server_mocker::server_mocker_instruction::{ServerMockerInstructionsList, ServerMockerInstruction};
/// use socket_server_mocker::udp_server_mocker::UdpServerMocker;
///
/// // 0 = random port
/// let udp_server_mocker = UdpServerMocker::new(0).unwrap();
/// let mut client = UdpSocket::bind("127.0.0.1:0").unwrap();
/// let server_addr = SocketAddr::from(([127, 0, 0, 1], udp_server_mocker.listening_port()));
///
/// udp_server_mocker.add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions([
///    ServerMockerInstruction::ReceiveMessage,
///    ServerMockerInstruction::SendMessage(vec![4, 5, 6]),
///    ServerMockerInstruction::StopExchange,
/// ].as_slice()));
/// client.send_to(&[1, 2, 3], server_addr).unwrap();
/// let mut buffer = [0; 3];
/// client.recv_from(&mut buffer).unwrap();
/// assert_eq!([4, 5, 6], buffer);
/// assert_eq!(Some(vec![1, 2, 3]), udp_server_mocker.pop_received_message());
/// assert!(udp_server_mocker.pop_server_error().is_none());
/// ```
impl ServerMocker for UdpServerMocker {
    fn new(port: u16) -> Result<Self, ServerMockerError> {
        let (instruction_tx, instruction_rx): (
            Sender<ServerMockerInstructionsList>,
            Receiver<ServerMockerInstructionsList>,
        ) = mpsc::channel();
        let (message_tx, message_rx): (Sender<BinaryMessage>, Receiver<BinaryMessage>) =
            mpsc::channel();
        let (error_tx, error_rx): (Sender<ServerMockerError>, Receiver<ServerMockerError>) =
            mpsc::channel();

        let socket = UdpSocket::bind(format!("127.0.0.1:{}", port)).map_err(|e| {
            ServerMockerError::new(
                &format!("Failed to create UDP socket on port {}: {}", port, e),
                ServerMockerErrorFatality::Fatal,
            )
        })?;

        let port = socket
            .local_addr()
            .map_err(|e| {
                ServerMockerError::new(
                    &format!("Failed to get local port of UDP socket: {}", e),
                    ServerMockerErrorFatality::Fatal,
                )
            })?
            .port();

        thread::spawn(move || {
            Self::handle_dgram_stream(socket, instruction_rx, message_tx, error_tx);
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
    ) -> Result<(), server_mocker_error::ServerMockerError> {
        self.instructions_sender
            .send(instructions_list)
            .map_err(|e| {
                ServerMockerError::new(
                    &format!("Failed to send instructions to UDP server mocker: {}", e),
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

/// Specific implementation methods and constants for UDP server mocker
impl UdpServerMocker {
    // Maximum size of a UDP packet in bytes, specified in RFC 768
    const MAX_UDP_PACKET_SIZE: usize = 65507;

    fn handle_dgram_stream(
        udp_socket: UdpSocket,
        instructions_receiver: Receiver<ServerMockerInstructionsList>,
        message_sender: Sender<BinaryMessage>,
        error_sender: Sender<ServerMockerError>,
    ) {
        if udp_socket
            .set_read_timeout(Some(std::time::Duration::from_millis(
                Self::DEFAULT_NET_TIMEOUT_MS,
            )))
            .is_err()
        {
            error_sender
                .send(ServerMockerError::new(
                    "Failed to set read timeout on UDP socket",
                    ServerMockerErrorFatality::Fatal,
                ))
                .unwrap();
            return;
        }

        // Last message received with the address of the client, used to send the response
        let mut last_received_packed_with_addr: Option<(SocketAddr, BinaryMessage)> = None;

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
                        if let Err(e) = Self::send_packet_to_last_client(
                            &udp_socket,
                            &binary_message,
                            &last_received_packed_with_addr,
                        ) {
                            error_sender.send(e).unwrap();
                        }
                    }
                    ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(
                        sent_message_calculator,
                    ) => {
                        // Pass None if no message has been received yet
                        let message_to_send =
                            sent_message_calculator(match last_received_packed_with_addr {
                                Some((_, ref message)) => Some(message.clone()),
                                None => None,
                            });
                        if let Some(message_to_send) = message_to_send {
                            if let Err(e) = Self::send_packet_to_last_client(
                                &udp_socket,
                                &message_to_send,
                                &last_received_packed_with_addr,
                            ) {
                                error_sender.send(e).unwrap();
                            }
                        }
                    }
                    ServerMockerInstruction::ReceiveMessage => {
                        let received_packet_with_addr =
                            match Self::receive_packet(&udp_socket, Self::MAX_UDP_PACKET_SIZE) {
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
                    ServerMockerInstruction::ReceiveMessageWithMaxSize(max_message_size) => {
                        let received_packet_with_addr =
                            match Self::receive_packet(&udp_socket, max_message_size) {
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
                    ServerMockerInstruction::StopExchange => {
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
            .map_err(|e| {
                ServerMockerError::new(
                    &format!("Failed to receive message from client: {}", e),
                    ServerMockerErrorFatality::NonFatal,
                )
            })?;

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
            .ok_or(ServerMockerError::new(
                "SendMessage instruction received before a ReceiveMessage",
                ServerMockerErrorFatality::NonFatal,
            ))?;
        udp_socket
            .send_to(
                &message_to_send,
                last_received_packed_with_addr.as_ref().unwrap().0.clone(),
            )
            .map_err(|e| {
                ServerMockerError::new(
                    &format!("Failed to send message to client: {}", e),
                    ServerMockerErrorFatality::NonFatal,
                )
            })?;
        Ok(())
    }
}
