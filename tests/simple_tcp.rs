use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_error::ServerMockerErrorFatality;
use socket_server_mocker::server_mocker_instruction::ServerMockerInstruction::{
    ReceiveMessage, ReceiveMessageWithMaxSize, SendMessage,
};
use socket_server_mocker::server_mocker_instruction::ServerMockerInstructionsList;
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::from_utf8;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_simple_tcp() {
    // Mock a TCP server listening on port 35642. Note that the mock will only listen on the local interface.
    let tcp_server_mocker = TcpServerMocker::new(35642).unwrap();

    // Create the TCP client to test
    let mut client = TcpStream::connect("127.0.0.1:35642").unwrap();

    // Mocked server behavior
    tcp_server_mocker
        .add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions(
            [
                ReceiveMessageWithMaxSize(16), // The mocked server will first wait for the client to send a message
                SendMessage("hello from server".as_bytes().to_vec()), // Then it will send a message to the client
            ]
            .as_slice(),
        ))
        .unwrap();

    // TCP client sends its first message
    client.write_all("hello from client".as_bytes()).unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = from_utf8(&buffer[..received_size]).unwrap();

    // Check that the message received by the client is the one sent by the mocked server
    assert_eq!("hello from server", received_message);

    // Check that the mocked server received the message sent by the client
    // The message is only 16 bytes, the letter 't' is dropped
    assert_eq!(
        "hello from clien",
        from_utf8(&tcp_server_mocker.pop_received_message().unwrap()).unwrap()
    );

    // New instructions for the mocked server
    let mut instructions = ServerMockerInstructionsList::new().with_added_receive_message(); // Wait for another message from the tested client
    instructions.add_send_message_depending_on_last_received_message(|_| None); // No message is sent to the server
    instructions.add_send_message_depending_on_last_received_message(|last_received_message| {
        // "hello2 from client"
        let mut received_message_string: String = from_utf8(&last_received_message.unwrap())
            .unwrap()
            .to_string();
        // "hello2"
        received_message_string.truncate(5);
        Some(
            format!("{received_message_string}2 from server")
                .as_bytes()
                .to_vec(),
        )
    }); // Send a message to the client depending on the last received message by the mocked server
    instructions.add_stop_exchange(); // Finally close the connection

    tcp_server_mocker
        .add_mock_instructions_list(instructions)
        .unwrap();

    // Tested client send a message to the mocked server
    client.write_all("hello2 from client".as_bytes()).unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = from_utf8(&buffer[..received_size]).unwrap();

    assert_eq!("hello2 from server", received_message);

    assert_eq!(
        "hello2 from client",
        from_utf8(&tcp_server_mocker.pop_received_message().unwrap()).unwrap()
    );

    // Check that no error has been raised by the mocked server
    assert!(tcp_server_mocker.pop_server_error().is_none());
}

#[test]
fn test_try_listen_twice_on_same_port() {
    // First TcpServerMocker will listen on a random free port
    let tcp_server_mocker = TcpServerMocker::new(0).unwrap();
    // Second TcpServerMocker will try to listen on the same port
    let tcp_server_mocker2 = TcpServerMocker::new(tcp_server_mocker.listening_port());
    // The second TcpServerMocker should fail to listen on the same port
    assert!(tcp_server_mocker2.is_err());
}

#[test]
fn test_receive_timeout() {
    // Mock a TCP server listening on a random free port
    let tcp_server_mocker = TcpServerMocker::new(0).unwrap();

    // Create the TCP client to test
    let _client =
        TcpStream::connect(format!("127.0.0.1:{}", tcp_server_mocker.listening_port())).unwrap();

    // Mocked server behavior
    tcp_server_mocker
        .add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions(&[
            // Expect to receive a message from the client
            ReceiveMessage,
        ]))
        .unwrap();

    // Wait twice the receive timeout
    sleep(Duration::from_millis(
        2 * TcpServerMocker::DEFAULT_NET_TIMEOUT_MS,
    ));

    // Check that the mocked server has raised an error
    let tcp_server_error = tcp_server_mocker.pop_server_error();
    assert!(tcp_server_error.is_some());
    let tcp_server_error = tcp_server_error.unwrap();
    assert_eq!(
        ServerMockerErrorFatality::NonFatal,
        tcp_server_error.fatality
    );
}
