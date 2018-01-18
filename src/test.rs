use DbInterface;
use ObjectId;
use data_encoding::BASE64URL_NOPAD;
use reqwest::Client;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::sync::{Arc, Mutex};
use web::run_web;

#[derive(Clone)]
struct FakeDb {
    storage: Arc<Mutex<HashMap<String, (Vec<u8>, Option<String>, String)>>>,
}

impl FakeDb {
    fn new() -> Self {
        Self { storage: Arc::new(Mutex::new(HashMap::new())), }
    }

    fn find_data(&self, id: String) -> Option<(Vec<u8>, Option<String>, String)> {
        self.storage.lock()
            .unwrap()
            .get(&id)
            .map(|data| data.clone())
    }

    fn put_data(&self, id: String, data: Vec<u8>, file_name: Option<String>, mime: String) {
        self.storage.lock()
            .unwrap()
            .insert(id, (data, file_name, mime));
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
                  mime: String)
                  -> Result<(), Self::Error> {
        self.put_data(oid_to_str(id), data.to_vec(), file_name, mime);
        Ok(())
    }

    fn load_data(&self,
                 id: ObjectId)
                 -> Result<Option<(Vec<u8>, Option<String>, String)>, Self::Error> {
        Ok(self.find_data(oid_to_str(id)))
    }

    fn remove_data(&self, id: ObjectId) -> Result<(), Self::Error> {
        self.storage.lock().unwrap().remove(&oid_to_str(id));
        Ok(())
    }

    fn max_data_size(&self) -> usize {
        15 * 1024 * 1024
    }
}

#[test]
fn post() {
    const LISTEN_ADDR: &'static str = "127.0.0.1:8000";
    let connection_addr = &format!("http://{}/", LISTEN_ADDR);
    let reference_data = "lol";
    let reference_mime = "text/plain";
    let url_prefix = "prefix://";

    let db = FakeDb::new();

    let mut web = run_web(db.clone(),
                          LISTEN_ADDR, Default::default(),
                          url_prefix.to_string()).unwrap();

    let mut response = Client::new().post(connection_addr)
                                    .body(reference_data)
                                    .send()
                                    .unwrap();

    web.close().unwrap();

    assert!(response.status().is_success());
    let received_text = response.text().unwrap();
    assert!(received_text.starts_with(url_prefix));
    let (_, received_id) = received_text.split_at(url_prefix.len());
    assert_eq!(db.find_data(received_id.trim_right().to_string()).as_ref()
                 .map(|&(ref v, ref file_name, ref mime)| (v as &[u8], file_name, mime)),
               Some((reference_data.as_bytes(), &None, &reference_mime.to_string())));
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
                "text/plain".into());

    let mut web = run_web(db.clone(), LISTEN_ADDR, Default::default(), Default::default()).unwrap();

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
                "text/plain".into());

    let mut web = run_web(db.clone(), LISTEN_ADDR, Default::default(), Default::default()).unwrap();
    let response = Client::new().delete(connection_addr).send().unwrap();
    web.close().unwrap();

    assert!(response.status().is_success());
    assert!(db.find_data(reference_id).is_none());
}
