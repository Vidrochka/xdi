use std::{fmt::Debug, sync::Arc};

use ahash::AHashMap;
use dashmap::DashMap;

use crate::{types::{boxed_service::BoxedService, error::{ServiceBuildError, ServiceBuildResult}, type_info::{TypeInfo, TypeInfoSource}}, ServiceProvider};

/// Service layer contain basic build info (constructor)
#[derive(Debug)]
pub (crate) struct ServiceLayer {
    services: AHashMap<TypeInfo, ServiceDescriptior>
}

impl ServiceLayer {
    /// Get service descriptor
    pub (crate) fn get(&self, ty: TypeInfo) -> ServiceBuildResult<ServiceDescriptior> {
        self.services.get(&ty).cloned().ok_or(ServiceBuildError::ServiceNotDound)
    }

    /// Create new service layer
    fn new(builder: ServiceLayerBuilder) -> Self {
        ServiceLayer {
            services: builder.services.into_iter().collect()
        }
    }
}

#[derive(Debug, Clone)]
pub (crate) struct ServiceDescriptior {
    ty: TypeInfo,
    factory: ServiceFactory,
}

impl ServiceDescriptior {
    /// Create new service descriptor from function factory
    fn from_factory<TService: 'static>(factory: impl Fn(ServiceProvider) -> ServiceBuildResult<TService> + Send + Sync + 'static) -> Self {
        Self {
            ty: TService::type_info(),
            factory: ServiceFactory(Arc::new(move |sp: ServiceProvider| -> ServiceBuildResult<BoxedService> {
                let service = factory(sp)?;
                Ok(BoxedService::new(service))
            }))
        }
    }
    
    /// Get service type info
    pub (crate) fn ty(&self) -> TypeInfo {
        self.ty
    }
    
    pub (crate) fn factory(&self) -> &ServiceFactory {
        &self.factory
    }
}

/// Service factory (constructor)
#[derive(Clone)]
pub (crate) struct ServiceFactory(Arc<dyn Fn(ServiceProvider) -> ServiceBuildResult<BoxedService> + Sync + Send>);

impl ServiceFactory {
    /// Build new service
    pub (crate) fn build(&self, sp: ServiceProvider) -> ServiceBuildResult<BoxedService> {
        (self.0)(sp)
    }
}

impl Debug for ServiceFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ServiceFactory").finish()
    }
}

/// Builder for service layer
#[derive(Debug)]
pub (crate) struct ServiceLayerBuilder {
    services: DashMap<TypeInfo, ServiceDescriptior, ahash::RandomState>,
}

impl ServiceLayerBuilder {
    pub (crate) fn new() -> Self {
        Self { services: Default::default() }
    }

    /// Add new service
    pub (crate) fn add_service<TService: 'static>(&self, factory: impl Fn(ServiceProvider) -> ServiceBuildResult<TService> + Send + Sync + 'static) {
        self.services.insert(TService::type_info(), ServiceDescriptior::from_factory(factory));
    }

    /// Build service layer
    pub (crate) fn build(self) -> ServiceLayer {
        ServiceLayer::new(self)
    }
}