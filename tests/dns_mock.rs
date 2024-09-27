use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_instruction::ServerMockerInstruction;
use socket_server_mocker::udp_server_mocker::UdpServerMocker;
use std::net::Ipv4Addr;
use std::str::FromStr;
use trust_dns_client::client::{Client, SyncClient};
use trust_dns_client::op::DnsResponse;
use trust_dns_client::rr::rdata::A;
use trust_dns_client::rr::{DNSClass, Name, RData, Record, RecordType};
use trust_dns_client::udp::UdpClientConnection;

#[test]
fn test_dns_mock() {
    let dns_server_mocker = UdpServerMocker::new(0).unwrap();

    dns_server_mocker
        .add_mock_instructions(&[
            // Receive a DNS query
            ServerMockerInstruction::ReceiveMessageWithMaxSize(512),
            // Send a DNS response
            ServerMockerInstruction::SendMessageDependingOnLastReceivedMessage(
                |previous_message| {
                    Some(vec![
                        previous_message.as_ref().unwrap()[0],
                        previous_message.as_ref().unwrap()[1],
                        0x81,
                        0x80,
                        0x00,
                        0x01,
                        0x00,
                        0x01,
                        0x00,
                        0x02,
                        0x00,
                        0x01,
                        0x03,
                        0x77,
                        0x77,
                        0x77,
                        0x07,
                        0x65,
                        0x78,
                        0x61,
                        0x6d,
                        0x70,
                        0x6c,
                        0x65,
                        0x03,
                        0x63,
                        0x6f,
                        0x6d,
                        0x00,
                        0x00,
                        0x01,
                        0x00,
                        0x01,
                        0xc0,
                        0x0c,
                        0x00,
                        0x01,
                        0x00,
                        0x01,
                        0x00,
                        0x01,
                        0x08,
                        0xa4,
                        0x00,
                        0x04,
                        0x5d,
                        0xb8,
                        0xd8,
                        0x22,
                        0xc0,
                        0x10,
                        0x00,
                        0x02,
                        0x00,
                        0x01,
                        0x00,
                        0x01,
                        0x08,
                        0xa3,
                        0x00,
                        0x14,
                        0x01,
                        0x61,
                        0x0c,
                        0x69,
                        0x61,
                        0x6e,
                        0x61,
                        0x2d,
                        0x73,
                        0x65,
                        0x72,
                        0x76,
                        0x65,
                        0x72,
                        0x73,
                        0x03,
                        0x6e,
                        0x65,
                        0x74,
                        0x00,
                        0xc0,
                        0x10,
                        0x00,
                        0x02,
                        0x00,
                        0x01,
                        0x00,
                        0x01,
                        0x08,
                        0xa3,
                        0x00,
                        0x04,
                        0x01,
                        0x62,
                        0xc0,
                        0x3f,
                        0x00,
                        0x00,
                        0x29,
                        0x10,
                        0x00,
                        0x00,
                        0x00,
                        0x00,
                        0x00,
                        0x00,
                        0x00,
                    ])
                },
            ),
            // Close the connection
            ServerMockerInstruction::StopExchange,
        ])
        .unwrap();

    let address = format!("127.0.0.1:{}", dns_server_mocker.listening_port())
        .parse()
        .unwrap();
    let conn = UdpClientConnection::new(address).unwrap();

    // create the DNS client
    let client = SyncClient::new(conn);

    let name = Name::from_str("www.example.com.").unwrap();

    // Send DNS query to mocked server
    let response: DnsResponse = client.query(&name, DNSClass::IN, RecordType::A).unwrap();

    let answers: &[Record] = response.answers();

    // Check returned IP address is correct
    if let Some(RData::A(ref ip)) = answers[0].data() {
        assert_eq!(*ip, A(Ipv4Addr::new(93, 184, 216, 34)));
    } else {
        panic!("unexpected result");
    }

    assert_eq!(
        vec![
            1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 3, 119, 119, 119, 7, 101, 120, 97, 109, 112, 108, 101, 3,
            99, 111, 109, 0, 0, 1, 0, 1, 0, 0, 41, 4, 208, 0, 0, 0, 0, 0, 0
        ],
        dns_server_mocker.pop_received_message().unwrap()[2..]
    );

    // Check that no error has been raised by the mocked server
    assert!(dns_server_mocker.pop_server_error().is_none());
}
