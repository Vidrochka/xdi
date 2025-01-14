use std::any::Any;

use super::type_info::{TypeInfo, TypeInfoSource};

#[derive(Debug)]
pub struct BoxedService {
    ty: TypeInfo,
    service: Box<dyn Any>,
}

impl BoxedService {
    pub fn new<TService: 'static>(service: TService) -> Self {
        Self {
            service: Box::new(service),
            ty: TService::type_info(),
        }
    }

    pub fn unbox<TService: 'static>(self) -> Result<TService, Self> {
        match self.service.downcast() {
            Ok(service) => Ok(*service),
            Err(service) => Err(BoxedService {
                service,
                ty: self.ty,
            }),
        }
    }

    pub fn ty(&self) -> TypeInfo {
        self.ty
    }
}
