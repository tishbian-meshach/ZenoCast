//! Base32 encoding and decoding formatted in human-friendly 5-character blocks.
//!
//! Example: `A7BF2-9XKM3-88QRT-11VWZ-55AA9`

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Encodes binary bytes into a Base32 string with hyphens every 5 characters.
pub fn encode_grouped(data: &[u8]) -> String {
    let mut bits = 0u64;
    let mut bit_count = 0;
    let mut chars = Vec::new();

    for &byte in data {
        bits = (bits << 8) | (byte as u64);
        bit_count += 8;

        while bit_count >= 5 {
            bit_count -= 5;
            let index = ((bits >> bit_count) & 0x1F) as usize;
            chars.push(ALPHABET[index] as char);
        }
    }

    if bit_count > 0 {
        let index = ((bits << (5 - bit_count)) & 0x1F) as usize;
        chars.push(ALPHABET[index] as char);
    }

    // Group in 5-character blocks separated by hyphens
    let mut formatted = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && i % 5 == 0 {
            formatted.push('-');
        }
        formatted.push(*ch);
    }

    formatted
}

/// Decodes a grouped Base32 string back into raw bytes.
/// Ignores hyphens, whitespace, and handles case-insensitivity.
pub fn decode_grouped(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();

    let mut bits = 0u64;
    let mut bit_count = 0;
    let mut bytes = Vec::new();

    for ch in clean.chars() {
        let val = match ALPHABET.iter().position(|&b| b == ch as u8) {
            Some(idx) => idx as u64,
            None => return Err(format!("Invalid Base32 character '{}'", ch)),
        };

        bits = (bits << 5) | val;
        bit_count += 5;

        if bit_count >= 8 {
            bit_count -= 8;
            let byte = ((bits >> bit_count) & 0xFF) as u8;
            bytes.push(byte);
        }
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_roundtrip() {
        let data = b"ZENOCAST-STUDIO-LICENSE-2026";
        let encoded = encode_grouped(data);
        assert!(encoded.contains('-'));

        let decoded = decode_grouped(&encoded).unwrap();
        assert_eq!(decoded, data);

        // Test case-insensitivity and ignore hyphens/spaces
        let lower = encoded.to_lowercase().replace('-', " ");
        let decoded_lower = decode_grouped(&lower).unwrap();
        assert_eq!(decoded_lower, data);
    }
}
