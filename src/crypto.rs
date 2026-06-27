use anyhow::Result;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub struct EncryptedBlob {
    pub ciphertext: Vec<u8>,
    pub payload_nonce: [u8; 12],
    pub wrapped_dek: Vec<u8>,
    pub wrapped_dek_nonce: [u8; 12],
}

pub fn encrypt_payload(master_key: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedBlob> {
    let mut dek = [0u8; 32];
    OsRng.fill_bytes(&mut dek);

    let payload_cipher = ChaCha20Poly1305::new(&dek.into());
    let payload_nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = payload_cipher
        .encrypt(&payload_nonce, plaintext)
        .map_err(|_| anyhow::anyhow!("failed to encrypt payload"))?;

    let master_cipher = ChaCha20Poly1305::new(master_key.into());
    let wrapped_dek_nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let wrapped_dek = master_cipher
        .encrypt(&wrapped_dek_nonce, dek.as_slice())
        .map_err(|_| anyhow::anyhow!("failed to wrap data encryption key"))?;

    Ok(EncryptedBlob {
        ciphertext,
        payload_nonce: nonce_to_array(payload_nonce),
        wrapped_dek,
        wrapped_dek_nonce: nonce_to_array(wrapped_dek_nonce),
    })
}

pub fn decrypt_payload(
    master_key: &[u8; 32],
    ciphertext: &[u8],
    payload_nonce: &[u8; 12],
    wrapped_dek: &[u8],
    wrapped_dek_nonce: &[u8; 12],
) -> Result<Vec<u8>> {
    let master_cipher = ChaCha20Poly1305::new(master_key.into());
    let dek = master_cipher
        .decrypt(wrapped_dek_nonce.into(), wrapped_dek)
        .map_err(|_| anyhow::anyhow!("failed to unwrap data encryption key"))?;

    if dek.len() != 32 {
        anyhow::bail!("invalid wrapped data encryption key length");
    }

    let mut dek_array = [0u8; 32];
    dek_array.copy_from_slice(&dek);

    let payload_cipher = ChaCha20Poly1305::new(&dek_array.into());
    payload_cipher
        .decrypt(payload_nonce.into(), ciphertext)
        .map_err(|_| anyhow::anyhow!("failed to decrypt payload"))
}

pub fn encrypt_secret(master_key: &[u8; 32], plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 12])> {
    let cipher = ChaCha20Poly1305::new(master_key.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| anyhow::anyhow!("failed to encrypt secret"))?;
    Ok((ciphertext, nonce_to_array(nonce)))
}

pub fn decrypt_secret(
    master_key: &[u8; 32],
    ciphertext: &[u8],
    nonce: &[u8; 12],
) -> Result<Vec<u8>> {
    ChaCha20Poly1305::new(master_key.into())
        .decrypt(nonce.into(), ciphertext)
        .map_err(|_| anyhow::anyhow!("failed to decrypt secret"))
}

pub fn generate_unlock_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

pub fn hash_unlock_token(token: &str) -> [u8; 32] {
    let digest = Sha256::digest(token.as_bytes());
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    hash
}

pub fn tokens_equal(expected: &[u8; 32], candidate: &[u8; 32]) -> bool {
    expected.as_slice().ct_eq(candidate.as_slice()).into()
}

fn nonce_to_array(nonce: Nonce) -> [u8; 12] {
    let mut array = [0u8; 12];
    array.copy_from_slice(nonce.as_slice());
    array
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encryption() {
        let master_key = [7u8; 32];
        let plaintext = b"hello from the past";

        let encrypted = encrypt_payload(&master_key, plaintext).unwrap();
        let decrypted = decrypt_payload(
            &master_key,
            &encrypted.ciphertext,
            &encrypted.payload_nonce,
            &encrypted.wrapped_dek,
            &encrypted.wrapped_dek_nonce,
        )
        .unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
