use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::server_mocker_instruction::{BinaryMessage, ServerMockerInstruction, ServerMockerInstructionsList};

pub struct TcpServerMocker {
    listening_port: u16,
    instructions_sender: Option<Sender<ServerMockerInstructionsList>>,
    message_receiver: Option<Receiver<BinaryMessage>>,
}

impl TcpServerMocker {

    const DEFAULT_TCP_TIMEOUT_MS : u64 = 100;
    const DEFAULT_THREAD_RECEIVER_TIMEOUT_MS : u64 = 100;

    pub fn new(port: u16) -> TcpServerMocker {
        TcpServerMocker {
            listening_port: port,
            instructions_sender: None,
            message_receiver: None,
        }
    }

    pub fn start(&mut self) {
        let listening_port = self.listening_port;
        let (instruction_tx, instruction_rx) : (Sender<ServerMockerInstructionsList>, Receiver<ServerMockerInstructionsList>) = mpsc::channel();
        let (message_tx, message_rx) : (Sender<BinaryMessage>, Receiver<BinaryMessage>) = mpsc::channel();
        self.instructions_sender = Some(instruction_tx);
        self.message_receiver = Some(message_rx);
        thread::spawn(move || {
            //rx.try_recv()
            let tcp_listener = TcpListener::bind(format!("127.0.0.1:{}", listening_port)).unwrap();
            let tcp_stream = tcp_listener.accept().unwrap().0;
            Self::handle_connection(tcp_stream, instruction_rx, message_tx);
        });
    }

    fn handle_connection(mut tcp_stream: TcpStream, instructions_receiver: Receiver<ServerMockerInstructionsList>, message_sender: Sender<BinaryMessage>) {
        tcp_stream.set_read_timeout(Some(std::time::Duration::from_millis(Self::DEFAULT_TCP_TIMEOUT_MS))).unwrap();
        loop {
            for instruction in instructions_receiver.recv_timeout(std::time::Duration::from_millis(Self::DEFAULT_THREAD_RECEIVER_TIMEOUT_MS)).unwrap().instructions {
                match instruction {
                    ServerMockerInstruction::SendMessage(binary_message) => {
                        tcp_stream.write_all(&binary_message).unwrap();
                        println!("Sending packet: {:?}", binary_message);
                    },
                    ServerMockerInstruction::ReceiveMessage => { // TODO: send packet
                        let mut buffer = [0; 1024]; // TODO: more
                        match tcp_stream.read(&mut buffer) {
                            Ok(size) => {
                                println!("Received {} bytes", size);
                                println!("{:?}", &buffer[..size]);
                                message_sender.send(buffer[..size].to_vec()).unwrap();
                            },
                            Err(e) => {
                                println!("Error: {}", e);
                            }
                        }
                    },
                    ServerMockerInstruction::StopExchange => {
                        return;
                    }
                }
            }
        }
    }

    pub fn add_mock_instructions_list(&self, instructions_list: ServerMockerInstructionsList) -> Result<(), ()> {
        match self.instructions_sender {
            Some(ref sender) => {
                sender.send(instructions_list).unwrap();
                Ok(())
            },
            None => Err(()) // server must be initialized
        }
    }

    pub fn pop_received_message(&self) -> Option<BinaryMessage> {
        match self.message_receiver {
            Some(ref receiver) => {
                match receiver.try_recv() {
                    Ok(message) => Some(message),
                    Err(_) => None
                }
            },
            None => None // server not initialized
        }
    }

    pub fn listening_port(&self) -> u16 {
        self.listening_port
    }
}