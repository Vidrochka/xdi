/*

Mapping allow convert any type to any other type

Example:

struct T {}

trait Tr {}

impl Tr for T {}

Then in abstraction convert service T to Box<dyn Tr>

*/

use ahash::AHashMap;
use dashmap::DashMap;

use crate::{types::{boxed_service::BoxedService, error::{ServiceBuildError, ServiceBuildResult}, type_info::{TypeInfo, TypeInfoSource}}, ServiceProvider};

use super::scope::ScopeLayer;

#[derive(Debug)]
pub struct MappingLayer {
    scope_layer: ScopeLayer,
    mappings: AHashMap<TypeInfo, Vec<MappingDescriptor>>
}

impl MappingLayer {
    pub fn resolve_raw(&self, ty: TypeInfo, sp: ServiceProvider) -> ServiceBuildResult<BoxedService> {
        let mapping = self.mappings.get(&ty)
            .and_then(|x| x.first())
            .ok_or(ServiceBuildError::MappingNotFound)?;

        let service= self.scope_layer.get(mapping.src_ty(), sp)?;

        assert_eq!(mapping.dest_ty(), ty);
        assert_eq!(mapping.src_ty(), service.ty());

        mapping.mapper.map(service)
    }

    pub fn resolve<TService: 'static>(&self, sp: ServiceProvider) -> ServiceBuildResult<TService> {
        let ty = TService::type_info();

        let service = self.resolve_raw(ty, sp)?;

        service.unbox::<TService>()
            .map_err(|e| ServiceBuildError::InvalidMappingLayerBoxedOutputType {
                expected: TService::type_info(),
                found: e.ty()
            })
    }

    pub fn new(builder: MappingLayerBuilder, scope_layer: ScopeLayer) -> Self {
        MappingLayer {
            scope_layer,
            mappings: builder.mappings.into_iter().collect()
        }
    }
}

#[derive(Debug)]
pub struct MappingDescriptor {
    src_ty: TypeInfo,
    dest_ty: TypeInfo,
    mapper: ServiceMapper,
}

impl MappingDescriptor {
    pub fn new<TSrc: 'static, TDst: 'static>(mapper: impl Fn(TSrc) -> ServiceBuildResult<TDst> + Send + Sync + 'static) -> Self {
        Self {
            src_ty: TSrc::type_info(),
            dest_ty: TSrc::type_info(),
            mapper: ServiceMapper(Box::new(move |service| {
                let service = service.unbox::<TSrc>()
                    .map_err(|e| ServiceBuildError::InvalidMappingLayerBoxedInputType {
                        expected: TSrc::type_info(),
                        found: e.ty()
                    })?;

                let service = mapper(service)?;

                Ok(BoxedService::new(service))
            }))
        }
    }
    
    pub fn src_ty(&self) -> TypeInfo {
        self.src_ty
    }
    
    pub fn dest_ty(&self) -> TypeInfo {
        self.dest_ty
    }
}

pub struct ServiceMapper(Box<dyn Fn(BoxedService) -> ServiceBuildResult<BoxedService> + Send + Sync>);

impl ServiceMapper {
    pub fn new(converter: impl Fn(BoxedService) -> ServiceBuildResult<BoxedService> + 'static + Send + Sync) -> Self {
        Self(Box::new(converter))
    }

    pub fn map(&self, service: BoxedService) -> ServiceBuildResult<BoxedService> {
        (self.0)(service)
    }
}

impl std::fmt::Debug for ServiceMapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ServiceTransormer").finish()
    }
}

#[derive(Debug)]
pub struct MappingLayerBuilder {
    mappings: DashMap<TypeInfo, Vec<MappingDescriptor>, ahash::RandomState>,
}

impl MappingLayerBuilder {
    pub fn new() -> Self {
        Self { mappings: Default::default() }
    }

    pub fn add_mapping<TSrc: 'static, TDst: 'static>(&self, mapper: impl Fn(TSrc) -> ServiceBuildResult<TDst> + Sync + Send + 'static) {
        match self.mappings.entry(TDst::type_info()) {
            dashmap::Entry::Occupied(mut occupied_entry) => {
                occupied_entry.get_mut().push(MappingDescriptor::new::<TSrc, TDst>(mapper));
            },
            dashmap::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(vec!(MappingDescriptor::new::<TSrc, TDst>(mapper)));
            }
        };
    }

    pub fn build(self, scope_layer: ScopeLayer) -> MappingLayer {
        MappingLayer::new(self, scope_layer)
    }
}