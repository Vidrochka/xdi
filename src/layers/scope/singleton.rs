use std::mem;

use crate::{
    ServiceProvider,
    types::{
        boxed_service::BoxedService, boxed_service_sync::SyncBoxedService,
        error::ServiceBuildResult,
    },
};

use super::{ServiceDescriptior, SyncSplitter, Syncer, UnSyncer};

/// Singletone state
pub(crate) enum SingletoneProducer {
    Pending {
        syncer: Syncer,
        splitter: SyncSplitter,
        unsyncer: UnSyncer,
    },
    Created {
        instance: SyncBoxedService,
        splitter: SyncSplitter,
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
    pub(crate) fn build(
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
