//! One status channel for the whole pipeline.

mod accessors;
mod dyn_view;
mod levels;
pub(crate) mod live;
pub(crate) mod op_desc;
mod status;

pub use accessors::{Accessors, Get, Marker, Mut};
pub use dyn_view::{Dyn, DynView};
pub use levels::LevelDesc;
pub use live::{AccSet, Idle, Live};
pub use op_desc::{ErrConflictAccess, ErrDuplicateResource, OpDesc, OpOk, OpShape};
pub use status::{
    ErrBadEvent, ErrInitAlreadySet, ErrNeedWaitFilter, ErrTooManySystems, NeedBuild, NeedInit,
    PipelineOk, Valid, WarnUnusedArgs,
};

/// Pipeline health and the live accessor set.
///
/// `Accessors` is a stub in this slice. Filter rules fill it later.
pub trait Descriptor {
    /// `Valid`, a warning, or a blocking status for the whole chain.
    type Status;
    /// Live resource accessors claimed along the chain.
    type Accessors;
}
