use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_instruction::Instruction::{
    ReceiveMessageWithMaxSize, SendMessageDependingOnLastReceivedMessage, StopExchange,
};
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
    let dns_server_mocker = UdpServerMocker::new().unwrap();

    dns_server_mocker
        .add_mock_instructions(vec![
            // Receive a DNS query
            ReceiveMessageWithMaxSize(512),
            // Send a DNS response
            SendMessageDependingOnLastReceivedMessage(|previous_message| {
                let mut response = vec![
                    previous_message.as_ref().unwrap()[0],
                    previous_message.as_ref().unwrap()[1],
                ];
                response.extend_from_slice(
                    b"\
\x81\x80\x00\x01\x00\x01\x00\x02\x00\x01\x03www\x07example\x03com\x00\x00\x01\x00\x01\xc0\x0c\x00\x01\x00\x01\
\x00\x01\x08\xa4\x00\x04\x5d\xb8\xd8\x22\xc0\x10\x00\x02\x00\x01\x00\x01\x08\xa3\x00\x14\x01a\x0ciana-servers\x03net\
\x00\xc0\x10\x00\x02\x00\x01\x00\x01\x08\xa3\x00\x04\x01b\xc0?\x00\x00)\x10\x00\x00\x00\x00\x00\x00\x00",
                );
                Some(response)
            }),
            // Close the connection
            StopExchange,
        ])
        .unwrap();

    let address = format!("127.0.0.1:{}", dns_server_mocker.port())
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
        b"\x01\x00\x00\x01\x00\x00\x00\x00\x00\x01\x03www\x07example\x03com\x00\x00\x01\x00\x01\x00\x00)\x04\xD0\x00\x00\x00\x00\x00\x00",
        &dns_server_mocker.pop_received_message().unwrap()[2..]
    );

    // Check that no error has been raised by the mocked server
    assert!(dns_server_mocker.pop_server_error().is_none());
}
