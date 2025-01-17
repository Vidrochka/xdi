use std::mem;

use dashmap::DashMap;
use parking_lot::Mutex;

use crate::{
    ServiceProvider,
    types::{
        boxed_service::BoxedService,
        boxed_service_sync::SyncBoxedService,
        error::{ServiceBuildError, ServiceBuildResult},
        type_info::TypeInfo,
    },
};

use super::{ServiceDescriptior, SyncSplitter, Syncer, UnSyncer};

tokio::task_local! {
    static TASK_LOCAL_CTX: TaskLocalCtx;
}

#[derive(Debug, Default)]
pub(crate) struct TaskLocalCtx {
    instances: DashMap<TypeInfo, Mutex<TaskLocalProducer>, ahash::RandomState>,
}

impl TaskLocalCtx {
    pub(crate) async fn span<F: Future>(f: F) -> F::Output {
        TASK_LOCAL_CTX.scope(TaskLocalCtx::default(), f).await
    }

    pub(crate) fn get(
        ty: TypeInfo,
        service_descriptor: ServiceDescriptior,
        sp: ServiceProvider,
        ctr_methods: &TaskLocalCtrMethods,
    ) -> ServiceBuildResult<BoxedService> {
        TASK_LOCAL_CTX
            .try_with(|ctx| ctx.resolve(ty, service_descriptor, sp, ctr_methods))
            .map_err(|_| ServiceBuildError::TaskLocalContextNotInitialized { ty })?
    }

    fn resolve(
        &self,
        ty: TypeInfo,
        service_descriptor: ServiceDescriptior,
        sp: ServiceProvider,
        ctr_methods: &TaskLocalCtrMethods,
    ) -> ServiceBuildResult<BoxedService> {
        self.instances
            .entry(ty)
            .or_insert_with(|| Mutex::new(TaskLocalProducer::Pending))
            .downgrade()
            .lock()
            .produce(service_descriptor, sp, ctr_methods)
    }
}

pub enum TaskLocalProducer {
    Pending,
    Created { instance: SyncBoxedService },
}

impl TaskLocalProducer {
    fn produce(
        &mut self,
        service_descriptor: ServiceDescriptior,
        sp: ServiceProvider,
        ctr_methods: &TaskLocalCtrMethods,
    ) -> ServiceBuildResult<BoxedService> {
        let old_val = mem::replace(self, Self::Pending);

        let service = match old_val {
            Self::Pending => {
                let service = service_descriptor.factory().build(sp)?;

                (ctr_methods.syncer)(service)?
            }
            Self::Created { instance } => instance,
        };

        let (instance, copy) = (ctr_methods.splitter)(service)?;

        let copy = (ctr_methods.unsyncer)(copy)?;

        *self = Self::Created { instance };

        Ok(copy)
    }
}

impl std::fmt::Debug for TaskLocalProducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.debug_struct("Pending").finish(),
            Self::Created { .. } => f.debug_struct("Created").finish(),
        }
    }
}

pub(crate) struct TaskLocalCtrMethods {
    syncer: Syncer,
    splitter: SyncSplitter,
    unsyncer: UnSyncer,
}

impl TaskLocalCtrMethods {
    pub(crate) fn new(syncer: Syncer, splitter: SyncSplitter, unsyncer: UnSyncer) -> Self {
        Self {
            syncer,
            splitter,
            unsyncer,
        }
    }
}

impl std::fmt::Debug for TaskLocalCtrMethods {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskLocalCtrMethods")
            .field("syncer", &"fn")
            .field("splitter", &"fn")
            .field("unsyncer", &"fn")
            .finish()
    }
}
