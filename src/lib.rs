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

extern crate bson;
extern crate data_encoding;
extern crate iron;
#[macro_use]
extern crate quick_error;

pub mod web;

#[cfg(test)]
mod test;

#[cfg(test)]
extern crate reqwest;

use bson::oid::ObjectId;
use iron::error::HttpResult;
use std::error;

/// Interface to a database.
///
/// To store and retrieve pastes from a database we only need several functions. And we can
/// describe them to be abstract enough to be easily used with just any kind of database, be it
/// SQL, NoSQL or just a hash table or whatever.
pub trait DbInterface: Send + Sync {
    type Error: Send + Sync + error::Error + 'static;

    /// Stores the data into the database under a given ID.
    ///
    /// Unique ID has to be generated before calling this method. It might be found to be an extra
    /// burden since usually a database will generate an ID for you, but generating it in advance
    /// actually makes you not to rely on a database to return the generated ID. As of MongoDB, the
    /// identifier is generated on the client side anyhow.
    fn store_data(&self, id: ObjectId, data: &[u8]) -> Result<(), Self::Error>;

    /// Loads data from the database.
    ///
    /// Returns a corresponding data if found, `None` otherwise.
    fn load_data(&self, id: ObjectId) -> Result<Option<Vec<u8>>, Self::Error>;

    /// Removes data from the database.
    ///
    /// Normally we don't care whether an object exists in the database or not, so an
    /// implementation doesn't have to check that fact, and usually databases are okay with
    /// attempts to remove something that doesn't exist.
    fn remove_data(&self, id: ObjectId) -> Result<(), Self::Error>;

    /// Tells the maximum data size that could be handled.
    ///
    /// This is useful, for example, for MongoDB which has a limit on a BSON document size.
    fn max_data_size(&self) -> usize;
}
