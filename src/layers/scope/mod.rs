mod builder;
pub(crate) use builder::*;

mod singleton;
use singleton::SingletoneProducer;

#[cfg(feature = "task-local")]
mod task_local;
#[cfg(feature = "task-local")]
use task_local::TaskLocalCtrMethods;
#[cfg(feature = "task-local")]
pub(crate) use task_local::TaskLocalCtx;

mod thread_local;

use ahash::AHashMap;
use parking_lot::Mutex;
use thread_local::{ThreadLocalCtrMethods, ThreadLocalCtx};

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
    pub(crate) service_layer: ServiceLayer,
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
            .ok_or(ServiceBuildError::MappingNotFound { ty })?;

        let service = self.service_layer.get(ty)?;

        assert_eq!(scope.ty(), ty);
        assert_eq!(scope.ty(), service.ty());

        match &scope.scope {
            Scope::Transient => service.factory().build(sp),
            Scope::Singletone(singletone_state) => {
                let mut singletone_state_lock = singletone_state.lock();

                return singletone_state_lock.build(service, sp);
            }
            #[cfg(feature = "task-local")]
            Scope::TaskLocal(cfr_methods) => {
                TaskLocalCtx::get(scope.ty(), service, sp, cfr_methods)
            }
            Scope::ThreadLocal(cfr_methods) => {
                ThreadLocalCtx::get(scope.ty(), service, sp, cfr_methods)
            }
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
pub(crate) struct ServiceScopeDescriptior {
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

    #[cfg(feature = "task-local")]
    /// Create new task local service scope descriptor
    fn task_local<TService: 'static + Sync + Send + Clone>() -> Self {
        use task_local::TaskLocalCtrMethods;

        Self {
            ty: TService::type_info(),
            scope: Scope::TaskLocal(TaskLocalCtrMethods::new(
                Box::new(|service| {
                    let service = service.unbox::<TService>().map_err(|e| {
                        ServiceBuildError::InvalidScopeLayerBoxedInputType {
                            expected: TService::type_info(),
                            found: e.ty(),
                        }
                    })?;
                    Ok(SyncBoxedService::new(service))
                }),
                Box::new(|service| {
                    let service = service.unbox::<TService>().map_err(|e| {
                        ServiceBuildError::UnexpectedSingletoneSplitterParams {
                            expected: TService::type_info(),
                            found: e.ty(),
                        }
                    })?;

                    let copy = service.clone();

                    Ok((SyncBoxedService::new(service), SyncBoxedService::new(copy)))
                }),
                Box::new(|service| {
                    let service = service.unbox::<TService>().map_err(|e| {
                        ServiceBuildError::InvalidScopeLayerBoxedOutputType {
                            expected: TService::type_info(),
                            found: e.ty(),
                        }
                    })?;

                    Ok(BoxedService::new(service))
                }),
            )),
        }
    }

    /// Create new task local service scope descriptor
    fn thread_local<TService: 'static + Clone>() -> Self {
        Self {
            ty: TService::type_info(),
            scope: Scope::ThreadLocal(ThreadLocalCtrMethods::new(Box::new(|service| {
                let service = service.unbox::<TService>().map_err(|e| {
                    ServiceBuildError::UnexpectedSingletoneSplitterParams {
                        expected: TService::type_info(),
                        found: e.ty(),
                    }
                })?;

                let copy = service.clone();

                Ok((BoxedService::new(service), BoxedService::new(copy)))
            }))),
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
    #[cfg(feature = "task-local")]
    TaskLocal(TaskLocalCtrMethods),
    ThreadLocal(ThreadLocalCtrMethods),
}

/// Syncer - Замыкание для конвертации !sync объекта в sync (требуется для sync замыкания разделителя singletone)
type Syncer = Box<dyn Fn(BoxedService) -> ServiceBuildResult<SyncBoxedService> + Send + Sync>;
/// Syncer - Замыкание для конвертации sync объекта в !sync (требуется для sync замыкания разделителя singletone)
type UnSyncer = Box<dyn Fn(SyncBoxedService) -> ServiceBuildResult<BoxedService> + Send + Sync>;
/// SyncSplitter - Замыкание для разделения объекта на два (требуется для singletone, task-local). Треьует sync сервис чтобы быть sync
type SyncSplitter = Box<
    dyn Fn(SyncBoxedService) -> ServiceBuildResult<(SyncBoxedService, SyncBoxedService)>
        + Send
        + Sync,
>;
/// SyncSplitter - Замыкание для разделения объекта на два (требуется для thread-local)
type Splitter =
    Box<dyn Fn(BoxedService) -> ServiceBuildResult<(BoxedService, BoxedService)> + Send + Sync>;
