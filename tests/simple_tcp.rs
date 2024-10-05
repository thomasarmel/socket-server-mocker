use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::from_utf8;
use std::thread::sleep;

use socket_server_mocker::Instruction::{
    ReceiveMessage, ReceiveMessageWithMaxSize, SendMessage,
    SendMessageDependingOnLastReceivedMessage, StopExchange,
};
use socket_server_mocker::{ServerMocker, TcpServerMocker};

#[test]
fn test_simple_tcp() {
    // Mock a TCP server listening on port 35642. Note that the mock will only listen on the local interface.
    let server = TcpServerMocker::new_with_port(35642).unwrap();

    // Create the TCP client to test
    let mut client = TcpStream::connect(server.socket_address()).unwrap();

    // Mocked server behavior
    server
        .add_mock_instructions(vec![
            ReceiveMessageWithMaxSize(16), // The mocked server will first wait for the client to send a message
            SendMessage(b"hello from server".to_vec()), // Then it will send a message to the client
        ])
        .unwrap();

    // TCP client sends its first message
    client.write_all(b"hello from client").unwrap();

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
        from_utf8(&server.pop_received_message().unwrap()).unwrap()
    );

    // New instructions for the mocked server
    let instructions = vec![
        // Wait for another message from the tested client
        ReceiveMessage,
        // No message is sent to the server
        SendMessageDependingOnLastReceivedMessage(|_| None),
        // Send a message to the client depending on the last received message by the mocked server
        SendMessageDependingOnLastReceivedMessage(|last_received_message| {
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
        }),
        // Finally close the connection
        StopExchange,
    ];

    server.add_mock_instructions(instructions).unwrap();

    // Tested client send a message to the mocked server
    client.write_all(b"hello2 from client").unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = from_utf8(&buffer[..received_size]).unwrap();

    assert_eq!("hello2 from server", received_message);

    assert_eq!(
        "hello2 from client",
        from_utf8(&server.pop_received_message().unwrap()).unwrap()
    );

    // Check that no error has been raised by the mocked server
    assert!(server.pop_server_error().is_none());
}

#[test]
fn test_try_listen_twice_on_same_port() {
    // First TcpServerMocker will listen on a random free port
    let server = TcpServerMocker::new().unwrap();
    // Second TcpServerMocker will try to listen on the same port
    let server2 = TcpServerMocker::new_with_port(server.port());
    // The second TcpServerMocker should fail to listen on the same port
    assert!(server2.is_err());
}

#[test]
fn test_receive_timeout() {
    // Mock a TCP server listening on a random free port
    let server = TcpServerMocker::new().unwrap();

    // Create the TCP client to test
    let _client = TcpStream::connect(server.socket_address()).unwrap();

    // Mocked server behavior
    server
        .add_mock_instructions(vec![
            // Expect to receive a message from the client
            ReceiveMessage,
        ])
        .unwrap();

    // Wait twice the timeout
    sleep(2 * server.options().rx_timeout);

    // Check that the mocked server has raised an error
    let tcp_server_error = server.pop_server_error();
    assert!(tcp_server_error.is_some());
    let tcp_server_error = tcp_server_error.unwrap();
    assert!(!tcp_server_error.is_fatal());
}
