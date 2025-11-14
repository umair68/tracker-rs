use anyhow::{bail, Context, Result};

/// Convert a hexadecimal string to bytes
/// 
/// # Arguments
/// * `hex_str` - A string containing hexadecimal characters (0-9, a-f, A-F)
/// 
/// # Returns
/// * `Result<Vec<u8>>` - The decoded bytes or an error if the input is invalid
pub fn hex_to_bytes(hex_str: &str) -> Result<Vec<u8>> {
    if hex_str.len() % 2 != 0 {
        bail!("Hex string must have even length");
    }

    let mut bytes = Vec::with_capacity(hex_str.len() / 2);
    
    for i in (0..hex_str.len()).step_by(2) {
        let byte_str = &hex_str[i..i + 2];
        let byte = u8::from_str_radix(byte_str, 16)
            .context("Invalid hex character")?;
        bytes.push(byte);
    }

    Ok(bytes)
}

/// Convert bytes to a hexadecimal string
/// 
/// # Arguments
/// * `bytes` - A slice of bytes to encode
/// 
/// # Returns
/// * `String` - The hexadecimal representation (lowercase)
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Decode URL-encoded bytes (percent-encoding)
/// 
/// This function handles URL-encoded strings like those used for info_hash and peer_id
/// in BitTorrent announce requests. It decodes percent-encoded sequences (%XX) and
/// returns the raw bytes.
/// 
/// # Arguments
/// * `encoded` - A URL-encoded string
/// 
/// # Returns
/// * `Result<Vec<u8>>` - The decoded bytes or an error if the encoding is invalid
pub fn url_decode(encoded: &str) -> Result<Vec<u8>> {
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut chars = encoded.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '%' => {
                // Read the next two hex digits
                let hex1 = chars.next()
                    .context("Incomplete percent-encoding: missing first hex digit")?;
                let hex2 = chars.next()
                    .context("Incomplete percent-encoding: missing second hex digit")?;
                
                let hex_str = format!("{}{}", hex1, hex2);
                let byte = u8::from_str_radix(&hex_str, 16)
                    .context("Invalid hex digits in percent-encoding")?;
                
                decoded.push(byte);
            }
            '+' => {
                // '+' is decoded as space in URL encoding
                decoded.push(b' ');
            }
            _ => {
                // Regular ASCII character
                decoded.push(ch as u8);
            }
        }
    }

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("48656c6c6f").unwrap(), b"Hello");
        assert_eq!(hex_to_bytes("deadbeef").unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(hex_to_bytes("DEADBEEF").unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(hex_to_bytes("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_hex_to_bytes_invalid() {
        assert!(hex_to_bytes("abc").is_err()); // Odd length
        assert!(hex_to_bytes("xyz").is_err()); // Invalid hex
        assert!(hex_to_bytes("12g4").is_err()); // Invalid character
    }

    #[test]
    fn test_bytes_to_hex() {
        assert_eq!(bytes_to_hex(b"Hello"), "48656c6c6f");
        assert_eq!(bytes_to_hex(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
        assert_eq!(bytes_to_hex(&[]), "");
    }

    #[test]
    fn test_hex_roundtrip() {
        let original = vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
        let hex = bytes_to_hex(&original);
        let decoded = hex_to_bytes(&hex).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_url_decode() {
        // Simple ASCII
        assert_eq!(url_decode("hello").unwrap(), b"hello");
        
        // Percent-encoded
        assert_eq!(url_decode("%48%65%6c%6c%6f").unwrap(), b"Hello");
        
        // Mixed
        assert_eq!(url_decode("hello%20world").unwrap(), b"hello world");
        
        // Plus sign as space
        assert_eq!(url_decode("hello+world").unwrap(), b"hello world");
        
        // Binary data (like info_hash)
        assert_eq!(
            url_decode("%de%ad%be%ef").unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
    }

    #[test]
    fn test_url_decode_invalid() {
        assert!(url_decode("%").is_err()); // Incomplete
        assert!(url_decode("%1").is_err()); // Incomplete
        assert!(url_decode("%GG").is_err()); // Invalid hex
    }

    #[test]
    fn test_url_decode_info_hash() {
        // Simulate a real info_hash (20 bytes)
        let encoded = "%12%34%56%78%9a%bc%de%f0%11%22%33%44%55%66%77%88%99%aa%bb%cc";
        let decoded = url_decode(encoded).unwrap();
        assert_eq!(decoded.len(), 20);
        assert_eq!(decoded[0], 0x12);
        assert_eq!(decoded[19], 0xcc);
    }
}
