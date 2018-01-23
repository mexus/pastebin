use DbInterface;
use PasteEntry;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use id::{decode_id, encode_id};
use iron;
use reqwest::Client;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use web;

#[derive(Clone)]
struct FakeDb {
    storage: Arc<Mutex<HashMap<u64, PasteEntry>>>,
}

impl FakeDb {
    fn new() -> Self {
        Self { storage: Arc::new(Mutex::new(HashMap::new())), }
    }

    fn find_data(&self, id: u64) -> Option<PasteEntry> {
        self.storage.lock()
            .unwrap()
            .get(&id)
            .map(|data| data.clone())
    }

    fn put_data(&self,
                data: Vec<u8>,
                file_name: Option<String>,
                mime_type: String,
                best_before: Option<DateTime<Utc>>)
                -> u64 {
        static COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        let id = COUNTER.fetch_add(1, Ordering::SeqCst) as u64;
        self.storage.lock().unwrap().insert(id,
                                            PasteEntry { data,
                                                         file_name,
                                                         mime_type,
                                                         best_before, });
        id
    }
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
                  data: &[u8],
                  file_name: Option<String>,
                  mime: String,
                  expires_at: Option<DateTime<Utc>>)
                  -> Result<u64, Self::Error> {
        let id = self.put_data(data.to_vec(), file_name, mime, expires_at);
        Ok(id)
    }

    fn load_data(&self, id: u64) -> Result<Option<PasteEntry>, Self::Error> {
        Ok(self.find_data(id))
    }

    fn get_file_name(&self, _id: u64) -> Result<Option<String>, Self::Error> {
        Ok(None)
    }

    fn remove_data(&self, id: u64) -> Result<(), Self::Error> {
        self.storage.lock().unwrap().remove(&id);
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
    web::run_web(db,
                 addr,
                 Default::default(),
                 url_prefix,
                 Duration::zero(),
                 Default::default()).unwrap()
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
    let id = decode_id(received_id.trim()).unwrap();

    let db_entry = db.find_data(id).unwrap();
    assert_eq!(db_entry.data, reference.data);
    assert_eq!(db_entry.file_name, reference.file_name);
    assert_eq!(db_entry.mime_type, reference.mime_type);
    assert_eq!(db_entry.best_before, reference.best_before);
}

#[test]
fn get() {
    const LISTEN_ADDR: &'static str = "127.0.0.1:8001";
    let reference_data = "Ahaha";

    let db = FakeDb::new();
    let id = db.put_data(reference_data.as_bytes().to_vec(),
                         None,
                         "text/plain".into(),
                         None);

    let mut web = run_web(db.clone(), LISTEN_ADDR, Default::default());

    let connection_addr = &format!("http://{}/{}", LISTEN_ADDR, encode_id(id));
    let mut response = Client::new().get(connection_addr).send().unwrap();

    web.close().unwrap();

    assert!(response.status().is_success(), "{:?}", response);
    let data = response.text().unwrap();
    assert_eq!(reference_data, data);
}

#[test]
fn remove() {
    const LISTEN_ADDR: &'static str = "127.0.0.1:8002";
    let reference_data = "Ahaha";

    let db = FakeDb::new();
    let id = db.put_data(reference_data.as_bytes().to_vec(),
                         None,
                         "text/plain".into(),
                         None);

    let mut web = run_web(db.clone(), LISTEN_ADDR, Default::default());

    let connection_addr = &format!("http://{}/{}", LISTEN_ADDR, encode_id(id));
    let response = Client::new().delete(connection_addr).send().unwrap();
    web.close().unwrap();

    assert!(response.status().is_success(), "{:?}", response);
    assert!(db.find_data(id).is_none());
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
    let id = decode_id(received_id.trim()).unwrap();

    let db_entry = db.find_data(id).unwrap();
    assert_eq!(db_entry.data, reference.data);
    assert_eq!(db_entry.file_name, reference.file_name);
    assert_eq!(db_entry.mime_type, reference.mime_type);
    assert_eq!(db_entry.best_before, reference.best_before);
}
