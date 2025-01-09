/*

Mapping allow convert any type to any other type

Example:

struct T {}

trait Tr {}

impl Tr for T {}

Then in abstraction convert service T to Box<dyn Tr>

*/

use std::sync::OnceLock;

use ahash::AHashMap;
use dashmap::DashMap;

use crate::types::{boxed_service::BoxedService, type_info::{TypeInfo, TypeInfoSource}};

use super::scope::ScopeLayer;

static MAPPING_LAYER: OnceLock<MappingLayer> = OnceLock::new();

#[derive(Debug)]
pub struct MappingLayer {
    mappings: AHashMap<TypeInfo, Vec<MappingDescriptor>>
}

impl MappingLayer {
    pub fn resolve_raw(ty: TypeInfo) -> Option<BoxedService> {
        let mapping_layer = MAPPING_LAYER.get_or_init(|| {
            MappingLayer {
                mappings: Default::default(),
            }
        });

        let mapping = mapping_layer.mappings.get(&ty)
            .and_then(|x| x.first())?;

        let service= ScopeLayer::get(mapping.src_ty())?;

        assert_eq!(mapping.dest_ty(), ty);
        assert_eq!(mapping.src_ty(), service.ty());

        Some(mapping.mapper.map(service))
    }

    pub fn resolve<TService: 'static>() -> Option<TService> {
        let ty = TService::type_info();

        let service = Self::resolve_raw(ty);

        service.map(|x| x.unbox::<TService>().unwrap())
    }

    pub fn set(builder: MappingLayerBuilder) {
        MAPPING_LAYER.set(MappingLayer {
            mappings: builder.mappings.into_iter().collect()
        }).unwrap();
    }
}

#[derive(Debug)]
pub struct MappingDescriptor {
    src_ty: TypeInfo,
    dest_ty: TypeInfo,
    mapper: ServiceMapper,
}

impl MappingDescriptor {
    pub fn new<TSrc: 'static, TDst: 'static>(mapper: impl Fn(TSrc) -> TDst + Send + Sync + 'static) -> Self {
        Self {
            src_ty: TSrc::type_info(),
            dest_ty: TSrc::type_info(),
            mapper: ServiceMapper(Box::new(move |service| {
                let service = mapper(service.unbox::<TSrc>().unwrap());
                BoxedService::new(service)
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

pub struct ServiceMapper(Box<dyn Fn(BoxedService) -> BoxedService + Send + Sync>);

impl ServiceMapper {
    pub fn new(converter: impl Fn(BoxedService) -> BoxedService + 'static + Send + Sync) -> Self {
        Self(Box::new(converter))
    }

    pub fn map(&self, service: BoxedService) -> BoxedService {
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

    pub fn add_mapping<TSrc: 'static, TDst: 'static>(&self, mapper: impl Fn(TSrc) -> TDst + Sync + Send + 'static) {
        match self.mappings.entry(TDst::type_info()) {
            dashmap::Entry::Occupied(mut occupied_entry) => {
                occupied_entry.get_mut().push(MappingDescriptor::new::<TSrc, TDst>(mapper));
            },
            dashmap::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(vec!(MappingDescriptor::new::<TSrc, TDst>(mapper)));
            }
        };
    }

    pub fn build(self) {
        MappingLayer::set(self);
    }
}