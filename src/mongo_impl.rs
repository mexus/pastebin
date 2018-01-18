//! MongoDB wrapper that implements `DbInterface`.

use bson::{self, Bson};
use bson::oid::ObjectId;
use chrono::{DateTime, Utc};
use mongo_driver::{CommandAndFindOptions, MongoError};
use mongo_driver::client::ClientPool;
use mongo_driver::collection::{Collection, FindAndModifyOperation};
use pastebin::{DbInterface, PasteEntry};
use std::convert::From;
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
    file_name: Option<String>,
    mime_type: String,
    best_before: Option<DateTime<Utc>>,
}

impl From<DbEntry> for bson::Document {
    fn from(entry: DbEntry) -> bson::Document {
        let mut doc = doc!{
            "_id": entry.id,
            "data": Bson::Binary(bson::spec::BinarySubtype::Generic, entry.data),
            "mime_type": entry.mime_type,
        };
        if let Some(file_name) = entry.file_name {
            doc.insert("file_name", file_name);
        }
        doc
    }
}

impl From<DbEntry> for PasteEntry {
    fn from(entry: DbEntry) -> PasteEntry {
        PasteEntry { data: entry.data,
                     file_name: entry.file_name,
                     mime_type: entry.mime_type,
                     best_before: entry.best_before, }
    }
}

impl DbEntry {
    /// Try to parse a BSON.
    fn from_bson(doc: bson::Document) -> Result<Self, bson::DecoderError> {
        let mut id = None;
        let mut data = None;
        let mut file_name = None;
        let mut mime_type = None;
        let mut best_before = None;
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
                ("file_name", bson::Bson::String(fname)) => file_name = Some(fname),
                ("file_name", val) => {
                    return wrong_type("file_name", val, "string");
                }
                ("best_before", bson::Bson::UtcDatetime(date)) => best_before = Some(date),
                ("best_before", val) => {
                    return wrong_type("best_before", val, "UtcDatetime");
                }
                _ => return Err(bson::DecoderError::UnknownField(key)),
            }
        }
        Ok(DbEntry { id: id.ok_or(bson::DecoderError::ExpectedField("_id"))?,
                     data: data.ok_or(bson::DecoderError::ExpectedField("data"))?,
                     file_name,
                     mime_type: mime_type.ok_or(bson::DecoderError::ExpectedField("mime_type"))?,
                     best_before, })
    }
}

/// Try to parse a BSON to extract only the file name (if any).
fn filename_from_bson(doc: bson::Document) -> Result<Option<String>, bson::DecoderError> {
    let mut file_name = None;
    let wrong_type = |field, val: bson::Bson, expected| {
        let msg = format!("Field `{}`, expected type {}, got {:?}",
                          field,
                          expected,
                          val.element_type());
        Err(bson::DecoderError::InvalidType(msg))
    };
    for (key, bson_value) in doc {
        match (key.as_str(), bson_value) {
            ("file_name", bson::Bson::String(fname)) => file_name = Some(fname),
            ("file_name", val) => {
                return wrong_type("file_name", val, "string");
            }
            _ => {}
        }
    }
    Ok(file_name)
}

impl DbInterface for MongoDbWrapper {
    type Error = MongoError;

    fn store_data(&self,
                  id: ObjectId,
                  data: &[u8],
                  file_name: Option<String>,
                  mime_type: String,
                  best_before: Option<DateTime<Utc>>)
                  -> Result<(), Self::Error> {
        let collection = self.get_collection();
        collection.insert(&DbEntry { id,
                                     data: data.to_vec(),
                                     file_name,
                                     mime_type,
                                     best_before, }.into(),
                          None)
    }

    fn load_data(&self, id: ObjectId) -> Result<Option<PasteEntry>, Self::Error> {
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
        Ok(Some(db_entry.into()))
    }

    fn get_file_name(&self, id: ObjectId) -> Result<Option<String>, Self::Error> {
        debug!("Looking for a file name for id = {:?}", id);
        let filter = doc!("_id": id);
        let collection = self.get_collection();
        let find_options = CommandAndFindOptions::with_fields(doc!("_id": 0, "file_name": 1));
        let entry = match collection.find(&filter, Some(&find_options))?
                                    .nth(0)
                                    .and_then(|doc| doc.ok())
        {
            None => return Ok(None),
            Some(entry) => entry,
        };
        Ok(filename_from_bson(entry)?)
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
