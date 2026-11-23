//! Query pipeline for beans.
//!
//! One [`Descriptor`] carries pipeline status. A linear [`Ctx`] holds the
//! bean, the resource args, that status, and the systems. [`Setup::setup`]
//! returns [`Tick`] only when the status is [`PipelineOk`].

#![forbid(unsafe_code)]

extern crate self as ph_beans;

pub mod bean;
pub mod borrow;
pub mod context;
pub mod descriptor;
pub mod filter;
pub mod query;

pub use bean::{
    BeanInfo, EventInfo, FromParent, Module, OfInject, OfInject0, OfInject1, OfParent, OfProvide,
    OfProvide0, OfProvide1,
};
pub use borrow::{At, Ix0, Ix1, Ix2, Ix3, Off, Take, TakeMany};
pub use context::{
    Context, ContextTick, Ctx, Leaf, List, MAX_MAP_SYSTEMS, MapList, NoSystems, Pipe, StartDesc,
    WithDefault, WithInit,
};
pub use descriptor::{
    Accessors, Descriptor, ErrBadEvent, ErrConflictAccess, ErrDuplicateResource, ErrInitAlreadySet,
    ErrNeedWaitFilter, ErrTooManySystems, Get, Idle, Live, Marker, Mut, NeedBuild, NeedInit,
    OpDesc, OpOk, OpShape, PipelineOk, Valid, WarnUnusedArgs,
};
pub use filter::{And, ApplyFilter, NestOk, Not, On, Or};
pub use query::{Built, Init, Query, Setup, Tick, Update};
