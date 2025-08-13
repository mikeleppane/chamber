use anyhow::{Result, anyhow};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroize;

pub type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug)]
pub struct KeyMaterial(pub [u8; 32]);
impl KeyMaterial {
    /// Generates a new instance of `Self` with random bytes.
    ///
    /// # Attributes
    /// * `#[allow(clippy::expect_used)]` - Suppresses the Clippy lint warning
    ///   for using `.expect()`.
    /// * `#[must_use]` - Indicates that the result of this function must be used
    ///   by the caller, preventing accidental omission.
    ///
    /// # Returns
    /// A new instance of `Self` initialized with 32 cryptographically secure
    /// random bytes.
    ///
    /// # Panics
    /// This function will panic if the system fails to generate random bytes.
    ///
    /// The panic occurs at the `expect` call if `getrandom::fill` does not succeed
    /// in generating the random numbers.
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn random() -> Self {
        let mut k = [0u8; 32];
        getrandom::fill(&mut k).expect("Failed to get random bytes");

        Self(k)
    }
}
impl Drop for KeyMaterial {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KdfParams {
    pub salt: Vec<u8>,
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}
impl KdfParams {
    /// Generates a default secure configuration for the given struct.
    ///
    /// This function initializes a secure default configuration with randomly generated salt
    /// and preset computational parameters. It leverages the `getrandom` crate to fill the salt
    /// with cryptographically secure random bytes. The computational parameters (`m_cost_kib`,
    /// `t_cost`, and `p_cost`) are chosen to provide a balance between security and practical resource
    /// usage:
    ///
    /// - `salt`: A 16-byte cryptographic salt used in hashing or encryption processes.
    /// - `m_cost_kib`: Memory cost in kibibytes, set to 19,456 (~19MB) to mitigate against brute-force attacks.
    /// - `t_cost`: Time cost or iterations, set to 3, determining the number of hashing passes.
    /// - `p_cost`: Parallelism factor, set to 1, determining the number of threads or lanes.
    ///
    /// # Returns
    ///
    /// A new instance of `Self` with a secure default configuration.
    ///
    /// # Panics
    ///
    /// This function will panic if the underlying system fails to generate random bytes
    /// using the `getrandom` crate. The error message will be `"Failed to get random bytes"`.
    ///
    /// # Attributes
    ///
    /// - `#[allow(clippy::expect_used)]`: Allows the `expect` function to be used without Clippy lint warnings.
    /// - `#[must_use]`: Indicates that the result of this function must be used; otherwise,
    ///   the compiler will issue a warning.
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn default_secure() -> Self {
        let mut salt = vec![0u8; 16];
        getrandom::fill(&mut salt).expect("Failed to get random bytes");
        Self {
            salt,
            m_cost_kib: 19456,
            t_cost: 3,
            p_cost: 1,
        } // ~19MB memory
    }
}

/// Derives a key from the given master password and key derivation function (KDF) parameters.
///
/// # Arguments
///
/// * `master` - A string slice representing the master password or secret from which the key
///   will be derived.
/// * `kdf` - A reference to `KdfParams` struct containing the parameters for the Argon2 key
///   derivation function, such as memory cost, time cost, parallelism, and salt.
///
/// # Returns
///
/// Returns a `Result` containing:
/// * `KeyMaterial` - On success, the derived key material.
/// * `anyhow::Error` - On failure, an error that indicates what went wrong.
///
/// # KDF Parameters
///
/// The function uses the Argon2 key derivation algorithm with the following:
/// * `Algorithm::Argon2id` - A hybrid of Argon2i and Argon2d.
/// * `Version::V0x13` - The Argon2 version 0x13 (current recommended version).
/// * `Params` - Configurable parameters (memory cost in KiB, time cost, parallelism, and output size, which is fixed to 32 bytes here).
///
/// # Errors
///
/// This function will return an error in the following scenarios:
/// * If the KDF parameters cannot be created (`Params::new` fails).
/// * If the hash computation with Argon2 fails (`hash_password_into` fails).
pub fn derive_key(master: &str, kdf: &KdfParams) -> Result<KeyMaterial> {
    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(kdf.m_cost_kib, kdf.t_cost, kdf.p_cost, Some(32)).map_err(|e| anyhow!("{e}"))?,
    );
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(master.as_bytes(), &kdf.salt, &mut out)
        .map_err(|e| anyhow!("{e}"))?;
    Ok(KeyMaterial(out))
}

