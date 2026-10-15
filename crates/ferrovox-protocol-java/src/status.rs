//! Status (server list ping) response construction.

use ferrovox_core::versions;
use serde_json::json;

/// Build the JSON status response sent in reply to a Status Request
/// packet, shown in the multiplayer server list.
pub fn build_status_json(motd: &str, online: u32, max: u32) -> String {
    json!({
        "version": {
            "name": versions::JAVA_EDITION_NAME,
            "protocol": versions::JAVA_PROTOCOL_VERSION
        },
        "players": {
            "max": max,
            "online": online,
            "sample": []
        },
        "description": { "text": motd },
        "enforcesSecureChat": false
    })
    .to_string()
}
