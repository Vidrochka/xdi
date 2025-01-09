use std::sync::{Arc, Mutex};

use crate::{builder::SimpleDiBuilder, ServiceProvider};

pub struct Service1 {
    pub payload: String
}

#[test]
pub fn set_get_singletone_ok() {
    let builder = SimpleDiBuilder::new();

    builder.singletone(|| Arc::new(Mutex::new(Service1 {
        payload: "1".to_string()
    })));

    builder.build();

    let service = ServiceProvider::resolve::<Arc<Mutex<Service1>>>().unwrap();

    assert_eq!(service.lock().unwrap().payload, "1");

    service.lock().unwrap().payload = "2".to_string();

    let service = ServiceProvider::resolve::<Arc<Mutex<Service1>>>().unwrap();

    assert_eq!(service.lock().unwrap().payload, "2");
} 