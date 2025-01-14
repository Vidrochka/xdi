use std::{any::Any, sync::Arc};

use super::type_info::{TypeInfo, TypeInfoSource};

#[derive(Clone)]
pub struct ArcService {
    ty: TypeInfo,
    service: Arc<dyn Any + Send + Sync>,
}

impl ArcService {
    pub fn new<TService: 'static + Send + Sync>(service: TService) -> Self {
        Self {
            service: Arc::new(service),
            ty: TService::type_info(),
        }
    }

    pub fn unbox_ref<TService: 'static>(&self) -> Option<&TService> {
        self.service.downcast_ref()
    }

    pub fn clone_unbox<TService: 'static + Clone + Sync + Send>(self) -> Result<TService, Self> {
        match self.service.downcast::<TService>() {
            Ok(service) => Ok(service.as_ref().clone()),
            Err(service) => Err(ArcService {
                service,
                ty: self.ty,
            }),
        }
    }

    pub fn ty(&self) -> TypeInfo {
        self.ty
    }
}
