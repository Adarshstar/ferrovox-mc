//! Per-connection packet loop: handshake -> status or login.
//!
//! This is the Phase 0 slice: it answers server-list pings fully and
//! takes a login handshake far enough to read the player's name and
//! respond (unencrypted / offline-mode flow for now — online-mode
//! session auth + encryption lands alongside world join in Phase 1).

use crate::packet::{read_packet, write_packet, PacketReader, PacketWriter};
use crate::state::ProtocolState;
use crate::status::build_status_json;
use ferrovox_core::ServerConfig;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, info, warn};

pub async fn handle_connection<S>(mut stream: S, cfg: Arc<ServerConfig>) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut state = ProtocolState::Handshake;

    loop {
        let packet = match read_packet(&mut stream).await {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };

        match state {
            ProtocolState::Handshake => {
                if packet.id != 0x00 {
                    warn!(id = packet.id, "unexpected packet during handshake");
                    return Ok(());
                }
                let mut r = PacketReader::new(&packet.data);
                let protocol_version = r.read_varint()?;
                let server_address = r.read_string()?;
                let server_port = r.read_u16()?;
                let next_state = r.read_varint()?;

                debug!(
                    protocol_version,
                    %server_address,
                    server_port,
                    next_state,
                    "received handshake"
                );

                state = ProtocolState::from_handshake_next_state(next_state).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "bad next_state")
                })?;
            }

            ProtocolState::Status => match packet.id {
                0x00 => {
                    let json = build_status_json(&cfg.server.motd, 0, cfg.server.max_players);
                    let mut w = PacketWriter::new();
                    w.write_string(&json);
                    write_packet(&mut stream, 0x00, &w.buf).await?;
                }
                0x01 => {
                    // Ping: echo the payload straight back (Pong).
                    write_packet(&mut stream, 0x01, &packet.data).await?;
                    return Ok(());
                }
                other => {
                    warn!(id = other, "unexpected packet during status");
                    return Ok(());
                }
            },

            ProtocolState::Login => {
                if packet.id == 0x00 {
                    let mut r = PacketReader::new(&packet.data);
                    let name = r.read_string()?;
                    info!(player = %name, "login start (offline-mode stub, play-phase join lands in Phase 1)");
                    // Phase 0 stops here: encryption + world join are
                    // Phase 1 work. Real server will branch into online
                    // mode auth or straight to Login Success here.
                }
                return Ok(());
            }

            ProtocolState::Configuration | ProtocolState::Play => {
                // Not yet implemented — reserved for Phase 1+.
                return Ok(());
            }
        }
    }
}
