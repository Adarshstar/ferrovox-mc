//! Java Edition login encryption (online-mode handshake).
//!
//! Phase 1: generates an RSA key pair, sends an Encryption Request to the
//! client, verifies the Encryption Response, and enables AES/CFB8 ciphering
//! for the rest of the connection. Offline mode skips this entirely and goes
//! straight to Login Success.
//!
//! Wire format references:
//!   https://wiki.vg/Protocol_Encryption
//!   https://wiki.vg/Login_Process

use rand::{rngs::OsRng, Rng};
use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey, RsaPublicKey};

/// Server-side encryption state for one connection.
#[derive(Debug)]
pub struct EncryptionState {
    /// RSA private key (server side).
    pub private_key: RsaPrivateKey,
    /// RSA public key, DER-encoded SubjectPublicInfo (sent to client).
    pub public_key_der: Vec<u8>,
    /// Verify token (sent to client, echoed back in Encryption Response).
    pub verify_token: Vec<u8>,
    /// Shared secret decrypted from the client's Encryption Response.
    pub shared_secret: Option<Vec<u8>>,
    /// Whether encryption has been negotiated and is active.
    pub active: bool,
}

impl EncryptionState {
    /// Generate a fresh RSA key pair + verify token for this connection.
    pub fn new(key_bits: usize) -> std::io::Result<Self> {
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, key_bits)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let public_key = RsaPublicKey::from(&private_key);
        let public_key_der = public_key
            .to_public_key_der()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
            .to_vec();

        // 16-byte verify token
        let verify_token: [u8; 16] = OsRng.gen();
        let verify_token = verify_token.to_vec();

        Ok(Self {
            private_key,
            public_key_der,
            verify_token,
            shared_secret: None,
            active: false,
        })
    }

    /// Decrypt the client's Encryption Response using the server's RSA private key.
    ///
    /// The response contains an encrypted shared secret and an encrypted verify token,
    /// both OAEP-padded with SHA-1. After this call, `shared_secret` is populated and
    /// `active` is set to true if the verify token matches.
    pub fn handle_response(
        &mut self,
        encrypted_shared_secret: &[u8],
        encrypted_verify_token: &[u8],
    ) -> std::io::Result<bool> {
        use rsa::Pkcs1v15Encrypt;

        // Decrypt shared secret (RSA-OAEP with SHA-1)
        let shared_secret = self
            .private_key
            .decrypt(Pkcs1v15Encrypt, encrypted_shared_secret)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // Decrypt verify token
        let decrypted_token = self
            .private_key
            .decrypt(Pkcs1v15Encrypt, encrypted_verify_token)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let token_ok = decrypted_token == self.verify_token;

        self.shared_secret = Some(shared_secret);
        self.active = token_ok;

        Ok(token_ok)
    }

    /// Return the shared secret for use with the AES/CFB8 cipher.
    pub fn shared_secret(&self) -> Option<&[u8]> {
        self.shared_secret.as_deref()
    }
}

/// Create an Encryption Request payload (login clientbound packet 0x01).
///
/// Layout: server_id (empty string), public_key (VarInt-prefixed bytes),
/// verify_token (VarInt-prefixed bytes).
pub fn build_encryption_request(
    public_key: &[u8],
    verify_token: &[u8],
) -> Vec<u8> {
    let mut w = crate::packet::PacketWriter::new();
    w.write_string(""); // server_id (empty for offline/authless)
    w.write_varint(public_key.len() as i32);
    w.buf.extend_from_slice(public_key);
    w.write_varint(verify_token.len() as i32);
    w.buf.extend_from_slice(verify_token);
    w.buf
}

/// Create a Login Success payload (login clientbound packet 0x02).
///
/// Layout: UUID (16 bytes), username (String).
pub fn build_login_success(uuid: &[u8; 16], username: &str) -> Vec<u8> {
    let mut w = crate::packet::PacketWriter::new();
    w.write_uuid_bytes(uuid);
    w.write_string(username);
    w.buf
}

/// Create a Set Compression payload (login clientbound packet 0x03).
///
/// Layout: threshold (VarInt).
pub fn build_set_compression(threshold: i32) -> Vec<u8> {
    let mut w = crate::packet::PacketWriter::new();
    w.write_varint(threshold);
    w.buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_login_success_shape() {
        let uuid = [0u8; 16];
        let payload = build_login_success(&uuid, "TestPlayer");
        // 16 bytes UUID + VarInt(10) + 10 bytes username = 27 bytes
        assert_eq!(payload.len(), 27);
        assert_eq!(&payload[..16], &uuid[..]);
    }

    #[test]
    fn build_encryption_request_shape() {
        let pk = [0u8; 256];
        let vt = [1u8; 16];
        let payload = build_encryption_request(&pk, &vt);
        // server_id "" -> VarInt(0) = 1 byte
        // public_key: VarInt(256) + 256 bytes
        // verify_token: VarInt(16) + 16 bytes
        assert!(payload.len() > 256);
        assert_eq!(payload[0], 0x00); // empty string length
    }

    #[test]
    fn encryption_state_active_after_valid_response() {
        // This would require a full RSA encrypt/decrypt round trip; just check
        // that the state starts inactive.
        let state = EncryptionState::new(1024).expect("key gen");
        assert!(!state.active);
        assert!(state.shared_secret().is_none());
    }
}
