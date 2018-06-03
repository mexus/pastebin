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
/// following HTTP requests: `GET`, `POST`, `PUT` and `DELETE`.
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
/// * `db_wrapper` is a layer that provides an access to your favourite database. Must implement
/// `DbInterface` and have a `'static` lifetime.
///
/// * `addr` is a local address which the webserver will use. Rust provides a very nice way to
/// handle it, please go ahead and read docs regarding the `ToSocketAddrs` trait, but if you need a
/// fast solution just pass a string like `"0.0.0.0:8000"` to make the server to listen to incoming
/// requests on port 8000 on all the available network interfaces.
///
/// * `templates` is an instance of the [Tera](https://github.com/Keats/tera) template engine.
/// Please refer to the following section to learn the requirements.
///
/// * `url_prefix` used for responding to `POST`/`PUT` requests: if a paste has been successfully
/// inserted into the database the server will reply with the following string: `${addr}id` (please
/// mind that `addr` will always end with a slash `/`), where `${addr}` is that url prefix (with a
/// slash…). So you probably want to put an external address of your paste service instance ;-).
///
/// * `default_ttl` represents the default expiration time which will be applied if not `expires`
/// argument for a `POST`/`PUT` request is given.
///
/// * `static_files_path` is a path relative to the working path (i.e. the path where you have
/// launched the service). As the name suggests it will be used to server static files that reside
/// in that directory. As for now, *sub-directories are not supported*, that is you can't serve
/// files that reside not directly at the path. To access a static file use a `GET` request on the
/// address `/<file-name>`, very simple and straightforward.
///
/// # Templates
///
/// The service uses `Tera` templates to build web pages, so it expects the engine to serve the
/// following files:
///
/// * `show.html.tera`: expects `id` (a paste id), `mime` (mime-type string), `file_name` (`null`
/// if there is no file name associated with the paste), and `data` which is actually the paste
/// itself.
/// * `upload.html.tera`: no parameters.
/// * `paste.sh.tera`: expects `prefix`, see `url_prefix` argument.
/// * `readme.html.tera`: also expects `prefix`.
///
/// All these files are provided with the service (`/templates/`).
///
/// # Notice
///
/// No matter how many ending slashes (`/`) you add to `url_prefix` (even zero), all of them will be
/// removed and one slash will be added. If for some reason you need two (or more) ending slashes
/// feel free to post an issue on github, because this is basically a hack and could be (and
/// probably should be) properly fixed.
///
/// # `PUT` vs `POST`
///
/// While [REST](https://en.wikipedia.org/wiki/Representational_state_transfer) differentiates
/// between those two request kinds, there is no difference in this service. Why? Well, just
/// because some CLI clients tend to use `POST` requests by default for sending data and some use
/// `PUT`, so that's why the service do not care. If you have any argument why this shouldn't be
/// the case please fill free to post an issue on github.
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
/// # use std::io;
/// # use chrono::{DateTime, Duration, Utc};
/// # struct DbImplementation;
/// # impl DbInterface for DbImplementation {
///   # type Error = io::Error;
///   # fn store_data(&self,
///   #               _data: Vec<u8>,
///   #               _file_name: Option<String>,
///   #               _mime_type: String,
///   #               _best_before: Option<DateTime<Utc>>)
///   #               -> Result<u64, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn load_data(&self, _: u64) -> Result<Option<PasteEntry>, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn get_file_name(&self, _: u64) -> Result<Option<String>, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn remove_data(&self, _: u64) -> Result<(), Self::Error> {
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
/// # use std::io;
/// # use chrono::{DateTime, Duration, Utc};
/// # struct DbImplementation;
/// # impl DbInterface for DbImplementation {
///   # type Error = io::Error;
///   # fn store_data(&self,
///   #               _data: Vec<u8>,
///   #               _file_name: Option<String>,
///   #               _mime_type: String,
///   #               _best_before: Option<DateTime<Utc>>)
///   #               -> Result<u64, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn load_data(&self, _: u64) -> Result<Option<PasteEntry>, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn get_file_name(&self, _: u64) -> Result<Option<String>, Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn remove_data(&self, _: u64) -> Result<(), Self::Error> {
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
