//! Library erros.

use base64;
use iron::IronError;
use iron::status;
use std::io;
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
        /// Malformed URI (no ID).
        NoIdSegment {
            description("ID segment not found in the URL")
        }
        /// Unknown ID.
        IdNotFound(id: u64) {
            description("ID not found")
            display("Id {} not found", id)
        }
        /// ID decoder error.
        IdDecode(err: base64::DecodeError) {
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
