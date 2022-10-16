use socket_server_mocker::server_mocker::ServerMocker;
use socket_server_mocker::server_mocker_instruction::ServerMockerInstruction;
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;

#[test]
fn http_get() {
    // Mock HTTP server on a random free port
    let http_server_mocker = TcpServerMocker::new(0).unwrap();

    http_server_mocker.add_mock_instructions(&[
        // Wait for a HTTP GET request
        ServerMockerInstruction::ReceiveMessage,
        // Send a HTTP response
        ServerMockerInstruction::SendMessage("HTTP/1.1 200 OK\r\nServer: socket-server-mocker-fake-http\r\nContent-Length: 12\r\nConnection: close\r\nContent-Type: text/plain\r\n\r\nHello, world".as_bytes().to_vec()),
        // Close the connection
        ServerMockerInstruction::StopExchange,
    ]).unwrap();

    // New reqwest blocking client
    let client = reqwest::blocking::Client::new();
    // Send a HTTP GET request to the mocked server
    let response = client
        .get(format!(
            "http://localhost:{}/",
            http_server_mocker.listening_port()
        ))
        .send()
        .unwrap();

    // Check response status code
    assert!(response.status().is_success());

    // Check response body
    assert_eq!(response.text().unwrap(), "Hello, world");
    // Check HTTP request received by the mocked server
    assert_eq!(
        format!(
            "GET / HTTP/1.1\r\naccept: */*\r\nhost: localhost:{}\r\n\r\n",
            http_server_mocker.listening_port()
        ),
        std::str::from_utf8(&*http_server_mocker.pop_received_message().unwrap()).unwrap()
    );

    // Check that no error has been raised by the mocked server
    assert!(http_server_mocker.pop_server_error().is_none());
}
