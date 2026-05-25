use sha2::{Sha256, Digest};
use hmac::Mac;
use rand::RngCore;
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use std::sync::OnceLock;

pub const BLOCK_SIZE: usize = 8;
pub const NONCE_SIZE: usize = 16;
pub const IV_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;
pub const MAC_SIZE: usize = 32;

const ARGON2_SALT: &str = "HaruhiCrypt_v2____";

static PERMUTATIONS: OnceLock<Vec<[u8; BLOCK_SIZE]>> = OnceLock::new();

pub struct HaruhiCipher {
    permutations: Vec<[u8; BLOCK_SIZE]>,
    seed: [u8; KEY_SIZE],
    nonce: [u8; NONCE_SIZE],
}

impl HaruhiCipher {
    pub fn new(key: &str, nonce: [u8; NONCE_SIZE]) -> Self {
        let salt = SaltString::encode_b64(ARGON2_SALT.as_bytes()).expect("Salt encoding failed");
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(key.as_bytes(), &salt).expect("Argon2 failed");
        let hash_output = hash.hash.expect("Argon2 no hash");
        let seed: [u8; KEY_SIZE] = hash_output.as_bytes()[0..KEY_SIZE].try_into().unwrap();

        let permutations = PERMUTATIONS.get_or_init(|| {
            let superperm = generate_superpermutation(8);
            extract_all_permutations(&superperm)
        }).clone();

        Self { permutations, seed, nonce }
    }

    fn get_keystream_block(&self, block_index: u64) -> [u8; BLOCK_SIZE] {
        let mut hasher = Sha256::new();
        hasher.update(&self.seed);
        hasher.update(&self.nonce);
        hasher.update(&block_index.to_le_bytes());
        let hash = hasher.finalize();

        let hash_u64 = u64::from_le_bytes(hash[0..8].try_into().unwrap());
        let offset = (hash_u64 as usize) % self.permutations.len();
        let perm = self.permutations[offset];

        let mut result = [0u8; BLOCK_SIZE];
        for i in 0..BLOCK_SIZE {
            result[perm[i] as usize] = hash[i];
        }
        result
    }

    pub fn encrypt_block(&self, block: &[u8; BLOCK_SIZE], block_index: usize) -> [u8; BLOCK_SIZE] {
        let keystream = self.get_keystream_block(block_index as u64);
        let mut result = [0u8; BLOCK_SIZE];
        for i in 0..BLOCK_SIZE {
            result[i] = block[i] ^ keystream[i];
        }
        result
    }

    pub fn decrypt_block(&self, block: &[u8; BLOCK_SIZE], block_index: usize) -> [u8; BLOCK_SIZE] {
        let keystream = self.get_keystream_block(block_index as u64);
        let mut result = [0u8; BLOCK_SIZE];
        for i in 0..BLOCK_SIZE {
            result[i] = block[i] ^ keystream[i];
        }
        result
    }
}

fn generate_superpermutation(n: usize) -> Vec<u8> {
    if n == 1 {
        return vec![1];
    }

    let mut superperm = Vec::new();
    let mut permutations: Vec<Vec<u8>> = Vec::new();
    generate_permutations_recursive(n as u8, &mut Vec::new(), &mut permutations);

    for (i, perm) in permutations.iter().enumerate() {
        let perm_bytes: Vec<u8> = perm.iter().map(|&x| x + 1).collect();
        if i == 0 {
            superperm.extend_from_slice(&perm_bytes);
        } else {
            let suffix = find_overlap(&superperm, &perm_bytes);
            superperm.extend_from_slice(&perm_bytes[suffix..]);
        }
    }

    superperm
}

fn generate_permutations_recursive(n: u8, current: &mut Vec<u8>, results: &mut Vec<Vec<u8>>) {
    if current.len() == n as usize {
        results.push(current.clone());
        return;
    }

    for i in 0..=current.len() {
        let mut new_current = current.clone();
        new_current.insert(i, current.len() as u8);
        generate_permutations_recursive(n, &mut new_current, results);
    }
}

fn find_overlap(superperm: &[u8], next_perm: &[u8]) -> usize {
    let max_overlap = superperm.len().min(next_perm.len());

    for overlap in (1..=max_overlap).rev() {
        let suffix = &superperm[superperm.len() - overlap..];
        let prefix = &next_perm[..overlap];

        if suffix == prefix {
            return overlap;
        }
    }
    0
}

