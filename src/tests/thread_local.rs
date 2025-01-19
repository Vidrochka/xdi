use std::{rc::Rc, sync::Mutex, thread};

use crate::builder::DiBuilder;

#[derive(Clone)]
pub struct Service1 {
    pub payload: Rc<Mutex<String>>,
}

pub trait IPayloadSrc {
    fn get(&self) -> String;
    fn set(&mut self, val: String);
}

impl IPayloadSrc for Service1 {
    fn get(&self) -> String {
        self.payload.lock().unwrap().clone()
    }

    fn set(&mut self, val: String) {
        *self.payload.lock().unwrap() = val;
    }
}

#[test]
fn set_get_thread_local_ok() {
    let builder = DiBuilder::new();

    builder.thread_local(|_| {
        Ok(Service1 {
            payload: Rc::new(Mutex::new("1".to_string())),
        })
    });

    let sp = builder.build();

    {
        let sp = sp.clone();

        thread::spawn(move || {
            let service = sp.resolve::<Service1>().unwrap();

            assert_eq!(*service.payload.lock().unwrap(), "1");

            *service.payload.lock().unwrap() = "2".to_string();

            let service = sp.resolve::<Service1>().unwrap();

            assert_eq!(*service.payload.lock().unwrap(), "2");
        })
        .join()
        .unwrap();
    }

    thread::spawn(move || {
        let service = sp.resolve::<Service1>().unwrap();

        assert_eq!(*service.payload.lock().unwrap(), "1");
    })
    .join()
    .unwrap();
}

#[test]
fn set_get_thread_local_trait_object_ok() {
    let builder = DiBuilder::new();

    builder
        .thread_local(|_| {
            Ok(Service1 {
                payload: Rc::new(Mutex::new("1".to_string())),
            })
        })
        .map_as_trait::<dyn IPayloadSrc>();

    let sp = builder.build();

    {
        let sp = sp.clone();

        thread::spawn(move || {
            let mut service = sp.resolve::<Box<dyn IPayloadSrc>>().unwrap();

            assert_eq!(service.get(), "1");

            service.set("2".to_string());

            let service = sp.resolve::<Box<dyn IPayloadSrc>>().unwrap();

            assert_eq!(service.get(), "2");
        })
        .join()
        .unwrap();
    }

    thread::spawn(move || {
        let service = sp.resolve::<Box<dyn IPayloadSrc>>().unwrap();

        assert_eq!(service.get(), "1");
    })
    .join()
    .unwrap();
}
