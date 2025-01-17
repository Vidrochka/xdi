use std::mem;

use dashmap::DashMap;
use parking_lot::Mutex;

use crate::{
    ServiceProvider,
    types::{
        boxed_service::BoxedService,
        error::{ServiceBuildError, ServiceBuildResult},
        type_info::TypeInfo,
    },
};

use super::{ServiceDescriptior, Splitter};

thread_local! {
    static THREAD_LOCAL_CTX: ThreadLocalCtx = ThreadLocalCtx::default();
}

#[derive(Debug, Default)]
pub(crate) struct ThreadLocalCtx {
    instances: DashMap<TypeInfo, Mutex<ThreadLocalProducer>, ahash::RandomState>,
}

impl ThreadLocalCtx {
    pub(crate) fn get(
        ty: TypeInfo,
        service_descriptor: ServiceDescriptior,
        sp: ServiceProvider,
        ctr_methods: &ThreadLocalCtrMethods,
    ) -> ServiceBuildResult<BoxedService> {
        THREAD_LOCAL_CTX
            .try_with(|ctx| ctx.resolve(ty, service_descriptor, sp, ctr_methods))
            .map_err(|_| ServiceBuildError::ThreadLocalContextNotInitialized { ty })?
    }

    fn resolve(
        &self,
        ty: TypeInfo,
        service_descriptor: ServiceDescriptior,
        sp: ServiceProvider,
        ctr_methods: &ThreadLocalCtrMethods,
    ) -> ServiceBuildResult<BoxedService> {
        self.instances
            .entry(ty)
            .or_insert_with(|| Mutex::new(ThreadLocalProducer::Pending))
            .downgrade()
            .lock()
            .produce(service_descriptor, sp, ctr_methods)
    }
}

pub enum ThreadLocalProducer {
    Pending,
    Created { instance: BoxedService },
}

impl ThreadLocalProducer {
    fn produce(
        &mut self,
        service_descriptor: ServiceDescriptior,
        sp: ServiceProvider,
        ctr_methods: &ThreadLocalCtrMethods,
    ) -> ServiceBuildResult<BoxedService> {
        let old_val = mem::replace(self, Self::Pending);

        let service = match old_val {
            Self::Pending => service_descriptor.factory().build(sp)?,
            Self::Created { instance } => instance,
        };

        let (instance, copy) = (ctr_methods.splitter)(service)?;

        *self = Self::Created { instance };

        Ok(copy)
    }
}

impl std::fmt::Debug for ThreadLocalProducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.debug_struct("Pending").finish(),
            Self::Created { .. } => f.debug_struct("Created").finish(),
        }
    }
}

pub(crate) struct ThreadLocalCtrMethods {
    splitter: Splitter,
}

impl ThreadLocalCtrMethods {
    pub(crate) fn new(splitter: Splitter) -> Self {
        Self { splitter }
    }
}

impl std::fmt::Debug for ThreadLocalCtrMethods {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadLocalCtrMethods")
            .field("splitter", &"fn")
            .finish()
    }
}
