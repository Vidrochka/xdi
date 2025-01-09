# simple-di
Simple dependency manager implementation

```rs
use simple_di::{builder::SimpleDiBuilder, ServiceProvider};

use std::sync::{Arc, Mutex};

use parking_lot::;

pub struct Service1 {
    pub payload: String
}

pub struct Service2 {
    pub payload: String
}

let builder = SimpleDiBuilder::new();

builder.transient(|| Service1 {
    payload: "1".to_string()
});

builder.singletone(|| Arc::new(Mutex::new(Service2 {
    payload: "2".to_string()
})));

builder.build();

let mut service = ServiceProvider::resolve::<Service1>().unwrap();
let mut service2 = ServiceProvider::resolve::<Arc<Mutex<Service2>>>().unwrap();

```