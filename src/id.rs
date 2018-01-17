//! Help utilities for handling paste IDs.

use ObjectId;
use bson;
use data_encoding::{self, BASE64URL_NOPAD};

quick_error!{
    /// Id conversion error.
    #[derive(Debug)]
    pub enum Error {
        /// ObjectID <-> BSON conversion error.
        BsonObjId(err: bson::oid::Error) {
            from()
            cause(err)
        }
        /// ID length error.
        BsonIdWrongLength(len: usize) {
            description("Wrong ID length")
            display("Expected an ID to have length of 12, but it is {}", len)
        }
        /// ID decoding error.
        Decoding(err: data_encoding::DecodeError) {
            from()
            cause(err)
        }
    }
}

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
