use DbOptions;
use MongoDbConnector;
use MongoDbInterface;
use ObjectId;
use bson::{self, Bson};
use mongo_driver::MongoError;
use mongo_driver::client::ClientPool;
use mongo_driver::collection::{Collection, FindAndModifyOperation};
use std::sync::Arc;

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
        let client_pool = Arc::new(ClientPool::new(options.uri.clone(), None));
        MongoDbWrapper { options,
                         client_pool, }
    }
}

impl MongoDbConnector for MongoDbWrapper {
    fn connect(&self) -> Box<MongoDbInterface> {
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
    }

    fn get_collection(&self) -> Collection {
        self.client_pool.pop()
            .take_collection(self.db_name.clone(), self.collection_name.clone())
    }
}

fn binary_to_bson(data: &[u8]) -> Bson {
    Bson::Binary(bson::spec::BinarySubtype::Generic, data.to_vec())
}

fn binary_from_bson(data: Bson) -> Result<Vec<u8>, bson::DecoderError> {
    use bson::DecoderError;
    use bson::spec::BinarySubtype;
    match data {
        Bson::Binary(BinarySubtype::Generic, x) => Ok(x),
        x => {
            let msg = format!("Should be generic binary, but: {:?}", x.element_type());
            Err(DecoderError::InvalidType(msg))
        }
    }
}

impl MongoDbInterface for MongoDbConnectionWrapper {
    fn store_data(&self, id: ObjectId, data: &[u8]) -> Result<(), MongoError> {
        let bson_data = binary_to_bson(data);
        let collection = self.get_collection();
        let new_doc = doc!("_id": id, "data": bson_data);
        collection.insert(&new_doc, None)?;
        Ok(())
    }

    fn load_data(&self, id: ObjectId) -> Result<Option<Vec<u8>>, MongoError> {
        debug!("Looking for a doc id = {:?}", id);
        let filter = doc!("_id": id);
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

    fn remove_data(&self, id: ObjectId) -> Result<(), MongoError> {
        debug!("Looking for a doc id = {:?}", id);
        let collection = self.get_collection();
        collection.find_and_modify(&doc!("id": id), FindAndModifyOperation::Remove, None)?;
        Ok(())
    }

    fn max_data_size(&self) -> usize {
        15 * 1024 * 1024
    }
}
