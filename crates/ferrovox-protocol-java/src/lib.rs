//! A from-scratch implementation of the Minecraft: Java Edition network
//! protocol, currently targeting Java Edition 26.3 (protocol version 777).
//!
//! Phase 0 scope: handshake, status (server list ping), and enough of the
//! login sequence to accept a connecting client. Play-phase packets land
//! in later phases as the world/entity systems come online.

pub mod connection;
pub mod encryption;
pub mod packet;
pub mod state;
pub mod status;

pub use connection::handle_connection;
pub use state::ProtocolState;
