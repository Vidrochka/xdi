use crate::builder::DiBuilder;

trait Call = Fn(&DiBuilder) + Send + Sync;

pub struct Registration {
    pub constructor: &'static dyn Call,
}
