use std::io::{Read, Write};
use std::net::TcpStream;
use socket_server_mocker::server_mocker_instruction::{ServerMockerInstruction, ServerMockerInstructionsList};
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;

#[test]
fn test_single_tcp() {
    let tcp_server_mocker = TcpServerMocker::new(35642);

    // Create a TCP client
    let mut client = TcpStream::connect("127.0.0.1:35642").unwrap();

    tcp_server_mocker.add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions([
            ServerMockerInstruction::ReceiveMessage,
            ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()),
        ].as_slice()
    ));

    // Send a message
    client.write_all("hello from client".as_bytes()).unwrap();

    // Read a message
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = std::str::from_utf8(&buffer[..received_size]).unwrap();
    assert_eq!("hello from server", received_message);

    assert_eq!("hello from client", std::str::from_utf8(&*tcp_server_mocker.pop_received_message().unwrap()).unwrap());

    let mut instructions = ServerMockerInstructionsList::new().with_added_receive_message();
    instructions.add_send_message("hello2 from server".as_bytes().to_vec());
    instructions.add_stop_exchange();

    tcp_server_mocker.add_mock_instructions_list(instructions);

    // Send a message
    client.write_all("hello2 from client".as_bytes()).unwrap();

    // Read a message
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = std::str::from_utf8(&buffer[..received_size]).unwrap();
    assert_eq!("hello2 from server", received_message);

    assert_eq!("hello2 from client", std::str::from_utf8(&*tcp_server_mocker.pop_received_message().unwrap()).unwrap());
}
