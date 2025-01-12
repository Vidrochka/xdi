# simple-di

Simple service dependency graph container implementation

- Allow resolve nested service dependency graph

- Support Transient
- Support Singletone
- Support Task local (`WIP`)
- Support Thread local (`WIP`)

- Allow to map service into any other representation as simple like `.map(|service| SomeOther { x: service.x })`
- Allow to map service into trait object as siple like `.map::<dyn SomeTrait>()`

- Resolve single (first) service by self or by any mapping
- Resolve all service wich has requested representation, usefull for trait object (`WIP`)

- Non blocking for transient, single lock for singletone init

- Allow `!Send` + `!Sync` transient

- Readable errors
- Simple architecture (constructor -> scope -> mapping)

- Allow global registration

- Main test cases allowed in tests folder

```rust
use simple_di::builder::SimpleDiBuilder;
use std::sync::{Arc, Mutex};

pub trait ISomeTrait {
    fn get(&self) -> String;
}

pub struct SomeService {
    pub payload: String
}

pub struct SomeServiceDeep {
    pub nested_service: Arc<Mutex<SomeService>>
}

impl ISomeTrait for SomeServiceDeep {
    fn get(&self) -> String {
        self.nested_service.lock().unwrap().payload.clone()
    }
}

pub struct SomeServiceDeeper {
    pub nested_service: SomeServiceDeep
}

fn main() {   
    let builder = SimpleDiBuilder::new();

    // register singletone
    builder.singletone(|_| Ok(Arc::new(Mutex::new(SomeService { payload: "1".to_string() }))));

    // register transient
    builder.transient(|sp| Ok(SomeServiceDeeper { nested_service: sp.resolve()? }));

    // register transient with mapping to trait
    builder.transient(|sp| Ok(SomeServiceDeep { nested_service: sp.resolve()? }))
        .map_as_trait::<dyn ISomeTrait>();

    let sp = builder.build();

    // automaticaly resolve all service dependency graph
    // SomeServiceDeeper -> SomeServiceDeep -> Arc<Mutex<SomeService>>
    let service = sp.resolve::<SomeServiceDeeper>().unwrap();

    assert_eq!(service.nested_service.nested_service.lock().unwrap().payload, "1");

    // change inner singletone
    service.nested_service.nested_service.lock().unwrap().payload = "2".to_string();

    // resolve dependency second time
    // new SomeServiceDeeper and SomeServiceDeep, but old Arc<Mutex<SomeService>>
    let service = sp.resolve::<SomeServiceDeeper>().unwrap();

    assert_eq!(service.nested_service.nested_service.lock().unwrap().payload, "2");

    // SomeServiceDeep also allowed as mapping into Box<dyn ISomeTrait>
    let service = sp.resolve::<Box<dyn ISomeTrait>>().unwrap();

    assert_eq!(service.get(), "2");
}
```
