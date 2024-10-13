# socket-server-mocker

[![GitHub](https://img.shields.io/badge/github-thomasarmel/socket--server--mocker-8da0cb?logo=github)](https://github.com/thomasarmel/socket-server-mocker)
[![crates.io version](https://img.shields.io/crates/v/socket-server-mocker.svg)](https://crates.io/crates/socket-server-mocker)
[![docs.rs docs](https://docs.rs/socket-server-mocker/badge.svg)](https://docs.rs/socket-server-mocker)
[![crates.io version](https://img.shields.io/crates/l/socket-server-mocker.svg)](https://github.com/thomasarmel/socket-server-mocker/blob/main/LICENSE)
[![CI build](https://github.com/thomasarmel/socket-server-mocker/actions/workflows/rust.yml/badge.svg)](https://github.com/thomasarmel/socket-server-mocker/actions)

_Mock socket server in Rust, for testing various network clients._

***

I was developing an application that needed to connect to an external server, and I was looking for a way to test the messages sent by the application to the server, directly with `cargo test`. So I looked for a way to directly mock a network server in Rust, without having to integrate a real server in docker each time the tests were launched.

With this crate, it is possible to directly test the messages sent by your application which normally connects to a server.


## Usage

Add the **socket-server-mocker** dependency to your `Cargo.toml` for testing compilation:

```toml
[dev-dependencies]
socket-server-mocker = "0.5"
```

## Example

You can view all example test codes in **[tests](./tests)** directory.
In particular, you there are examples of mocking the protocols [PostgreSQL](tests/postgres_mock.rs), [HTTP](tests/http_reqwest_api_mock.rs), [DNS](./tests/dns_mock.rs) and [SMTP](./tests/smtp_mock.rs).

Here is a simple example in TCP:

```rust
use socket_server_mocker::ServerMocker;
use socket_server_mocker::Instruction::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::from_utf8;

// Mock a TCP server listening on port 35642. Note that the mock will only listen on the local interface.
let server = ServerMocker::tcp_with_port(35642).unwrap();

// Create the TCP client to test
let mut client = TcpStream::connect(server.socket_address()).unwrap();

// Mocked server behavior
server.add_mock_instructions(vec![
    ReceiveMessageWithMaxSize(16), // The mocked server will first wait for the client to send a message
    SendMessage(b"hello from server".to_vec()), // Then it will send a message to the client
]);

// TCP client sends its first message
client.write_all(b"hello from client").unwrap();

// Read a message sent by the mocked server
let mut buffer = [0; 1024];
let received_size = client.read(&mut buffer).unwrap();

// convert shrunk buffer to string
let received_message = from_utf8(&buffer[..received_size]).unwrap();

// Check that the message received by the client is the one sent by the mocked server
assert_eq!("hello from server", received_message);

// Check that the mocked server received the message sent by the client
assert_eq!(
    "hello from clien", // Max 16 bytes, the word "client" is truncated
    from_utf8(server.pop_received_message().unwrap().as_ref()).unwrap()
);

// New instructions for the mocked server
server.add_mock_instructions(vec![
    ReceiveMessage, // Wait for another message from the tested client
    SendMessageDependingOnLastReceivedMessage(|_| {
        None
    }), // No message is sent to the server
    SendMessageDependingOnLastReceivedMessage(|last_received_message| {
        // "hello2 from client"
        let mut received_message_string: String = from_utf8(&last_received_message.unwrap()).unwrap().to_string();
        // "hello2"
        received_message_string.truncate(5);
        Some(format!("{}2 from server", received_message_string).as_bytes().to_vec())
    }), // Send a message to the client depending on the last received message by the mocked server
    StopExchange, // Finally close the connection
]);

// Tested client send a message to the mocked server
client.write_all(b"hello2 from client").unwrap();

// Read a message sent by the mocked server
let mut buffer = [0; 1024];
let received_size = client.read(&mut buffer).unwrap();

// convert shrunk buffer to string
let received_message = from_utf8(&buffer[..received_size]).unwrap();

assert_eq!("hello2 from server", received_message);

assert_eq!(
    "hello2 from client",
    from_utf8(&*server.pop_received_message().unwrap()).unwrap()
);

// Check that no error has been raised by the mocked server
assert!(server.pop_server_error().is_none());
```

Another example in UDP:

```rust
use socket_server_mocker::ServerMocker;
use socket_server_mocker::Instruction::{SendMessage, SendMessageDependingOnLastReceivedMessage, ReceiveMessageWithMaxSize};
use std::net::UdpSocket;
use std::str::from_utf8;

// Mock a UDP server listening on port 35642. Note that the mock will only listen on the local interface.
let server = ServerMocker::udp_with_port(35642).unwrap();

// Create the UDP client to test at a random port
let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
client_socket.connect(server.socket_address()).unwrap();

// Mocked server behavior
server.add_mock_instructions(vec![
    // The mocked server will first wait for the client to send a message, with max size = 32 bytes
    ReceiveMessageWithMaxSize(32),
    // Then it will send a message to the client
    SendMessage(b"hello from server".to_vec()),
    // Send nothing
    SendMessageDependingOnLastReceivedMessage(|_| {
        None
    }),
    // Send a message to the client depending on the last received message by the mocked server
    SendMessageDependingOnLastReceivedMessage(|last_received_message| {
        // "hello2 from client"
        let mut received_message_string: String = from_utf8(&last_received_message.unwrap()).unwrap().to_string();
        // "hello2"
        received_message_string.truncate(5);
        Some(format!("{}2 from server", received_message_string).as_bytes().to_vec())
    }),
]
);

// UDP client sends its first message
client_socket.send(b"hello from client").unwrap();

// Read a message sent by the mocked server
let mut buffer = [0; 32];
let received_size = client_socket.recv(&mut buffer).unwrap();

// convert shrunk buffer to string
let received_message = from_utf8(&buffer[..received_size]).unwrap();

// Check that the message received by the client is the one sent by the mocked server
assert_eq!("hello from server", received_message);

// Check that the mocked server received the message sent by the client
assert_eq!(
    "hello from client",
    from_utf8(&*server.pop_received_message().unwrap()).unwrap()
);

let received_size = client_socket.recv(&mut buffer).unwrap();
// convert shrunk buffer to string
let received_message = from_utf8(&buffer[..received_size]).unwrap();

// Check that the message received by the client is the one sent by the mocked server
assert_eq!("hello2 from server", received_message);

// Check that no error has been raised by the mocked server
assert!(server.pop_server_error().is_none());
```

## Development

* This project is easier to develop with [just](https://github.com/casey/just#readme), a modern alternative to `make`.
  Install it with `cargo install just`.
* To get a list of available commands, run `just`.
* To run tests, use `just test`.
