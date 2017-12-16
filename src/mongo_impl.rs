use DbOptions;
use MongoDbConnector;
use MongoDbInterface;
use bson;
// use mongodb;
// use mongodb::ThreadedClient;
// use mongodb::coll::options::FindOneAndUpdateOptions;
// use mongodb::db::ThreadedDatabase;

use mongo_driver::MongoError;
use mongo_driver::client::{ClientPool, Uri};
use mongo_driver::collection::{Collection, FindAndModifyOperation};
use std::sync::Arc;

use rand;
use rand::Rng;

/// A MongoDB wrapper that produces `MongoDbConnectionWrapper`s.
pub struct MongoDbConnectionWrapper {
    db_name: String,
    collection_name: String,
    client_pool: Arc<ClientPool>,
}

/// A MongoDB client poll wrapper.
/// This class initializes and holds a reference for a client pool, and it uses it to create
/// `MongoDbConnectionWrapper` instances.
#[derive(Debug)]
pub struct MongoDbWrapper {
    options: DbOptions,
    client_pool: Arc<ClientPool>,
}

impl MongoDbWrapper {
    /// Creates a new connections producer with a given options.
    pub fn new(options: DbOptions) -> Self {
        let addr = match (options.db_user.as_ref(), options.db_pass.as_ref()) {
            (Some(user), Some(pass)) => {
                format!("mongodb://{}:{}@{}:{}/{}",
                        user, pass, options.host, options.port, options.db_name)
            }
            _ => format!("mongodb://{}:{}/{}",
                         options.host, options.port, options.db_name),
        };
        let client_pool = Arc::new(ClientPool::new(Uri::new(addr).expect("Expected a URI"), None));
        MongoDbWrapper { options,
                         client_pool, }
    }
}

impl MongoDbConnector for MongoDbWrapper {
    fn connect(&self) -> Box<MongoDbInterface> {
        // let client = self.client_pool.pop();
        // let collection = client.take_collection(self.options.db_name.clone(),
        //                                         self.options.collection_name.clone());
        Box::new(MongoDbConnectionWrapper::new(self.options.db_name.clone(),
                                               self.options.collection_name.clone(),
                                               self.client_pool.clone()))
    }
}

impl MongoDbConnectionWrapper {
    /// Constructs a new mongodb wrapper.
    fn new(db_name: String, collection_name: String, client_pool: Arc<ClientPool>) -> Self {
        Self { db_name,
               collection_name,
               client_pool, }

        // let client = pool.pop();
        // let collection = client.take_collection(self.options.db_name.clone(),
        //                                         self.options.collection_name.clone());
        // Self { collection }
    }

    fn get_collection(&self) -> Collection {
        self.client_pool.pop()
            .take_collection(self.db_name.clone(), self.collection_name.clone())
    }
}

/// Generates a random ID and finds out whether it is unique or not.
fn try_to_store_unique_id(collection: &Collection,
                          bson_data: bson::Bson)
                          -> Result<Option<[u8; 4]>, MongoError> {
    let id = {
        let mut id = [0u8; 4];
        rand::thread_rng().fill_bytes(&mut id);
        id
    };
    let bson_id = binary_to_bson(&id);
    let filter = doc! {"id": bson_id.clone()};
    let new_doc = doc! { "$setOnInsert" => { "id": bson_id, "data": bson_data.clone() } };
    debug!("Trying ID {:?}", id);
    let result_doc: bson::Document =
        collection.find_and_modify(&filter, FindAndModifyOperation::Upsert(&new_doc), None)?;
    let val: &bson::Bson = match result_doc.get("value") {
        None => {
            error!("Can't find a 'value' field in a server's response: {:?}",
                   result_doc);
            return Ok(None);
        }
        Some(x) => x,
    };
    Ok(match val {
        &bson::Bson::Null => Some(id),
        _ => {
            info!("ID {:?} is already in use", id);
            None
        }
    })
}

fn binary_to_bson(data: &[u8]) -> bson::Bson {
    bson::Bson::Binary(bson::spec::BinarySubtype::Generic, data.to_vec())
}

fn binary_from_bson(data: bson::Bson) -> Result<Vec<u8>, bson::DecoderError> {
    match data {
        bson::Bson::Binary(bson::spec::BinarySubtype::Generic, x) => Ok(x),
        x => {
            Err(bson::DecoderError::InvalidType(format!("Should be generic binary, but: {:?}",
                                                        x.element_type())))
        }
    }
}

impl MongoDbInterface for MongoDbConnectionWrapper {
    fn store_data(&self, data: &[u8]) -> Result<[u8; 4], MongoError> {
        let bson_data = binary_to_bson(data);
        // let mut opts = FindAndModifyOptions::default();
        // opts.new
        // opts.upsert = Some(true);
        let collection = self.get_collection();
        loop {
            if let Some(id) = try_to_store_unique_id(&collection, bson_data.clone())? {
                debug!("Inserted data to id = {:?}", id);
                return Ok(id);
            }
        }
    }

    fn load_data(&self, id: &[u8]) -> Result<Option<Vec<u8>>, MongoError> {
        debug!("Looking for a doc id = {:?}", id);
        let bson_id = binary_to_bson(id);
        let filter = doc!{"id": bson_id};
        let collection = self.get_collection();
        let doc = collection.find(&filter, None)?
                            .nth(0)
                            .and_then(|doc| doc.ok())
                            .and_then(|doc| doc.get("data").cloned());
        Ok(match doc {
            None => None,
            Some(data) => Some(binary_from_bson(data)?),
        })
    }

    fn remove_data(&self, id: &[u8]) -> Result<(), MongoError> {
        debug!("Looking for a doc id = {:?}", id);
        let bson_id = binary_to_bson(id);
        let collection = self.get_collection();
        collection.find_and_modify(&doc!{"id": bson_id}, FindAndModifyOperation::Remove, None)?;
        Ok(())
    }

    fn max_data_size(&self) -> usize {
        15 * 1024 * 1024
    }
}
