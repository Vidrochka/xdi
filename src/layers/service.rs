/*

Service layer contain basic build info (constructor)

*/

use std::{fmt::Debug, sync::Arc};

use ahash::AHashMap;
use dashmap::DashMap;

use crate::{types::{boxed_service::BoxedService, type_info::{TypeInfo, TypeInfoSource}}, ServiceProvider};

#[derive(Debug)]
pub struct ServiceLayer {
    services: AHashMap<TypeInfo, ServiceDescriptior>
}

impl ServiceLayer {
    pub fn get(&self, ty: TypeInfo) -> Option<ServiceDescriptior> {
        self.services.get(&ty).cloned()
    }

    pub fn new(builder: ServiceLayerBuilder) -> Self {
        ServiceLayer {
            services: builder.services.into_iter().collect()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceDescriptior {
    ty: TypeInfo,
    pub factory: ServiceFactory,
}

impl ServiceDescriptior {
    pub fn from_factory<TService: 'static>(factory: impl Fn(ServiceProvider) -> TService + Send + Sync + 'static) -> Self {
        Self {
            ty: TService::type_info(),
            factory: ServiceFactory(Arc::new(move |sp: ServiceProvider| {
                let service = factory(sp);
                BoxedService::new(service)
            }))
        }
    }
    
    pub fn ty(&self) -> TypeInfo {
        self.ty
    }
}

#[derive(Clone)]
pub struct ServiceFactory(Arc<dyn Fn(ServiceProvider) -> BoxedService + Sync + Send>);

impl ServiceFactory {
    pub fn build(&self, sp: ServiceProvider) -> BoxedService {
        (self.0)(sp)
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

    pub fn add_service<TService: 'static>(&self, factory: impl Fn(ServiceProvider) -> TService + Send + Sync + 'static) {
        self.services.insert(TService::type_info(), ServiceDescriptior::from_factory(factory));
    }

    pub fn build(self) -> ServiceLayer {
        ServiceLayer::new(self)
    }
}