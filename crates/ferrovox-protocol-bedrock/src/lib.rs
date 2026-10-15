//! A from-scratch implementation of the relevant parts of RakNet needed to
//! speak Minecraft: Bedrock Edition, currently targeting Bedrock Edition
//! 26.52.3 (network protocol version 2193).
//!
//! Phase 0 scope: offline RakNet handshake (unconnected ping/pong so the
//! server shows up in the Bedrock server list, plus connection request
//! parsing). Full reliability-layer datagram handling and the Bedrock
//! login sequence land in Phase 1.

pub mod raknet;

pub use raknet::{build_unconnected_pong, parse_unconnected_ping, UnconnectedPing};
