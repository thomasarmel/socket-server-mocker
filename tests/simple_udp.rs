use std::net::UdpSocket;
use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_instruction::{ServerMockerInstruction, ServerMockerInstructionsList};
use socket_server_mocker::udp_server_mocker;

#[test]
fn test_simple_udp() {
    // Mock a UDP server listening on port 35642. Note that the mock will only listen on the local interface.
    let udp_server_mocker = udp_server_mocker::UdpServerMocker::new(35642);

    // Create the UDP client to test
    let client_socket = UdpSocket::bind("127.0.0.1:34254").unwrap();
    client_socket.connect("127.0.0.1:35642").unwrap();

    // Mocked server behavior
    udp_server_mocker.add_mock_instructions_list(
        ServerMockerInstructionsList::new_with_instructions(
            &[
                // The mocked server will first wait for the client to send a message, with max size = 32 bytes
                ServerMockerInstruction::ReceiveMessageWithMaxSize(32),
                // Then it will send a message to the client
                ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()),
                // Send nothing
                ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(|_| {
                    None
                }),
                // Send a message to the client depending on the last received message by the mocked server
                ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(|last_received_message| {
                    // "hello2 from client"
                    let mut received_message_string: String = std::str::from_utf8(&last_received_message.unwrap()).unwrap().to_string();
                    // "hello2"
                    received_message_string.truncate(5);
                    Some(format!("{}2 from server", received_message_string).as_bytes().to_vec())
                }),
            ]
        ),
    );

    // UDP client sends its first message
    client_socket.send("hello from client".as_bytes()).unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 32];
    let received_size = client_socket.recv(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = std::str::from_utf8(&buffer[..received_size]).unwrap();

    // Check that the message received by the client is the one sent by the mocked server
    assert_eq!("hello from server", received_message);

    // Check that the mocked server received the message sent by the client
    assert_eq!(
        "hello from client",
        std::str::from_utf8(&*udp_server_mocker.pop_received_message().unwrap()).unwrap()
    );

    let received_size = client_socket.recv(&mut buffer).unwrap();
    // convert shrunk buffer to string
    let received_message = std::str::from_utf8(&buffer[..received_size]).unwrap();

    // Check that the message received by the client is the one sent by the mocked server
    assert_eq!("hello2 from server", received_message);
}