//! Library erros.

use bson;
use bson::oid::ObjectId;
use data_encoding;
use iron::IronError;
use iron::status;
use std::io;
use std::str::Utf8Error;
use tera;

quick_error!{
    /// Container for errors that might happen during processing requests.
    #[derive(Debug)]
    pub enum Error {
        /// Input/output error.
        Io(err: io::Error) {
            from()
            cause(err)
        }
        /// Data limit exceeded.
        TooBig {
            description("Too large paste")
        }
        /// ID decoding error.
        Decoding(err: data_encoding::DecodeError) {
            from()
            cause(err)
        }
        /// ObjectID conversion error.
        BsonObjId(err: bson::oid::Error) {
            from()
            cause(err)
        }
        /// ID length error.
        BsonIdWrongLength(len: usize) {
            description("Wrong ID length")
            display("Expected an ID to have length of 12, but it is {}", len)
        }
        /// Malformed URI (no ID).
        NoIdSegment {
            description("ID segment not found in the URL")
        }
        /// Unknown ID.
        IdNotFound(id: ObjectId) {
            description("ID not found")
            display("Id {} not found", id)
        }
        /// UTF8 conversion error.
        Utf8(err: Utf8Error) {
            from()
            cause(err)
        }
        /// Tera rendering error.
        Tera(err: tera::Error) {
            from()
            cause(err)
        }
        /// URL parsing error.
        Url(err: String) {
            description("Can't parse URL")
            display("Can't parse URL: {}", err)
        }
    }
}

impl From<Error> for IronError {
    fn from(err: Error) -> IronError {
        match err {
            e @ Error::IdNotFound(_) => IronError::new(e, status::NotFound),
            e @ Error::TooBig => IronError::new(e, status::PayloadTooLarge),
            e => IronError::new(e, status::BadRequest),
        }
    }
}
