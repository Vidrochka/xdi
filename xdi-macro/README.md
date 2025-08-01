# xdi-macro

Type injection for xdi

## Injection

You can inject service as fn constructor  
All type ctors will be automatically registered on builder creation  
Injection use `inventory`, so you can add injection from dependency crate

```rust

pub struct SomeService {}

trait ISomeService1 {}

impl ISomeService1 for SomeService {}

trait ISomeService2 {}

impl ISomeService2 for SomeService {}

// As transient (for transient scope param can be omitted)
#[xdi_macro::register_constructor(scope = "transient")]
fn some_service_ctor(_sp: ServiceProvider) -> ServiceBuildResult<SomeService> {
    Ok(SomeService{})
}

// As singleton
#[xdi_macro::register_constructor(scope = "singleton")]
fn some_service_ctor(_sp: ServiceProvider) -> ServiceBuildResult<SomeService> {
    Ok(SomeService{})
}

// As thread local
#[xdi_macro::register_constructor(scope = "thread_local")]
fn some_service_ctor(_sp: ServiceProvider) -> ServiceBuildResult<SomeService> {
    Ok(SomeService{})
}

// As task local
#[xdi_macro::register_constructor(scope = "task_local")]
fn some_service_ctor(_sp: ServiceProvider) -> ServiceBuildResult<SomeService> {
    Ok(SomeService{})
}

// For every scope you can define multiple trait map
#[xdi_macro::register_constructor(scope = "transient", map = [ISomeService1, ISomeService2])]
fn some_service_ctor() {
    Ok(SomeService{})
}

```
