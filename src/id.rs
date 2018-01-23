//! Short ID generator/decoder, based on `base64` (url-safe, no-padding version).

use base64;
use error::Error;

/// Combines `u8` numbers into one `u64`. Will panic if there are more than 8 elements provided.
fn combine_bits(buf: &[u8]) -> u64 {
    let mut res = 0u64;
    for i in 0..(buf.len()) {
        res += (buf[buf.len() - i - 1] as u64) << (i * 8);
    }
    res
}

/// Splits an `u8` number into an array of `u8`-s.
fn split_into_bits(n: u64) -> [u8; 8] {
    let mut buf = [0; 8];
    for i in 0..8 {
        buf[i] = ((n << (i * 8)) >> (7 * 8)) as u8;
    }
    buf
}

/// Returns a reference to a first non-zero element of the provided array. If there are no non-zero
/// elements, a reference to `[0]` is returned.
fn trim(b: &[u8]) -> &[u8] {
    for index in 0..(b.len()) {
        if b[index] != 0 {
            return &b[index..];
        }
    }
    &[0]
}

/// Encodes a given `u64` number into a string as short as possible.
pub fn encode_id(id: u64) -> String {
    base64::encode_config(trim(&split_into_bits(id)), base64::URL_SAFE_NO_PAD)
}

/// Converts a string created with `encode_id` function back into a number.
pub fn decode_id(id: &str) -> Result<u64, Error> {
    Ok(combine_bits(&base64::decode_config(id, base64::URL_SAFE_NO_PAD)?))
}