fn extract_all_permutations(superperm: &[u8]) -> Vec<[u8; BLOCK_SIZE]> {
    use std::collections::HashSet;

    let mut permutations = Vec::new();
    let mut seen = HashSet::new();
    let mut current = Vec::with_capacity(BLOCK_SIZE);

    for &byte in superperm {
        current.push(byte);
        if current.len() == BLOCK_SIZE {
            let is_valid = {
                let mut check = [false; BLOCK_SIZE];
                let mut valid = true;
                for &v in &current {
                    if v < 1 || v > 8 || check[(v - 1) as usize] {
                        valid = false;
                        break;
                    }
                    check[(v - 1) as usize] = true;
                }
                valid
            };

            if is_valid {
                let key: u64 = current.iter().fold(0u64, |acc, &x| acc * 10 + x as u64);
                if !seen.contains(&key) {
                    seen.insert(key);
                    let mut arr = [0u8; BLOCK_SIZE];
                    for (i, &v) in current.iter().enumerate() {
                        arr[i] = v - 1;
                    }
                    permutations.push(arr);
                }
            }
            current.remove(0);
        }
    }

    while permutations.len() < 40320 {
        let mut arr: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
        let idx = permutations.len();
        for i in 0..BLOCK_SIZE {
            arr[i] = ((idx >> (i * 3)) % 8) as u8;
        }
        let is_valid = {
            let mut check = [false; BLOCK_SIZE];
            let mut valid = true;
            for &v in &arr {
                if check[v as usize] {
                    valid = false;
                    break;
                }
                check[v as usize] = true;
            }
            valid
        };
        if is_valid {
            let key: u64 = arr.iter().fold(0u64, |acc, &x| acc * 10 + x as u64);
            if !seen.contains(&key) {
                seen.insert(key);
                permutations.push(arr);
            }
        } else {
            break;
        }
    }

    permutations
}

#[derive(Debug)]
pub struct EncryptedFile {
    pub nonce: [u8; NONCE_SIZE],
    pub iv: [u8; IV_SIZE],
    pub ext_len: u8,
    pub ext: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub mac: [u8; MAC_SIZE],
}

impl EncryptedFile {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.nonce);
        result.push(self.ext_len);
        result.extend_from_slice(&self.ext);
        result.extend_from_slice(&self.iv);
        result.extend_from_slice(&self.ciphertext);
        result.extend_from_slice(&self.mac);
        result
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < NONCE_SIZE + 1 + 16 + 32 {
            return Err("Data too short".to_string());
        }

        let nonce = data[0..NONCE_SIZE].try_into().unwrap();
        let ext_len = data[NONCE_SIZE] as usize;
        let ext_start = NONCE_SIZE + 1;
        let ext_end = ext_start + ext_len;
        let iv_start = ext_end;
        let iv_end = iv_start + IV_SIZE;
        let ciphertext_end = data.len() - MAC_SIZE;

        if data.len() < iv_end {
            return Err("Data too short for IV".to_string());
        }

        let ext = data[ext_start..ext_end].to_vec();
        let iv: [u8; IV_SIZE] = data[iv_start..iv_end].try_into().unwrap();
        let ciphertext = data[iv_end..ciphertext_end].to_vec();
        let mac: [u8; MAC_SIZE] = data[ciphertext_end..].try_into().unwrap();

        Ok(Self { nonce, iv, ext_len: ext_len as u8, ext, ciphertext, mac })
    }
}

pub fn encrypt_data(key: &str, data: &[u8], original_ext: Option<&str>) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut nonce = [0u8; NONCE_SIZE];
    rng.fill_bytes(&mut nonce);

    let cipher = HaruhiCipher::new(key, nonce);

    let ext_bytes = original_ext.unwrap_or("bin").as_bytes();
    let ext_len = ext_bytes.len().min(255) as u8;

    let padding_needed = if data.len() % BLOCK_SIZE == 0 {
        BLOCK_SIZE
    } else {
        BLOCK_SIZE - (data.len() % BLOCK_SIZE)
    };

    let mut padded_data = data.to_vec();
    for _ in 0..padding_needed {
        padded_data.push(padding_needed as u8);
    }

    let mut ciphertext = Vec::new();
    for (block_index, chunk) in padded_data.chunks_exact(BLOCK_SIZE).enumerate() {
        let block: [u8; BLOCK_SIZE] = chunk.try_into().unwrap();
        let encrypted = cipher.encrypt_block(&block, block_index);
        ciphertext.extend_from_slice(&encrypted);
    }

    let mut file = EncryptedFile {
        nonce,
        iv: [0u8; IV_SIZE],
        ext_len,
        ext: ext_bytes.to_vec(),
        ciphertext,
        mac: [0u8; MAC_SIZE],
    };

    let mut hasher = hmac::Hmac::<sha2::Sha256>::new_from_slice(&cipher.seed).expect("HMAC init failed");
    hasher.update(&nonce);
    hasher.update(&[ext_len]);
    hasher.update(&file.ext);
    hasher.update(&file.iv);
    hasher.update(&file.ciphertext);
    let mac: [u8; MAC_SIZE] = hasher.finalize().into_bytes().into();

    file.mac = mac;

    file.to_bytes()
}

