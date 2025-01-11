use thiserror::Error;

use super::type_info::TypeInfo;


#[derive(Debug, Error)]
pub enum ServiceBuildError {
    #[error("Service not found")]
    ServiceNotDound,
    #[error("Scope not found")]
    ScopeNotFound,
    #[error("Mapping not found")]
    MappingNotFound,

    #[error("Invalid mapping layer boxed input type. Expected {expected:?} found {found:?}")]
    InvalidMappingLayerBoxedInputType{
        expected: TypeInfo,
        found: TypeInfo,
    },
    #[error("Invalid mapping layer boxed output type. Expected {expected:?} found {found:?}")]
    InvalidMappingLayerBoxedOutputType{
        expected: TypeInfo,
        found: TypeInfo,
    },

    #[error("Invalid scope layer boxed input type. Expected {expected:?} found {found:?}")]
    InvalidScopeLayerBoxedInputType {
        expected: TypeInfo,
        found: TypeInfo,
    },
    #[error("Unexpected singletone splitter params. Expected {expected:?} found {found:?}")]
    UnexpectedSingletoneSplitterParams {
        expected: TypeInfo,
        found: TypeInfo,
    },
    #[error("Invalid scope layer boxed output type. Expected {expected:?} found {found:?}")]
    InvalidScopeLayerBoxedOutputType {
        expected: TypeInfo,
        found: TypeInfo,
    },

    #[error(transparent)]
    Custom(#[from] anyhow::Error),
}

pub type ServiceBuildResult<TRes> = Result<TRes, ServiceBuildError>;