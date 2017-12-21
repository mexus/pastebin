use MongoDbConnector;
use ObjectId;
use bson;
use data_encoding::{self, BASE64URL_NOPAD};
use mongo_driver;
use rocket;
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
    }
}

fn load_data(msg: rocket::Data, limit: usize) -> Result<Vec<u8>, Error> {
    let mut result = Vec::with_capacity(limit);
    let mut stream = msg.open();
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

#[post("/", data = "<msg>")]
fn push(msg: rocket::Data,
        db_wrapper: rocket::State<Box<MongoDbConnector>>)
        -> Result<String, Error> {
    let db = db_wrapper.connect();
    let data = load_data(msg, db.max_data_size())?;
    let id = ObjectId::new()?;
    db.store_data(id.clone(), &data)?;
    Ok(BASE64URL_NOPAD.encode(&id.bytes()))
}

fn id_from_string(src: String) -> Result<ObjectId, Error> {
    let dyn_bytes = BASE64URL_NOPAD.decode(src.as_bytes())?;
    if dyn_bytes.len() != 12 {
        return Err(Error::BsonIdWrongLength(src.len()));
    }
    let mut bytes = [0u8; 12];
    for i in 0..12usize {
        bytes[i] = dyn_bytes[i];
    }
    Ok(ObjectId::with_bytes(bytes))
}

#[get("/<id>")]
fn get(id: String,
       db_wrapper: rocket::State<Box<MongoDbConnector>>)
       -> Result<Option<Vec<u8>>, Error> {
    let db = db_wrapper.connect();
    let id = id_from_string(id)?;
    Ok(db.load_data(id)?)
}

#[delete("/<id>")]
fn remove(id: String, db_wrapper: rocket::State<Box<MongoDbConnector>>) -> Result<(), Error> {
    let db = db_wrapper.connect();
    let id = id_from_string(id)?;
    db.remove_data(id)?;
    Ok(())
}

pub fn run_web(db_wrapper: Box<MongoDbConnector>) -> rocket::error::LaunchError {
    rocket::ignite().mount("/", routes![push, get, remove])
                    .manage(db_wrapper)
                    .launch()
}
