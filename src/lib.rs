#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate bson;
extern crate data_encoding;
#[macro_use]
extern crate log;
extern crate mongo_driver;
#[macro_use]
extern crate quick_error;
extern crate rocket;

pub mod mongo_impl;
pub mod web;

pub use bson::oid::ObjectId;
pub use mongo_driver::MongoError;
pub use mongo_driver::client::Uri;
pub type RocketError = rocket::error::LaunchError;

/// Database options.
#[derive(Debug, Clone)]
pub struct DbOptions {
    /// Database URI.
    pub uri: Uri,
    /// Database name.
    pub db_name: String,
    /// Collection name in the database.
    pub collection_name: String,
}

pub trait MongoDbConnector: Sync + Send + 'static {
    /// Establish a connection to a database.
    fn connect(&self) -> Box<MongoDbInterface>;
}

/// Interface to a MongoDB database.
pub trait MongoDbInterface {
    /// Stores the data into the database.
    fn store_data(&self, id: ObjectId, data: &[u8]) -> Result<(), MongoError>;

    /// Loads data from the database.
    /// \return A corresponding data if found, `None` otherwise.
    fn load_data(&self, id: ObjectId) -> Result<Option<Vec<u8>>, MongoError>;

    /// Removes data from the database.
    /// \return `None` if a corresponding data is not found, `Ok(())` otherwise.
    fn remove_data(&self, id: ObjectId) -> Result<(), MongoError>;

    /// Tells the maximum data size that could be handled.
    fn max_data_size(&self) -> usize;
}
