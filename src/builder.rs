use crate::layers::{mapping::MappingLayerBuilder, scope::ScopeLayerBuilder, service::ServiceLayerBuilder};


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

    pub fn transient<TService: 'static>(&self, factory: impl Fn() -> TService + Send + Sync + 'static) {
        self.service_layer.add_service(factory);
        self.scope_layer.add_transient::<TService>();
        self.mapping_layer.add_mapping::<TService, TService>(|x| x);
    }

    pub fn singletone<TService: Send + Sync + Clone + 'static>(&self, factory: impl Fn() -> TService + Send + Sync + 'static) {
        self.service_layer.add_service(factory);
        self.scope_layer.add_singletone::<TService>();
        self.mapping_layer.add_mapping::<TService, TService>(|x| x);
    }

    pub fn build(self) {
        self.service_layer.build();
        self.scope_layer.build();
        self.mapping_layer.build();
    }
}

