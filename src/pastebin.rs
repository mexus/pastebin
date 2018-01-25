use DbInterface;
use Error;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use id::{decode_id, encode_id};
use iron::{status, Handler, Url};
use iron::headers::ContentType;
use iron::method::Method;
use iron::modifiers::Redirect;
use iron::prelude::*;
use iron::response::BodyReader;
use mime;
use read::load_data;
use request::RequestExt;
use serde_json;
use std;
use std::borrow::Cow;
use std::fs::File;
use std::ops::Add;
use std::path::PathBuf;
use std::str::from_utf8;
use tera::{escape_html, Tera};

/// An intermediate structure that handles information about a MongoDB connection and web templates
/// engine.
pub struct Pastebin<E> {
    db: Box<DbInterface<Error = E>>,
    templates: Tera,
    url_prefix: String,
    default_ttl: Duration,
    static_path: PathBuf,
}

impl<E> Pastebin<E>
    where E: Send + Sync + std::error::Error + 'static
{
    /// Initializes a pastebin web server with a database interface.
    pub fn new(db: Box<DbInterface<Error = E>>,
               templates: Tera,
               url_prefix: String,
               default_ttl: Duration,
               static_path: String)
               -> Self {
        Pastebin { db,
                   templates,
                   url_prefix,
                   default_ttl,
                   static_path: static_path.into(), }
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
                       id: u64,
                       mime: &str,
                       file_name: Option<String>,
                       data: &[u8])
                       -> IronResult<Response> {
        self.render_template(
            "show.html",
            ContentType::html(),
            &json!({
                    "id": id,
                    "mime": escape_html(mime),
                    "file_name": file_name.map(|s| escape_html(&s)),
                    "data": escape_html(itry!(from_utf8(data)))
                }),
        )
    }

    /// Loads a paste from the database.
    fn get_paste(&self,
                 str_id: &str,
                 is_browser: bool,
                 name_provided: bool)
                 -> IronResult<Response> {
        let id = itry!(decode_id(str_id));
        if !name_provided {
            if let Some(name) = itry!(self.db.get_file_name(id)) {
                let new_url =
                    Url::parse(&format!("{}{}/{}", self.url_prefix, str_id, name))
                        .map_err(|e| Error::Url(e))?;
                return Ok(Response::with((status::MovedPermanently, Redirect(new_url))));
            }
        }
        let paste = itry!(self.db.load_data(id)).ok_or(Error::IdNotFound(id))?;
        if mime::is_text(&paste.mime_type) && is_browser {
            self.serve_data_html(id, &paste.mime_type, paste.file_name, &paste.data)
        } else {
            let mut response = Response::new();
            response.headers.set(mime::to_content_type(paste.mime_type));
            response.set_mut((status::Ok, paste.data));
            Ok(response)
        }
    }

    /// Serves a static file.
    fn serve_static(&self, file_name: &str) -> IronResult<Response> {
        let path = self.static_path.join(file_name);
        let mut response = Response::new();
        response.headers.set(mime::file_content_type(&path));
        response.set_mut(status::Ok);
        response.set_mut(BodyReader(itry!(File::open(path))));
        Ok(response)
    }

    /// Handles `GET` requests.
    ///
    /// If a URI segment is not provided then the upload form is rendered, otherwise the first
    /// segment is considered to be a paste ID, and hence the paste is fetched from the DB.
    fn get(&self, req: &mut Request) -> IronResult<Response> {
        match req.url_segment_n(0) {
            None => self.render_template("upload.html", ContentType::html(), &json!({})),
            Some("paste.sh") => self.render_template("paste.sh",
                                                     ContentType::plaintext(),
                                                     &json!({"prefix": &self.url_prefix})),
            Some("readme") => self.render_template("readme.html",
                                                   ContentType::html(),
                                                   &json!({"prefix": &self.url_prefix})),
            Some(file_name) if self.static_path.join(file_name).is_file() => {
                self.serve_static(file_name)
            }
            Some(id) => self.get_paste(id, req.is_browser(), req.url_segment_n(1).is_some()),
        }
    }

    /// Handles `POST` and `PUT` requests.
    fn post(&self, req: &mut Request) -> IronResult<Response> {
        let file_name = req.url_segment_n(0).map(|s| s.to_string());
        debug!("File name: {:?}", file_name);
        let data_length = req.get_length().ok_or(Error::NoContentLength)?;
        if data_length > self.db.max_data_size() as u64 {
            return Err(Error::TooBig.into());
        }
        let data = load_data(&mut req.body, data_length)?;
        let mime_type = mime::data_mime_type(file_name.as_ref(), &data);
        let expires_at = match req.get_arg("expires") {
            Some(Cow::Borrowed("never")) => None,
            Some(x) => {
                Some(DateTime::from_utc(NaiveDateTime::from_timestamp(itry!(x.parse()), 0), Utc))
            }
            _ => Some(Utc::now().add(self.default_ttl)),
        };
        let id = itry!(self.db.store_data(data, file_name, mime_type, expires_at));
        debug!("Generated id: {}", id);
        Ok(Response::with((status::Created,
                          format!("{}{}\n",
                                   self.url_prefix,
                                   encode_id(id)))))
    }

    /// Handles `DELETE` requests.
    fn remove(&self, req: &mut Request) -> IronResult<Response> {
        let id = itry!(decode_id(&req.url_segment_n(0).ok_or(Error::NoIdSegment)?));
        itry!(self.db.remove_data(id));
        Ok(Response::with(status::Ok))
    }
}

impl<E> Handler for Pastebin<E>
    where E: Send + Sync + std::error::Error + 'static
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
