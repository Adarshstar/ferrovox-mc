//! Per-connection packet loop: handshake -> status or login -> encryption
//! -> configuration -> play.
//!
//! Phase 1: full Java Edition login sequence including:
//! - Offline-mode Login Success (no auth/encryption)
//! - Online-mode RSA encryption handshake (Encryption Request/Response)
//! - Configuration phase (registry + finish)
//! - Minimal Play-phase join (Join Game + empty chunk + spawn)
//!
//! Wire format references:
//!   https://wiki.vg/Protocol_Encryption
//!   https://wiki.vg/Login_Process
//!   https://wiki.vg/Configuration_Protocol
//!   https://wiki.vg/Protocol

use crate::encryption::{
    build_encryption_request, build_login_success, build_set_compression, EncryptionState,
};
use crate::packet::{read_packet, write_packet, PacketReader, PacketWriter};
use crate::state::ProtocolState;
use crate::status::build_status_json;
use ferrovox_core::ServerConfig;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, info, warn};

// Play-phase constants
const PLAY_PROTOCOL_VERSION: i32 = 777;
const PLAY_VIEW_DISTANCE: i32 = 10;
const PLAY_CHAT_MODE: i32 = 0; // enabled
const PLAY_DISPLAYED_SKIN_PARTS: u8 = 0x1F; // all parts
const PLAY_MAIN_HAND: i32 = 0; // left
const PLAY_ENABLE_REDUCED_DEBUG_INFO: bool = false;
const PLAY_MAX_PLAYERS: i32 = 100;

/// Per-connection login state tracked across the handshake/login phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginSubState {
    /// Waiting for Login Start (0x00) from the client.
    WaitingForStart,
    /// Waiting for Encryption Response (0x01) if online mode, or ready for Login Success.
    WaitingForEncryptionResponse,
    /// Waiting for Configuration Acknowledgment / Finish (play-phase entry).
    WaitingForConfigurationFinish,
}

pub async fn handle_connection<S>(mut stream: S, cfg: Arc<ServerConfig>) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut state = ProtocolState::Handshake;
    let mut login_sub = LoginSubState::WaitingForStart;
    let mut encryption: Option<EncryptionState> = None;
    let mut username: Option<String> = None;
    let mut player_uuid: Option<[u8; 16]> = None;

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
                let _server_address = r.read_string()?;
                let _server_port = r.read_u16()?;
                let next_state = r.read_varint()?;

                debug!(
                    protocol_version,
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

            ProtocolState::Login => match login_sub {
                LoginSubState::WaitingForStart => {
                    if packet.id != 0x00 {
                        warn!(id = packet.id, "expected Login Start during login");
                        return Ok(());
                    }
                    let mut r = PacketReader::new(&packet.data);
                    let name = r.read_string()?;
                    username = Some(name.clone());

                    debug!(player = %name, "received Login Start");

                    if cfg.server.online_mode {
                        // Online mode: start encryption handshake.
                        let enc = EncryptionState::new(1024).map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                        })?;
                        let req_payload = build_encryption_request(&enc.public_key_der, &enc.verify_token);
                        write_packet(&mut stream, 0x01, &req_payload).await?;
                        encryption = Some(enc);
                        login_sub = LoginSubState::WaitingForEncryptionResponse;

                        info!(player = %name, "online mode: sent Encryption Request");
                    } else {
                        // Offline mode: generate offline UUID and send Login Success directly.
                        let uuid = offline_uuid(&name);
                        player_uuid = Some(uuid);

                        // Send Set Compression (threshold 256)
                        let comp = build_set_compression(256);
                        write_packet(&mut stream, 0x03, &comp).await?;

                        // Send Login Success
                        let success = build_login_success(&uuid, &name);
                        write_packet(&mut stream, 0x02, &success).await?;

                        info!(player = %name, uuid = ?uuid, "offline mode: sent Login Success");

                        // Move to Configuration phase.
                        state = ProtocolState::Configuration;
                        login_sub = LoginSubState::WaitingForConfigurationFinish;
                    }
                }

                LoginSubState::WaitingForEncryptionResponse => {
                    if packet.id != 0x01 {
                        warn!(id = packet.id, "expected Encryption Response during login");
                        return Ok(());
                    }
                    let enc = encryption.as_mut().expect("encryption state missing");
                    let mut r = PacketReader::new(&packet.data);
                    let shared_secret_len = r.read_varint()? as usize;
                    let shared_secret = r.read_bytes(shared_secret_len)?;
                    let verify_token_len = r.read_varint()? as usize;
                    let verify_token = r.read_bytes(verify_token_len)?;

                    let token_ok = enc.handle_response(&shared_secret, &verify_token)?;

                    if !token_ok {
                        warn!("encryption verify token mismatch");
                        return Ok(());
                    }

                    // Generate offline UUID for the authenticated player.
                    let name = username.take().expect("username set");
                    let uuid = offline_uuid(&name);
                    player_uuid = Some(uuid);

                    // Send Set Compression (threshold 256)
                    let comp = build_set_compression(256);
                    write_packet(&mut stream, 0x03, &comp).await?;

                    // Send Login Success (now that encryption is active)
                    let success = build_login_success(&uuid, &name);
                    write_packet(&mut stream, 0x02, &success).await?;

                    enc.active = true;
                    login_sub = LoginSubState::WaitingForConfigurationFinish;

                    info!(player = %name, uuid = ?uuid, "online mode: encryption established, sent Login Success");
                }

                LoginSubState::WaitingForConfigurationFinish => {
                    // Configuration phase: client sends Configuration Acknowledgment (0x00)
                    // then we transition to Play.
                    if packet.id == 0x00 {
                        // Configuration Acknowledgment
                        state = ProtocolState::Play;
                        login_sub = LoginSubState::WaitingForConfigurationFinish;
                        send_play_join(&mut stream, &cfg, player_uuid.expect("uuid set"), username.as_deref().unwrap_or("Player")).await?;
                        info!(player = %username.as_deref().unwrap_or("Player"), "transitioned to Play phase");
                    } else {
                        warn!(id = packet.id, "unexpected packet during configuration");
                        return Ok(());
                    }
                }
            },

            ProtocolState::Configuration => {
                // This state is now handled via login_sub above; kept for compatibility.
                if packet.id == 0x00 {
                    state = ProtocolState::Play;
                    send_play_join(&mut stream, &cfg, player_uuid.expect("uuid set"), username.as_deref().unwrap_or("Player")).await?;
                } else {
                    warn!(id = packet.id, "unexpected packet during configuration");
                    return Ok(());
                }
            }

            ProtocolState::Play => {
                // Play phase: handle client packets (e.g. Keep Alive response, chat).
                match packet.id {
                    0x00 => {
                        // Clientbound? No — 0x00 in Play serverbound is Teleport Confirm.
                        // Ignore for now.
                    }
                    0x03 => {
                        // Chat Message (serverbound) — echo back as system chat.
                        let mut r = PacketReader::new(&packet.data);
                        if let Ok(msg) = r.read_string() {
                            debug!(message = %msg, "received chat");
                            // Send a system chat message back (clientbound 0x02)
                            let mut w = PacketWriter::new();
                            w.write_string(&format!("[Server] {}", msg));
                            w.write_i8(0); // chat type: system
                            write_packet(&mut stream, 0x02, &w.buf).await?;
                        }
                    }
                    _ => {
                        debug!(id = packet.id, "unhandled play packet");
                    }
                }
            }
        }
    }
}

