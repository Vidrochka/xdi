use std::{rc::Rc, sync::Mutex};

use crate::builder::SimpleDiBuilder;

pub struct Service1 {
    pub payload: String
}

pub struct ServiceNotSend1 {
    pub payload: Rc<Mutex<String>>
}

pub struct Service2 {
    pub service1: Service1,
    pub service2: ServiceNotSend1,
}

#[test]
pub fn set_get_transient_ok() {
    let builder = SimpleDiBuilder::new();

    builder.transient(|_| Service1 {
        payload: "1".to_string()
    });

    let sp = builder.build();

    let mut service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");

    service.payload = "2".to_string();

    let service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");
}

#[test]
pub fn set_get_not_send_transient_ok() {
    let builder = SimpleDiBuilder::new();

    builder.transient(|_| ServiceNotSend1 {
        payload: Rc::new(Mutex::new("1".to_string()))
    });

    let sp = builder.build();

    let service = sp.resolve::<ServiceNotSend1>().unwrap();

    assert_eq!(*service.payload.lock().unwrap(), "1");

    *service.payload.lock().unwrap() = "2".to_string();

    let service = sp.resolve::<ServiceNotSend1>().unwrap();

    assert_eq!(*service.payload.lock().unwrap(), "1");
}

#[test]
pub fn set_get_nested_transient_ok() {
    let builder = SimpleDiBuilder::new();

    builder.transient(|_| Service1 {
        payload: "1".to_string()
    });

    builder.transient(|_| ServiceNotSend1 {
        payload: Rc::new(Mutex::new("2".to_string()))
    });

    builder.transient(|sp| Service2 {
        service1: sp.resolve().unwrap(),
        service2: sp.resolve().unwrap(),
    });

    let sp = builder.build();

    let service = sp.resolve::<Service2>().unwrap();

    assert_eq!(service.service1.payload, "1");
    assert_eq!(*service.service2.payload.lock().unwrap(), "2");
}