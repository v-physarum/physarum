//! One status channel for the whole pipeline.

mod accessors;
mod dyn_view;
pub(crate) mod error;
pub(crate) mod live;
mod module;
pub(crate) mod op_desc;
mod provided;
mod status;

pub use accessors::{Accessors, Get, Marker, Mut, State};
pub use dyn_view::{Dyn, DynObject};
pub use error::{ClaimError, FilterError, PipelineError};
pub use live::{AccSet, ClaimDesc, Idle, Live};
pub use module::ModuleDesc;
pub use op_desc::{JoinStatus, OpDesc, OpOk};
pub use provided::{Allows, Exclusive, Named, Provided, Shared, Unmarked};
pub use status::{NeedBuild, NeedInit, PipelineOk, Valid, WarnUnusedArgs};

/// Pipeline health and the live accessor set.
///
/// `Accessors` is a stub in this slice. Filter rules fill it later.
pub trait Descriptor {
    /// `Valid`, a warning, or a blocking status for the whole chain.
    type Status;
    /// Live resource accessors claimed along the chain.
    type Accessors;
}
