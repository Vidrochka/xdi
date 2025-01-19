#![feature(unsize)]
#![feature(iterator_try_collect)]
#![feature(slice_as_array)]

//! # xdi
//! 
//! Simple service dependency graph container implementation
//! 
//! - Allow resolve nested service dependency graph
//! 
//! - Support Transient
//! - Support Singletone
//! - Support Task local (singletone in task scope)
//! - Support Thread local (singletone in thread scope)
//! 
//! - Allow to map service into any other representation as simple like `.map_as(|service| SomeOther { x: service.x })`
//! - Allow to map service into trait object as siple like `.map_as_trait::<dyn SomeTrait>()`
//! 
//! - Resolve single (first) service by self or by any mapping
//! - Resolve all service wich has requested representation, usefull for trait object
//! 
//! - Non blocking for transient, single lock for singletone/task_local/thread_local init
//! 
//! - Allow `!Send` + `!Sync` for transient and thread_local
//! 
//! - Readable errors
//! - Simple architecture (constructor -> scope -> mapping)
//! 
//! - Allow global `ServiceProvider` registration
//! 
//! - Main test cases allowed in tests folder
//! 
//! ```rust
//! use xdi::builder::DiBuilder;
//! use std::sync::{Arc, Mutex};
//! 
//! pub trait ISomeTrait {
//!     fn get(&self) -> String;
//! }
//! 
//! pub struct SomeService {
//!     pub payload: String
//! }
//! 
//! pub struct SomeServiceDeep {
//!     pub nested_service: Arc<Mutex<SomeService>>
//! }
//! 
//! impl ISomeTrait for SomeServiceDeep {
//!     fn get(&self) -> String {
//!         self.nested_service.lock().unwrap().payload.clone()
//!     }
//! }
//! 
//! pub struct SomeServiceDeeper {
//!     pub nested_service: SomeServiceDeep
//! }
//! 
//! fn main() {   
//!     let builder = DiBuilder::new();
//! 
//!     // register singletone
//!     builder.singletone(|_| Ok(Arc::new(Mutex::new(SomeService { payload: "1".to_string() }))));
//! 
//!     // register transient
//!     builder.transient(|sp| Ok(SomeServiceDeeper { nested_service: sp.resolve()? }));
//! 
//!     // register transient with mapping to trait
//!     builder.transient(|sp| Ok(SomeServiceDeep { nested_service: sp.resolve()? }))
//!         .map_as_trait::<dyn ISomeTrait>();
//! 
//!     let sp = builder.build();
//! 
//!     // automaticaly resolve all service dependency graph
//!     // SomeServiceDeeper -> SomeServiceDeep -> Arc<Mutex<SomeService>>
//!     let service = sp.resolve::<SomeServiceDeeper>().unwrap();
//! 
//!     assert_eq!(service.nested_service.nested_service.lock().unwrap().payload, "1");
//! 
//!     // change inner singletone
//!     service.nested_service.nested_service.lock().unwrap().payload = "2".to_string();
//! 
//!     // resolve dependency second time
//!     // new SomeServiceDeeper and SomeServiceDeep, but old Arc<Mutex<SomeService>>
//!     let service = sp.resolve::<SomeServiceDeeper>().unwrap();
//! 
//!     assert_eq!(service.nested_service.nested_service.lock().unwrap().payload, "2");
//! 
//!     // SomeServiceDeep also allowed as mapping into Box<dyn ISomeTrait>
//!     let service = sp.resolve::<Box<dyn ISomeTrait>>().unwrap();
//! 
//!     assert_eq!(service.get(), "2");
//! }
//! ```
//! 
//! ---
//! 
//! # How to use
//! 
//! Create container builder
//! 
//! ```rust
//! # use xdi::builder::DiBuilder;
//! #
//! let builder = DiBuilder::new();
//! // or
//! let builder = DiBuilder::default();
//! ```
//! 
//! ### Register the service
//! - Mutable access not required, builder can be shared by ref
//! - Registration fn takes used ServiceProvider and can resolve nested dependency 
//! 
//! ##### As transient
//! - Create new instance every call
//! - Allowed !Send + !Sync
//! 
//! ```rust
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! pub struct DbConnection {}
//! 
//! pub struct Repository {
//!     conn: DbConnection,
//! }
//! 
//! # let builder = DiBuilder::new();
//! #
//! builder.transient(|_sp: ServiceProvider| Ok(DbConnection {}));
//! 
//! builder.transient(|sp: ServiceProvider| Ok(Repository {
//!     conn: sp.resolve::<DbConnection>()?,
//! }));
//! ```
//! 
//! ##### As singletone  
//! - Lazy creation on the first invocation and return a clone on every next invocation
//! - Singletone required clone for service (you can wrap to Arc or derive Clone)
//! - Singletone required Sync + Send because it can be shared anywhere
//! 
//! ```rust
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! #[derive(Clone)]
//! pub struct SomeService {}
//! 
//! # let builder = DiBuilder::new();
//! #
//! builder.singletone(|_sp: ServiceProvider| Ok(SomeService {
//!     //... some initialization
//! }));
//! ```
//! 
//! ##### As task local
//! - Lazy creation on the first invocation from the task scope and return a clone on every next invocation in same task scope
//! - Task local required clone for service (you can wrap to Arc or derive Clone)
//! - Task local required Sync + Send because it can be shared anywhere
//! 
//! ```rust
//! #[cfg(feature = "task-local")]
//! {
//!     # use xdi::{builder::DiBuilder, ServiceProvider};
//!     #
//!     #[derive(Clone)]
//!     pub struct SomeService {}
//!     
//!     # let builder = DiBuilder::new();
//!     #
//!     builder.task_local(|_sp: ServiceProvider| Ok(SomeService {
//!         //... some initialization
//!     }));
//! }
//! ```
//! 
//! ##### As thread local
//! - Lazy creation on the first invocation from the thread scope and return a clone on every next invocation in same thread scope
//! - Thread local required clone for service (you can wrap to Rc or derive Clone)
//! - Allowed !Send + !Sync
//! 
//! ```rust
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! #[derive(Clone)]
//! pub struct SomeService {}
//! 
//! # let builder = DiBuilder::new();
//! #
//! builder.thread_local(|_sp: ServiceProvider| Ok(SomeService {
//!     //... some initialization
//! }));
//! ```
//! 
//! 
//! ### Map service
//! - Mapping allow add new service representation for same constructor
//! - Mapping (Service -> Service) auto-generated
//! - You can add as many mappings for a single service as you need
//! 
//! ##### Custom map
//! 
//! ```rust
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! pub struct DbConnectionInner {}
//! 
//! pub struct DbConnectionPool {}
//! 
//! impl DbConnectionPool {
//!    fn get(&self) -> DbConnectionInner { DbConnectionInner {} }
//! }
//! 
//! pub struct DbConnection {
//!     conn: DbConnectionInner,
//! }
//! #
//! # let builder = DiBuilder::new();
//! 
//! builder.transient(|_sp: ServiceProvider| Ok(DbConnectionPool {
//!     //... some initialization
//! }))
//! .map_as(|pool| Ok(DbConnection { conn: pool.get() }));
//! ```
//! 
//! ##### Trait object map
//! - Create mapping to `Box<dyn ISomeTrait>` if service impl ISomeTrait
//! 
//! ```rust
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! # pub struct SomeService {}
//! #
//! # pub trait ISomeTrait {}
//! #
//! # impl ISomeTrait for SomeService {}
//! #
//! # let builder = DiBuilder::new();
//! #
//! builder.transient(|_sp: ServiceProvider| Ok(SomeService {
//!     //... some initialization
//! }))
//! .map_as_trait::<dyn ISomeTrait>();
//! ```
//! 
//! ### Build container
//! - You can build container as var, or register global
//! 
//! ##### Build container as var
//! 
//! ```rust
//! # use xdi::builder::DiBuilder;
//! #
//! # pub struct SomeService {}
//! #
//! # let builder = DiBuilder::new();
//! #
//! let sp = builder.build();
//! ```
//! 
//! ##### Build and register global
//! 
//! ```rust
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! # pub struct SomeService {}
//! #
//! # let builder = DiBuilder::new();
//! #
//! # builder.transient(|_sp: ServiceProvider| Ok(SomeService {}));
//! #
//! builder.build_global();
//! 
//! // then access by static global var
//! let service = ServiceProvider::get().unwrap().resolve::<SomeService>().unwrap();
//! ```
//! 
//! ### Resolve service by mapping
//! 
//! ##### As service
//! 
//! ```rust
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! # pub struct SomeService {}
//! #
//! # let builder = DiBuilder::new();
//! #
//! # builder.transient(|_| Ok(SomeService {}));
//! #
//! # let sp = builder.build();
//! # 
//! let service: SomeService = sp.resolve().unwrap();
//! // let service: Box<dyn ISomeTrait> = sp.resolve().unwrap();
//! ```
//! 
//! ##### As boxed service
//! 
//! ```rust
//! use xdi::types::type_info::TypeInfoSource;
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! # pub struct SomeService {}
//! #
//! # let builder = DiBuilder::new();
//! #
//! # builder.transient(|_| Ok(SomeService {}));
//! #
//! # let sp = builder.build();
//! 
//! let service = sp.resolve_raw(SomeService::type_info()).unwrap();
//! // let service = sp.resolve(Box::<dyn ISomeTrait>::type_info()).unwrap();
//! 
//! let service = service.unbox::<SomeService>().unwrap();
//! // let service = service.unbox::<Box<dyn ISomeTrait>>().unwrap();
//! ```
//! 
//! ##### As vector of services, which has some mapping
//! 
//! ```rust
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! # pub struct SomeService {}
//! #
//! # pub struct OtherService {}
//! #
//! # pub trait ISomeTrait {}
//! #
//! # impl ISomeTrait for SomeService {}
//! # 
//! # impl ISomeTrait for OtherService {}
//! #
//! # let builder = DiBuilder::new();
//! #
//! builder.transient(|_| Ok(SomeService {}))
//!     .map_as_trait::<dyn ISomeTrait>();
//! 
//! builder.transient(|_| Ok(OtherService {}))
//!     .map_as_trait::<dyn ISomeTrait>();
//! #
//! # let sp = builder.build();
//! 
//! let services: Vec<Box<dyn ISomeTrait>> = sp.resolve_all().unwrap();
//! ```
//! 
//! ##### As vector of boxed services, which has some mapping
//! 
//! ```rust
//! use xdi::types::{type_info::TypeInfoSource, boxed_service::BoxedService};
//! # use xdi::{builder::DiBuilder, ServiceProvider};
//! #
//! # pub struct SomeService {}
//! #
//! # pub struct OtherService {}
//! #
//! # pub trait ISomeTrait {}
//! #
//! # impl ISomeTrait for SomeService {}
//! # 
//! # impl ISomeTrait for OtherService {}
//! #
//! # let builder = DiBuilder::new();
//! 
//! builder.transient(|_| Ok(SomeService {}))
//!     .map_as_trait::<dyn ISomeTrait>();
//! 
//! builder.transient(|_| Ok(OtherService {}))
//!     .map_as_trait::<dyn ISomeTrait>();
//! #
//! # let sp = builder.build();
//! 
//! let services: Vec<BoxedService> = sp.resolve_all_raw(Box::<dyn ISomeTrait>::type_info()).unwrap();
//! ```
//! 
//! ##### As dependency in task scope
//! 
//! ```rust
//! #[cfg(feature = "task-local")]
//! {
//!     use xdi::IAsyncTaskScope;
//!     # use xdi::{builder::DiBuilder, ServiceProvider};
//!     #
//!     # #[derive(Clone)]
//!     # pub struct SomeService {}
//!     #
//!     # let builder = DiBuilder::new();
//!     
//!     builder.task_local(|_| Ok(SomeService {}));
//!     
//!     let sp = builder.build();
//!     let sp2 = sp.clone();
//!     
//!     # let runtime = tokio::runtime::Builder::new_multi_thread()
//!     #   .worker_threads(4)
//!     #   .build()
//!     #   .unwrap();
//!     #
//!     # runtime.block_on(async move {
//!     #
//!     tokio::spawn(async move {
//!         let service = sp.resolve::<SomeService>().unwrap();
//!     
//!         // In second time resolve return instanse clone (like singletone)
//!         let service = sp.resolve::<SomeService>().unwrap();
//!     }.add_service_span());
//!     
//!     tokio::spawn(async move {
//!         // New task has own SomeService instance
//!         let service = sp2.resolve::<SomeService>().unwrap();
//!     }.add_service_span());
//!     #
//!     # });
//! }
//! ```

