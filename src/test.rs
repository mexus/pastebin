use DbInterface;
use ObjectId;
use PasteEntry;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use data_encoding::BASE64URL_NOPAD;
use iron;
use reqwest::Client;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::sync::{Arc, Mutex};
use web;

#[derive(Clone)]
struct FakeDb {
    storage: Arc<Mutex<HashMap<String, PasteEntry>>>,
}

impl FakeDb {
    fn new() -> Self {
        Self { storage: Arc::new(Mutex::new(HashMap::new())), }
    }

    fn find_data(&self, id: String) -> Option<PasteEntry> {
        self.storage.lock()
            .unwrap()
            .get(&id)
            .map(|data| data.clone())
    }

    fn put_data(&self,
                id: String,
                data: Vec<u8>,
                file_name: Option<String>,
                mime_type: String,
                best_before: Option<DateTime<Utc>>) {
        self.storage.lock().unwrap().insert(id,
                                            PasteEntry { data,
                                                         file_name,
                                                         mime_type,
                                                         best_before, });
    }
}

fn oid_to_str(id: ObjectId) -> String {
    BASE64URL_NOPAD.encode(&id.bytes())
}

#[derive(Debug)]
struct FakeError;
impl error::Error for FakeError {
    fn description(&self) -> &str {
        "nothing happened"
    }
}

impl fmt::Display for FakeError {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

impl DbInterface for FakeDb {
    type Error = FakeError;

    fn store_data(&self,
                  id: ObjectId,
                  data: &[u8],
                  file_name: Option<String>,
                  mime: String,
                  expires_at: Option<DateTime<Utc>>)
                  -> Result<(), Self::Error> {
        self.put_data(oid_to_str(id), data.to_vec(), file_name, mime, expires_at);
        Ok(())
    }

    fn load_data(&self, id: ObjectId) -> Result<Option<PasteEntry>, Self::Error> {
        Ok(self.find_data(oid_to_str(id)))
    }

    fn get_file_name(&self, _id: ObjectId) -> Result<Option<String>, Self::Error> {
        Ok(None)
    }

    fn remove_data(&self, id: ObjectId) -> Result<(), Self::Error> {
        self.storage.lock().unwrap().remove(&oid_to_str(id));
        Ok(())
    }

    fn max_data_size(&self) -> usize {
        15 * 1024 * 1024
    }
}

fn remove_milliseconds(dt: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_utc(NaiveDateTime::from_timestamp(dt.timestamp(), 0), Utc)
}

fn run_web(db: FakeDb, addr: &str, url_prefix: &str) -> iron::Listening {
    web::run_web(db, addr, Default::default(), url_prefix, Duration::zero()).unwrap()
}

#[test]
fn post() {
    const LISTEN_ADDR: &'static str = "127.0.0.1:8000";
    let reference = PasteEntry { data: b"lol".to_vec(),
                                 file_name: None,
                                 mime_type: "text/plain".into(),
                                 best_before: Some(remove_milliseconds(Utc::now())), };
    let connection_addr = &format!("http://{}/?expires={}",
                                   LISTEN_ADDR,
                                   reference.best_before.unwrap().timestamp());
    let url_prefix = "prefix://example.com/";

    let db = FakeDb::new();

    let mut web = run_web(db.clone(), LISTEN_ADDR, url_prefix);

    let mut response = Client::new().post(connection_addr)
                                    .body(reference.data.clone())
                                    .send()
                                    .unwrap();

    web.close().unwrap();

    assert!(response.status().is_success());
    let received_text = response.text().unwrap();
    assert!(received_text.starts_with(url_prefix));
    let (_, received_id) = received_text.split_at(url_prefix.len());
    let db_entry = db.find_data(received_id.trim_right().to_string()).unwrap();
    assert_eq!(db_entry.data, reference.data);
    assert_eq!(db_entry.file_name, reference.file_name);
    assert_eq!(db_entry.mime_type, reference.mime_type);
    assert_eq!(db_entry.best_before, reference.best_before);
}

#[test]
fn get() {
    const LISTEN_ADDR: &'static str = "127.0.0.1:8001";
    let reference_id = "WkP2bzc2My4Voyqk".to_string();
    let connection_addr = &format!("http://{}/{}", LISTEN_ADDR, reference_id);
    let reference_data = "Ahaha";

    let db = FakeDb::new();
    db.put_data(reference_id.clone(),
                reference_data.as_bytes().to_vec(),
                None,
                "text/plain".into(),
                None);

    let mut web = run_web(db.clone(), LISTEN_ADDR, Default::default());

    let mut response = Client::new().get(connection_addr).send().unwrap();

    web.close().unwrap();

    assert!(response.status().is_success());
    let data = response.text().unwrap();
    assert_eq!(reference_data, data);
}

#[test]
fn remove() {
    const LISTEN_ADDR: &'static str = "127.0.0.1:8002";
    let reference_id = "WkP2bzc2My4Voyqk".to_string();
    let connection_addr = &format!("http://{}/{}", LISTEN_ADDR, reference_id);
    let reference_data = "Ahaha";

    let db = FakeDb::new();
    db.put_data(reference_id.clone(),
                reference_data.as_bytes().to_vec(),
                None,
                "text/plain".into(),
                None);

    let mut web = run_web(db.clone(), LISTEN_ADDR, Default::default());
    let response = Client::new().delete(connection_addr).send().unwrap();
    web.close().unwrap();

    assert!(response.status().is_success());
    assert!(db.find_data(reference_id).is_none());
}

#[test]
fn post_never_expire() {
    const LISTEN_ADDR: &'static str = "127.0.0.1:8003";
    let reference = PasteEntry { data: b"lol".to_vec(),
                                 file_name: None,
                                 mime_type: "text/plain".into(),
                                 best_before: None, };
    let connection_addr = &format!("http://{}/?expires=never", LISTEN_ADDR,);
    let url_prefix = "prefix://example.com/";

    let db = FakeDb::new();

    let mut web = run_web(db.clone(), LISTEN_ADDR, url_prefix);

    let mut response = Client::new().post(connection_addr)
                                    .body(reference.data.clone())
                                    .send()
                                    .unwrap();

    web.close().unwrap();

    assert!(response.status().is_success());
    let received_text = response.text().unwrap();
    assert!(received_text.starts_with(url_prefix));
    let (_, received_id) = received_text.split_at(url_prefix.len());
    let db_entry = db.find_data(received_id.trim_right().to_string()).unwrap();
    assert_eq!(db_entry.data, reference.data);
    assert_eq!(db_entry.file_name, reference.file_name);
    assert_eq!(db_entry.mime_type, reference.mime_type);
    assert_eq!(db_entry.best_before, reference.best_before);
}
