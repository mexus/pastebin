//! Help utilities for handling paste IDs.

use ObjectId;
use data_encoding::BASE64URL_NOPAD;
use error::Error;

/// Decodes string into an ObjectID.
pub fn id_from_string(src: &str) -> Result<ObjectId, Error> {
    let dyn_bytes: Vec<u8> = BASE64URL_NOPAD.decode(src.as_bytes())?;
    if dyn_bytes.len() != 12 {
        return Err(Error::BsonIdWrongLength(dyn_bytes.len()));
    }
    let mut bytes = [0u8; 12];
    for i in 0..12usize {
        bytes[i] = dyn_bytes[i];
    }
    Ok(ObjectId::with_bytes(bytes))
}

/// Converts an ObjectID into a string.
pub fn id_to_string(id: ObjectId) -> String {
    BASE64URL_NOPAD.encode(&id.bytes())
}
