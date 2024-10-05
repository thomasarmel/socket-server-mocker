use std::io::Write;
use std::net::TcpStream;

use socket_server_mocker::Instruction::{ReceiveMessage, StopExchange};
use socket_server_mocker::{ServerMocker, TcpServerMocker};

#[test]
fn simple_receiving_message_test() {
    let server = TcpServerMocker::new_with_port(1234).unwrap();
    let mut client = TcpStream::connect(server.socket_address()).unwrap();

    server
        .add_mock_instructions(vec![ReceiveMessage, StopExchange])
        .unwrap();
    client.write_all(&[1, 2, 3]).unwrap();

    let mock_server_received_message = server.pop_received_message();
    assert_eq!(Some(vec![1, 2, 3]), mock_server_received_message);

    // Check that no error has been raised by the mocked server
    assert!(server.pop_server_error().is_none());
}
