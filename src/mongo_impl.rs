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

/// A helper type to encode/decode a BSON database entry.
struct DbEntry {
    id: ObjectId,
    data: Vec<u8>,
    mime_type: String,
}

impl DbEntry {
    /// Convert the entry into a BSON document.
    fn to_bson(self) -> bson::Document {
        doc!{
            "_id": self.id,
            "data": Bson::Binary(bson::spec::BinarySubtype::Generic, self.data),
            "mime_type": self.mime_type,
        }
    }

    /// Try to parse a BSON.
    fn from_bson(doc: bson::Document) -> Result<Self, bson::DecoderError> {
        let mut id = None;
        let mut data = None;
        let mut mime_type = None;
        let wrong_type = |field, val: bson::Bson, expected| {
            let msg = format!("Field `{}`, expected type {}, got {:?}",
                              field,
                              expected,
                              val.element_type());
            Err(bson::DecoderError::InvalidType(msg))
        };
        for (key, bson_value) in doc {
            match (key.as_str(), bson_value) {
                ("_id", bson::Bson::ObjectId(oid)) => {
                    id = Some(oid);
                }
                ("_id", val) => {
                    return wrong_type("_id", val, "object id");
                }
                ("data", bson::Bson::Binary(bson::spec::BinarySubtype::Generic, bin_data)) => {
                    data = Some(bin_data);
                }
                ("data", val) => {
                    return wrong_type("data", val, "generic binary");
                }
                ("mime_type", bson::Bson::String(mime)) => mime_type = Some(mime),
                ("mime_type", val) => {
                    return wrong_type("mime_type", val, "string");
                }
                _ => return Err(bson::DecoderError::UnknownField(key)),
            }
        }
        Ok(DbEntry { id: id.ok_or(bson::DecoderError::ExpectedField("_id"))?,
                     data: data.ok_or(bson::DecoderError::ExpectedField("data"))?,
                     mime_type: mime_type.ok_or(bson::DecoderError::ExpectedField("mime_type"))?, })
    }
}

impl DbInterface for MongoDbWrapper {
    type Error = MongoError;

    fn store_data(&self, id: ObjectId, data: &[u8], mime_type: String) -> Result<(), Self::Error> {
        let collection = self.get_collection();
        collection.insert(&DbEntry { id,
                                     data: data.to_vec(),
                                     mime_type, }.to_bson(),
                          None)
    }

    fn load_data(&self, id: ObjectId) -> Result<Option<(Vec<u8>, String)>, Self::Error> {
        debug!("Looking for a doc id = {:?}", id);
        let filter = doc!("_id": id);
        let collection = self.get_collection();
        let entry = match collection.find(&filter, None)?
                                    .nth(0)
                                    .and_then(|doc| doc.ok())
        {
            None => return Ok(None),
            Some(entry) => entry,
        };
        let db_entry = DbEntry::from_bson(entry)?;
        Ok(Some((db_entry.data, db_entry.mime_type)))
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
