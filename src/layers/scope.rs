/*

Scope layer apply scope filter (clone/build singletone, clone/build task, build transient)

*/

use std::mem;

use ahash::AHashMap;
use dashmap::DashMap;
use parking_lot::Mutex;

use crate::{types::{boxed_service::BoxedService, boxed_service_sync::SyncBoxedService, type_info::{TypeInfo, TypeInfoSource}}, ServiceProvider};

use super::service::{ServiceDescriptior, ServiceLayer};

#[derive(Debug)]
pub struct ScopeLayer {
    service_layer: ServiceLayer,
    scopes: AHashMap<TypeInfo, ServiceScopeDescriptior>,
}

impl ScopeLayer {
    pub fn get(&self, ty: TypeInfo, sp: ServiceProvider) -> Option<BoxedService> {
        let service = self.service_layer.get(ty)?;

        let Some(scope) = self.scopes.get(&ty) else {
            return Some(service.factory.build(sp));
        };

        assert_eq!(scope.ty(), ty);
        assert_eq!(scope.ty(), service.ty());
        
        match &scope.scope {
            Scope::Transient => Some(service.factory.build(sp)),
            Scope::Singletone(singletone_state) => {
                let mut singletone_state_lock = singletone_state.lock();

                let service =  singletone_state_lock.build(service, sp);

                return Some(service);                
            },
            Scope::Task => todo!(),
        }
    }

    pub fn new(builder: ScopeLayerBuilder, service_layer: ServiceLayer) -> Self {
        ScopeLayer {
            service_layer,
            scopes: builder.scopes.into_iter().collect()
        }
    }
}

#[derive(Debug)]
pub struct ServiceScopeDescriptior {
    ty: TypeInfo,
    scope: Scope,
}

impl ServiceScopeDescriptior {
    pub fn transient<TService: 'static>() -> Self {
        Self {
            ty: TService::type_info(),
            scope: Scope::Transient
        }
    }

    pub fn task<TService: 'static>() -> Self {
        Self {
            ty: TService::type_info(),
            scope: Scope::Task
        }
    }

    pub fn singletone<TService: 'static + Sync + Send + Clone>() -> Self {
        Self {
            ty: TService::type_info(),
            scope: Scope::Singletone(
                Mutex::new(SingletoneProducer::Pending {
                    syncer: Box::new(|service| {
                        let service = service.unbox::<TService>().unwrap();
                        SyncBoxedService::new(service)
                    }),
                    splitter: Box::new(|service| {
                        let service = service.unbox::<TService>().unwrap();
                        let copy = service.clone();
                        (SyncBoxedService::new(service), SyncBoxedService::new(copy))
                    }),
                    unsyncer: Box::new(|service| {
                        let service = service.unbox::<TService>().unwrap();
                        BoxedService::new(service)
                    }),
                })
            )
        }
    }

    pub fn ty(&self) -> TypeInfo {
        self.ty
    }
}

#[derive(Debug)]
pub enum Scope {
    Transient,
    // TODO: возможно стоит переделать на RwLock, пока непонятно на сколько такое усложнение обосновано
    Singletone(Mutex<SingletoneProducer>),
    Task,
}

type Syncer = Box<dyn Fn(BoxedService) -> SyncBoxedService + Send + Sync>;
type UnSyncer = Box<dyn Fn(SyncBoxedService) -> BoxedService + Send + Sync>;
type SingletoneSplitter = Box<dyn Fn(SyncBoxedService) -> (SyncBoxedService, SyncBoxedService) + Send + Sync>;

pub enum SingletoneProducer {
    Pending {
        // syncer и unsyncer нужны чтобы в SingletoneProducer хранить ссылку как sync + send, но не заставлять service слой зависить от этого
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
    pub fn pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }

    pub fn build(&mut self, service_descriptor: ServiceDescriptior, sp: ServiceProvider) -> BoxedService {
        let old_val = mem::replace(self, Self::Empty);

        match old_val {
            SingletoneProducer::Pending { syncer, splitter, unsyncer, } => {
                let service = service_descriptor.factory.build(sp);

                let service = syncer(service);

                let (instance, copy) = splitter(service);

                let copy = unsyncer(copy);

                *self = SingletoneProducer::Created {
                    instance,
                    splitter,
                    unsyncer,
                };

                copy
            },
            SingletoneProducer::Created { instance, splitter, unsyncer } => {
                let (instance, copy) = splitter(instance);
                
                let copy = unsyncer(copy);

                *self = SingletoneProducer::Created {
                    instance,
                    splitter,
                    unsyncer,
                };

                copy
            },
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

#[derive(Debug)]
pub struct ScopeLayerBuilder {
    scopes: DashMap<TypeInfo, ServiceScopeDescriptior, ahash::RandomState>,
}

impl ScopeLayerBuilder {
    pub fn new() -> Self {
        Self { scopes: Default::default() }
    }

    pub fn add_transient<TService: 'static>(&self) {
        self.scopes.insert(TService::type_info(), ServiceScopeDescriptior::transient::<TService>());
    }

    pub fn add_singletone<TService: 'static + Send + Sync + Clone>(&self) {
        self.scopes.insert(TService::type_info(), ServiceScopeDescriptior::singletone::<TService>());
    }

    pub fn add_task<TService: 'static>(&self) {
        self.scopes.insert(TService::type_info(), ServiceScopeDescriptior::task::<TService>());
    }
    
    pub fn build(self, service_layer: ServiceLayer) -> ScopeLayer {
        ScopeLayer::new(self, service_layer)
    }
}