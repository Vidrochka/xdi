use std::{
    marker::{PhantomData, Unsize},
    sync::Arc,
};

use crate::{
    ServiceProvider,
    layers::{
        mapping::MappingLayerBuilder, scope::ScopeLayerBuilder, service::ServiceLayerBuilder,
    },
    types::error::ServiceBuildResult,
};

/// Builder for DI container
#[derive(Debug, Default)]
pub struct SimpleDiBuilder {
    service_layer: ServiceLayerBuilder,
    scope_layer: ScopeLayerBuilder,
    mapping_layer: MappingLayerBuilder,
}

impl SimpleDiBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Register transient service
    ///
    /// # Example
    ///
    /// ```rust
    /// use simple_di::builder::SimpleDiBuilder;
    ///
    /// pub struct SomeService {
    ///   pub payload: String
    /// }
    ///
    /// pub struct SomeServiceDeep {
    ///   pub nested_service: SomeService
    /// }
    ///
    /// pub struct SomeServiceDeeper {
    ///   pub nested_service: SomeServiceDeep
    /// }
    ///
    /// let builder = SimpleDiBuilder::new();
    ///
    /// builder.transient(|_| Ok(SomeService { payload: "1".to_string() }));
    /// builder.transient(|sp| Ok(SomeServiceDeep { nested_service: sp.resolve()? }));
    /// builder.transient(|sp| Ok(SomeServiceDeeper { nested_service: sp.resolve()? }));
    ///
    /// let sp = builder.build();
    ///
    /// let service = sp.resolve::<SomeServiceDeeper>().unwrap();
    ///
    /// assert_eq!(service.nested_service.nested_service.payload, "1");
    ///
    /// ```
    pub fn transient<TService: 'static>(
        &self,
        factory: impl Fn(ServiceProvider) -> ServiceBuildResult<TService> + Send + Sync + 'static,
    ) -> SimpleDiBuilderService<TService> {
        self.service_layer.add_service(factory);
        self.scope_layer.add_transient::<TService>();
        self.mapping_layer
            .add_mapping::<TService, TService>(|x| Ok(x));

        SimpleDiBuilderService::new(self)
    }

    /// Register scoped service
    ///
    /// # Example
    ///
    /// ```rust
    /// use simple_di::builder::SimpleDiBuilder;
    /// use std::sync::{Arc, Mutex};
    ///
    /// pub struct SomeService {
    ///   pub payload: String
    /// }
    ///
    /// pub struct SomeServiceDeep {
    ///   pub nested_service: Arc<Mutex<SomeService>>
    /// }
    ///
    /// pub struct SomeServiceDeeper {
    ///   pub nested_service: SomeServiceDeep
    /// }
    ///
    /// let builder = SimpleDiBuilder::new();
    ///
    /// builder.singletone(|_| Ok(Arc::new(Mutex::new(SomeService { payload: "1".to_string() }))));
    /// builder.transient(|sp| Ok(SomeServiceDeep { nested_service: sp.resolve()? }));
    /// builder.transient(|sp| Ok(SomeServiceDeeper { nested_service: sp.resolve()? }));
    ///
    /// let sp = builder.build();
    ///
    /// let service = sp.resolve::<SomeServiceDeeper>().unwrap();
    ///
    /// assert_eq!(service.nested_service.nested_service.lock().unwrap().payload, "1");
    ///
    /// service.nested_service.nested_service.lock().unwrap().payload = "2".to_string();
    ///
    /// let service = sp.resolve::<SomeServiceDeeper>().unwrap();
    ///
    /// assert_eq!(service.nested_service.nested_service.lock().unwrap().payload, "2");
    ///
    /// ```
    pub fn singletone<TService: Send + Sync + Clone + 'static>(
        &self,
        factory: impl Fn(ServiceProvider) -> ServiceBuildResult<TService> + Send + Sync + 'static,
    ) -> SimpleDiBuilderService<TService> {
        self.service_layer.add_service(factory);
        self.scope_layer.add_singletone::<TService>();
        self.mapping_layer
            .add_mapping::<TService, TService>(|x| Ok(x));

        SimpleDiBuilderService::new(self)
    }

    /// Build service provider
    ///
    /// # Example
    ///
    /// ```rust
    /// use simple_di::builder::SimpleDiBuilder;
    ///
    /// pub struct SomeService {
    ///   pub payload: String
    /// }
    ///
    /// let builder = SimpleDiBuilder::new();
    ///
    /// builder.transient(|_| Ok(SomeService { payload: "1".to_string() }));
    ///
    /// let sp = builder.build();
    ///
    /// let service = sp.resolve::<SomeService>().unwrap();
    ///
    /// assert_eq!(service.payload, "1");
    ///
    /// ```
    pub fn build(self) -> ServiceProvider {
        let service_layer = self.service_layer.build();
        let scope_layer = self.scope_layer.build(service_layer);
        let mapping_layer = self.mapping_layer.build(scope_layer);

        ServiceProvider {
            mapping_layer: Arc::new(mapping_layer),
        }
    }

    /// Build service provider as gobal var
    ///
    /// # Example
    ///
    /// ```rust
    /// use simple_di::{builder::SimpleDiBuilder, ServiceProvider};
    ///
    /// pub struct SomeService {
    ///   pub payload: String
    /// }
    ///
    /// let builder = SimpleDiBuilder::new();
    ///
    /// builder.transient(|_| Ok(SomeService { payload: "1".to_string() }));
    ///
    /// builder.build_global();
    ///
    /// let service = ServiceProvider::get().unwrap().resolve::<SomeService>().unwrap();
    ///
    /// assert_eq!(service.payload, "1");
    ///
    /// ```
    pub fn build_global(self) {
        self.build().install_global();
    }
}

