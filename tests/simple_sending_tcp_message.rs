use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_instruction::ServerMockerInstruction;
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;
use std::io::Read;
use std::net::TcpStream;

#[test]
fn simple_sending_message_test_random_port() {
    // Use random free port
    let tcp_server_mocker = TcpServerMocker::new(0).unwrap();

    // Connect to the mocked server
    let mut client =
        TcpStream::connect(format!("127.0.0.1:{}", tcp_server_mocker.listening_port())).unwrap();

    tcp_server_mocker
        .add_mock_instructions(&[
            ServerMockerInstruction::SendMessage(vec![1, 2, 3]),
            // We accidentally forgot ServerMockerInstruction::StopExchange,
        ])
        .unwrap();

    // Read a message sent by the mocked server
    let mut buffer: [u8; 16] = [0; 16];
    let received_size = client.read(&mut buffer).unwrap();

    assert_eq!([1, 2, 3], buffer[..received_size]);

    // Check that no error has been raised by the mocked server
    assert!(tcp_server_mocker.pop_server_error().is_none());
}
