#[macro_use]
extern crate bson;
extern crate data_encoding;
extern crate iron;
#[macro_use]
extern crate log;
extern crate mongo_driver;
#[macro_use]
extern crate quick_error;

pub mod mongo_impl;
pub mod web;

use bson::oid::ObjectId;
use iron::error::HttpResult;
use mongo_driver::MongoError;

type MongoUri = mongo_driver::client::Uri;

/// Database options.
#[derive(Debug, Clone)]
pub struct DbOptions {
    /// Database URI.
    pub uri: MongoUri,
    /// Database name.
    pub db_name: String,
    /// Collection name in the database.
    pub collection_name: String,
}

/// Interface for a connection pool.
pub trait MongoDbConnector: Sync + Send + 'static {
    /// Establish a connection to a database.
    fn connect(&self) -> Box<MongoDbInterface>;
}

/// Interface to a MongoDB database.
pub trait MongoDbInterface {
    /// Stores the data into the database.
    fn store_data(&self, id: ObjectId, data: &[u8]) -> Result<(), MongoError>;

    /// Loads data from the database.
    /// Returns a corresponding data if found, `None` otherwise.
    fn load_data(&self, id: ObjectId) -> Result<Option<Vec<u8>>, MongoError>;

    /// Removes data from the database.
    /// Returns `None` if a corresponding data is not found, `Ok(())` otherwise.
    fn remove_data(&self, id: ObjectId) -> Result<(), MongoError>;

    /// Tells the maximum data size that could be handled.
    fn max_data_size(&self) -> usize;
}
