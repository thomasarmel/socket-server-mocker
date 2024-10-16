#![doc = include_str!("../README.md")]

//! # socket-server-mocker
//!
//! `socket-server-mocker` is a library to mock a socket server.
//! It can be used to test a code that uses network socket to connect to a server.
//!
//! ## Example
//! Mock an HTTP server:
//! ```rust
//! use std::str::from_utf8;
//! use socket_server_mocker::ServerMocker;
//! use socket_server_mocker::Instruction::{ReceiveMessage, SendMessage, StopExchange};
//!
//! // Mock HTTP server on a random free port
//! let server = ServerMocker::tcp().unwrap();
//!
//! server.add_mock_instructions(vec![
//!   // Wait for an HTTP GET request
//!   ReceiveMessage,
//!   // Send an HTTP response
//!   SendMessage(b"HTTP/1.1 200 OK\r\nServer: socket-server-mocker-fake-http\r\nContent-Length: 12\r\nConnection: close\r\nContent-Type: text/plain\r\n\r\nHello, world".to_vec()),
//!   // Close the connection
//!   StopExchange,
//! ]).unwrap();
//!
//! // New reqwest blocking client
//! let client = reqwest::blocking::Client::new();
//! // Send an HTTP GET request to the mocked server
//! let response = client
//!   .get(format!("http://localhost:{}/", server.port()))
//!   .send()
//!   .unwrap();
//!
//! // Check response status code
//! assert!(response.status().is_success());
//!
//! // Check response body
//! assert_eq!(response.text().unwrap(), "Hello, world");
//! // Check HTTP request received by the mocked server
//! assert_eq!(
//!   format!(
//!     "GET / HTTP/1.1\r\naccept: */*\r\nhost: localhost:{}\r\n\r\n",
//!     server.port()
//!     ),
//!     from_utf8(&*server.pop_received_message().unwrap()).unwrap()
//!   );
//!
//! // Check that no error has been raised by the mocked server
//! assert!(server.pop_server_error().is_none());
//! ```

mod errors;
mod instructions;
mod server_mocker;
mod tcp_server;
mod udp_server;

pub use errors::ServerMockerError;
pub use instructions::Instruction;
pub use server_mocker::ServerMocker;
pub use tcp_server::TcpMocker;
pub use udp_server::UdpMocker;