/// Builder for service
pub struct SimpleDiBuilderService<'a, TService: 'static> {
    pd: PhantomData<TService>,
    builder: &'a SimpleDiBuilder,
}

impl<'a, TService> SimpleDiBuilderService<'a, TService> {
    fn new(builder: &'a SimpleDiBuilder) -> Self {
        Self {
            pd: PhantomData,
            builder,
        }
    }

    /// Map service as another service
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// use simple_di::builder::SimpleDiBuilder;
    ///
    /// pub struct SomeService {
    ///   pub payload: String
    /// }
    ///
    /// pub struct SomeServiceExtra {
    ///  pub payload: String
    /// }
    ///
    /// let builder = SimpleDiBuilder::new();
    ///
    /// builder.transient(|_| Ok(SomeService {payload: "1".to_string()}))
    ///    .map_as(|x| Ok(SomeServiceExtra { payload: format!("{}2", x.payload) }));
    ///
    /// let sp = builder.build();
    ///
    /// let service = sp.resolve::<SomeService>().unwrap();
    ///
    /// assert_eq!(service.payload, "1");
    ///
    /// let service = sp.resolve::<SomeServiceExtra>().unwrap();
    ///
    /// assert_eq!(service.payload, "12");
    ///
    /// ```
    pub fn map_as<TDst: 'static>(
        &self,
        mapper: impl Fn(TService) -> ServiceBuildResult<TDst> + Sync + Send + 'static,
    ) -> &Self {
        self.builder
            .mapping_layer
            .add_mapping::<TService, TDst>(mapper);
        self
    }

    /// Map service as trait
    ///
    /// # Example
    ///
    /// ```rust
    /// use simple_di::builder::SimpleDiBuilder;
    ///
    /// pub struct SomeService {
    ///    pub payload: String
    /// }
    ///
    /// pub trait GetServicePayload {
    ///     fn get(&self) -> &str;
    /// }
    ///
    /// impl GetServicePayload for SomeService {
    ///     fn get(&self) -> &str {
    ///        &self.payload
    ///     }
    /// }
    ///
    /// let builder = SimpleDiBuilder::new();
    ///
    /// builder.transient(|_| Ok(SomeService {payload: "1".to_string()}))
    ///     .map_as_trait::<dyn GetServicePayload>();
    ///
    /// let sp = builder.build();
    ///
    /// let service = sp.resolve::<SomeService>().unwrap();
    ///
    /// assert_eq!(service.get(), "1");
    ///
    /// let boxed_service = sp.resolve::<Box<dyn GetServicePayload>>().unwrap();
    ///
    /// assert_eq!(boxed_service.get(), "1");
    ///
    /// ```
    pub fn map_as_trait<TDst: ?Sized + 'static>(&self) -> &Self
    where
        TService: Unsize<TDst> + Sized,
    {
        self.builder
            .mapping_layer
            .add_mapping::<TService, Box<TDst>>(|service| Ok(Box::new(service) as Box<TDst>));
        self
    }
}
