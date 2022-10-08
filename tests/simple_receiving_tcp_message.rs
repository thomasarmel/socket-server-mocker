use socket_server_mocker::server_mocker_instruction::{
    ServerMockerInstruction, ServerMockerInstructionsList,
};
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;
use std::io::Write;
use std::net::TcpStream;
use socket_server_mocker::server_mocker::ServerMocker;

#[test]
fn simple_receiving_message_test() {
    let tcp_server_mocker = TcpServerMocker::new(1234);
    let mut client = TcpStream::connect("127.0.0.1:1234").unwrap();

    tcp_server_mocker.add_mock_instructions_list(
        ServerMockerInstructionsList::new_with_instructions(
            [
                ServerMockerInstruction::ReceiveMessage,
                ServerMockerInstruction::StopExchange,
            ]
            .as_slice(),
        ),
    );
    client.write_all(&[1, 2, 3]).unwrap();

    let mock_server_received_message = tcp_server_mocker.pop_received_message();
    assert_eq!(Some(vec![1, 2, 3]), mock_server_received_message);
}
