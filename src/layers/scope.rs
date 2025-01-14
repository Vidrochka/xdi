use std::mem;

use ahash::AHashMap;
use dashmap::DashMap;
use parking_lot::Mutex;

use crate::{
    ServiceProvider,
    types::{
        boxed_service::BoxedService,
        boxed_service_sync::SyncBoxedService,
        error::{ServiceBuildError, ServiceBuildResult},
        type_info::{TypeInfo, TypeInfoSource},
    },
};

use super::service::{ServiceDescriptior, ServiceLayer};

/// Scope layer apply scope filter (clone/build singletone, clone/build task, build transient)
#[derive(Debug)]
pub(crate) struct ScopeLayer {
    service_layer: ServiceLayer,
    scopes: AHashMap<TypeInfo, ServiceScopeDescriptior>,
}

impl ScopeLayer {
    /// Get service throw scope layer
    pub(crate) fn get(
        &self,
        ty: TypeInfo,
        sp: ServiceProvider,
    ) -> ServiceBuildResult<BoxedService> {
        let scope = self
            .scopes
            .get(&ty)
            .ok_or(ServiceBuildError::MappingNotFound)?;

        let service = self.service_layer.get(ty)?;

        assert_eq!(scope.ty(), ty);
        assert_eq!(scope.ty(), service.ty());

        match &scope.scope {
            Scope::Transient => service.factory().build(sp),
            Scope::Singletone(singletone_state) => {
                let mut singletone_state_lock = singletone_state.lock();

                return singletone_state_lock.build(service, sp);
            }
            Scope::Task => todo!(),
        }
    }

    /// Create new scope layer
    fn new(builder: ScopeLayerBuilder, service_layer: ServiceLayer) -> Self {
        ScopeLayer {
            service_layer,
            scopes: builder.scopes.into_iter().collect(),
        }
    }
}

/// Service scope descriptor
#[derive(Debug)]
struct ServiceScopeDescriptior {
    ty: TypeInfo,
    scope: Scope,
}

impl ServiceScopeDescriptior {
    /// Create new transient service scope descriptor
    fn transient<TService: 'static>() -> Self {
        Self {
            ty: TService::type_info(),
            scope: Scope::Transient,
        }
    }

    /// Create new task local service scope descriptor
    fn task<TService: 'static>() -> Self {
        Self {
            ty: TService::type_info(),
            scope: Scope::Task,
        }
    }

    /// Create new singletone service scope descriptor
    fn singletone<TService: 'static + Sync + Send + Clone>() -> Self {
        Self {
            ty: TService::type_info(),
            scope: Scope::Singletone(Mutex::new(SingletoneProducer::Pending {
                syncer: Box::new(|service| {
                    let service = service.unbox::<TService>().map_err(|e| {
                        ServiceBuildError::InvalidScopeLayerBoxedInputType {
                            expected: TService::type_info(),
                            found: e.ty(),
                        }
                    })?;
                    Ok(SyncBoxedService::new(service))
                }),
                splitter: Box::new(|service| {
                    let service = service.unbox::<TService>().map_err(|e| {
                        ServiceBuildError::UnexpectedSingletoneSplitterParams {
                            expected: TService::type_info(),
                            found: e.ty(),
                        }
                    })?;

                    let copy = service.clone();

                    Ok((SyncBoxedService::new(service), SyncBoxedService::new(copy)))
                }),
                unsyncer: Box::new(|service| {
                    let service = service.unbox::<TService>().map_err(|e| {
                        ServiceBuildError::InvalidScopeLayerBoxedOutputType {
                            expected: TService::type_info(),
                            found: e.ty(),
                        }
                    })?;

                    Ok(BoxedService::new(service))
                }),
            })),
        }
    }

    /// Get service scope type info
    fn ty(&self) -> TypeInfo {
        self.ty
    }
}

/// Service scope kinds
#[derive(Debug)]
enum Scope {
    Transient,
    // TODO: возможно стоит переделать на RwLock, пока непонятно на сколько такое усложнение обосновано
    Singletone(Mutex<SingletoneProducer>),
    Task,
}

/// Syncer - Замыкание для конвертации !sync объекта в sync (требуется для sync замыкания разделителя singletone)
type Syncer = Box<dyn Fn(BoxedService) -> ServiceBuildResult<SyncBoxedService> + Send + Sync>;
/// Syncer - Замыкание для конвертации sync объекта в !sync (требуется для sync замыкания разделителя singletone)
type UnSyncer = Box<dyn Fn(SyncBoxedService) -> ServiceBuildResult<BoxedService> + Send + Sync>;
/// SingletoneSplitter - Замыкание для разделения объекта на два (требуется для singletone). Треьует sync сервис чтобы быть sync
type SingletoneSplitter = Box<
    dyn Fn(SyncBoxedService) -> ServiceBuildResult<(SyncBoxedService, SyncBoxedService)>
        + Send
        + Sync,
>;

/// Singletone state
enum SingletoneProducer {
    Pending {
        syncer: Syncer,
        splitter: SingletoneSplitter,
        unsyncer: UnSyncer,
    },
    Created {
        instance: SyncBoxedService,
        splitter: SingletoneSplitter,
        unsyncer: UnSyncer,
    },
    Empty,
}

impl SingletoneProducer {
    /// Check if singletone is pending
    #[allow(unused)]
    fn pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }

    /// Create new singletone instance
    fn build(
        &mut self,
        service_descriptor: ServiceDescriptior,
        sp: ServiceProvider,
    ) -> ServiceBuildResult<BoxedService> {
        let old_val = mem::replace(self, Self::Empty);

        match old_val {
            SingletoneProducer::Pending {
                syncer,
                splitter,
                unsyncer,
            } => {
                let service = service_descriptor.factory().build(sp)?;

                let service = syncer(service)?;

                let (instance, copy) = splitter(service)?;

                let copy = unsyncer(copy)?;

                *self = SingletoneProducer::Created {
                    instance,
                    splitter,
                    unsyncer,
                };

                Ok(copy)
            }
            SingletoneProducer::Created {
                instance,
                splitter,
                unsyncer,
            } => {
                let (instance, copy) = splitter(instance)?;

                let copy = unsyncer(copy)?;

                *self = SingletoneProducer::Created {
                    instance,
                    splitter,
                    unsyncer,
                };

                Ok(copy)
            }
            SingletoneProducer::Empty => unreachable!("Empty state only for data transition"),
        }
    }
}

impl std::fmt::Debug for SingletoneProducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending { .. } => f.debug_struct("Pending").finish(),
            Self::Created { .. } => f.debug_struct("Created").finish(),
            Self::Empty { .. } => f.debug_struct("Empty").finish(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ScopeLayerBuilder {
    scopes: DashMap<TypeInfo, ServiceScopeDescriptior, ahash::RandomState>,
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

    #[allow(unused)]
    pub(crate) fn add_task<TService: 'static>(&self) {
        self.scopes.insert(
            TService::type_info(),
            ServiceScopeDescriptior::task::<TService>(),
        );
    }

    pub(crate) fn build(self, service_layer: ServiceLayer) -> ScopeLayer {
        ScopeLayer::new(self, service_layer)
    }
}
