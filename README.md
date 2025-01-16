# simple-di

Simple service dependency graph container implementation

- Allow resolve nested service dependency graph

- Support Transient
- Support Singletone
- Support Task local
- Support Thread local (`WIP`)

- Allow to map service into any other representation as simple like `.map(|service| SomeOther { x: service.x })`
- Allow to map service into trait object as siple like `.map::<dyn SomeTrait>()`

- Resolve single (first) service by self or by any mapping
- Resolve all service wich has requested representation, usefull for trait object

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

---

# How to use

Create container builder

```rust

let builder = SimpleDiBuilder::new();
// or
// let builder = SimpleDiBuilder::default();
```

### Register the service
- Mutable access not required, builder can be shared by ref
- Registration fn takes used ServiceProvider and can resolve nested dependency 

##### As transient
- Create new instance every call
- Allowed !Send + !Sync

```rust

builder.transient(|sp: ServiceProvider| Ok(SomeService {
    //... some initialization
    some_field: sp.resolve::<SomeNestedService>(),
}));
```

##### As singletone  
- Lazy creation on the first invocation and return a clone on every next invocation
- Singletone required clone for service (you can wrap to Arc or derive Clone)
- Singletone required Sync + Send because it can be shared anywhere

```rust

builder.singletone(|_sp: ServiceProvider| Ok(SomeService {
    //... some initialization
}));
```

##### As task local
- Lazy creation on the first invocation from the task scope and return a clone on every next invocation in same task scope
- Task local required clone for service (you can wrap to Arc or derive Clone)
- Task local required Sync + Send because it can be shared anywhere

```rust
builder.task_local(|_sp: ServiceProvider| Ok(SomeService {
    //... some initialization
}));
```

##### As thread local
- Lazy creation on the first invocation from the thread scope and return a clone on every next invocation in same thread scope
- Task local required clone for service (you can wrap to Arc or derive Clone)
- Task local required Sync + Send because it can be shared anywhere
- WIP


### Map service
- Mapping allow add new service representation for same constructor
- Mapping (Service -> Service) auto-generated
- You can add as many mappings for a single service as you need

##### Custom map

```rust

    builder.transient(|_| Ok(SomeService {
        //... some initialization
    }))
    .map_as(|some_service| Ok(OtherService { some_field: x.some_field }));
```

##### Trait object map
- Create mapping to `Box<dyn ISomeTrait>` if service impl ISomeTrait

```rust

    builder.transient(|_| Ok(SomeService {
        //... some initialization
    }))
    .map_as_trait::<dyn ISomeTrait>();
```

### Build container
- You can build container as var, or register global

##### Build container as var

```rust

let sp = builder.build();
```

##### Build and register global

```rust
builder.build_global();

// then access by static global var
let service = ServiceProvider::get().unwrap().resolve::<SomeService>().unwrap()
```

### Resolve service by mapping

##### As service

```rust

let service: SomeService = sp.resolve().unwrap();
// let service: Box<dyn ISomeTrait> = sp.resolve().unwrap();
```

##### As boxed service

```rust
use simple_di::types::type_info::TypeInfoSource;

let service = sp.resolve_raw(SomeService::type_info()).unwrap();
// let service = sp.resolve(Box<dyn ISomeTrait>::type_info()).unwrap();

let service = service.unbox::<SomeService>().unwrap();
```

##### As vector of services, which has some mapping

```rust
let services: Vec<Box<dyn ISomeTrait>> = sp.resolve_all().unwrap();
```

##### As vector of boxed services, which has some mapping

```rust
use simple_di::types::type_info::TypeInfoSource;

let services: Vec<BoxedService> = sp.resolve_all_raw(Box<dyn ISomeTrait>::type_info()).unwrap();
```

##### As dependency in task scope

```rust

tokio::spawn(async move {
    let service = sp.resolve::<SomeService>().unwrap();

    // In second time resolve return instanse clone (like singletone)
    let service = sp.resolve::<SomeService>().unwrap();
}.add_service_span())

tokio::spawn(async move {
    // New task has own SomeService instance
    let service = sp.resolve::<SomeService>().unwrap();
}.add_service_span())
```