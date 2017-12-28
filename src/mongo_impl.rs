//! MongoDB wrapper that implements `DbInterface`.

use bson::{self, Bson};
use bson::oid::ObjectId;
use mongo_driver::MongoError;
use mongo_driver::client::ClientPool;
use mongo_driver::collection::{Collection, FindAndModifyOperation};
use pastebin::DbInterface;
use std::sync::Arc;

/// A MongoDB wrapper.
pub struct MongoDbWrapper {
    db_name: String,
    collection_name: String,
    client_pool: Arc<ClientPool>,
}

impl MongoDbWrapper {
    /// Constructs a new mongodb wrapper.
    pub fn new(db_name: String, collection_name: String, client_pool: ClientPool) -> Self {
        Self { db_name,
               collection_name,
               client_pool: Arc::new(client_pool), }
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

impl DbInterface for MongoDbWrapper {
    type Error = MongoError;

    fn store_data(&self, id: ObjectId, data: &[u8]) -> Result<(), Self::Error> {
        let bson_data = binary_to_bson(data);
        let collection = self.get_collection();
        let new_doc = doc!("_id": id, "data": bson_data);
        collection.insert(&new_doc, None)
    }

    fn load_data(&self, id: ObjectId) -> Result<Option<Vec<u8>>, Self::Error> {
        debug!("Looking for a doc id = {:?}", id);
        let filter = doc!("_id": id);
        let collection = self.get_collection();
        let data = collection.find(&filter, None)?
                             .nth(0)
                             .and_then(|doc| doc.ok())
                             .and_then(|doc| doc.get("data").cloned())
                             .map(|data| binary_from_bson(data));
        if let Some(res) = data {
            res.map(|data| Some(data)).map_err(Into::into)
        } else {
            Ok(None)
        }
    }

    fn remove_data(&self, id: ObjectId) -> Result<(), Self::Error> {
        debug!("Looking for a doc id = {:?}", id);
        let collection = self.get_collection();
        collection.find_and_modify(&doc!("_id": id), FindAndModifyOperation::Remove, None)?;
        Ok(())
    }

    fn max_data_size(&self) -> usize {
        15 * 1024 * 1024
    }
}
