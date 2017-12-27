use HttpResult;
use MongoDbConnector;
use ObjectId;
use bson;
use data_encoding::{self, BASE64URL_NOPAD};
use iron;
use iron::Handler;
use iron::method::Method;
use iron::prelude::*;
use iron::status;
use mongo_driver;
use std::convert::From;
use std::io::{self, Read};

quick_error!{
    /// Errors descriptions.
    #[derive(Debug)]
    pub enum Error {
        // Input/output error.
        Io(err: io::Error) {
            from()
            cause(err)
        }
        Mongo(err: mongo_driver::MongoError) {
            from()
            cause(err)
        }
        Encoding(err: data_encoding::DecodeError) {
            from()
            cause(err)
        }
        TooBig {
            description("Too large paste")
        }
        BsonObjId(err: bson::oid::Error) {
            from()
            cause(err)
        }
        BsonIdWrongLength(len: usize) {
            description("Wrong ID length")
            display("Expected an ID to have length of 12, but it is {}", len)
        }
        NoIdSegment {
            description("ID segment not found in the URL")
        }
        IdNotFound(id: ObjectId) {
            description("ID not found")
            display("Id {} not found", id)
        }
    }
}

impl From<Error> for IronError {
    fn from(err: Error) -> IronError {
        match err {
            e @ Error::IdNotFound(_) => IronError::new(e, status::BadRequest),
            e => IronError::new(e, status::BadRequest),
        }
    }
}

struct Pastebin {
    db: Box<MongoDbConnector>,
}

fn id_from_request(req: &Request) -> Result<ObjectId, Error> {
    req.url.as_ref()
       .path_segments()
       .and_then(|mut it| it.next())
       .ok_or(Error::NoIdSegment)
       .and_then(id_from_string)
}

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

impl Pastebin {
    fn new(db: Box<MongoDbConnector>) -> Pastebin {
        Pastebin { db }
    }

    fn get(&self, req: &mut Request) -> IronResult<Response> {
        let id = id_from_request(req)?;
        let data = self.db.connect()
                       .load_data(id.clone())
                       .map_err(Into::<Error>::into)?
                       .ok_or(Error::IdNotFound(id))?;
        Ok(Response::with((status::Ok, data)))
    }

    fn post(&self, req: &mut Request) -> IronResult<Response> {
        let connection = self.db.connect();
        let data = match load_data(&mut req.body, connection.max_data_size()) {
            Ok(data) => data,
            Err(Error::TooBig) => return Ok(Response::with(status::PayloadTooLarge)),
            Err(e) => panic!{"Error {:?}", e},
        };
        let id = bson::oid::ObjectId::new().map_err(Into::<Error>::into)?;
        connection.store_data(id.clone(), &data)
                  .map_err(Into::<Error>::into)?;
        Ok(Response::with((status::Ok, BASE64URL_NOPAD.encode(&id.bytes()))))
    }

    fn remove(&self, req: &mut Request) -> IronResult<Response> {
        let id = id_from_request(req)?;
        self.db.connect()
            .remove_data(id)
            .map_err(Into::<Error>::into)?;
        Ok(Response::with(status::Ok))
    }
}

impl Handler for Pastebin {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.method {
            Method::Get => self.get(req),
            Method::Post => self.post(req),
            Method::Delete => self.remove(req),
            _ => Ok(Response::with(status::MethodNotAllowed)),
        }
    }
}

fn load_data<R: Read>(stream: &mut R, limit: usize) -> Result<Vec<u8>, Error> {
    let mut result = Vec::with_capacity(limit);
    loop {
        let mut buffer: Vec<_> = vec![0u8; 512];
        let size = stream.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        if result.len() + size > limit {
            return Err(Error::TooBig);
        }
        buffer.resize(size, 0u8);
        result.append(&mut buffer);
    }
    Ok(result)
}

pub fn run_web(db_wrapper: Box<MongoDbConnector>) -> HttpResult<iron::Listening> {
    let pastebin = Pastebin::new(db_wrapper);
    Iron::new(pastebin).http("localhost:8000")
}
