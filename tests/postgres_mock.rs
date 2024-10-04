//! database: mockeddatabase, user: admin, password: password
//! CREATE TABLE playground (
//! id serial PRIMARY KEY,
//! data1 varchar (50) NOT NULL,
//! data2 varchar (50) NOT NULL
//! );
//! Note: In modern `PostgreSQL`, the default authentication method is scram-sha-256.
//! This hash method is secured by a nonce, so this mocked server uses md5 instead.

use postgres::{Client, NoTls};
use socket_server_mocker::Instruction::{ReceiveMessage, SendMessage};
use socket_server_mocker::{ServerMocker, TcpServerMocker};

#[test]
fn postgres_insert_mock() {
    // Mock PostgreSQL server on a port 54321 (default PostgresSQL port is 5432)
    let postgres_server_mocker = TcpServerMocker::new_with_port(54321).unwrap();

    // Add mock binary messages corresponding to client connection and authentication
    postgres_server_mocker
        .add_mock_instructions(vec![
            ReceiveMessage,
            SendMessage(b"R\x00\x00\x00\x0c\x00\x00\x00\x05\x1cS\xa5\xf3".into()),
            ReceiveMessage,
            SendMessage(
                b"R\x00\x00\x00\x08\x00\x00\x00\x00S\x00\x00\x00\
\x16application_name\x00\x00S\x00\x00\x00\x19client_encoding\x00UTF8\x00S\x00\x00\x00\x17DateStyle\x00ISO, DMY\x00S\x00\x00\
\x00&default_transaction_read_only\x00off\x00S\x00\x00\x00\x17in_hot_standby\x00off\x00S\x00\x00\x00\x19integer_datetimes\x00on\x00S\
\x00\x00\x00\x1bIntervalStyle\x00postgres\x00S\x00\x00\x00\x14is_superuser\x00on\x00S\x00\x00\x00\x19server_encoding\x00UTF8\x00S\x00\
\x00\x004server_version\x0014.5 (Ubuntu 14.5-1.pgdg22.04+1)\x00S\x00\x00\x00 session_authorization\x00admin\x00S\x00\x00\x00\
#standard_conforming_strings\x00on\x00S\x00\x00\x00\x1aTimeZone\x00Europe/Paris\x00K\x00\x00\x00\x0c\x00\x00\x0a\x04EE\x04\xb9Z\x00\x00\x00\x05I"
                    .into(),
            ),
        ])
        .unwrap();

    // Connect to local mocked PostgreSQL server
    let mut client = Client::connect(
        "host=localhost user=admin password=password dbname=mockeddatabase port=54321",
        NoTls,
    )
    .unwrap();

    // Check connection message sent by the client to mock server is correct
    assert_eq!(
        b"\x00\x00\x00A\x00\x03\x00\x00client_encoding\x00UTF8\x00user\x00admin\x00database\x00mockeddatabase\x00\x00",
        postgres_server_mocker.pop_received_message().unwrap().as_slice()
    );

    // Cannot verify the authentication message sent by the client to mock server because it contains a random salt
    postgres_server_mocker.pop_received_message().unwrap();

    // Add mock instructions corresponding to the client INSERT query
    postgres_server_mocker
        .add_mock_instructions(vec![
            ReceiveMessage,
            SendMessage(b"1\x00\x00\x00\x04t\x00\x00\x00\x0e\x00\x02\x00\x00\x04\x13\x00\x00\x04\x13n\x00\x00\x00\x04Z\x00\x00\x00\x05I".into()),
            ReceiveMessage,
            SendMessage(b"2\x00\x00\x00\x04C\x00\x00\x00\x0fINSERT 0 1\x00Z\x00\x00\x00\x05I".into()),
        ])
        .unwrap();

    // Execute the INSERT query
    client
        .execute(
            "INSERT INTO playground (data1, data2) VALUES ($1, $2)",
            &[&"test1", &"test2"],
        )
        .unwrap();

    // Check that no error has been raised by the mocked server
    assert!(postgres_server_mocker.pop_server_error().is_none());
}

