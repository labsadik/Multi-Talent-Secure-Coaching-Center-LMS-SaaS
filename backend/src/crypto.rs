use aes_gcm::{
    aead::{Aead, KeyInit, OsRng, Payload, rand_core::RngCore},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use crate::error::{AppError, AppResult};

const NONCE_LEN: usize = 12;

/// Encrypt a tenant secret using AES-256-GCM. Bind ciphertext to company ID as AAD.
pub fn encrypt_tenant_secret(
    plaintext: &[u8],
    company_id: &str,
    key: &[u8; 32],
) -> AppResult<String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| AppError::Internal)?;
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: company_id.as_bytes(),
            },
        )
        .map_err(|_| AppError::Internal)?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(URL_SAFE_NO_PAD.encode(combined))
}

/// Decrypt only when the same company ID and encryption key are supplied.
pub fn decrypt_tenant_secret(
    encoded: &str,
    company_id: &str,
    key: &[u8; 32],
) -> AppResult<Vec<u8>> {
    let combined = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| AppError::Internal)?;
    if combined.len() <= NONCE_LEN {
        return Err(AppError::Internal);
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| AppError::Internal)?;
    cipher
        .decrypt(
            Nonce::from_slice(nonce_bytes),
            Payload {
                msg: ciphertext,
                aad: company_id.as_bytes(),
            },
        )
        .map_err(|_| AppError::Internal)
}

#[cfg(test)]
mod tests {
    use super::{decrypt_tenant_secret, encrypt_tenant_secret};

    #[test]
    fn tenant_secret_round_trip_and_tenant_binding() {
        let key = [42_u8; 32];
        let encrypted = encrypt_tenant_secret(b"provider-secret", "company-a", &key)
            .expect("encryption should succeed");
        let decrypted = decrypt_tenant_secret(&encrypted, "company-a", &key)
            .expect("decryption should succeed");
        assert_eq!(decrypted, b"provider-secret");
        assert!(decrypt_tenant_secret(&encrypted, "company-b", &key).is_err());
    }
}