use std::sync::{Arc, OnceLock};

#[cfg(feature = "task-local")]
use layers::scope::TaskLocalCtx;
use layers::mapping::MappingLayer;
use types::{boxed_service::BoxedService, error::ServiceBuildResult, type_info::TypeInfo};

pub mod builder;
pub mod layers;
pub mod types;

#[cfg(test)]
pub mod tests;

static SERVICE_PROVIDER: OnceLock<ServiceProvider> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ServiceProvider {
    pub(crate) mapping_layer: Arc<MappingLayer>,
}

impl ServiceProvider {
    /// Get global service provider
    /// 
    /// # Example
    /// ```rust
    /// # use xdi::{builder::DiBuilder, ServiceProvider};
    /// #
    /// # pub struct SomeService {}
    /// # 
    /// let builder = DiBuilder::new();
    /// 
    /// builder.transient(|_| Ok(SomeService {}));
    /// 
    /// builder.build_global();
    ///  
    /// let service = ServiceProvider::get().unwrap().resolve::<SomeService>().unwrap();
    /// ```
    pub fn get<'a>() -> Option<&'a ServiceProvider> {
        SERVICE_PROVIDER.get()
    }

    /// Build new service
    /// 
    /// # Example
    /// ```rust
    /// # use xdi::{builder::DiBuilder, ServiceProvider};
    /// #
    /// # pub struct SomeService {}
    /// #
    /// # let builder = DiBuilder::new();
    /// #
    /// # builder.transient(|_| Ok(SomeService {}));
    /// #
    /// # let sp = builder.build();
    /// # 
    /// let service: SomeService = sp.resolve().unwrap();
    /// // let service: Box<dyn ISomeTrait> = sp.resolve().unwrap();
    /// ```
    pub fn resolve<TService: 'static>(&self) -> ServiceBuildResult<TService> {
        self.mapping_layer.resolve::<TService>(self.clone())
    }

    /// Build new service by type info
    /// 
    /// # Example
    /// ```rust
    /// use xdi::types::type_info::TypeInfoSource;
    /// # use xdi::{builder::DiBuilder, ServiceProvider};
    /// #
    /// # pub struct SomeService {}
    /// #
    /// # let builder = DiBuilder::new();
    /// #
    /// # builder.transient(|_| Ok(SomeService {}));
    /// #
    /// # let sp = builder.build();
    /// 
    /// let service = sp.resolve_raw(SomeService::type_info()).unwrap();
    /// // let service = sp.resolve(Box::<dyn ISomeTrait>::type_info()).unwrap();
    /// 
    /// let service = service.unbox::<SomeService>().unwrap();
    /// // let service = service.unbox::<Box<dyn ISomeTrait>>().unwrap();
    /// ```
    pub fn resolve_raw(&self, ty: TypeInfo) -> ServiceBuildResult<BoxedService> {
        self.mapping_layer.resolve_raw(ty, self.clone())
    }

    /// Create all services by type
    /// 
    /// # Example
    /// ```rust
    /// # use xdi::{builder::DiBuilder, ServiceProvider};
    /// #
    /// # pub struct SomeService {}
    /// #
    /// # pub struct OtherService {}
    /// #
    /// # pub trait ISomeTrait {}
    /// #
    /// # impl ISomeTrait for SomeService {}
    /// # 
    /// # impl ISomeTrait for OtherService {}
    /// #
    /// # let builder = DiBuilder::new();
    /// #
    /// builder.transient(|_| Ok(SomeService {}))
    ///     .map_as_trait::<dyn ISomeTrait>();
    /// 
    /// builder.transient(|_| Ok(OtherService {}))
    ///     .map_as_trait::<dyn ISomeTrait>();
    /// #
    /// # let sp = builder.build();
    /// 
    /// let services: Vec<Box<dyn ISomeTrait>> = sp.resolve_all().unwrap();
    /// ```
    pub fn resolve_all<TService: 'static>(&self) -> ServiceBuildResult<Vec<TService>> {
        self.mapping_layer.resolve_all::<TService>(self.clone())
    }

    /// Create all services by type info
    /// 
    /// # Example
    /// ```rust
    /// use xdi::types::{type_info::TypeInfoSource, boxed_service::BoxedService};
    /// # use xdi::{builder::DiBuilder, ServiceProvider};
    /// #
    /// # pub struct SomeService {}
    /// #
    /// # pub struct OtherService {}
    /// #
    /// # pub trait ISomeTrait {}
    /// #
    /// # impl ISomeTrait for SomeService {}
    /// # 
    /// # impl ISomeTrait for OtherService {}
    /// #
    /// # let builder = DiBuilder::new();
    /// 
    /// builder.transient(|_| Ok(SomeService {}))
    ///     .map_as_trait::<dyn ISomeTrait>();
    /// 
    /// builder.transient(|_| Ok(OtherService {}))
    ///     .map_as_trait::<dyn ISomeTrait>();
    /// #
    /// # let sp = builder.build();
    /// 
    /// let services: Vec<BoxedService> = sp.resolve_all_raw(Box::<dyn ISomeTrait>::type_info()).unwrap();
    /// ```
    pub fn resolve_all_raw(&self, ty: TypeInfo) -> ServiceBuildResult<Vec<BoxedService>> {
        self.mapping_layer.resolve_all_raw(ty, self.clone())
    }

    /// Register service provider as global object
    /// 
    /// # Example
    /// ```rust
    /// # use xdi::{builder::DiBuilder, ServiceProvider};
    /// #
    /// # pub struct SomeService {}
    /// # 
    /// let builder = DiBuilder::new();
    /// 
    /// builder.transient(|_| Ok(SomeService {}));
    /// 
    /// let sp = builder.build();
    /// 
    /// sp.install_global();
    /// 
    /// let service = ServiceProvider::get().unwrap().resolve::<SomeService>().unwrap();
    /// ```
    pub fn install_global(self) {
        SERVICE_PROVIDER.set(self).unwrap();
    }

    #[cfg(feature = "task-local")]
    /// Build span for async closure
    /// 
    /// Dont use manualy. Use `tokio::spawn(async {}.async_task_span())`
    pub async fn async_task_span<F: Future>(f: F) -> F::Output {
        TaskLocalCtx::span(f).await
    }
}

