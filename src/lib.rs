use layers::mapping::MappingLayer;
use types::{boxed_service::BoxedService, type_info::TypeInfo};

pub mod layers;
pub mod types;
pub mod builder;

#[cfg(test)]
pub mod tests;

pub struct ServiceProvider;

impl ServiceProvider {
    pub fn resolve<TService: 'static>() -> Option<TService> {
        MappingLayer::resolve::<TService>()
    }

    pub fn resolve_raw(ty: TypeInfo) -> Option<BoxedService> {
        MappingLayer::resolve_raw(ty)
    }
}