//! Human codes: opening and end codes, receipts and fingerprints, in Crockford base 32.
//!
//! The alphabet has no I, L, O or U, so a code read aloud or copied from a board survives
//! confusion: `O` is read as `0`, `I` and `L` as `1`, case and dashes do not matter.

use hpke::rand_core::CryptoRng;

/// Symbols of a generated code: 12 symbols of 5 bits, 60 random bits.
pub const CODE_SYMBOLS: usize = 12;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A fresh code such as `K7QM-2XWD-9FHA`.
pub fn generate_code(rng: &mut impl CryptoRng) -> String {
    let mut bytes = [0_u8; 8];
    rng.fill_bytes(&mut bytes);
    group(&encode(&bytes, CODE_SYMBOLS), 4)
}

/// The canonical form of a typed code (12 symbols, no dash), or `None` when it is not one.
pub fn normalize_code(input: &str) -> Option<String> {
    let mut out = String::with_capacity(CODE_SYMBOLS);
    for c in input.chars() {
        let c = match c.to_ascii_uppercase() {
            '-' | ' ' | '\t' => continue,
            'O' => '0',
            'I' | 'L' => '1',
            c if c.is_ascii() && ALPHABET.contains(&(c as u8)) => c,
            _ => return None,
        };
        out.push(c);
    }
    (out.len() == CODE_SYMBOLS).then_some(out)
}

/// The first `symbols` base-32 symbols of `bytes`, most significant bits first.
pub(crate) fn encode(bytes: &[u8], symbols: usize) -> String {
    let mut out = String::with_capacity(symbols);
    let (mut acc, mut bits) = (0_u32, 0_u32);
    for &byte in bytes {
        if out.len() == symbols {
            break;
        }
        acc = (acc << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 && out.len() < symbols {
            bits -= 5;
            out.push(char::from(ALPHABET[((acc >> bits) & 31) as usize]));
        }
        acc &= (1 << bits) - 1;
    }
    out
}

/// `ABCDEFGH` grouped by 4 gives `ABCD-EFGH`.
pub(crate) fn group(symbols: &str, size: usize) -> String {
    let chars: Vec<char> = symbols.chars().collect();
    chars
        .chunks(size)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_is_most_significant_first() {
        assert_eq!(encode(&[0, 0], 3), "000");
        assert_eq!(encode(&[0xFF, 0xFF], 3), "ZZZ");
        assert_eq!(encode(&[0b0000_1000, 0b0100_0000], 3), "110");
    }

    #[test]
    fn normalization_forgives_reading_mistakes() {
        assert_eq!(
            normalize_code("k7qm-2xwd-9fha"),
            Some("K7QM2XWD9FHA".to_string())
        );
        assert_eq!(
            normalize_code("O1IL 0000 0000"),
            Some("011100000000".to_string())
        );
        assert_eq!(normalize_code("K7QM-2XWD-9FH"), None);
        assert_eq!(normalize_code("K7QM-2XWD-9FHU"), None);
        assert_eq!(normalize_code("K7QM-2XWD-9FHÉ"), None);
    }

    #[test]
    fn grouping() {
        assert_eq!(group("ABCDEFGHIJ", 4), "ABCD-EFGH-IJ");
    }
}