#[test]
fn postgres_select_mock() {
    // Mock PostgreSQL server on a random free port (default PostgresSQL port is 5432)
    let postgres_server_mocker = TcpServerMocker::new().unwrap();

    // Add mock binary messages corresponding to client connection and authentication
    postgres_server_mocker
        .add_mock_instructions(vec![
            ReceiveMessage,
            SendMessage(b"R\x00\x00\x00\x0c\x00\x00\x00\x05\xb8(/\xf6".into()),
            ReceiveMessage,
            SendMessage(b"\
R\x00\x00\x00\x08\x00\x00\x00\x00S\x00\x00\x00\x16application_name\x00\x00S\x00\x00\x00\x19client_encoding\x00\
UTF8\x00S\x00\x00\x00\x17DateStyle\x00ISO, DMY\x00S\x00\x00\x00&default_transaction_read_only\x00off\x00\
S\x00\x00\x00\x17in_hot_standby\x00off\x00S\x00\x00\x00\x19integer_datetimes\x00on\x00S\x00\x00\x00\x1bIntervalStyle\x00\
postgres\x00S\x00\x00\x00\x14is_superuser\x00on\x00S\x00\x00\x00\x19server_encoding\x00UTF8\x00S\x00\x00\x004server_version\x00\
14.5 (Ubuntu 14.5-1.pgdg22.04+1)\x00S\x00\x00\x00 session_authorization\x00admin\x00S\x00\x00\x00#standard_conforming_strings\x00\
on\x00S\x00\x00\x00\x1aTimeZone\x00Europe/Paris\x00K\x00\x00\x00\x0c\x00\x00\x0a\xb6\xe4kH\xa2Z\x00\x00\x00\x05I".into()),
        ])
        .unwrap();

    // Connect to local mocked PostgreSQL server
    let mut client = Client::connect(
        &format!(
            "host=localhost user=admin password=password dbname=mockeddatabase port={}",
            postgres_server_mocker.port()
        ),
        NoTls,
    )
    .unwrap();

    // Check connection message sent by the client to mock server is correct
    assert_eq!(
        b"\x00\x00\x00A\x00\x03\x00\x00client_encoding\x00UTF8\x00user\x00admin\x00database\x00mockeddatabase\x00\x00",
        postgres_server_mocker.pop_received_message().unwrap().as_slice()
    );

    // Cannot verify the authentication message sent by the client to mock server because it contains a random salt
    postgres_server_mocker.pop_received_message().unwrap();

    // Add mock instructions corresponding to the client SELECT query
    postgres_server_mocker
        .add_mock_instructions(vec![
            ReceiveMessage,
            SendMessage(b"1\x00\x00\x00\x04t\x00\x00\x00\x06\x00\x00T\x00\x00\x00K\x00\x03id\x00\x00\x00@\x0a\x00\x01\x00\x00\x00\x17\x00\x04\xff\xff\xff\xff\x00\x00data1\x00\x00\x00@\x0a\x00\x02\x00\x00\x04\x13\xff\xff\x00\x00\x00\x36\x00\x00data2\x00\x00\x00@\x0a\x00\x03\x00\x00\x04\x13\xff\xff\x00\x00\x006\x00\x00Z\x00\x00\x00\x05I".into()),
            ReceiveMessage,
            SendMessage(b"2\x00\x00\x00\x04D\x00\x00\x00 \x00\x03\x00\x00\x00\x04\x00\x00\x00\x01\x00\x00\x00\x05test1\x00\x00\x00\x05test2C\x00\x00\x00\x0dSELECT 1\x00Z\x00\x00\x00\x05I".into()),
        ])
        .unwrap();

    // Execute the client SELECT query
    let rows = client.query("SELECT * FROM playground", &[]).unwrap();

    // Check the SELECT query result
    assert_eq!(1, rows.len());
    assert_eq!(1, rows[0].get::<_, i32>("id"));
    assert_eq!("test1", rows[0].get::<_, String>("data1"));
    assert_eq!("test2", rows[0].get::<_, String>("data2"));

    // Check that no error has been raised by the mocked server
    assert!(postgres_server_mocker.pop_server_error().is_none());
}
