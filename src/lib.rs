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

/// A helper type to store an arbitrary error.
///
/// Use DbError::new() to create a new instance.
#[derive(Debug)]
pub struct DbError(Box<error::Error + Send + Sync>);

impl DbError {
    /// A helper method that creates a new instance of DbError using an arbitrary error.
    pub fn new<E: error::Error + Send + Sync + 'static>(e: E) -> Self {
        DbError(Box::new(e))
    }
}

/// Interface to a database.
pub trait DbInterface: Send + Sync {
    /// Stores the data into the database.
    fn store_data(&self, id: ObjectId, data: &[u8]) -> Result<(), DbError>;

    /// Loads data from the database.
    /// Returns a corresponding data if found, `None` otherwise.
    fn load_data(&self, id: ObjectId) -> Result<Option<Vec<u8>>, DbError>;

    /// Removes data from the database.
    /// Returns `None` if a corresponding data is not found, `Ok(())` otherwise.
    fn remove_data(&self, id: ObjectId) -> Result<(), DbError>;

    /// Tells the maximum data size that could be handled.
    fn max_data_size(&self) -> usize;
}
