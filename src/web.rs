//! Module that deals with a web server.
//!
//! See [run_web](fn.run_web.html) documentation for details.

use DbInterface;
use HttpResult;
use chrono::Duration;
use iron::Listening;
use iron::prelude::*;
use pastebin::Pastebin;
use std::net::ToSocketAddrs;
use tera::Tera;

/// Runs a web server.
///
/// This is the main function of the library. Starts a web server and serves the
/// following HTTP requests: `GET`, `POST` and `DELETE`.
///
/// Basically it is just a layer between an `Iron` web server and a `DbInterface` implementation.
///
/// The call returns a `HttpResult` which comes directly from `Iron`, which means you can possibly
/// terminate the server in a clean way. If you don't `close` it explicitly, the object will hang
/// forever in its `drop` implementation. For more details have a look at the
/// `iron::error::HttpResult` documentation.
///
/// # Arguments
///
/// * `default_ttl` represents the default expiration time which will be applied if not
///   `expires` argument for a `POST` request is given.
///
/// # Notice
///
/// * No matter how many ending slashes you added to `url_prefix` (even zero), all of them will be
///   removed and one slash will be added.
///
/// # Example
///
/// Let's say you have some kind of a database wrapper implemented (`DbImplementation`) and you
/// want to run a server with it:
///
/// ```
/// # extern crate pastebin;
/// # extern crate bson;
/// # extern crate chrono;
/// # use pastebin::{DbInterface, PasteEntry};
/// # use bson::oid::ObjectId;
/// # use std::io;
/// # use chrono::{DateTime, Duration, Utc};
/// # struct DbImplementation;
/// # impl DbInterface for DbImplementation {
///   # type Error = io::Error;
///   # fn store_data(&self,
///   #               _id: ObjectId,
///   #               _data: &[u8],
///   #               _file_name: Option<String>,
///   #               _mime_type: String,
///   #               _best_before: Option<DateTime<Utc>>)
///   #               -> Result<(), Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn load_data(&self, _: ObjectId) -> Result<Option<PasteEntry>, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn get_file_name(&self, _: ObjectId) -> Result<Option<String>, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn remove_data(&self, _: ObjectId) -> Result<(), Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn max_data_size(&self) -> usize {
///   #   unimplemented!()
///   # }
/// # }
/// # impl DbImplementation {
/// #   fn new() -> Self { Self{} }
/// # }
/// # fn main() {
/// let mut web = pastebin::web::run_web(
///     DbImplementation::new(/* ... */),
///     "127.0.0.1:8000",
///     // ...
///     # Default::default(),
///     # Default::default(),
///     # Duration::zero(),
///     # Default::default(),
///     ).unwrap();
/// // ... do something ...
/// web.close(); // Graceful termination.
/// println!("Server terminated, exiting");
/// # }
/// ```
///
/// Simple, isn't it? It can be even simplier if you don't care about graceful termination and
/// you're okay with the application working forever:
///
/// ```no_run
/// # extern crate pastebin;
/// # extern crate bson;
/// # extern crate chrono;
/// # use pastebin::{DbInterface, PasteEntry};
/// # use bson::oid::ObjectId;
/// # use std::io;
/// # use chrono::{DateTime, Duration, Utc};
/// # struct DbImplementation;
/// # impl DbInterface for DbImplementation {
///   # type Error = io::Error;
///   # fn store_data(&self,
///   #               _id: ObjectId,
///   #               _data: &[u8],
///   #               _file_name: Option<String>,
///   #               _mime_type: String,
///   #               _best_before: Option<DateTime<Utc>>)
///   #               -> Result<(), Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn load_data(&self, _: ObjectId) -> Result<Option<PasteEntry>, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn get_file_name(&self, _: ObjectId) -> Result<Option<String>, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn remove_data(&self, _: ObjectId) -> Result<(), Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn max_data_size(&self) -> usize {
///   #   unimplemented!()
///   # }
/// # }
/// # impl DbImplementation {
/// #   fn new() -> Self { Self{} }
/// # }
/// # fn main() {
/// pastebin::web::run_web(
///     DbImplementation::new(/* ... */),
///     "127.0.0.1:8000",
///     // ...
///     # Default::default(),
///     # Default::default(),
///     # Duration::zero(),
///     # Default::default(),
///     ).unwrap();
/// println!("Ok done"); // <-- will never be reached.
/// # }
/// ```
pub fn run_web<Db, A>(db_wrapper: Db,
                      addr: A,
                      templates: Tera,
                      url_prefix: &str,
                      default_ttl: Duration,
                      static_files_path: String)
                      -> HttpResult<Listening>
    where Db: DbInterface + 'static,
          A: ToSocketAddrs
{
    // Make sure there is only one trailing slash.
    let url_prefix = format!("{}/", url_prefix.trim_right_matches('/'));
    let pastebin = Pastebin::new(Box::new(db_wrapper),
                                 templates,
                                 url_prefix,
                                 default_ttl,
                                 static_files_path);
    Iron::new(pastebin).http(addr)
}