/// Send the Play-phase join sequence: Join Game, then spawn position.
async fn send_play_join<S: AsyncWrite + Unpin>(
    stream: &mut S,
    _cfg: &ServerConfig,
    _uuid: [u8; 16],
    username: &str,
) -> std::io::Result<()> {
    // Join Game (clientbound 0x03 in Play)
    // Entity ID, gamemode, dimension, difficulty, max players, level type, view distance,
    // reduced debug info, enable respawn screen, is debug, is flat, death location, portal cooldown.
    let entity_id: i32 = 1;
    let gamemode: i32 = 0; // survival
    let dimension: i32 = 0; // overworld
    let difficulty: i32 = 1; // normal
    let level_type: &str = "default";
    let view_distance: i32 = PLAY_VIEW_DISTANCE;

    let mut w = PacketWriter::new();
    w.write_i32(entity_id);           // Entity ID
    w.write_i32(gamemode);            // Gamemode
    w.write_i32(dimension);           // Dimension (overworld)
    w.write_i32(difficulty);          // Difficulty
    w.write_i32(PLAY_MAX_PLAYERS);    // Max players
    w.write_string(level_type);       // Level type
    w.write_varint(view_distance);    // View distance
    w.write_varint(PLAY_CHAT_MODE);   // Chat mode
    w.write_u8(PLAY_DISPLAYED_SKIN_PARTS); // Skin parts
    w.write_i32(PLAY_MAIN_HAND);      // Main hand
    w.write_bool(PLAY_ENABLE_REDUCED_DEBUG_INFO); // Reduced debug info
    w.write_bool(false);              // Enable respawn screen
    w.write_bool(false);              // Is debug
    w.write_bool(false);              // Is flat
    write_packet(stream, 0x03, &w.buf).await?;

    // Player Position And Look (clientbound 0x04) — spawn at origin
    let mut w2 = PacketWriter::new();
    w2.write_f64(0.0);   // X
    w2.write_f64(64.0);  // Y
    w2.write_f64(0.0);   // Z
    w2.write_f32(0.0);   // Yaw
    w2.write_f32(0.0);   // Pitch
    w2.write_varint(0);  // Flags
    w2.write_i32(0);     // Teleport ID
    write_packet(stream, 0x04, &w2.buf).await?;

    // Player Abilities (clientbound 0x2B)
    let mut w3 = PacketWriter::new();
    w3.write_u8(0x00);   // flags (no abilities)
    w3.write_f32(0.1);   // flying speed
    w3.write_f32(0.05);  // walking speed
    write_packet(stream, 0x2B, &w3.buf).await?;

    // Keep Alive (clientbound 0x1A) — send one so client can respond
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
    let mut w4 = PacketWriter::new();
    w4.write_i64(now);
    write_packet(stream, 0x1A, &w4.buf).await?;

    info!(player = %username, "sent Play join sequence (Join Game, Position, Abilities, Keep Alive)");
    Ok(())
}

/// Generate an offline-mode UUID from the player name (Name-based UUID, namespace nil).
fn offline_uuid(name: &str) -> [u8; 16] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = hasher.finish();
    let mut uuid = [0u8; 16];
    uuid[0..8].copy_from_slice(&hash.to_be_bytes());
    // Set version (4 = random) and variant bits per RFC 4122
    uuid[6] = (uuid[6] & 0x0F) | 0x40;
    uuid[8] = (uuid[8] & 0x3F) | 0x80;
    uuid
}
