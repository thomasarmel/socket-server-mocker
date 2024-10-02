use std::io::Write;
use std::net::TcpStream;

use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_instruction::Instruction::{ReceiveMessage, StopExchange};
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;

#[test]
fn simple_receiving_message_test() {
    let tcp_server_mocker = TcpServerMocker::new_with_port(1234).unwrap();
    let mut client = TcpStream::connect("127.0.0.1:1234").unwrap();

    tcp_server_mocker
        .add_mock_instructions(vec![ReceiveMessage, StopExchange])
        .unwrap();
    client.write_all(&[1, 2, 3]).unwrap();

    let mock_server_received_message = tcp_server_mocker.pop_received_message();
    assert_eq!(Some(vec![1, 2, 3]), mock_server_received_message);

    // Check that no error has been raised by the mocked server
    assert!(tcp_server_mocker.pop_server_error().is_none());
}
