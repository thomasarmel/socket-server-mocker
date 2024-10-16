use std::time::Duration;

use lettre::transport::smtp::client::Tls;
use lettre::{Message, SmtpTransport, Transport};
use socket_server_mocker::Instruction::{ReceiveMessage, SendMessage, StopExchange};
use socket_server_mocker::ServerMocker;

#[test]
fn test_smtp_mock() {
    // Create an SMTP TCP server mocker listening on port 2525 (SMTP default port is 25)
    let server = ServerMocker::tcp_with_port(2525).unwrap();

    // Mocked server behavior
    server.add_mock_instructions(vec![
        SendMessage(b"220 smtp.localhost.mock ESMTP Mocker\r\n".to_vec()),
        ReceiveMessage,
        SendMessage(b"250-smtp.localhost.mock\r\n250-PIPELINING\r\n250-SIZE 20971520\r\n250-ETRN\r\n250-STARTTLS\r\n250-ENHANCEDSTATUSCODES\r\n250 8BITMIME\r\n".to_vec()),
        ReceiveMessage,
        SendMessage(b"250 2.1.0 Ok\r\n".to_vec()),
        ReceiveMessage,
        SendMessage(b"250 2.1.5 Ok\r\n".to_vec()),
        ReceiveMessage,
        SendMessage(b"354 End data with <CR><LF>.<CR><LF>\r\n".to_vec()),
        ReceiveMessage,
        SendMessage(b"250 2.0.0 Ok: queued as 1C1A1B1C1D1E1F1G1H1I1J1K1L1M1N1O1P1Q1R1S1T1U1V1W1X1Y1Z\r\n".to_vec()),
        StopExchange,
    ]).unwrap();

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
        .port(server.port())
        .timeout(Some(Duration::from_secs(1)))
        .build();
    // Send the email
    mailer.send(&email_builder).unwrap();

    // Check that the server received the expected SMTP message
    assert_eq!(b"EHLO ", &server.pop_received_message().unwrap()[..5]);
    assert_eq!(
        b"MAIL FROM:<alice.dupont@localhost.mock>\r\n",
        server.pop_received_message().unwrap().as_slice()
    );
    assert_eq!(
        b"RCPT TO:<bob.dupond@localhost.mock>\r\n",
        server.pop_received_message().unwrap().as_slice()
    );
    assert_eq!(
        b"DATA\r\n",
        server.pop_received_message().unwrap().as_slice()
    );

    let mail_payload_str = String::from_utf8(server.pop_received_message().unwrap()).unwrap();
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

    // Check that no error has been raised by the mocked server
    assert!(server.pop_server_error().is_none());
}
