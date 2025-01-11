use std::sync::{Arc, OnceLock};

use layers::mapping::MappingLayer;
use types::{boxed_service::BoxedService, type_info::TypeInfo};

pub mod layers;
pub mod types;
pub mod builder;

#[cfg(test)]
pub mod tests;

static SERVICE_PROVIDER: OnceLock<ServiceProvider> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ServiceProvider {
    mapping_layer: Arc<MappingLayer>,
}

impl ServiceProvider {
    pub fn get<'a>() -> Option<&'a ServiceProvider> {
        SERVICE_PROVIDER.get()
    }

    pub fn resolve<TService: 'static>(&self) -> Option<TService> {
        self.mapping_layer.resolve::<TService>(self.clone())
    }

    pub fn resolve_raw(&self, ty: TypeInfo) -> Option<BoxedService> {
        self.mapping_layer.resolve_raw(ty, self.clone())
    }

    pub fn install_global(self) {
        SERVICE_PROVIDER.set(self).unwrap();
    }
}