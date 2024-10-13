use std::io::Read;
use std::net::TcpStream;

use socket_server_mocker::Instruction::SendMessage;
use socket_server_mocker::ServerMocker;

#[test]
fn simple_sending_message_test_random_port() {
    // Use random free port
    let server = ServerMocker::tcp().unwrap();

    // Connect to the mocked server
    let mut client = TcpStream::connect(server.socket_address()).unwrap();

    server
        .add_mock_instructions(vec![
            SendMessage(vec![1, 2, 3]),
            // We accidentally forgot ServerMockerInstruction::StopExchange,
        ])
        .unwrap();

    // Read a message sent by the mocked server
    let mut buffer: [u8; 16] = [0; 16];
    let received_size = client.read(&mut buffer).unwrap();

    assert_eq!([1, 2, 3], buffer[..received_size]);

    // Check that no error has been raised by the mocked server
    assert!(server.pop_server_error().is_none());
}