// We implement a simple key wrap: derive an AEAD from the master-derived key,
// generate random nonce and encrypt the vault key; store nonce+ciphertext.
// Add a verifier: HMAC(master_derived, "chamber-verifier")
#[derive(Serialize, Deserialize)]
pub struct WrappedVaultKey {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Encrypts a vault key using a master derived key and returns the wrapped vault key along with a verification tag.
///
/// # Parameters
/// - `master_derived`: A reference to the master derived `KeyMaterial` used to encrypt the vault key and generate a MAC.
/// - `vault_key`: A reference to the `KeyMaterial` representing the vault key to be encrypted.
///
/// # Returns
/// - `Ok((WrappedVaultKey, Vec<u8>))`: On success, returns a tuple containing:
///   - `WrappedVaultKey`: A struct containing the encrypted vault key (ciphertext) and the nonce used for encryption.
///   - `Vec<u8>`: A verification tag generated using HMAC-SHA256 to ensure integrity.
/// - `Err(anyhow::Error)`: If encryption or random nonce generation fails.
///
/// # Errors
/// - Returns an error if:
///   - The random nonce generation (`getrandom::fill`) fails.
///   - The AEAD encryption with `XChaCha20Poly1305` fails.
///   - The HMAC-SHA256 initialization or computation fails.
///
/// # Implementation Details
/// - A nonce of 24 bytes is generated using the `getrandom` library to ensure randomization for the AEAD encryption.
/// - `XChaCha20Poly1305` is used for authenticated encryption, which requires:
///   - A 256-bit encryption key derived from `master_derived`.
///   - A nonce of 24 bytes.
/// - The vault key is encrypted using the AEAD scheme, producing a ciphertext.
/// - Alongside encryption, an HMAC-SHA256 tag is computed for message authentication using the `master_derived` key and the predefined string `chamber-verifier`.
///
/// # Dependencies
/// - `XChaCha20Poly1305` for the encryption process.
/// - `HmacSha256` from the `Mac` trait for generating the authentication tag.
/// - `getrandom` for generating a secure random nonce.
pub fn wrap_vault_key(master_derived: &KeyMaterial, vault_key: &KeyMaterial) -> Result<(WrappedVaultKey, Vec<u8>)> {
    let aead = XChaCha20Poly1305::new((&master_derived.0).into());
    let mut nonce = [0u8; 24];
    getrandom::fill(&mut nonce)?;
    let ct = aead
        .encrypt(XNonce::from_slice(&nonce), vault_key.0.as_ref())
        .map_err(|_| anyhow!("AEAD encrypt failed"))?;
    let wrapped = WrappedVaultKey {
        nonce: nonce.to_vec(),
        ciphertext: ct,
    };

    let mut mac = <HmacSha256 as Mac>::new_from_slice(&master_derived.0)?;
    mac.update(b"chamber-verifier");
    let tag = mac.finalize().into_bytes().to_vec();

    Ok((wrapped, tag))
}

/// Unwraps a wrapped vault key using a master derived key, optionally verifying the unwrapping process
/// with an additional verifier.
///
/// # Parameters
/// - `master_derived`: A reference to the master derived key (`KeyMaterial`) that is used to unwrap
///   the encrypted vault key.
/// - `wrapped`: A reference to a `WrappedVaultKey` structure containing the nonce and ciphertext of
///   the encrypted vault key.
/// - `verifier`: An optional byte slice containing a verifier. If provided, this verifies that the
///   unwrapping is being performed with the expected master derived key.
///
/// # Returns
/// - `Ok(KeyMaterial)`: Returns the unwrapped vault key as a `KeyMaterial` object if successful.
/// - `Err(anyhow::Error)`: Returns an error if the verification or decryption fails.
///
/// # Errors
/// - Returns an error if the verifier is provided and does not match the expected value. The mismatch
///   is reported as a "Verifier mismatch".
/// - Returns an error if the AEAD (Authenticated Encryption with Associated Data) decryption fails,
///   reported as "AEAD decrypt failed".
///
/// # Verification Process
/// If a verifier is provided:
/// 1. Uses HMAC-SHA256, initialized with the `master_derived` key, to compute a message authentication
///    code (MAC) over a constant string, `"chamber-verifier"`.
/// 2. Compares the computed MAC with the provided verifier. If they do not match, an error is returned.
///
/// # Decryption Process
/// 1. Initializes an AEAD cipher (`XChaCha20Poly1305`) using the `master_derived` key.
/// 2. Constructs a nonce using the `wrapped` data.
/// 3. Uses the AEAD cipher to decrypt the provided ciphertext into plaintext.
/// 4. Extracts 32 bytes from the plaintext to construct the unwrapped key.
pub fn unwrap_vault_key(
    master_derived: &KeyMaterial,
    wrapped: &WrappedVaultKey,
    verifier: Option<&[u8]>,
) -> Result<KeyMaterial> {
    if let Some(v) = verifier {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&master_derived.0)?;
        mac.update(b"chamber-verifier");
        mac.verify_slice(v).map_err(|_| anyhow!("Verifier mismatch"))?;
    }
    let aead = XChaCha20Poly1305::new((&master_derived.0).into());
    let nonce = XNonce::from_slice(&wrapped.nonce);
    let pt = aead
        .decrypt(nonce, wrapped.ciphertext.as_ref())
        .map_err(|_| anyhow!("AEAD decrypt failed"))?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&pt);
    Ok(KeyMaterial(key))
}

