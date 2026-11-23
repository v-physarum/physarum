//! One status channel for the whole pipeline.

mod accessors;
mod dyn_view;
pub(crate) mod live;
pub(crate) mod op_desc;
mod status;

pub use accessors::{Accessors, Get, Marker, Mut};
pub use dyn_view::{Dyn, DynView};
pub use live::{AccSet, Idle, Live};
pub use op_desc::{ErrConflictAccess, ErrDuplicateResource, OpDesc, OpOk, OpShape};
pub use status::{
    ErrBadEvent, ErrInitAlreadySet, ErrNeedWaitFilter, ErrTooManySystems, NeedBuild, NeedInit,
    PipelineOk, Valid, WarnUnusedArgs,
};

/// Pipeline health and the live accessor set.
///
/// Inject claims of the context bean are gathered by [`Collected`], not walked
/// as a nested tail.
pub trait Descriptor {
    /// `Valid`, a warning, or a blocking status for the whole chain.
    type Status;
    /// Live resource accessors claimed along the chain.
    type Accessors;
}

/// Inject claims gathered from the context bean onto the pipeline descriptor.
///
/// The set is [`BeanInfo::Injects`](crate::BeanInfo::Injects): one claim, or one
/// flat tuple of claims. Readers use this associated type.
pub trait Collected: Descriptor {
    /// Inject claims of the context bean.
    type Injects;
}
