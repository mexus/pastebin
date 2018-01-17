//! Module that deals with a web server.
//!
//! See [run_web](fn.run_web.html) documentation for details.

use DbInterface;
use HttpResult;
use ObjectId;
use bson;
use id;
use iron;
use iron::Handler;
use iron::headers::ContentType;
use iron::method::Method;
use iron::prelude::*;
use iron::status;
use mime_guess;
use std::{error, str};
use std::convert::From;
use std::io::{self, Read};
use std::net::ToSocketAddrs;
use std::path::Path;
use tera::{self, escape_html, Tera};
use tree_magic;

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
        /// Id convertion error.
        IdRelated(err: id::Error) {
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
        Utf8(err: str::Utf8Error) {
            from()
            cause(err)
        }
        /// Tera rendering error.
        Tera(err: tera::Error) {
            from()
            cause(err)
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

/// Convenience functions for a `Request`.
trait RequestExt {
    /// Checks if a request has been made from a known browser as opposed to a command line client
    /// (like wget or curl).
    fn is_browser(&self) -> bool;

    /// Tries to guess a MIME type from a provided file name.
    fn mime_from_request(&self) -> Option<&'static str>;

    /// Takes the first URI segment (like `ID` in `http://localhost:8000/ID`).
    fn id_from_request(&self) -> Result<String, Error>;
}

impl<'a, 'b> RequestExt for Request<'a, 'b> {
    fn is_browser(&self) -> bool {
        lazy_static! {
            static ref BROWSERS: Vec<&'static str> =
                vec!["Gecko/", "AppleWebKit/", "Opera/", "Trident/", "Chrome/"];
        }
        self.headers.get::<iron::headers::UserAgent>()
            .map(|agent| {
                     debug!("User agent: [{}]", agent);
                     BROWSERS.iter().any(|browser| agent.contains(browser))
                 })
            .unwrap_or(false)
    }

    fn mime_from_request(&self) -> Option<&'static str> {
        self.url.as_ref()
            .path_segments()
            .and_then(|mut it| it.next())
            .and_then(|f| Path::new(f).extension().and_then(|s| s.to_str()))
            .and_then(mime_guess::get_mime_type_str)
    }

    fn id_from_request(&self) -> Result<String, Error> {
        self.url.as_ref()
            .path_segments()
            .and_then(|mut it| it.next())
            .ok_or(Error::NoIdSegment)
            .map(|s| s.to_string())
    }
}

/// An intermediate structure that handles information about a MongoDB connection and web templates
/// engine.
struct Pastebin<E> {
    db: Box<DbInterface<Error = E>>,
    templates: Tera,
    url_prefix: String,
}

fn is_text(mime: &str) -> bool {
    match mime {
        "text/plain" => true,
        "text/x-markdown" => true,
        "text/x-python" => true,
        "text/x-rust" => true,
        "text/x-toml" => true,
        "application/x-sh" => true,
        _ => false,
    }
}

impl<E> Pastebin<E>
    where E: Send + Sync + error::Error + 'static
{
    /// Initializes a pastebin web server with a database interface.
    fn new(db: Box<DbInterface<Error = E>>, templates: Tera, url_prefix: String) -> Self {
        Pastebin { db,
                   templates,
                   url_prefix, }
    }

    /// Handles `GET` requests.
    fn get(&self, req: &mut Request) -> IronResult<Response> {
        let str_id = req.id_from_request()?;
        let id = itry!(id::id_from_string(&str_id));
        let (data, mime) = itry!(self.db.load_data(id.clone())).ok_or(Error::IdNotFound(id))?;
        debug!("Mime: {}", mime);
        if is_text(&mime) && req.is_browser() {
            let mut response = Response::new();
            response.headers.set(ContentType::html());
            response.set_mut(itry!(self.templates.render(
                "show.html.tera",
                &json!({
                    "id": escape_html(&str_id),
                    "mime": escape_html(&mime),
                    "data": escape_html(itry!(str::from_utf8(&data)))
                }),
            )))
                    .set_mut(status::Ok);
            Ok(response)
        } else {
            Ok(Response::with((status::Ok, data)))
        }
    }

    /// Handles `POST` requests.
    fn post(&self, req: &mut Request) -> IronResult<Response> {
        let data = load_data(&mut req.body, self.db.max_data_size())?;
        let mime_type = req.mime_from_request().map(Into::into)
                           .unwrap_or_else(|| tree_magic::from_u8(&data));
        let id = itry!(bson::oid::ObjectId::new());
        itry!(self.db.store_data(id.clone(), &data, mime_type));
        Ok(Response::with((status::Ok,
                          format!("{}{}\n",
                                   self.url_prefix,
                                   id::id_to_string(id)))))
    }

    /// Handles `DELETE` requests.
    fn remove(&self, req: &mut Request) -> IronResult<Response> {
        let id = itry!(id::id_from_string(&req.id_from_request()?));
        itry!(self.db.remove_data(id));
        Ok(Response::with(status::Ok))
    }
}

impl<E> Handler for Pastebin<E>
    where E: Send + Sync + error::Error + 'static
{
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.method {
            Method::Get => self.get(req),
            Method::Post | Method::Put => self.post(req),
            Method::Delete => self.remove(req),
            _ => Ok(Response::with(status::MethodNotAllowed)),
        }
    }
}

