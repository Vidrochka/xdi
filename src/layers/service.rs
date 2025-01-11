/*

Service layer contain basic build info (constructor)

*/

use std::{fmt::Debug, sync::{Arc, OnceLock}};

use ahash::AHashMap;
use dashmap::DashMap;

use crate::types::{boxed_service::BoxedService, type_info::{TypeInfo, TypeInfoSource}};

static SERVICE_LAYER: OnceLock<ServiceLayer> = OnceLock::new();

#[derive(Debug)]
pub struct ServiceLayer {
    services: AHashMap<TypeInfo, ServiceDescriptior>
}

impl ServiceLayer {
    pub fn get(ty: TypeInfo) -> Option<ServiceDescriptior> {
        let service_layer = SERVICE_LAYER.get()?;

        service_layer.services.get(&ty).cloned()
    }

    pub fn set(builder: ServiceLayerBuilder) {
        SERVICE_LAYER.set(ServiceLayer {
            services: builder.services.into_iter().collect()
        }).unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct ServiceDescriptior {
    ty: TypeInfo,
    pub factory: ServiceFactory,
}

impl ServiceDescriptior {
    pub fn from_factory<TService: 'static>(factory: impl Fn() -> TService + Send + Sync + 'static) -> Self {
        Self {
            ty: TService::type_info(),
            factory: ServiceFactory(Arc::new(move || {
                let service = factory();
                BoxedService::new(service)
            }))
        }
    }
    
    pub fn ty(&self) -> TypeInfo {
        self.ty
    }
}

#[derive(Clone)]
pub struct ServiceFactory(Arc<dyn Fn() -> BoxedService + Sync + Send>);

impl ServiceFactory {
    pub fn build(&self) -> BoxedService {
        (self.0)()
    }
}

impl Debug for ServiceFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ServiceFactory").finish()
    }
}

#[derive(Debug)]
pub struct ServiceLayerBuilder {
    services: DashMap<TypeInfo, ServiceDescriptior, ahash::RandomState>,
}

impl ServiceLayerBuilder {
    pub fn new() -> Self {
        Self { services: Default::default() }
    }

    pub fn add_service<TService: 'static>(&self, factory: impl Fn() -> TService + Send + Sync + 'static) {
        self.services.insert(TService::type_info(), ServiceDescriptior::from_factory(factory));
    }

    pub fn build(self) {
        ServiceLayer::set(self);
    }
}