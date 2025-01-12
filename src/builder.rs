use std::{marker::{PhantomData, Unsize}, sync::Arc};

use crate::{layers::{mapping::MappingLayerBuilder, scope::ScopeLayerBuilder, service::ServiceLayerBuilder}, types::error::ServiceBuildResult, ServiceProvider};


#[derive(Debug)]
pub struct SimpleDiBuilder {
    service_layer: ServiceLayerBuilder,
    scope_layer: ScopeLayerBuilder,
    mapping_layer: MappingLayerBuilder
}

impl SimpleDiBuilder {
    pub fn new() -> Self {
        Self {
            service_layer: ServiceLayerBuilder::new(),
            scope_layer: ScopeLayerBuilder::new(),
            mapping_layer: MappingLayerBuilder::new(),
        }
    }

    pub fn transient<TService: 'static>(&self, factory: impl Fn(ServiceProvider) -> ServiceBuildResult<TService> + Send + Sync + 'static) -> SimpleDiBuilderService<TService> {
        self.service_layer.add_service(factory);
        self.scope_layer.add_transient::<TService>();
        self.mapping_layer.add_mapping::<TService, TService>(|x| Ok(x));

        SimpleDiBuilderService::new(self)
    }

    pub fn singletone<TService: Send + Sync + Clone + 'static>(&self, factory: impl Fn(ServiceProvider) -> ServiceBuildResult<TService> + Send + Sync + 'static) -> SimpleDiBuilderService<TService> {
        self.service_layer.add_service(factory);
        self.scope_layer.add_singletone::<TService>();
        self.mapping_layer.add_mapping::<TService, TService>(|x| Ok(x));

        SimpleDiBuilderService::new(self)
    }

    pub fn build(self) -> ServiceProvider {
        let service_layer = self.service_layer.build();
        let scope_layer = self.scope_layer.build(service_layer);
        let mapping_layer = self.mapping_layer.build(scope_layer);

        ServiceProvider { mapping_layer: Arc::new(mapping_layer) }
    }

    pub fn build_global(self) {
        self.build().install_global();
    } 
}

pub struct SimpleDiBuilderService<'a, TService: 'static> {
    pd: PhantomData<TService>,
    builder: &'a SimpleDiBuilder,
}

impl<'a, TService> SimpleDiBuilderService<'a, TService> {
    pub fn new(builder: &'a SimpleDiBuilder) -> Self {
        Self { pd: PhantomData, builder }
    }

    pub fn map_as<TDst: 'static>(&self, mapper: impl Fn(TService) -> ServiceBuildResult<TDst> + Sync + Send + 'static) -> &Self {
        self.builder.mapping_layer.add_mapping::<TService, TDst>(mapper);
        self
    }

    pub fn map_as_trait<TDst: ?Sized + 'static>(&self) -> &Self
    where
        TService: Unsize<TDst> + Sized,
    {
        self.builder.mapping_layer.add_mapping::<TService, Box<TDst>>(|service| Ok(Box::new(service) as Box<TDst>));
        self
    }
}