/// Read a portion of data.
fn read_data_portion<R: Read>(stream: &mut R,
                              buffer: &mut Vec<u8>,
                              portion_size: usize,
                              limit: usize)
                              -> Result<bool, Error> {
    let mut portion = vec![0u8; portion_size];
    let size = stream.read(&mut portion)?;
    if size == 0 {
        return Ok(false);
    }
    if buffer.len() + size > limit {
        return Err(Error::TooBig);
    }
    portion.resize(size, 0u8);
    buffer.append(&mut portion);
    Ok(true)
}

/// Loads data from stream in portions of 512 bytes until an end of data or the limit is reached.
/// If a limit is reached Error::TooBig is returned.
fn load_data<R: Read>(stream: &mut R, limit: usize) -> Result<Vec<u8>, Error> {
    const PORTION_SIZE: usize = 1024;
    let mut result = Vec::with_capacity(limit);
    while read_data_portion(stream, &mut result, PORTION_SIZE, limit)? {}
    Ok(result)
}

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
/// # Example
///
/// Let's say you have some kind of a database wrapper implemented (`DbImplementation`) and you
/// want to run a server with it:
///
/// ```
/// # extern crate pastebin;
/// # extern crate bson;
/// # use pastebin::DbInterface;
/// # use bson::oid::ObjectId;
/// # use std::io;
/// # struct DbImplementation;
/// # impl DbInterface for DbImplementation {
///   # type Error = io::Error;
///   # fn store_data(&self, _: ObjectId, _: &[u8], _: String) -> Result<(), Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn load_data(&self, _: ObjectId) -> Result<Option<(Vec<u8>, String)>, Self::Error> {
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
/// # use pastebin::DbInterface;
/// # use bson::oid::ObjectId;
/// # use std::io;
/// # struct DbImplementation;
/// # impl DbInterface for DbImplementation {
///   # type Error = io::Error;
///   # fn store_data(&self, _: ObjectId, _: &[u8], _: String) -> Result<(), Self::Error> {
///   #   unimplemented!()
///   # }
///   # fn load_data(&self, _: ObjectId) -> Result<Option<(Vec<u8>, String)>, Self::Error> {
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
///     ).unwrap();
/// println!("Ok done"); // <-- will never be reached.
/// # }
/// ```
pub fn run_web<Db, A>(db_wrapper: Db,
                      addr: A,
                      templates: Tera,
                      url_prefix: String)
                      -> HttpResult<iron::Listening>
    where Db: DbInterface + 'static,
          A: ToSocketAddrs
{
    Iron::new(Pastebin::new(Box::new(db_wrapper), templates, url_prefix)).http(addr)
}
