use std::sync::{Arc, Mutex};

use tokio::runtime::Builder;

use crate::{
    IAsyncTaskScope, ServiceProvider, builder::DiBuilder, types::error::ServiceBuildResult,
};

#[derive(Clone)]
pub struct Service1 {
    pub payload: Arc<Mutex<String>>,
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
fn set_get_task_local_ok() {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();

    let builder = DiBuilder::new();

    builder.task_local(|_| {
        Ok(Service1 {
            payload: Arc::new(Mutex::new("1".to_string())),
        })
    });

    let sp = builder.build();

    {
        let sp = sp.clone();

        let task = async move {
            let service = sp.resolve::<Service1>().unwrap();

            assert_eq!(*service.payload.lock().unwrap(), "1");

            *service.payload.lock().unwrap() = "2".to_string();

            let service = sp.resolve::<Service1>().unwrap();

            assert_eq!(*service.payload.lock().unwrap(), "2");
        }
        .add_service_span();

        runtime.block_on(task);
    }

    let task = runtime.spawn(
        async move {
            let service = sp.resolve::<Service1>().unwrap();

            assert_eq!(*service.payload.lock().unwrap(), "1");
        }
        .add_service_span(),
    );

    runtime.block_on(task).unwrap();
}

#[test]
fn set_get_task_local_trait_object_ok() {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();

    let builder = DiBuilder::new();

    builder
        .task_local(|_| {
            Ok(Service1 {
                payload: Arc::new(Mutex::new("1".to_string())),
            })
        })
        .map_as_trait::<dyn IPayloadSrc>();

    let sp = builder.build();

    {
        let sp = sp.clone();

        let task = async move {
            let mut service = sp.resolve::<Box<dyn IPayloadSrc>>().unwrap();

            assert_eq!(service.get(), "1");

            service.set("2".to_string());

            let service = sp.resolve::<Box<dyn IPayloadSrc>>().unwrap();

            assert_eq!(service.get(), "2");
        }
        .add_service_span();

        runtime.block_on(task);
    }

    let task = runtime.spawn(
        async move {
            let service = sp.resolve::<Box<dyn IPayloadSrc>>().unwrap();

            assert_eq!(service.get(), "1");
        }
        .add_service_span(),
    );

    runtime.block_on(task).unwrap();
}

#[test]
pub fn inventory_registration() {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();

    #[derive(Clone)]
    struct TestTaskLocal {
        pub payload: Arc<Mutex<String>>,
    }

    #[xdi_macro::register_constructor(scope = "task_local")]
    fn registration(_: ServiceProvider) -> ServiceBuildResult<TestTaskLocal> {
        Ok(TestTaskLocal {
            payload: Arc::new(Mutex::new("1".to_string())),
        })
    }

    let builder = DiBuilder::new();

    builder.inject();

    let sp = builder.build();

    {
        let sp = sp.clone();

        let task = async move {
            let service = sp.resolve::<TestTaskLocal>().unwrap();

            assert_eq!(*service.payload.lock().unwrap(), "1");

            *service.payload.lock().unwrap() = "2".to_string();

            let service = sp.resolve::<TestTaskLocal>().unwrap();

            assert_eq!(*service.payload.lock().unwrap(), "2");
        }
        .add_service_span();

        runtime.block_on(task);
    }

    let task = runtime.spawn(
        async move {
            let service = sp.resolve::<TestTaskLocal>().unwrap();

            assert_eq!(*service.payload.lock().unwrap(), "1");
        }
        .add_service_span(),
    );

    runtime.block_on(task).unwrap();
}
