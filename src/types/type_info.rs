use std::any::{TypeId, type_name};

#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord)]
pub struct TypeInfo {
    pub id: TypeId,
    pub name: &'static str,
}

impl std::hash::Hash for TypeInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl TypeInfo {
    pub fn from_type<TType: ?Sized + 'static>() -> Self {
        Self {
            id: TypeId::of::<TType>(),
            name: type_name::<TType>(),
        }
    }
}

pub trait TypeInfoSource {
    fn type_info() -> TypeInfo;
}

impl<T: 'static> TypeInfoSource for T {
    fn type_info() -> TypeInfo {
        TypeInfo::from_type::<T>()
    }
}
