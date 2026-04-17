use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};

use crate::error::{Error, Result};
use argon2::Argon2;
use rand::{Rng, rng};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

// Constants
const KEY_LEN: usize = 32; // 256 bits for AES-256
const NONCE_LEN: usize = 12; // 96 bits recommended for GCM
const SALT_LEN: usize = 32; // 256 bits salt

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptedPackage {
    #[serde_as(as = "Base64")]
    pub salt: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub ciphertext: Vec<u8>,
}

/// Generate a random salt
pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; SALT_LEN];
    rng().fill_bytes(&mut salt);
    salt
}

/// Generate a nonce
pub fn generate_nonce_bytes() -> Vec<u8> {
    let mut nonce = vec![0u8; NONCE_LEN];
    rng().fill_bytes(&mut nonce);
    nonce
}

/// Derive a key from password using Argon2id
pub fn generate_key_from_password(password: &[u8], salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let argon2 = Argon2::default();
    let mut output_key = [0u8; KEY_LEN];

    argon2
        .hash_password_into(password, salt, &mut output_key)
        .map_err(|e| Error::Encryption(format!("Failed to derive key from password: {}", e)))?;

    Ok(output_key)
}

/// Encrypt data using AES-256-GCM
pub fn encrypt_data(plaintext: &[u8], key: &[u8; KEY_LEN]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    // Generate a random nonce
    let nonce_bytes = generate_nonce_bytes();
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the data
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| Error::Encryption(format!("Encryption failed: {}", e)))?;

    // Prepend the nonce to the ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);

    Ok(result)
}

/// Decrypt data using AES-256-GCM
pub fn decrypt_data(encrypted_data: &[u8], key: &[u8; KEY_LEN]) -> Result<Vec<u8>> {
    if encrypted_data.len() < NONCE_LEN {
        return Err(Error::Decryption("Encrypted data too short".to_string()));
    }

    // Extract nonce and ciphertext
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    // Decrypt the data
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        Error::Decryption(format!(
            "Decryption failed - incorrect password or corrupted data: {}",
            e
        ))
    })?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let original_data = b"Hello, World! This is a test message.";
        let password = b"test_password";
        let salt = generate_salt();

        let key = generate_key_from_password(password, &salt).unwrap();
        let encrypted = encrypt_data(original_data, &key).unwrap();
        let decrypted = decrypt_data(&encrypted, &key).unwrap();

        assert_eq!(original_data.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_password() {
        let original_data = b"Test data";
        let password = b"correct_password";
        let wrong_password = b"wrong_password";
        let salt = generate_salt();

        let key = generate_key_from_password(password, &salt).unwrap();
        let wrong_key = generate_key_from_password(wrong_password, &salt).unwrap();

        let encrypted = encrypt_data(original_data, &key).unwrap();
        let result = decrypt_data(&encrypted, &wrong_key);

        assert!(result.is_err());
    }
}
