use crate::{builder::SimpleDiBuilder, ServiceProvider};

pub struct Service1 {
    pub payload: String
}

#[test]
pub fn set_get_transient_ok() {
    let builder = SimpleDiBuilder::new();

    builder.transient(|| Service1 {
        payload: "1".to_string()
    });

    builder.build();

    let mut service = ServiceProvider::resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");

    service.payload = "2".to_string();

    let service = ServiceProvider::resolve::<Service1>().unwrap();

    assert_eq!(service.payload, "1");
} 