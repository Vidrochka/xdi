use dashmap::DashMap;

use crate::types::type_info::{TypeInfo, TypeInfoSource};

use super::{ScopeLayer, ServiceLayer, ServiceScopeDescriptior};

#[derive(Debug, Default)]
pub(crate) struct ScopeLayerBuilder {
    pub(crate) scopes: DashMap<TypeInfo, ServiceScopeDescriptior, ahash::RandomState>,
}

impl ScopeLayerBuilder {
    pub(crate) fn add_transient<TService: 'static>(&self) {
        self.scopes.insert(
            TService::type_info(),
            ServiceScopeDescriptior::transient::<TService>(),
        );
    }

    pub(crate) fn add_singletone<TService: 'static + Send + Sync + Clone>(&self) {
        self.scopes.insert(
            TService::type_info(),
            ServiceScopeDescriptior::singletone::<TService>(),
        );
    }

    #[cfg(feature = "task-local")]
    pub(crate) fn add_task_local<TService: 'static + Sync + Send + Clone>(&self) {
        self.scopes.insert(
            TService::type_info(),
            ServiceScopeDescriptior::task_local::<TService>(),
        );
    }

    pub(crate) fn add_thread_local<TService: 'static + Clone>(&self) {
        self.scopes.insert(
            TService::type_info(),
            ServiceScopeDescriptior::thread_local::<TService>(),
        );
    }

    pub(crate) fn build(self, service_layer: ServiceLayer) -> ScopeLayer {
        ScopeLayer::new(self, service_layer)
    }
}
