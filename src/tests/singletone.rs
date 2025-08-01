use std::sync::{Arc, Mutex};

use crate::{ServiceProvider, builder::DiBuilder, types::error::ServiceBuildResult};

pub struct Service1 {
    pub payload: String,
}

#[test]
pub fn set_get_singletone_ok() {
    let builder = DiBuilder::new();

    builder.singletone(|_| {
        Ok(Arc::new(Mutex::new(Service1 {
            payload: "1".to_string(),
        })))
    });

    let sp = builder.build();

    let service = sp.resolve::<Arc<Mutex<Service1>>>().unwrap();

    assert_eq!(service.lock().unwrap().payload, "1");

    service.lock().unwrap().payload = "2".to_string();

    let service = sp.resolve::<Arc<Mutex<Service1>>>().unwrap();

    assert_eq!(service.lock().unwrap().payload, "2");
}

#[test]
pub fn inventory_registration() {
    struct TestSingleton {
        pub value: String,
    }

    #[xdi_macro::register_constructor(scope = "singleton")]
    fn registration(_: ServiceProvider) -> ServiceBuildResult<Arc<Mutex<TestSingleton>>> {
        Ok(Arc::new(Mutex::new(TestSingleton {
            value: "Hello, Inventory!".to_string(),
        })))
    }

    let builder = DiBuilder::new();

    builder.inject();

    let sp = builder.build();

    let service = sp.resolve::<Arc<Mutex<TestSingleton>>>().unwrap();

    assert_eq!(service.lock().unwrap().value, "Hello, Inventory!");

    service.lock().unwrap().value = "Updated Value".to_string();

    let service = sp.resolve::<Arc<Mutex<TestSingleton>>>().unwrap();

    assert_eq!(service.lock().unwrap().value, "Updated Value");
}
