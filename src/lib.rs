#![feature(unsize)]
#![feature(iterator_try_collect)]
#![feature(slice_as_array)]

use std::sync::{Arc, OnceLock};

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
    mapping_layer: Arc<MappingLayer>,
}

impl ServiceProvider {
    pub fn get<'a>() -> Option<&'a ServiceProvider> {
        SERVICE_PROVIDER.get()
    }

    pub fn resolve<TService: 'static>(&self) -> ServiceBuildResult<TService> {
        self.mapping_layer.resolve::<TService>(self.clone())
    }

    pub fn resolve_raw(&self, ty: TypeInfo) -> ServiceBuildResult<BoxedService> {
        self.mapping_layer.resolve_raw(ty, self.clone())
    }

    pub fn resolve_all<TService: 'static>(&self) -> ServiceBuildResult<Vec<TService>> {
        self.mapping_layer.resolve_all::<TService>(self.clone())
    }

    pub fn resolve_all_raw(&self, ty: TypeInfo) -> ServiceBuildResult<Vec<BoxedService>> {
        self.mapping_layer.resolve_all_raw(ty, self.clone())
    }

    pub fn install_global(self) {
        SERVICE_PROVIDER.set(self).unwrap();
    }
}