pub fn decrypt_data(key: &str, data: &[u8]) -> Result<(Vec<u8>, String), String> {
    let file = EncryptedFile::from_bytes(data)?;

    let cipher = HaruhiCipher::new(key, file.nonce);

    let mut mac_data = Vec::new();
    mac_data.extend_from_slice(&file.nonce);
    mac_data.push(file.ext_len);
    mac_data.extend_from_slice(&file.ext);
    mac_data.extend_from_slice(&file.iv);
    mac_data.extend_from_slice(&file.ciphertext);

    let mut hasher = hmac::Hmac::<sha2::Sha256>::new_from_slice(&cipher.seed).expect("HMAC init failed");
    hasher.update(&mac_data);
    let computed_mac = hasher.finalize().into_bytes();

    if computed_mac.as_slice() != file.mac.as_slice() {
        return Err("Authentication failed: invalid key or corrupted data".to_string());
    }

    let mut result = Vec::new();
    for (block_index, chunk) in file.ciphertext.chunks_exact(BLOCK_SIZE).enumerate() {
        let block: [u8; BLOCK_SIZE] = chunk.try_into().unwrap();
        let decrypted = cipher.decrypt_block(&block, block_index);
        result.extend_from_slice(&decrypted);
    }

    let pad = result[result.len() - 1] as usize;
    if pad == 0 || pad > BLOCK_SIZE {
        return Err("Invalid padding".to_string());
    }

    if result.len() < pad {
        return Err("Invalid padding length".to_string());
    }

    let is_valid_padding = result[result.len() - pad..].iter().all(|&b| b == pad as u8);
    if !is_valid_padding {
        return Err("Invalid padding bytes".to_string());
    }

    result.truncate(result.len() - pad);
    let ext = String::from_utf8(file.ext).unwrap_or_else(|_| "bin".to_string());

    Ok((result, ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_superpermutation_n4() {
        let superperm = generate_superpermutation(4);
        println!("Superpermutation n=4 length: {}", superperm.len());
        assert!(superperm.len() >= 33, "Minimal length for n=4 is 33");
    }

    #[test]
    fn test_superpermutation_n8() {
        let superperm = generate_superpermutation(8);
        println!("Superpermutation n=8 length: {}", superperm.len());

        let perms = extract_all_permutations(&superperm);
        println!("Extracted {} permutations", perms.len());
        assert_eq!(perms.len(), 40320);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let key = "test_key_123";
        let data = b"Hello, HaruhiCrypt! This is a test message.";

        let encrypted = encrypt_data(key, data, Some("txt"));
        let (decrypted, ext) = decrypt_data(key, &encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted, "Decrypted data doesn't match original");
        assert_eq!(ext, "txt");
    }

    #[test]
    fn test_different_keys() {
        let data = b"Test message";
        let key1 = "key1";
        let key2 = "key2";

        let encrypted1 = encrypt_data(key1, data, None);
        let encrypted2 = encrypt_data(key2, data, None);

        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_different_nonces() {
        let data = b"Test message";
        let key = "same_key";

        let encrypted1 = encrypt_data(key, data, None);
        let encrypted2 = encrypt_data(key, data, None);

        assert_ne!(encrypted1, encrypted2, "Different nonces should produce different ciphertext");
    }

    #[test]
    fn test_block_cipher_inverse() {
        let key = "0123-4567-89ab-cdef";
        let nonce = [0u8; NONCE_SIZE];
        let cipher = HaruhiCipher::new(key, nonce);

        for block_index in 0..100 {
            let block: [u8; 8] = rand::random();
            let encrypted = cipher.encrypt_block(&block, block_index);
            let decrypted = cipher.decrypt_block(&encrypted, block_index);
            assert_eq!(block, decrypted, "Block {} failed", block_index);
        }
    }

    #[test]
    fn test_corrupted_mac() {
        let key = "test_key";
        let data = b"Important data";

        let mut encrypted = encrypt_data(key, data, Some("txt"));
        let last_idx = encrypted.len() - 5;
        encrypted[last_idx] ^= 0xFF;

        let result = decrypt_data(key, &encrypted);
        assert!(result.is_err(), "Should reject corrupted data");
    }

    #[test]
    fn test_wrong_key() {
        let data = b"Secret message";
        let encrypted = encrypt_data("correct_key", data, None);

        let result = decrypt_data("wrong_key", &encrypted);
        assert!(result.is_err(), "Should reject wrong key");
    }
}