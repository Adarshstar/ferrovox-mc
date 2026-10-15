/// Connection lifecycle states per the Java Edition protocol handshake.
///
/// <https://minecraft.wiki/w/Java_Edition_protocol#Handshaking>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

impl ProtocolState {
    pub fn from_handshake_next_state(id: i32) -> Option<Self> {
        match id {
            1 => Some(ProtocolState::Status),
            2 => Some(ProtocolState::Login),
            3 => Some(ProtocolState::Login), // transfer, treated like login for now
            _ => None,
        }
    }
}