/// Encrypts plaintext data using the AEAD (Authenticated Encryption with Associated Data) construction
/// provided by the `XChaCha20Poly1305` algorithm, ensuring confidentiality, integrity, and authenticity.
///
/// # Arguments
///
/// - `vault_key`: A reference to a `KeyMaterial` that serves as the encryption key.
/// - `plaintext`: A slice of bytes representing the data to encrypt.
/// - `ad`: A slice of bytes representing the associated data (AD) to include in the encryption.
///   The AD is authenticated but not encrypted, allowing external validation during decryption.
///
/// # Returns
///
/// Returns a `Result` containing:
/// - On success: A tuple `(nonce, ciphertext)`:
///   - `nonce`: A 24-byte vector representing the randomly generated nonce used during encryption.
///   - `ciphertext`: A vector of bytes representing the encrypted data.
/// - On failure: An error with a descriptive message.
///
/// # Errors
///
/// - Returns an error if random nonce generation fails using the `getrandom` crate.
/// - Returns an error if the encryption process fails (e.g., due to an internal library failure).
///
/// # Security Considerations
///
/// - Ensure that the `vault_key` is securely managed and never reused across key contexts.
/// - Nonces must be unique for each encryption operation. This function generates random nonces
///   automatically to prevent reuse.
/// - The associated data (`ad`) must be consistent during encryption and decryption for the integrity
///   check to succeed.
///
/// # Dependencies
///
/// This function relies on the `chacha20poly1305` crate for encryption and the `getrandom` crate
/// for secure random number generation.
pub fn aead_encrypt(vault_key: &KeyMaterial, plaintext: &[u8], ad: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let aead = XChaCha20Poly1305::new((&vault_key.0).into());
    let mut nonce = [0u8; 24];
    getrandom::fill(&mut nonce)?;
    let ct = aead
        .encrypt(
            XNonce::from_slice(&nonce),
            chacha20poly1305::aead::Payload {
                msg: plaintext,
                aad: ad,
            },
        )
        .map_err(|_| anyhow!("encrypt failed"))?;
    Ok((nonce.to_vec(), ct))
}