#[cfg(feature = "task-local")]
pub trait IAsyncTaskScope {
    type TFutRes;

    fn add_service_span(self) -> impl Future<Output = Self::TFutRes>;
}

#[cfg(feature = "task-local")]
impl<TFut: Future> IAsyncTaskScope for TFut {
    type TFutRes = TFut::Output;

    /// Add service span for async closure
    /// 
    /// # Example
    /// ```rust
    /// use xdi::IAsyncTaskScope;
    /// # use xdi::{builder::DiBuilder, ServiceProvider};
    /// #
    /// # #[derive(Clone)]
    /// # pub struct SomeService {}
    /// #
    /// # let builder = DiBuilder::new();
    /// 
    /// builder.task_local(|_| Ok(SomeService {}));
    /// 
    /// let sp = builder.build();
    /// let sp2 = sp.clone();
    /// 
    /// # let runtime = tokio::runtime::Builder::new_multi_thread()
    /// #   .worker_threads(4)
    /// #   .build()
    /// #   .unwrap();
    /// #
    /// # runtime.block_on(async move {
    /// #
    /// tokio::spawn(async move {
    ///     let service = sp.resolve::<SomeService>().unwrap();
    /// 
    ///     // In second time resolve return instanse clone (like singletone)
    ///     let service = sp.resolve::<SomeService>().unwrap();
    /// }.add_service_span());
    /// 
    /// tokio::spawn(async move {
    ///     // New task has own SomeService instance
    ///     let service = sp2.resolve::<SomeService>().unwrap();
    /// }.add_service_span());
    /// #
    /// # });
    /// ```
    fn add_service_span(self) -> impl Future<Output = Self::TFutRes> {
        ServiceProvider::async_task_span(self)
    }
}
