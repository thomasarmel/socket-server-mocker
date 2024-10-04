use std::net::UdpSocket;
use std::str::from_utf8;
use std::thread::sleep;

use socket_server_mocker::Instruction::{
    ReceiveMessageWithMaxSize, SendMessage, SendMessageDependingOnLastReceivedMessage,
};
use socket_server_mocker::{ServerMocker, UdpServerMocker};

#[test]
fn test_simple_udp() {
    // Mock a UDP server listening on port 35642. Note that the mock will only listen on the local interface.
    let udp_server_mocker = UdpServerMocker::new_with_port(35642).unwrap();

    // Create the UDP client to test
    let client_socket = UdpSocket::bind("127.0.0.1:34254").unwrap();
    client_socket.connect("127.0.0.1:35642").unwrap();

    // Mocked server behavior
    udp_server_mocker
        .add_mock_instructions(vec![
            // The mocked server will first wait for the client to send a message, with max size = 32 bytes
            ReceiveMessageWithMaxSize(32),
            // Then it will send a message to the client
            SendMessage("hello from server".as_bytes().to_vec()),
            // Send nothing
            SendMessageDependingOnLastReceivedMessage(|_| None),
            // Send a message to the client depending on the last received message by the mocked server
            SendMessageDependingOnLastReceivedMessage(|last_received_message| {
                // "hello2 from client"
                let mut received_message_string: String =
                    from_utf8(&last_received_message.unwrap())
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
        ])
        .unwrap();

    // UDP client sends its first message
    client_socket.send("hello from client".as_bytes()).unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 32];
    let received_size = client_socket.recv(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = from_utf8(&buffer[..received_size]).unwrap();

    // Check that the message received by the client is the one sent by the mocked server
    assert_eq!("hello from server", received_message);

    // Check that the mocked server received the message sent by the client
    assert_eq!(
        "hello from client",
        from_utf8(&udp_server_mocker.pop_received_message().unwrap()).unwrap()
    );

    let received_size = client_socket.recv(&mut buffer).unwrap();
    // convert shrunk buffer to string
    let received_message = from_utf8(&buffer[..received_size]).unwrap();

    // Check that the message received by the client is the one sent by the mocked server
    assert_eq!("hello2 from server", received_message);

    // Check that no error has been raised by the mocked server
    assert!(udp_server_mocker.pop_server_error().is_none());
}

#[test]
fn test_try_listen_twice_on_same_port() {
    // First UdpServerMocker will listen on a random free port
    let udp_server_mocker = UdpServerMocker::new().unwrap();
    // Second UdpServerMocker will try to listen on the same port
    let udp_server_mocker2 = UdpServerMocker::new_with_port(udp_server_mocker.port());
    // The second UdpServerMocker should fail to listen on the same port
    assert!(udp_server_mocker2.is_err());
}

#[test]
fn test_try_receive_before_send() {
    // Mock a UDP server listening on random port
    let udp_server_mocker = UdpServerMocker::new().unwrap();

    // Mocked server behavior
    udp_server_mocker
        .add_mock_instructions(vec![
            // The mocked server will send a message before receiving anything from the client
            SendMessage("hello from server".as_bytes().to_vec()),
        ])
        .unwrap();

    let mocked_server_error_received = udp_server_mocker.pop_server_error();

    // Error has been raised because the mocked server tried to send a message before receiving anything from the client
    assert!(mocked_server_error_received.is_some());

    let mocked_server_error = mocked_server_error_received.unwrap();

    // Check that the error raised by the mocked server is the expected one
    assert_eq!(
        "Non fatal: SendMessage instruction received before a ReceiveMessage",
        mocked_server_error.to_string()
    );
    assert!(!mocked_server_error.is_fatal());
}

#[test]
fn test_receive_timeout() {
    // Mock a UDP server listening on a random free port
    let udp_server_mocker = UdpServerMocker::new().unwrap();

    // Mocked server behavior
    udp_server_mocker
        .add_mock_instructions(vec![
            // Expect to receive a message from the client
            ReceiveMessageWithMaxSize(32),
        ])
        .unwrap();

    // Wait twice the receive timeout
    sleep(2 * UdpServerMocker::DEFAULT_NET_TIMEOUT);

    // Check that the mocked server has raised an error
    let mocked_server_error_received = udp_server_mocker.pop_server_error();
    assert!(mocked_server_error_received.is_some());
    assert!(!mocked_server_error_received.unwrap().is_fatal());
}
