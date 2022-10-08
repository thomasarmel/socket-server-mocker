use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_instruction::{
    ServerMockerInstruction, ServerMockerInstructionsList,
};
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;
use std::io::{Read, Write};
use std::net::TcpStream;

#[test]
fn test_simple_tcp() {
    // Mock a TCP server listening on port 35642. Note that the mock will only listen on the local interface.
    let tcp_server_mocker = TcpServerMocker::new(35642);

    // Create the TCP client to test
    let mut client = TcpStream::connect("127.0.0.1:35642").unwrap();

    // Mocked server behavior
    tcp_server_mocker.add_mock_instructions_list(
        ServerMockerInstructionsList::new_with_instructions(
            [
                ServerMockerInstruction::ReceiveMessageWithMaxSize(16), // The mocked server will first wait for the client to send a message
                ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()), // Then it will send a message to the client
            ]
            .as_slice(),
        ),
    );

    // TCP client sends its first message
    client.write_all("hello from client".as_bytes()).unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = std::str::from_utf8(&buffer[..received_size]).unwrap();

    // Check that the message received by the client is the one sent by the mocked server
    assert_eq!("hello from server", received_message);

    // Check that the mocked server received the message sent by the client
    assert_eq!(
        "hello from clien",
        std::str::from_utf8(&*tcp_server_mocker.pop_received_message().unwrap()).unwrap()
    );

    // New instructions for the mocked server
    let mut instructions = ServerMockerInstructionsList::new().with_added_receive_message(); // Wait for another message from the tested client
    instructions.add_send_message_depending_on_last_received_message(|_| None); // No message is sent to the server
    instructions.add_send_message_depending_on_last_received_message(|last_received_message| {
        // "hello2 from client"
        let mut received_message_string: String =
            std::str::from_utf8(&last_received_message.unwrap())
                .unwrap()
                .to_string();
        // "hello2"
        received_message_string.truncate(5);
        Some(
            format!("{}2 from server", received_message_string)
                .as_bytes()
                .to_vec(),
        )
    }); // Send a message to the client depending on the last received message by the mocked server
    instructions.add_stop_exchange(); // Finally close the connection

    tcp_server_mocker.add_mock_instructions_list(instructions);

    // Tested client send a message to the mocked server
    client.write_all("hello2 from client".as_bytes()).unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = std::str::from_utf8(&buffer[..received_size]).unwrap();

    assert_eq!("hello2 from server", received_message);

    assert_eq!(
        "hello2 from client",
        std::str::from_utf8(&*tcp_server_mocker.pop_received_message().unwrap()).unwrap()
    );
}
