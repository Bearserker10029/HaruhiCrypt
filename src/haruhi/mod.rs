use sha2::{Sha256, Digest};
use hmac::Mac;
use rand::RngCore;

pub const BLOCK_SIZE: usize = 8;

pub struct HaruhiCipher {
    permutations: Vec<[u8; BLOCK_SIZE]>,
    seed: [u8; 32],
}

impl HaruhiCipher {
    pub fn new(key: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let seed: [u8; 32] = hasher.finalize().into();

        let superperm = generate_superpermutation(8);
        let permutations = extract_all_permutations(&superperm);

        Self { permutations, seed }
    }

    pub fn encrypt_block(&self, block: &[u8; BLOCK_SIZE], block_index: usize) -> [u8; BLOCK_SIZE] {
        let mut hasher = Sha256::new();
        hasher.update(&self.seed);
        hasher.update(&(block_index as u64).to_le_bytes());
        let hash = hasher.finalize();
        let hash_u64 = u64::from_le_bytes(hash[0..8].try_into().unwrap());
        let offset = (hash_u64 as usize) % self.permutations.len();
        let perm = self.permutations[offset];

        let mut result = [0u8; BLOCK_SIZE];
        for i in 0..BLOCK_SIZE {
            result[perm[i] as usize] = block[i];
        }
        result
    }

    pub fn decrypt_block(&self, block: &[u8; BLOCK_SIZE], block_index: usize) -> [u8; BLOCK_SIZE] {
        let mut hasher = Sha256::new();
        hasher.update(&self.seed);
        hasher.update(&(block_index as u64).to_le_bytes());
        let hash = hasher.finalize();
        let hash_u64 = u64::from_le_bytes(hash[0..8].try_into().unwrap());
        let offset = (hash_u64 as usize) % self.permutations.len();
        let perm = self.permutations[offset];

        let mut result = [0u8; BLOCK_SIZE];
        for i in 0..BLOCK_SIZE {
            result[i] = block[perm[i] as usize];
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

pub fn encrypt_data(key: &str, data: &[u8], original_ext: Option<&str>) -> Vec<u8> {
    let cipher = HaruhiCipher::new(key);
    let mut rng = rand::thread_rng();

    let mut iv = [0u8; 16];
    rng.fill_bytes(&mut iv);

    let ext_bytes = original_ext.unwrap_or("bin").as_bytes();
    let ext_len = ext_bytes.len().min(255) as u8;

    let mut result = Vec::with_capacity(1 + ext_len as usize + 16 + data.len() + 32);
    result.push(ext_len);
    result.extend_from_slice(ext_bytes);
    result.extend_from_slice(&iv);

    let padding_needed = if data.len() % BLOCK_SIZE == 0 {
        0
    } else {
        BLOCK_SIZE - (data.len() % BLOCK_SIZE)
    };

    let mut padded_data = data.to_vec();
    for _ in 0..padding_needed {
        padded_data.push(padding_needed as u8);
    }

    for (block_index, chunk) in padded_data.chunks_exact(BLOCK_SIZE).enumerate() {
        let block: [u8; BLOCK_SIZE] = chunk.try_into().unwrap();
        let encrypted = cipher.encrypt_block(&block, block_index);
        result.extend_from_slice(&encrypted);
    }

    let mut hasher = hmac::Hmac::<sha2::Sha256>::new_from_slice(&cipher.seed).expect("HMAC init failed");
    hasher.update(&result[1 + ext_len as usize + 16..]);
    let mac = hasher.finalize().into_bytes();
    result.extend_from_slice(&mac);

    result
}

pub fn decrypt_data(key: &str, data: &[u8]) -> Result<(Vec<u8>, String), String> {
    if data.len() < 16 + 32 + 2 {
        return Err("Invalid data: too short".to_string());
    }

    let ext_len = data[0] as usize;
    if data.len() < 1 + ext_len + 16 + 32 {
        return Err("Invalid data: too short".to_string());
    }

    let ext_bytes = &data[1..1 + ext_len];
    let original_ext = String::from_utf8(ext_bytes.to_vec()).unwrap_or_else(|_| "bin".to_string());

    let ciphertext_start = 1 + ext_len + 16;
    let ciphertext_end = data.len() - 32;
    let ciphertext = &data[ciphertext_start..ciphertext_end];
    let stored_mac = &data[ciphertext_end..];

    let cipher = HaruhiCipher::new(key);

    let mut hasher = hmac::Hmac::<sha2::Sha256>::new_from_slice(&cipher.seed).expect("HMAC init failed");
    hasher.update(ciphertext);
    let computed_mac = hasher.finalize().into_bytes();

    if computed_mac.as_slice() != stored_mac {
        return Err("Authentication failed: invalid key or corrupted data".to_string());
    }

    let mut result = Vec::new();

    for (block_index, chunk) in ciphertext.chunks_exact(BLOCK_SIZE).enumerate() {
        let block: [u8; BLOCK_SIZE] = chunk.try_into().unwrap();
        let decrypted = cipher.decrypt_block(&block, block_index);
        result.extend_from_slice(&decrypted);
    }

    if let Some(&pad) = result.last() {
        if pad <= BLOCK_SIZE as u8 && pad > 0 {
            let pad_len = pad as usize;
            if result.len() >= pad_len {
                let is_valid_padding = result[result.len() - pad_len..].iter().all(|&b| b == pad);
                if is_valid_padding {
                    result.truncate(result.len() - pad_len);
                }
            }
        }
    }

    Ok((result, original_ext))
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

        println!("First 20 bytes: {:?}", &superperm[..20.min(superperm.len())]);

        let perms = extract_all_permutations(&superperm);
        println!("Extracted {} permutations", perms.len());
        if let Some(p) = perms.first() {
            println!("First permutation: {:?}", p);
        }
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
    fn test_block_cipher_inverse() {
        let key = "0123-4567-89ab-cdef";
        let cipher = HaruhiCipher::new(key);

        for block_index in 0..100 {
            let block: [u8; 8] = rand::random();
            let encrypted = cipher.encrypt_block(&block, block_index);
            let decrypted = cipher.decrypt_block(&encrypted, block_index);
            assert_eq!(block, decrypted, "Block {} failed", block_index);
        }
    }
}