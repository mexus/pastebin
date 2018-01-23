//! MongoDB wrapper that implements `DbInterface`.

use bson::{self, Bson};
use chrono::{DateTime, Utc};
use mongo_driver::{CommandAndFindOptions, MongoError};
use mongo_driver::client::ClientPool;
use mongo_driver::collection::{Collection, FindAndModifyOperation, FindAndModifyOptions};
use mongo_driver::database::Database;
use pastebin::{DbInterface, PasteEntry};
use std::convert::From;
use std::sync::Arc;

/// A MongoDB wrapper.
pub struct MongoDbWrapper {
    db_name: String,
    collection_name: String,
    ids_collection_name: String,
    client_pool: Arc<ClientPool>,
}

impl MongoDbWrapper {
    /// Constructs a new mongodb wrapper.
    pub fn new(db_name: String,
               collection_name: String,
               ids_collection_name: String,
               client_pool: ClientPool)
               -> Self {
        Self { db_name,
               collection_name,
               ids_collection_name,
               client_pool: Arc::new(client_pool), }
    }

    fn get_db(&self) -> Database {
        self.client_pool.pop().take_database(self.db_name.clone())
    }

    fn get_collection(&self) -> Collection {
        self.client_pool.pop()
            .take_collection(self.db_name.clone(), self.collection_name.clone())
    }

    fn get_new_id(&self, db: &Database) -> Result<u64, MongoError> {
        let ids = db.get_collection(self.ids_collection_name.clone());
        let opts = {
            let mut opts = FindAndModifyOptions::default();
            opts.new = true;
            opts
        };

        let result =
            ids.find_and_modify(&doc!("_id": "paste"),
                                 FindAndModifyOperation::Upsert(&doc!("$inc": { "counter": 1i64 })),
                                 Some(&opts))?;
        let entry = result.get_document("value")?;
        Ok(entry.get_i64("counter")? as u64)
    }
}

/// A helper type to encode/decode a BSON database entry.
struct DbEntry {
    id: u64,
    data: Vec<u8>,
    file_name: Option<String>,
    mime_type: String,
    best_before: Option<DateTime<Utc>>,
}

fn bson_binary(data: Vec<u8>) -> Bson {
    Bson::Binary(bson::spec::BinarySubtype::Generic, data)
}

impl From<DbEntry> for bson::Document {
    fn from(entry: DbEntry) -> bson::Document {
        let mut doc = doc!{
            "_id": entry.id as i64,
            "data": bson_binary(entry.data),
            "mime_type": entry.mime_type,
        };
        if let Some(file_name) = entry.file_name {
            doc.insert("file_name", file_name);
        }
        if let Some(best_before) = entry.best_before {
            doc.insert("best_before", best_before);
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
                ("_id", bson::Bson::I64(signed)) => {
                    id = Some(signed as u64);
                }
                ("_id", val) => {
                    return wrong_type("_id", val, "i64");
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
                  data: &[u8],
                  file_name: Option<String>,
                  mime_type: String,
                  best_before: Option<DateTime<Utc>>)
                  -> Result<u64, Self::Error> {
        let db = self.get_db();
        let id = self.get_new_id(&db)?;
        let collection = db.get_collection(self.collection_name.clone());
        collection.insert(&DbEntry { id,
                                      data: data.to_vec(),
                                      file_name,
                                      mime_type,
                                      best_before, }.into(),
                           None)?;
        Ok(id)
    }

    fn load_data(&self, id: u64) -> Result<Option<PasteEntry>, Self::Error> {
        debug!("Looking for a doc id = {:?}", id);
        let filter = doc!("_id": id as u64);
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

    fn get_file_name(&self, id: u64) -> Result<Option<String>, Self::Error> {
        debug!("Looking for a file name for id = {:?}", id as u64);
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

    fn remove_data(&self, id: u64) -> Result<(), Self::Error> {
        debug!("Looking for a doc id = {:?}", id);
        let collection = self.get_collection();
        collection.find_and_modify(&doc!("_id": id as u64),
                                    FindAndModifyOperation::Remove,
                                    None)?;
        Ok(())
    }

    fn max_data_size(&self) -> usize {
        15 * 1024 * 1024
    }
}
