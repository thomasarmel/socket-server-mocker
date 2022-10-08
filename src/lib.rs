//! # socket-server-mocker
//!
//! `socket-server-mocker` is a library to mock a server socket.
//! It can be used to test a code that uses network socket to connect to a server.

pub mod server_mocker;
pub mod server_mocker_instruction;
pub use server_mocker::tcp_server_mocker;
pub use server_mocker::udp_server_mocker;
