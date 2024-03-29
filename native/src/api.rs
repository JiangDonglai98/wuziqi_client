pub use crate::structs::*;
use anyhow::{Error, Result};
use async_std::channel::Sender;
use async_std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use async_std::stream::StreamExt;
use async_std::sync::Mutex;
use async_std::task;
use async_std::task::block_on;
use flutter_rust_bridge::StreamSink;
use std::io::{Cursor, Read};
use std::time::Duration;
use wuziqi;
use wuziqi::{Conn, Received};

const CLIENT_PING_INTERVAL: Option<Duration> = Some(Duration::from_secs(5));
const MAX_DATA_SIZE: u32 = 1024;
static CONN_SENDER: Mutex<Option<Sender<Messages>>> = Mutex::new(None);

pub fn connect_to_server(
    sink: StreamSink<Responses>,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    server_port: u16,
    user_name: String,
) -> Result<()> {
    let tcp = block_on(TcpStream::connect(SocketAddrV4::new(
        Ipv4Addr::new(a, b, c, d),
        server_port,
    )))?;
    let mut conn: Conn<Messages, Responses> = Conn::init(tcp, CLIENT_PING_INTERVAL, MAX_DATA_SIZE);
    let _ = block_on(conn.sender().send(Messages::UserName(user_name)));
    let sender = conn.sender().clone();
    let mut conn_sender_lock = block_on(CONN_SENDER.lock());
    conn_sender_lock.replace(sender);
    task::spawn(async move {
        while let Some(rsp) = conn.next().await {
            if let Received::Response(rsp) = rsp {
                if !sink.add(rsp) {
                    break;
                }
            }
        }
    });
    Ok(())
}

/// use this function to send messages to server
/// this function does nothing if the connection is not yet established
pub fn send(msg: Messages) -> Result<()> {
    let conn_sender_lock = block_on(CONN_SENDER.lock());
    if let Some(sender) = conn_sender_lock.as_ref() {
        block_on(sender.send(msg)).map_err(|_| Error::msg("connection already shutdown"))
    } else {
        Err(Error::msg("connection uninitialized"))
    }
}

/// for flutter debug use only
pub fn empty_field() -> Field {
    Field {
        latest_x: None,
        latest_y: None,
        rows: vec![
            FieldRow {
                columns: vec![SingleState::E; 15]
            };
            15
        ],
    }
}

/// for flutter debug use only.
/// 0 for empty, 1 for black, 2 for white
/// ! do not use this in production
pub fn construct_field_with_latest(latest_x: i32, latest_y: i32, seeds: Vec<u8>) -> Field {
    assert_eq!(seeds.len(), 15 * 15);
    let mut cursor = Cursor::new(seeds);
    let mut row = [0u8; 15];
    let mut rows = Vec::with_capacity(15);
    for _ in 0..15 {
        cursor.read_exact(&mut row).unwrap();
        rows.push(FieldRow {
            columns: row
                .map(|i| match i {
                    1 => SingleState::B,
                    2 => SingleState::W,
                    _ => SingleState::E,
                })
                .to_vec(),
        });
    }
    Field {
        latest_x: Some(latest_x),
        latest_y: Some(latest_y),
        rows,
    }
}

pub fn default_session_config() -> SessionConfig {
    SessionConfig {
        undo_request_timeout: 0,
        undo_dialogue_extra_seconds: 0,
        play_timeout: 0,
    }
}

pub fn set_undo_request_timeout(mut config: SessionConfig, secs: u64) -> SessionConfig {
    config.undo_request_timeout = secs;
    config
}

pub fn set_undo_dialogue_extra_seconds(mut config: SessionConfig, secs: u64) -> SessionConfig {
    config.undo_dialogue_extra_seconds = secs;
    config
}

pub fn set_play_timeout(mut config: SessionConfig, secs: u64) -> SessionConfig {
    config.play_timeout = secs;
    config
}
