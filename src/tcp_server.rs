use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::server_mocker::MockerOptions;
use crate::Instruction::{
    self, ReceiveMessageWithMaxSize, SendMessage, SendMessageDependingOnLastReceivedMessage,
};
use crate::ServerMockerError::{
    self, UnableToAcceptConnection, UnableToBindListener, UnableToGetLocalAddress,
    UnableToReadTcpStream, UnableToSetReadTimeout, UnableToWriteTcpStream,
};

/// Options for the TCP server mocker
#[derive(Debug, Clone)]
pub struct TcpMocker {
    /// Socket address on which the server will listen. Will be set to `127.0.0.1:0` by default.
    pub socket_addr: SocketAddr,
    /// Timeout for the server to wait for a message from the client.
    pub net_timeout: Duration,
    /// Timeout if no more instruction is available and [`Instruction::StopExchange`] hasn't been sent
    pub rx_timeout: Duration,
    /// Buffer size for TCP socket
    pub reader_buffer_size: usize,
}

impl Default for TcpMocker {
    fn default() -> Self {
        Self {
            socket_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            net_timeout: Duration::from_millis(100),
            rx_timeout: Duration::from_millis(100),
            reader_buffer_size: 1024,
        }
    }
}

impl MockerOptions for TcpMocker {
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
        let listener = TcpListener::bind(self.socket_addr)
            .map_err(|e| UnableToBindListener(self.socket_addr, e))?;
        let socket_addr = listener.local_addr().map_err(UnableToGetLocalAddress)?;

        thread::spawn(move || match listener.accept() {
            Ok((stream, _addr)) => {
                TcpServerImpl {
                    options: self,
                    stream,
                    instruction_rx,
                    message_tx,
                    error_tx,
                }
                .run();
            }
            Err(err) => {
                error_tx
                    .send(UnableToAcceptConnection(socket_addr, err))
                    .unwrap();
            }
        });

        Ok(socket_addr)
    }
}

/// TCP server mocker thread implementation
pub(crate) struct TcpServerImpl {
    options: TcpMocker,
    stream: TcpStream,
    instruction_rx: Receiver<Vec<Instruction>>,
    message_tx: Sender<Vec<u8>>,
    error_tx: Sender<ServerMockerError>,
}

/// TCP server mocker thread implementation
impl TcpServerImpl {
    fn run(mut self) {
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
