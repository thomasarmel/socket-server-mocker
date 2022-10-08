use lettre::transport::smtp::client::Tls;
use lettre::{Message, SmtpTransport, Transport};
use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_instruction::ServerMockerInstruction;
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;

#[test]
fn test_smtp_mock() {
    // Create a SMTP TCP server mocker listening on port 2525 (SMTP default port is 25)
    let smtp_server_mocker = TcpServerMocker::new(2525);

    // Mocked server behavior
    smtp_server_mocker.add_mock_instructions(&[
        ServerMockerInstruction::SendMessage("220 smtp.localhost.mock ESMTP Mocker\r\n".as_bytes().to_vec()),
        ServerMockerInstruction::ReceiveMessage,
        ServerMockerInstruction::SendMessage("250-smtp.localhost.mock\r\n250-PIPELINING\r\n250-SIZE 20971520\r\n250-ETRN\r\n250-STARTTLS\r\n250-ENHANCEDSTATUSCODES\r\n250 8BITMIME\r\n".as_bytes().to_vec()),
        ServerMockerInstruction::ReceiveMessage,
        ServerMockerInstruction::SendMessage("250 2.1.0 Ok\r\n".as_bytes().to_vec()),
        ServerMockerInstruction::ReceiveMessage,
        ServerMockerInstruction::SendMessage("250 2.1.5 Ok\r\n".as_bytes().to_vec()),
        ServerMockerInstruction::ReceiveMessage,
        ServerMockerInstruction::SendMessage("354 End data with <CR><LF>.<CR><LF>\r\n".as_bytes().to_vec()),
        ServerMockerInstruction::ReceiveMessage,
        ServerMockerInstruction::SendMessage("250 2.0.0 Ok: queued as 1C1A1B1C1D1E1F1G1H1I1J1K1L1M1N1O1P1Q1R1S1T1U1V1W1X1Y1Z\r\n".as_bytes().to_vec()),
        ServerMockerInstruction::StopExchange,
    ]);

    // Create a client based on a SmtpTransport
    let email_builder = Message::builder()
        .from(
            "Alice Dupont <alice.dupont@localhost.mock>"
                .parse()
                .unwrap(),
        )
        .reply_to(
            "Alice Dupont <alice.dupont@localhost.mock>"
                .parse()
                .unwrap(),
        )
        .to("Bob Dupond <bob.dupond@localhost.mock>".parse().unwrap())
        .subject("Happy new year")
        .body(String::from("Be happy!"))
        .unwrap();

    // Mail client opens a remote connection on mocked SMTP server
    let mailer = SmtpTransport::relay("127.0.0.1")
        .unwrap()
        .tls(Tls::None)
        .port(2525)
        .timeout(Some(std::time::Duration::from_secs(1)))
        .build();
    // Send the email
    mailer.send(&email_builder).unwrap();

    // Check that the server received the expected SMTP message
    assert_eq!(
        "EHLO ".as_bytes().to_vec(),
        smtp_server_mocker.pop_received_message().unwrap()[..5]
    );
    assert_eq!(
        "MAIL FROM:<alice.dupont@localhost.mock>\r\n"
            .as_bytes()
            .to_vec(),
        smtp_server_mocker.pop_received_message().unwrap()
    );
    assert_eq!(
        "RCPT TO:<bob.dupond@localhost.mock>\r\n"
            .as_bytes()
            .to_vec(),
        smtp_server_mocker.pop_received_message().unwrap()
    );
    assert_eq!(
        "DATA\r\n".as_bytes().to_vec(),
        smtp_server_mocker.pop_received_message().unwrap()
    );

    let mail_payload_str =
        String::from_utf8(smtp_server_mocker.pop_received_message().unwrap()).unwrap();
    let mut mail_payload_lines = mail_payload_str.lines();

    // Check that the server received the expected mail payload
    assert_eq!(
        "From: \"Alice Dupont\" <alice.dupont@localhost.mock>",
        mail_payload_lines.next().unwrap()
    );
    assert_eq!(
        "Reply-To: \"Alice Dupont\" <alice.dupont@localhost.mock>",
        mail_payload_lines.next().unwrap()
    );
    assert_eq!(
        "To: \"Bob Dupond\" <bob.dupond@localhost.mock>",
        mail_payload_lines.next().unwrap()
    );
    assert_eq!(
        "Subject: Happy new year",
        mail_payload_lines.next().unwrap()
    );
    assert_eq!(
        "Content-Transfer-Encoding: 7bit",
        mail_payload_lines.next().unwrap()
    );
    assert!(Option::is_some(&mail_payload_lines.next())); // Email date
    assert_eq!("", mail_payload_lines.next().unwrap());
    assert_eq!("Be happy!", mail_payload_lines.next().unwrap());
    // Last message line with only a dot "." is not returned by lines() method
    assert_eq!(None, mail_payload_lines.next());
}
