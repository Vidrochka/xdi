use std::{rc::Rc, sync::Mutex};

use crate::{
    ServiceProvider,
    builder::DiBuilder,
    types::{error::ServiceBuildResult, type_info::TypeInfoSource},
};

pub struct Service1 {
    pub payload: String,
}

pub struct Service2 {
    pub payload: String,
}

pub struct ServiceNotSend1 {
    pub payload: Rc<Mutex<String>>,
}

pub struct ServiceWithDependency {
    pub service1: Service1,
    pub service2: ServiceNotSend1,
}

pub trait IGetInner {
    fn get(&self) -> &str;
}

impl IGetInner for Service1 {
    fn get(&self) -> &str {
        &self.payload
    }
}

impl IGetInner for Service2 {
    fn get(&self) -> &str {
        &self.payload
    }
}

pub trait IGetInnerWithModyfy {
    fn get(&self) -> String;
}

impl IGetInnerWithModyfy for Service1 {
    fn get(&self) -> String {
        format!("{}2", self.payload)
    }
}

#[test]
pub fn set_get_transient_from_closure_ok() {
    let builder = DiBuilder::new();

    builder.transient(|_| {
        Ok(Service1 {
            payload: "1".to_string(),
        })
    });

    let sp = builder.build();

    let mut service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");

    service.payload = "2".to_string();

    let service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");
}

#[test]
pub fn set_get_transient_from_fn_ok() {
    let builder = DiBuilder::new();

    fn service_ctr(_sp: ServiceProvider) -> ServiceBuildResult<Service1> {
        Ok(Service1 {
            payload: "1".to_string(),
        })
    }

    builder.transient(service_ctr);

    let sp = builder.build();

    let mut service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");

    service.payload = "2".to_string();

    let service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");
}

#[test]
pub fn set_get_transient_from_method_ok() {
    let builder = DiBuilder::new();

    trait Ctor {
        fn service_ctr(_sp: ServiceProvider) -> ServiceBuildResult<Service1>;
    }

    impl Ctor for Service1 {
        fn service_ctr(_sp: ServiceProvider) -> ServiceBuildResult<Service1> {
            Ok(Service1 {
                payload: "1".to_string(),
            })
        }
    }

    builder.transient(Service1::service_ctr);

    let sp = builder.build();

    let mut service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");

    service.payload = "2".to_string();

    let service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");
}

#[test]
pub fn set_get_raw_transient_ok() {
    let builder = DiBuilder::new();

    builder.transient(|_| {
        Ok(Service1 {
            payload: "1".to_string(),
        })
    });

    let sp = builder.build();

    let service = sp.resolve_raw(Service1::type_info()).unwrap();

    assert_eq!(
        service
            .unbox::<Service1>()
            .expect("Expecred Service1")
            .payload,
        "1"
    );
}

#[test]
pub fn set_get_not_send_transient_ok() {
    let builder = DiBuilder::new();

    builder.transient(|_| {
        Ok(ServiceNotSend1 {
            payload: Rc::new(Mutex::new("1".to_string())),
        })
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
    let builder = DiBuilder::new();

    builder.transient(|_| {
        Ok(Service1 {
            payload: "1".to_string(),
        })
    });

    builder.transient(|_| {
        Ok(ServiceNotSend1 {
            payload: Rc::new(Mutex::new("2".to_string())),
        })
    });

    builder.transient(|sp| {
        Ok(ServiceWithDependency {
            service1: sp.resolve()?,
            service2: sp.resolve()?,
        })
    });

    let sp = builder.build();

    let service = sp.resolve::<ServiceWithDependency>().unwrap();

    assert_eq!(service.service1.payload, "1");
    assert_eq!(*service.service2.payload.lock().unwrap(), "2");
}

pub struct Service1Extra {
    pub payload: String,
}

#[test]
pub fn set_get_transient_with_mapping_ok() {
    let builder = DiBuilder::new();

    builder
        .transient(|_| {
            Ok(Service1 {
                payload: "1".to_string(),
            })
        })
        .map_as(|x| {
            Ok(Service1Extra {
                payload: format!("{}2", x.payload),
            })
        });

    let sp = builder.build();

    let service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");

    let service = sp.resolve::<Service1Extra>().unwrap();

    assert_eq!(service.payload, "12");
}

#[test]
pub fn set_get_transient_with_mapping_trait_ok() {
    let builder = DiBuilder::new();

    builder
        .transient(|_| {
            Ok(Service1 {
                payload: "1".to_string(),
            })
        })
        .map_as_trait::<dyn IGetInner>()
        .map_as_trait::<dyn IGetInnerWithModyfy>();

    let sp = builder.build();

    let service = sp.resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");

    let service = sp.resolve::<Box<dyn IGetInner>>().unwrap();

    assert_eq!(service.get(), "1");

    let service = sp.resolve::<Box<dyn IGetInnerWithModyfy>>().unwrap();

    assert_eq!(service.get(), "12");
}

#[test]
pub fn get_all_transient_ok() {
    let builder = DiBuilder::new();

    builder
        .transient(|_| {
            Ok(Service1 {
                payload: "1".to_string(),
            })
        })
        .map_as_trait::<dyn IGetInner>();

    builder
        .transient(|_| {
            Ok(Service2 {
                payload: "2".to_string(),
            })
        })
        .map_as_trait::<dyn IGetInner>();

    let sp = builder.build();

    let services = sp.resolve_all::<Box<dyn IGetInner>>().unwrap();

    let [service1, service2] = services.as_slice().as_array().expect("Expected 2 service");

    assert_eq!(service1.get(), "1");
    assert_eq!(service2.get(), "2");
}

#[test]
pub fn get_all_transient_raw_ok() {
    let builder = DiBuilder::new();

    builder
        .transient(|_| {
            Ok(Service1 {
                payload: "1".to_string(),
            })
        })
        .map_as_trait::<dyn IGetInner>();

    builder
        .transient(|_| {
            Ok(Service2 {
                payload: "2".to_string(),
            })
        })
        .map_as_trait::<dyn IGetInner>();

    let sp = builder.build();

    let services = sp
        .resolve_all_raw(Box::<dyn IGetInner>::type_info())
        .unwrap();

    let [service1, service2] = *services
        .into_boxed_slice()
        .into_array()
        .expect("Expected 2 service");

    assert_eq!(
        service1
            .unbox::<Box<dyn IGetInner>>()
            .expect("Expected Box<dyn IGetInner> in service1")
            .get(),
        "1"
    );
    assert_eq!(
        service2
            .unbox::<Box<dyn IGetInner>>()
            .expect("Expected Box<dyn IGetInner> in service2")
            .get(),
        "2"
    );
}
