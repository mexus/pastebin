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
extern crate rand;
extern crate rocket;

pub mod mongo_impl;
pub mod web;

pub use mongo_driver::MongoError;
pub type RocketError = rocket::error::LaunchError;

/// Database options.
#[derive(Debug, Clone)]
pub struct DbOptions {
    /// Database host.
    pub host: String,
    /// Database port.
    pub port: u16,
    /// Database name.
    pub db_name: String,
    /// Database user.
    pub db_user: Option<String>,
    /// Database user's password.
    pub db_pass: Option<String>,
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
    /// \return A unique identifier of a stored data.
    fn store_data(&self, data: &[u8]) -> Result<[u8; 4], MongoError>;

    /// Loads data from the database.
    /// \return A corresponding data if found, `None` otherwise.
    fn load_data(&self, id: &[u8]) -> Result<Option<Vec<u8>>, MongoError>;

    /// Removes data from the database.
    /// \return `None` if a corresponding data is not found, `Ok(())` otherwise.
    fn remove_data(&self, id: &[u8]) -> Result<(), MongoError>;

    /// Tells the maximum data size that could be handled.
    fn max_data_size(&self) -> usize;
}