/// Decrypts a ciphertext using the AEAD (Authenticated Encryption with Associated Data)
/// encryption scheme with the `XChaCha20Poly1305` algorithm.
///
/// # Parameters
/// - `vault_key`: A reference to the key material (`KeyMaterial`) used for decryption. The key is used to initialize
///   the `XChaCha20Poly1305` encryption algorithm.
/// - `nonce`: A byte slice representing the unique nonce required for decryption. The nonce must match the one
///   used during encryption.
/// - `ciphertext`: A byte slice representing the encrypted data to be decrypted.
/// - `ad`: A byte slice containing the associated data (AD) that was provided during encryption.
///   The AD is authenticated but not encrypted, and must match exactly during decryption.
///
/// # Returns
/// - `Result<Vec<u8>>`: On success, returns the decrypted plaintext as a vector of bytes. On failure, returns an error wrapped
///   in a `Result`. An error could occur if the decryption fails due to a mismatch in the key, nonce, ciphertext, or associated data.
///
/// # Errors
/// - Returns an error if the decryption fails, such as in cases of an invalid key, mismatched nonce or associated data, or corrupted ciphertext.
///
/// # Notes
/// - The `XChaCha20Poly1305` cipher ensures both confidentiality and authenticity of the ciphertext and associated data.
/// - The caller must ensure the `nonce`, `ciphertext`, and `ad` provided match exactly with those used during encryption.
/// - Incorrect inputs will result in a decryption failure.
pub fn aead_decrypt(vault_key: &KeyMaterial, nonce: &[u8], ciphertext: &[u8], ad: &[u8]) -> Result<Vec<u8>> {
    let aead = XChaCha20Poly1305::new((&vault_key.0).into());
    let pt = aead
        .decrypt(
            XNonce::from_slice(nonce),
            chacha20poly1305::aead::Payload {
                msg: ciphertext,
                aad: ad,
            },
        )
        .map_err(|_| anyhow!("decrypt failed"))?;
    Ok(pt)
}

// Rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use hex::encode as hex_encode;

    // Use a reduced-cost KDF for fast tests
    fn small_kdf(salt: &[u8]) -> KdfParams {
        let mut s = salt.to_vec();
        if s.len() < 8 {
            s.resize(8, 0); // pad with zeros to meet Argon2 salt requirement
        }
        KdfParams {
            salt: s,
            m_cost_kib: 8, // very small memory for test speed
            t_cost: 1,
            p_cost: 1,
        }
    }

    #[test]
    fn test_keymaterial_random_and_length() {
        let k1 = KeyMaterial::random();
        let k2 = KeyMaterial::random();
        assert_eq!(k1.0.len(), 32);
        assert_eq!(k2.0.len(), 32);
        // Very likely different
        assert_ne!(hex_encode(k1.0), hex_encode(k2.0));
    }

    #[test]
    fn test_derive_key_deterministic_and_salt_sensitive() {
        let kdf1 = small_kdf(b"salt-1");
        let kdf2 = small_kdf(b"salt-2");
        let master = "correct horse battery staple";

        let a = derive_key(master, &kdf1).unwrap();
        let b = derive_key(master, &kdf1).unwrap();
        let c = derive_key(master, &kdf2).unwrap();

        // Deterministic with same params
        assert_eq!(hex_encode(a.0), hex_encode(b.0));
        // Different salt -> different key
        assert_ne!(hex_encode(a.0), hex_encode(c.0));
    }

    #[test]
    fn test_aead_encrypt_decrypt_roundtrip_with_ad() {
        let key = KeyMaterial::random();
        let msg = b"secret message";
        let ad = b"associated-data";

        let (nonce, ct) = aead_encrypt(&key, msg, ad).unwrap();
        let pt = aead_decrypt(&key, &nonce, &ct, ad).unwrap();
        assert_eq!(pt, msg);
    }

    #[test]
    fn test_aead_decrypt_wrong_ad_fails() {
        let key = KeyMaterial::random();
        let msg = b"message";
        let ad_ok = b"ad-ok";
        let ad_bad = b"ad-bad";

        let (nonce, ct) = aead_encrypt(&key, msg, ad_ok).unwrap();
        let err = aead_decrypt(&key, &nonce, &ct, ad_bad).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("decrypt"));
    }

    #[test]
    fn test_aead_decrypt_wrong_key_fails() {
        let key1 = KeyMaterial::random();
        let key2 = KeyMaterial::random();
        let (nonce, ct) = aead_encrypt(&key1, b"data", b"ad").unwrap();

        let err = aead_decrypt(&key2, &nonce, &ct, b"ad").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("decrypt"));
    }

    #[test]
    fn test_aead_tamper_detection() {
        let key = KeyMaterial::random();
        let (nonce, mut ct) = aead_encrypt(&key, b"payload", b"ad").unwrap();

        // Flip one bit in ciphertext
        if let Some(byte) = ct.get_mut(0) {
            *byte ^= 0x01;
        }
        let err = aead_decrypt(&key, &nonce, &ct, b"ad").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("decrypt"));
    }

    #[test]
    fn test_wrap_unwrap_vault_key_roundtrip_and_verifier() {
        let master = "test-master";
        let kdf = small_kdf(b"wrapsalt");
        let master_derived = derive_key(master, &kdf).unwrap();

        let vk = KeyMaterial::random();
        let (wrapped, verifier) = wrap_vault_key(&master_derived, &vk).unwrap();

        // Unwrap with verifier ok
        let unwrapped = unwrap_vault_key(&master_derived, &wrapped, Some(&verifier)).unwrap();
        assert_eq!(hex_encode(vk.0), hex_encode(unwrapped.0));

        // Unwrap without verifier also ok
        let unwrapped2 = unwrap_vault_key(&master_derived, &wrapped, None).unwrap();
        assert_eq!(hex_encode(vk.0), hex_encode(unwrapped2.0));
    }

    #[test]
    fn test_unwrap_verifier_mismatch_fails() {
        let master_ok = "master-ok";
        let master_bad = "master-bad";
        let kdf = small_kdf(b"v-salt");

        let md_ok = derive_key(master_ok, &kdf).unwrap();
        let md_bad = derive_key(master_bad, &kdf).unwrap();

        let vk = KeyMaterial::random();
        let (wrapped, verifier) = wrap_vault_key(&md_ok, &vk).unwrap();

        // Using wrong master-derived key with correct verifier should fail verification
        let err = unwrap_vault_key(&md_bad, &wrapped, Some(&verifier)).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("verifier"));
    }

    #[test]
    fn test_unwrap_with_tampered_ciphertext_fails() {
        let master = "master";
        let kdf = small_kdf(b"salt-x");
        let md = derive_key(master, &kdf).unwrap();

        let vk = KeyMaterial::random();
        let (mut wrapped, verifier) = wrap_vault_key(&md, &vk).unwrap();

        // Tamper with ciphertext
        if let Some(byte) = wrapped.ciphertext.get_mut(0) {
            *byte ^= 0x80;
        }

        let err = unwrap_vault_key(&md, &wrapped, Some(&verifier)).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("aead"));
    }

    #[test]
    fn test_kdfparams_default_secure_has_expected_shape() {
        let kdf = KdfParams::default_secure();
        // Basic invariants; we don't assert exact costs, but ensure they are non-trivial
        assert_eq!(kdf.salt.len(), 16);
        assert!(kdf.m_cost_kib >= 1024);
        assert!(kdf.t_cost >= 1);
        assert!(kdf.p_cost >= 1);
        // Derive works
        let km = derive_key("pw", &kdf).unwrap();
        assert_eq!(km.0.len(), 32);
    }

    #[test]
    fn test_hmac_verifier_stable_for_same_key() {
        let master = "verifier-master";
        let kdf = small_kdf(b"vsalt");
        let md = derive_key(master, &kdf).unwrap();
        let vk = KeyMaterial::random();

        let (_, tag1) = wrap_vault_key(&md, &vk).unwrap();
        let (_, tag2) = wrap_vault_key(&md, &vk).unwrap();

        // HMAC is deterministic for the same key and data
        assert_eq!(hex_encode(tag1), hex_encode(tag2));
    }

    #[test]
    fn test_hmac_verifier_differs_for_different_master_keys() {
        let kdf = small_kdf(b"vsalt");
        let md1 = derive_key("m1", &kdf).unwrap();
        let md2 = derive_key("m2", &kdf).unwrap();
        let vk = KeyMaterial::random();

        let (_, tag1) = wrap_vault_key(&md1, &vk).unwrap();
        let (_, tag2) = wrap_vault_key(&md2, &vk).unwrap();

        assert_ne!(hex_encode(tag1), hex_encode(tag2));
    }

    // Helper: ensure hex codec available in tests
    mod hex {
        #[allow(clippy::format_collect)]
        pub fn encode<T: AsRef<[u8]>>(data: T) -> String {
            data.as_ref().iter().map(|b| format!("{b:02x}")).collect()
        }
    }
}
