//! Module that deals with a web server.
//!
//! See [run_web](fn.run_web.html) documentation for details.

use DbInterface;
use HttpResult;
use ObjectId;
use bson;
use data_encoding::{self, BASE64URL_NOPAD};
use handlebars_iron::{HandlebarsEngine, Template};
use iron;
use iron::Handler;
use iron::method::Method;
use iron::prelude::*;
use iron::status;
use mime_guess;
use std::{error, str};
use std::convert::From;
use std::io::{self, Read};
use std::net::ToSocketAddrs;
use std::path::Path;
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
        /// ID decoding error.
        Decoding(err: data_encoding::DecodeError) {
            from()
            cause(err)
        }
        /// Data limit exceeded.
        TooBig {
            description("Too large paste")
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
    }
}

struct DbError<E: Sync + Send + error::Error>(E);

impl<E> From<DbError<E>> for IronError
    where E: Sync + Send + error::Error + 'static
{
    fn from(err: DbError<E>) -> IronError {
        IronError::new(err.0, status::BadRequest)
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

/// An intermediate structure that handles information about a MongoDB connection.
struct Pastebin<E> {
    db: Box<DbInterface<Error = E>>,
}

/// Takes the first URI segment (like `ID` in `http://localhost:8000/ID`) and tries to convert it
/// to an ObjectId.
fn id_from_request(req: &Request) -> Result<ObjectId, Error> {
    req.url.as_ref()
       .path_segments()
       .and_then(|mut it| it.next())
       .ok_or(Error::NoIdSegment)
       .and_then(id_from_string)
}

/// Tries to guess a MIME type from a provided file name.
fn mime_from_request(req: &Request) -> Option<&'static str> {
    req.url.as_ref()
       .path_segments()
       .and_then(|mut it| it.next())
       .and_then(|f| Path::new(f).extension().and_then(|s| s.to_str()))
       .and_then(mime_guess::get_mime_type_str)
}

/// Decodes string into an ObjectID.
fn id_from_string(src: &str) -> Result<ObjectId, Error> {
    let dyn_bytes = BASE64URL_NOPAD.decode(src.as_bytes())?;
    if dyn_bytes.len() != 12 {
        return Err(Error::BsonIdWrongLength(dyn_bytes.len()));
    }
    let mut bytes = [0u8; 12];
    for i in 0..12usize {
        bytes[i] = dyn_bytes[i];
    }
    Ok(ObjectId::with_bytes(bytes))
}

/// Checks if a request has been made from a known browser as opposed to a command line client
/// (like wget or curl).
fn is_browser(req: &Request) -> bool {
    req.headers.get::<iron::headers::UserAgent>()
       .map(|agent| {
                debug!("User agent: [{}]", agent);
                agent.contains("Gecko/") || agent.contains("AppleWebKit/")
                || agent.contains("Opera/") || agent.contains("Trident/")
                || agent.contains("Chrome/")
            })
       .unwrap_or(false)
}

fn is_text(mime: &str) -> bool {
    match mime {
        "text/plain" => true,
        "text/x-markdown" => true,
        "text/x-python" => true,
        "text/x-rust" => true,
        "application/x-sh" => true,
        _ => false,
    }
}

impl<E> Pastebin<E>
    where E: Send + Sync + error::Error + 'static
{
    /// Initializes a pastebin web server with a database interface.
    fn new(db: Box<DbInterface<Error = E>>) -> Self {
        Pastebin { db }
    }

    /// Handles `GET` requests.
    fn get(&self, req: &mut Request) -> IronResult<Response> {
        let id = id_from_request(req)?;
        let (data, mime) = self.db.load_data(id.clone())
                               .map_err(DbError)?
                               .ok_or(Error::IdNotFound(id))?;
        debug!("Mime: {}", mime);
        if is_text(&mime) && is_browser(req) {
            let mut response = Response::new();
            response.set_mut(Template::new(
                "show",
                json!({"data": str::from_utf8(&data).map_err(Into::<Error>::into)?}),
            )).set_mut(status::Ok);
            Ok(response)
        } else {
            Ok(Response::with((status::Ok, data)))
        }
    }

    /// Handles `POST` requests.
    fn post(&self, req: &mut Request) -> IronResult<Response> {
        let data = load_data(&mut req.body, self.db.max_data_size())?;
        let mime_type = mime_from_request(req).map(Into::into)
                                              .unwrap_or_else(|| tree_magic::from_u8(&data));
        let id = bson::oid::ObjectId::new().map_err(Into::<Error>::into)?;
        self.db.store_data(id.clone(), &data, mime_type)
            .map_err(DbError)?;
        Ok(Response::with((status::Ok, BASE64URL_NOPAD.encode(&id.bytes()))))
    }

    /// Handles `DELETE` requests.
    fn remove(&self, req: &mut Request) -> IronResult<Response> {
        let id = id_from_request(req)?;
        self.db.remove_data(id).map_err(DbError)?;
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
/// let mut web = pastebin::web::run_web(DbImplementation::new(/* ... */),
///                                      "127.0.0.1:8000").unwrap();
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
/// pastebin::web::run_web(DbImplementation::new(/* ... */), "127.0.0.1:8000").unwrap();
/// println!("Ok done"); // <-- will never be reached.
/// # }
/// ```
pub fn run_web<Db, A>(db_wrapper: Db,
                      addr: A,
                      handlebars: HandlebarsEngine)
                      -> HttpResult<iron::Listening>
    where Db: DbInterface + 'static,
          A: ToSocketAddrs
{
    let mut chain = Chain::new(Pastebin::new(Box::new(db_wrapper)));
    chain.link_after(handlebars);
    Iron::new(chain).http(addr)
}
