//! Minimal RakNet offline-message handling: unconnected ping/pong, which
//! is all that's needed for the server to appear in the Bedrock LAN /
//! "Friends" / direct-connect server list.
//!
//! Wire format reference: <https://wiki.vg/Raknet_Protocol>

use ferrovox_core::versions;

/// RakNet magic bytes present in every offline message.
pub const RAKNET_MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

pub const ID_UNCONNECTED_PING: u8 = 0x01;
pub const ID_UNCONNECTED_PONG: u8 = 0x1c;

#[derive(Debug)]
pub struct UnconnectedPing {
    pub timestamp: i64,
    pub client_guid: i64,
}

/// Parse an incoming Unconnected Ping datagram. Returns `None` if the
/// packet doesn't match the expected id/magic.
pub fn parse_unconnected_ping(buf: &[u8]) -> Option<UnconnectedPing> {
    if buf.is_empty() || buf[0] != ID_UNCONNECTED_PING {
        return None;
    }
    // id(1) + timestamp(8) + magic(16) + client_guid(8)
    if buf.len() < 1 + 8 + 16 + 8 {
        return None;
    }
    let timestamp = i64::from_be_bytes(buf[1..9].try_into().ok()?);
    if buf[9..25] != RAKNET_MAGIC {
        return None;
    }
    let client_guid = i64::from_be_bytes(buf[25..33].try_into().ok()?);
    Some(UnconnectedPing {
        timestamp,
        client_guid,
    })
}

/// Build the Unconnected Pong reply, including the MOTD/server info
/// string Bedrock clients parse to populate the server list entry.
pub fn build_unconnected_pong(
    ping: &UnconnectedPing,
    server_guid: i64,
    motd: &str,
    port: u16,
    max_players: u32,
) -> Vec<u8> {
    // Semicolon-delimited server list string, per the Bedrock RakNet
    // "MOTD" convention (edition;motd1;protocol;version;online;max;guid;
    // motd2;gamemode;gamemode_numeric;port4;port6).
    let info = format!(
        "MCPE;{motd};{proto};{ver};0;{max};{guid};{motd};Survival;1;{port4};{port6};",
        motd = motd,
        proto = versions::BEDROCK_PROTOCOL_VERSION,
        ver = versions::BEDROCK_EDITION_NAME,
        max = max_players,
        guid = server_guid,
        port4 = port,
        port6 = port,
    );

    let mut out = Vec::with_capacity(1 + 8 + 8 + 16 + 2 + info.len());
    out.push(ID_UNCONNECTED_PONG);
    out.extend_from_slice(&ping.timestamp.to_be_bytes());
    out.extend_from_slice(&server_guid.to_be_bytes());
    out.extend_from_slice(&RAKNET_MAGIC);
    out.extend_from_slice(&(info.len() as u16).to_be_bytes());
    out.extend_from_slice(info.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ping_pong() {
        let mut ping_buf = Vec::new();
        ping_buf.push(ID_UNCONNECTED_PING);
        ping_buf.extend_from_slice(&1234i64.to_be_bytes());
        ping_buf.extend_from_slice(&RAKNET_MAGIC);
        ping_buf.extend_from_slice(&5678i64.to_be_bytes());

        let ping = parse_unconnected_ping(&ping_buf).expect("valid ping");
        assert_eq!(ping.timestamp, 1234);
        assert_eq!(ping.client_guid, 5678);

        let pong = build_unconnected_pong(&ping, 999, "A FerroVox Server", 19132, 100);
        assert_eq!(pong[0], ID_UNCONNECTED_PONG);
    }
}
