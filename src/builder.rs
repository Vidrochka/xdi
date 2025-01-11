use std::sync::Arc;

use crate::{layers::{mapping::MappingLayerBuilder, scope::ScopeLayerBuilder, service::ServiceLayerBuilder}, ServiceProvider};


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

    pub fn transient<TService: 'static>(&self, factory: impl Fn(ServiceProvider) -> TService + Send + Sync + 'static) {
        self.service_layer.add_service(factory);
        self.scope_layer.add_transient::<TService>();
        self.mapping_layer.add_mapping::<TService, TService>(|x| x);
    }

    pub fn singletone<TService: Send + Sync + Clone + 'static>(&self, factory: impl Fn(ServiceProvider) -> TService + Send + Sync + 'static) {
        self.service_layer.add_service(factory);
        self.scope_layer.add_singletone::<TService>();
        self.mapping_layer.add_mapping::<TService, TService>(|x| x);
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

