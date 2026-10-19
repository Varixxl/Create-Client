use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const REMOTE_HOST: &str = "argon.dathost.net";
const REMOTE_PORT: u16 = 17155;
const SERVER_DISPLAY_NAME: &str = "Create Server";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub server_name: String,
    pub online: bool,
    pub ping_ms: u64,
    pub online_players: u32,
    pub max_players: u32,
    pub motd: String,
    pub version: String,
}

#[derive(Deserialize)]
struct SlpResponseJson {
    players: Option<SlpPlayers>,
    version: Option<SlpVersion>,
    description: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct SlpPlayers {
    online: Option<u32>,
    max: Option<u32>,
}

#[derive(Deserialize)]
struct SlpVersion {
    name: Option<String>,
}

fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        let mut temp = (value & 0x7F) as u8;
        value = (value >> 7) & (i32::MAX >> 6); // logical shift
        if value != 0 {
            temp |= 0x80;
        }
        buf.push(temp);
        if value == 0 {
            break;
        }
    }
}

async fn read_varint<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<i32, String> {
    let mut num_read = 0;
    let mut result: i32 = 0;
    loop {
        let mut byte = [0u8; 1];
        reader
            .read_exact(&mut byte)
            .await
            .map_err(|e| format!("VarInt read error: {}", e))?;
        let read = byte[0];
        let value = (read & 0x7F) as i32;
        result |= value << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err("VarInt is too big".to_string());
        }
        if (read & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

pub async fn query_server_status() -> ServerStatus {
    ping_server_address(REMOTE_HOST, REMOTE_PORT, SERVER_DISPLAY_NAME).await
}

pub async fn ping_server_address(host: &str, port: u16, name: &str) -> ServerStatus {
    match timeout(Duration::from_millis(3500), ping_minecraft_server(host, port, name)).await {
        Ok(Ok(status)) => status,
        Ok(Err(_)) | Err(_) => ServerStatus {
            server_name: name.to_string(),
            online: false,
            ping_ms: 0,
            online_players: 0,
            max_players: 0,
            motd: "Offline".to_string(),
            version: "Unknown".to_string(),
        },
    }
}

async fn ping_minecraft_server(host: &str, port: u16, name: &str) -> Result<ServerStatus, String> {
    let start_time = Instant::now();
    let mut stream = TcpStream::connect((host, port))
        .await
        .map_err(|e| format!("Failed to connect to server: {}", e))?;

    // 1. Send Handshake Packet (ID 0x00)
    let mut handshake_payload = Vec::new();
    write_varint(&mut handshake_payload, 0x00); // Packet ID
    write_varint(&mut handshake_payload, 763); // Protocol version for 1.20.1 (works for generic SLP handshake)
    write_varint(&mut handshake_payload, host.len() as i32); // Host length
    handshake_payload.extend_from_slice(host.as_bytes()); // Host string
    handshake_payload.extend_from_slice(&port.to_be_bytes()); // Port (u16)
    write_varint(&mut handshake_payload, 1); // Next state: Status

    let mut handshake_packet = Vec::new();
    write_varint(&mut handshake_packet, handshake_payload.len() as i32);
    handshake_packet.extend(handshake_payload);

    stream
        .write_all(&handshake_packet)
        .await
        .map_err(|e| format!("Handshake write failed: {}", e))?;

    // 2. Send Status Request Packet (ID 0x00, Length 1)
    let request_packet = [0x01, 0x00];
    stream
        .write_all(&request_packet)
        .await
        .map_err(|e| format!("Status request write failed: {}", e))?;

    // 3. Read Status Response
    let packet_len = read_varint(&mut stream).await?;
    if packet_len <= 0 || packet_len > 65536 {
        return Err(format!("Invalid SLP packet length: {}", packet_len));
    }
    let packet_id = read_varint(&mut stream).await?;
    if packet_id != 0x00 {
        return Err(format!("Unexpected packet ID: {}", packet_id));
    }

    let json_len = read_varint(&mut stream).await?;
    if json_len <= 0 || json_len > 65536 {
        return Err(format!("Invalid SLP JSON length: {}", json_len));
    }
    let mut json_buf = vec![0u8; json_len as usize];
    stream
        .read_exact(&mut json_buf)
        .await
        .map_err(|e| format!("Failed to read JSON payload: {}", e))?;

    let ping_duration = start_time.elapsed().as_millis() as u64;

    let json_str = String::from_utf8_lossy(&json_buf);
    let slp: SlpResponseJson = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse SLP JSON: {}", e))?;

    let (online_players, max_players) = if let Some(p) = slp.players {
        (p.online.unwrap_or(0), p.max.unwrap_or(20))
    } else {
        (0, 20)
    };

    let motd = match slp.description {
        Some(serde_json::Value::String(s)) => s,
        Some(serde_json::Value::Object(obj)) => {
            obj.get("text")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string()
        }
        _ => name.to_string(),
    };

    let version = slp
        .version
        .and_then(|v| v.name)
        .unwrap_or_else(|| "Online".to_string());

    Ok(ServerStatus {
        server_name: name.to_string(),
        online: true,
        ping_ms: ping_duration,
        online_players,
        max_players,
        motd,
        version,
    })
}
