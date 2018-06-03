//! A *Pastebin* server library.
//!
//! This library implements a very simple *pastebin* service, which is basically a web-interface
//! between a user and a database (be it MongoDB, or hash table, or MariaDB, or anything else).
//! The library is database-agnostic, which means a database wrapper has to be implemented for a
//! desired DB kind my implementing a quite simple interface `DbInterface`.
//!
//! [Iron](https://github.com/iron/iron) is used as a web-backend, so all its features could be
//! utilized (at least theoretically). The actual code is in the [web](web/index.html) module,
//! useful examples are also there.

extern crate base64;
extern crate chrono;
#[macro_use]
extern crate iron;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate mime_guess;
#[macro_use]
extern crate quick_error;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate tera;
extern crate tree_magic;

pub mod web;

mod error;
mod id;
mod mime;
mod pastebin;
mod read;
mod request;
#[cfg(test)]
mod test;

#[cfg(test)]
extern crate reqwest;

use chrono::{DateTime, Utc};
pub use error::Error;
use iron::error::HttpResult;

/// A paste representation. As simple as that.
#[derive(Debug, Clone)]
pub struct PasteEntry {
    /// Raw paste data.
    pub data: Vec<u8>,
    /// File name associated with the pate, if any.
    pub file_name: Option<String>,
    /// Mime type of the paste.
    pub mime_type: String,
    /// Expiration date, if any.
    pub best_before: Option<DateTime<Utc>>,
}

/// Interface to a database.
///
/// To store and retrieve pastes from a database we only need several functions. And we can
/// describe them to be abstract enough to be easily used with just any kind of database, be it
/// SQL, NoSQL or just a hash table or whatever.
///
/// # Thread safety
///
/// This trait is required to be thread safe (`Send + Sync`) since it will be used from multiple
/// threads.
///
/// # Errors handling
///
/// An implementation must provide an `Error` type, which must be thread safe as well and also
/// have a `'static` lifetime.
///
/// Should some method return an error it will be logged by the web server, but will not be send to
/// an http client, it will just receive an internal server error:
/// [500](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/500).
pub trait DbInterface: Send + Sync {
    type Error: Send + Sync + std::error::Error + 'static;

    /// Stores the data into the database and returns a unique ID that should be used later to
    /// access the data.
    ///
    /// # Return value
    ///
    /// The function is expected to return a unique ID.
    fn store_data(&self,
                  data: Vec<u8>,
                  file_name: Option<String>,
                  mime_type: String,
                  best_before: Option<DateTime<Utc>>)
                  -> Result<u64, Self::Error>;

    /// Loads data from the database.
    ///
    /// Returns corresponding data if found, `None` otherwise.
    fn load_data(&self, id: u64) -> Result<Option<PasteEntry>, Self::Error>;

    /// Gets a file name of a paste (if any).
    fn get_file_name(&self, id: u64) -> Result<Option<String>, Self::Error>;

    /// Removes data from the database.
    ///
    /// Normally we don't care whether an object exists in the database or not, so an
    /// implementation doesn't have to check that fact, and usually databases are okay with
    /// attempts to remove something that doesn't exist.
    fn remove_data(&self, id: u64) -> Result<(), Self::Error>;

    /// Returns the maximum data size that could be handled.
    ///
    /// This is useful, for example, for MongoDB which has a limit on a BSON document size.
    fn max_data_size(&self) -> usize;
}
