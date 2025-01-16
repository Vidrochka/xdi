use ahash::AHashMap;
use dashmap::DashMap;

use crate::{
    ServiceProvider,
    types::{
        boxed_service::BoxedService,
        error::{ServiceBuildError, ServiceBuildResult},
        type_info::{TypeInfo, TypeInfoSource},
    },
};

use super::scope::ScopeLayer;

/// Mapping allow convert any type to any other type
///
/// - Service to another service
/// - Service to trait object
#[derive(Debug)]
pub(crate) struct MappingLayer {
    pub(crate) scope_layer: ScopeLayer,
    mappings: AHashMap<TypeInfo, Vec<MappingDescriptor>>,
}

impl MappingLayer {
    /// Resolve service by type info
    pub(crate) fn resolve_raw(
        &self,
        ty: TypeInfo,
        sp: ServiceProvider,
    ) -> ServiceBuildResult<BoxedService> {
        let mapping = self
            .mappings
            .get(&ty)
            .and_then(|x| x.first())
            .ok_or(ServiceBuildError::MappingNotFound { ty })?;

        let service = self.scope_layer.get(mapping.src_ty(), sp)?;

        assert_eq!(mapping.dest_ty(), ty);
        assert_eq!(mapping.src_ty(), service.ty());

        mapping.mapper.map(service)
    }

    /// Resolve service by type
    pub(crate) fn resolve<TService: 'static>(
        &self,
        sp: ServiceProvider,
    ) -> ServiceBuildResult<TService> {
        let ty = TService::type_info();

        let service = self.resolve_raw(ty, sp)?;

        service.unbox::<TService>().map_err(|e| {
            ServiceBuildError::InvalidMappingLayerBoxedOutputType {
                expected: TService::type_info(),
                found: e.ty(),
            }
        })
    }

    /// Resolve all service by type info
    pub(crate) fn resolve_all_raw(
        &self,
        ty: TypeInfo,
        sp: ServiceProvider,
    ) -> ServiceBuildResult<Vec<BoxedService>> {
        let mappings = self
            .mappings
            .get(&ty)
            .ok_or(ServiceBuildError::MappingNotFound { ty })?;

        mappings
            .iter()
            .map(|mapping| {
                let service = self.scope_layer.get(mapping.src_ty(), sp.clone())?;

                assert_eq!(mapping.dest_ty(), ty);
                assert_eq!(mapping.src_ty(), service.ty());

                mapping.mapper.map(service)
            })
            .try_collect()
    }

    /// Resolve service by type
    pub(crate) fn resolve_all<TService: 'static>(
        &self,
        sp: ServiceProvider,
    ) -> ServiceBuildResult<Vec<TService>> {
        let ty = TService::type_info();

        let services = self.resolve_all_raw(ty, sp)?;

        services
            .into_iter()
            .map(|service| {
                service.unbox::<TService>().map_err(|e| {
                    ServiceBuildError::InvalidMappingLayerBoxedOutputType {
                        expected: TService::type_info(),
                        found: e.ty(),
                    }
                })
            })
            .try_collect()
    }

    fn new(builder: MappingLayerBuilder, scope_layer: ScopeLayer) -> Self {
        MappingLayer {
            scope_layer,
            mappings: builder.mappings.into_iter().collect(),
        }
    }
}

/// Mapping descriptor
#[derive(Debug)]
struct MappingDescriptor {
    src_ty: TypeInfo,
    dest_ty: TypeInfo,
    mapper: ServiceMapper,
}

impl MappingDescriptor {
    /// Create new mapping descriptor
    fn new<TSrc: 'static, TDst: 'static>(
        mapper: impl Fn(TSrc) -> ServiceBuildResult<TDst> + Send + Sync + 'static,
    ) -> Self {
        Self {
            src_ty: TSrc::type_info(),
            dest_ty: TDst::type_info(),
            mapper: ServiceMapper::new(Box::new(move |service: BoxedService| {
                let service = service.unbox::<TSrc>().map_err(|e| {
                    ServiceBuildError::InvalidMappingLayerBoxedInputType {
                        expected: TSrc::type_info(),
                        found: e.ty(),
                    }
                })?;

                let service = mapper(service)?;

                Ok(BoxedService::new(service))
            })),
        }
    }

    /// Get source type info
    fn src_ty(&self) -> TypeInfo {
        self.src_ty
    }

    /// Get destination type info
    fn dest_ty(&self) -> TypeInfo {
        self.dest_ty
    }
}

/// Service mapper. Map service to another service
struct ServiceMapper(Box<dyn Fn(BoxedService) -> ServiceBuildResult<BoxedService> + Send + Sync>);

impl ServiceMapper {
    /// Create new service mapper
    fn new(
        converter: impl Fn(BoxedService) -> ServiceBuildResult<BoxedService> + 'static + Send + Sync,
    ) -> Self {
        Self(Box::new(converter))
    }

    /// Map service to another service
    fn map(&self, service: BoxedService) -> ServiceBuildResult<BoxedService> {
        (self.0)(service)
    }
}

impl std::fmt::Debug for ServiceMapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ServiceTransormer").finish()
    }
}

#[derive(Debug, Default)]
pub(crate) struct MappingLayerBuilder {
    mappings: DashMap<TypeInfo, Vec<MappingDescriptor>, ahash::RandomState>,
}

impl MappingLayerBuilder {
    /// Add new mapping
    pub(crate) fn add_mapping<TSrc: 'static, TDst: 'static>(
        &self,
        mapper: impl Fn(TSrc) -> ServiceBuildResult<TDst> + Sync + Send + 'static,
    ) {
        match self.mappings.entry(TDst::type_info()) {
            dashmap::Entry::Occupied(mut occupied_entry) => {
                occupied_entry
                    .get_mut()
                    .push(MappingDescriptor::new::<TSrc, TDst>(mapper));
            }
            dashmap::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(vec![MappingDescriptor::new::<TSrc, TDst>(mapper)]);
            }
        };
    }

    /// Build mapping layer
    pub(crate) fn build(self, scope_layer: ScopeLayer) -> MappingLayer {
        MappingLayer::new(self, scope_layer)
    }
}
