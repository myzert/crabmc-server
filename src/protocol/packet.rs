use super::varint::{VarIntRead, VarIntWrite};
use serde::{Deserialize, Serialize};
use std::io::{self, Cursor, Read};

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Handshaking,
    Status,
    Login,
    Play,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct HandshakePacket {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: ConnectionState,
}

impl HandshakePacket {
    pub fn decode(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let protocol_version = cursor.read_varint()?;
        let server_address = cursor.read_minecraft_string()?;
        let mut port_buf = [0u8; 2];
        cursor.read_exact(&mut port_buf)?;
        let server_port = u16::from_be_bytes(port_buf);
        let next_state_raw = cursor.read_varint()?;

        let next_state = match next_state_raw {
            1 => ConnectionState::Status,
            2 => ConnectionState::Login,
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid next_state: {}", other),
                ))
            }
        };

        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerVersionInfo {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerSample {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayersInfo {
    pub max: i32,
    pub online: i32,
    #[serde(default)]
    pub sample: Vec<PlayerSample>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DescriptionInfo {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponseJson {
    pub version: ServerVersionInfo,
    pub players: PlayersInfo,
    pub description: DescriptionInfo,
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: bool,
    #[serde(rename = "previewsChat")]
    pub previews_chat: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
}

pub fn encode_packet(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    let mut id_buf = Vec::new();
    id_buf.write_varint(packet_id).unwrap();

    let total_len = id_buf.len() + payload.len();
    let mut result = Vec::new();
    result.write_varint(total_len as i32).unwrap();
    result.extend_from_slice(&id_buf);
    result.extend_from_slice(payload);
    result
}

pub fn encode_status_response(json_string: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.write_minecraft_string(json_string).unwrap();
    encode_packet(0x00, &payload)
}

pub fn encode_disconnect_login(reason_json: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.write_minecraft_string(reason_json).unwrap();
    encode_packet(0x00, &payload)
}
