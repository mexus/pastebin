//! Module that deals with a web server.
//!
//! See [run_web](fn.run_web.html) documentation for details.

use DbInterface;
use HttpResult;
use ObjectId;
use bson;
use id;
use iron::{self, status, Handler, Url};
use iron::headers::ContentType;
use iron::method::Method;
use iron::modifiers::Redirect;
use iron::prelude::*;
use mime_guess;
use serde_json;
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
        /// URL parsing error.
        Url(err: String) {
            description("Can't parse URL")
            display("Can't parse URL: {}", err)
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

    /// Tries to obtain an `n`-th segment of the URI.
    fn url_segment_n(&self, n: usize) -> Option<String>;
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

    fn url_segment_n(&self, n: usize) -> Option<String> {
        self.url.as_ref()
            .path_segments()
            .and_then(|mut it| it.nth(n))
            .and_then(|s| {
                          if s.is_empty() {
                              None
                          } else {
                              Some(s.to_string())
                          }
                      })
    }
}

fn mime_from_file_name<P: AsRef<str>>(name: P) -> Option<&'static str> {
    Path::new(name.as_ref()).extension()
                            .and_then(|s| s.to_str())
                            .and_then(mime_guess::get_mime_type_str)
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
        "application/x-sh" => true,
        s if s.starts_with("text/") => true,
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

    /// Render a template.
    fn render_template(&self,
                       name: &str,
                       content_type: ContentType,
                       data: &serde_json::Value)
                       -> IronResult<Response> {
        let mut response = Response::new();
        response.headers.set(content_type);
        response.set_mut(itry!(self.templates.render(&format!("{}.tera", name), data,)))
                .set_mut(status::Ok);
        Ok(response)
    }

    /// Serves data in a form of HTML.
    fn serve_data_html(&self,
                       id: &str,
                       mime: &str,
                       file_name: Option<String>,
                       data: &[u8])
                       -> IronResult<Response> {
        self.render_template(
            "show.html",
            ContentType::html(),
            &json!({
                    "id": escape_html(id),
                    "mime": escape_html(mime),
                    "file_name": file_name.map(|s| escape_html(&s)),
                    "data": escape_html(itry!(str::from_utf8(data)))
                }),
        )
    }

    /// Loads a paste from the database.
    fn get_paste(&self, id: &str, is_browser: bool, name_provided: bool) -> IronResult<Response> {
        debug!("Id: '{}'", id);
        let object_id = itry!(id::id_from_string(id));
        if !name_provided {
            if let Some(name) = itry!(self.db.get_file_name(object_id.clone())) {
                let new_url =
                    Url::parse(&format!("{}{}/{}", self.url_prefix, id, name))
                        .map_err(|e| Error::Url(e))?;
                return Ok(Response::with((status::MovedPermanently, Redirect(new_url))));
            }
        }
        let paste =
            itry!(self.db.load_data(object_id.clone())).ok_or(Error::IdNotFound(object_id))?;
        if is_text(&paste.mime_type) && is_browser {
            self.serve_data_html(id, &paste.mime_type, paste.file_name, &paste.data)
        } else {
            Ok(Response::with((status::Ok, paste.data)))
        }
    }

    /// Handles `GET` requests.
    ///
    /// If a URI segment is not provided then the upload form is rendered, otherwise the first
    /// segment is considered to be a paste ID, and hence the paste is fetched from the DB.
    fn get(&self, req: &mut Request) -> IronResult<Response> {
        match req.url_segment_n(0).as_ref().map(String::as_str) {
            None => self.render_template("upload.html", ContentType::html(), &json!({})),
            Some("paste.sh") => self.render_template("paste.sh",
                                                     ContentType::plaintext(),
                                                     &json!({"prefix": &self.url_prefix})),
            Some("readme") => self.render_template("readme.html",
                                                   ContentType::html(),
                                                   &json!({"prefix": &self.url_prefix})),
            Some(id) => self.get_paste(id, req.is_browser(), req.url_segment_n(1).is_some()),
        }
    }

    /// Handles `POST` requests.
    fn post(&self, req: &mut Request) -> IronResult<Response> {
        let file_name = req.url_segment_n(0);
        debug!("File name: {:?}", file_name);
        let data = load_data(&mut req.body, self.db.max_data_size())?;
        let mime_type = file_name.as_ref()
                                 .and_then(mime_from_file_name)
                                 .map(Into::into)
                                 .unwrap_or_else(|| tree_magic::from_u8(&data));
        let id = itry!(bson::oid::ObjectId::new());
        itry!(self.db.store_data(id.clone(), &data, file_name, mime_type));
        Ok(Response::with((status::Ok,
                          format!("{}{}\n",
                                   self.url_prefix,
                                   id::id_to_string(id)))))
    }

    /// Handles `DELETE` requests.
    fn remove(&self, req: &mut Request) -> IronResult<Response> {
        let id = itry!(id::id_from_string(&req.url_segment_n(0).ok_or(Error::NoIdSegment)?));
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
/// # use pastebin::{DbInterface, PasteEntry};
/// # use bson::oid::ObjectId;
/// # use std::io;
/// # struct DbImplementation;
/// # impl DbInterface for DbImplementation {
///   # type Error = io::Error;
///   # fn store_data(&self, _: ObjectId, _: &[u8], _: Option<String>, _: String) -> Result<(), Self::Error> {
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
/// # use pastebin::{DbInterface, PasteEntry};
/// # use bson::oid::ObjectId;
/// # use std::io;
/// # struct DbImplementation;
/// # impl DbInterface for DbImplementation {
///   # type Error = io::Error;
///   # fn store_data(&self, _: ObjectId, _: &[u8], _: Option<String>, _: String) -> Result<(), Self::Error> {
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
///     ).unwrap();
/// println!("Ok done"); // <-- will never be reached.
/// # }
/// ```
pub fn run_web<Db, A>(db_wrapper: Db,
                      addr: A,
                      templates: Tera,
                      url_prefix: &str)
                      -> HttpResult<iron::Listening>
    where Db: DbInterface + 'static,
          A: ToSocketAddrs
{
    // Make sure there is only one trailing slash.
    let url_prefix = format!("{}/", url_prefix.trim_right_matches('/'));
    let pastebin = Pastebin::new(Box::new(db_wrapper), templates, url_prefix);
    Iron::new(pastebin).http(addr)
}
