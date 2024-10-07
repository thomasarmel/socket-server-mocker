use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::server_mocker::MockerOptions;
use crate::Instruction::{
    self, ReceiveMessageWithMaxSize, SendMessage, SendMessageDependingOnLastReceivedMessage,
};
use crate::ServerMockerError::{
    self, FailedToSendUdpMessage, GotSendMessageBeforeReceiveMessage, UnableToBindListener,
    UnableToGetLocalAddress, UnableToReadUdpStream, UnableToSetReadTimeout,
};

/// Options for the UDP server mocker
#[derive(Debug, Clone)]
pub struct UdpMocker {
    /// Socket address on which the server will listen. Will be set to `127.0.0.1:0` by default.
    pub socket_addr: SocketAddr,
    /// Timeout for the server to wait for a message from the client.
    pub net_timeout: Duration,
    /// Timeout if no more instruction is available and [`Instruction::StopExchange`] hasn't been sent
    pub rx_timeout: Duration,
    /// Maximum size of a UDP packet in bytes, specified in RFC 768
    pub max_packet_size: usize,
}

impl Default for UdpMocker {
    fn default() -> Self {
        Self {
            socket_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            net_timeout: Duration::from_millis(100),
            rx_timeout: Duration::from_millis(100),
            max_packet_size: 65507,
        }
    }
}

impl MockerOptions for UdpMocker {
    fn socket_address(&self) -> SocketAddr {
        self.socket_addr
    }

    fn net_timeout(&self) -> Duration {
        self.net_timeout
    }

    fn run(
        self,
        instruction_rx: Receiver<Vec<Instruction>>,
        message_tx: Sender<Vec<u8>>,
        error_tx: Sender<ServerMockerError>,
    ) -> Result<SocketAddr, ServerMockerError> {
        let connection = UdpSocket::bind(self.socket_addr)
            .map_err(|e| UnableToBindListener(self.socket_addr, e))?;
        let socket_addr = connection.local_addr().map_err(UnableToGetLocalAddress)?;

        thread::spawn(move || {
            UdpServerImpl {
                options: self,
                connection,
                instruction_rx,
                message_tx,
                error_tx,
            }
            .run();
        });

        Ok(socket_addr)
    }
}

/// TCP server mocker thread implementation
struct UdpServerImpl {
    options: UdpMocker,
    connection: UdpSocket,
    instruction_rx: Receiver<Vec<Instruction>>,
    message_tx: Sender<Vec<u8>>,
    error_tx: Sender<ServerMockerError>,
}

/// Specific implementation methods and constants for UDP server mocker
impl UdpServerImpl {
    fn run(&self) {
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
