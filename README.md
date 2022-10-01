# socket-server-mocker

_Mock socket server in Rust, for testing various network clients._

***

I was developing an application that needed to connect to an external server, and I was looking for a way to test the messages sent by the application to the server, directly with `cargo test`. So I looked for a way to directly mock a network server in Rust, without having to integrate a real server in docker each time the tests were launched.

With this crate, it is possible to directly test the messages sent by your application which normally connects to a server.


## Usage

Add the **socket-server-mocker** dependency to your `Cargo.toml` for testing compilation:

```toml
[dev-dependencies]
socket-server-mocker = "0.0.1"
```

## Example

You can view all example test codes in **[tests](tests)** directory.

Here is a simple example:

```rust
use std::io::{Read, Write};
use std::net::TcpStream;
use socket_server_mocker::server_mocker_instruction::{ServerMockerInstruction, ServerMockerInstructionsList};
use socket_server_mocker::tcp_server_mocker::TcpServerMocker;

#[test]
fn test_simple_tcp() {
    // Mock a TCP server listening on port 35642. Note that the mock will only listen on the local interface.
    let tcp_server_mocker = TcpServerMocker::new(35642);

    // Create the TCP client to test
    let mut client = TcpStream::connect("127.0.0.1:35642").unwrap();

    // Mocked server behavior
    tcp_server_mocker.add_mock_instructions_list(ServerMockerInstructionsList::new_with_instructions([
            ServerMockerInstruction::ReceiveMessage, // The mocked server will first wait for the client to send a message
            ServerMockerInstruction::SendMessage("hello from server".as_bytes().to_vec()), // Then it will send a message to the client
        ].as_slice()
    ));

    // TCP client sends its first message
    client.write_all("hello from client".as_bytes()).unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = std::str::from_utf8(&buffer[..received_size]).unwrap();

    // Check that the message received by the client is the one sent by the mocked server
    assert_eq!("hello from server", received_message);

    // Check that the mocked server received the message sent by the client
    assert_eq!("hello from client", std::str::from_utf8(&*tcp_server_mocker.pop_received_message().unwrap()).unwrap());

    // New instructions for the mocked server
    let mut instructions = ServerMockerInstructionsList::new().with_added_receive_message(); // Wait for another message from the tested client
    instructions.add_send_message("hello2 from server".as_bytes().to_vec()); // Send another message to the tested client
    instructions.add_stop_exchange(); // Finally close the connection

    tcp_server_mocker.add_mock_instructions_list(instructions);

    // Tested client send a message to the mocked server
    client.write_all("hello2 from client".as_bytes()).unwrap();

    // Read a message sent by the mocked server
    let mut buffer = [0; 1024];
    let received_size = client.read(&mut buffer).unwrap();

    // convert shrunk buffer to string
    let received_message = std::str::from_utf8(&buffer[..received_size]).unwrap();

    assert_eq!("hello2 from server", received_message);

    assert_eq!("hello2 from client", std::str::from_utf8(&*tcp_server_mocker.pop_received_message().unwrap()).unwrap());
}
```