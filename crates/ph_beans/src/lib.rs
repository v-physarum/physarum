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
pub mod core;
pub mod descriptor;
pub mod filter;
pub mod query;

pub use bean::{BeanInfo, EventInfo, FromParent, Module, OfInject, OfParent, OfProvide};
pub use borrow::{At, Ix, Off, Take, TakeMany};
pub use context::{
    CarrierDesc, Context, ContextTick, Ctx, Direct, DirectFn, End, EventBuf, ExactDrive, Full,
    Group, Head, List, MAX_MAP_SYSTEMS, MapEntry, MapList, NoSystems, Pipe, ScopeHost, StartDesc,
    WithDefault, WithInit,
};
pub use descriptor::{
    Accessors, ClaimDesc, ClaimError, Descriptor, Dyn, DynView, FilterError, Get, Idle, Live,
    Marker, Mut, NeedBuild, NeedInit, OpDesc, OpOk, PipelineError, PipelineOk, State, Valid,
    WarnUnusedArgs,
};
pub use filter::{
    And, ApplyFilter, Dense, EmitBit, EmitMask, Gate, NestOk, Not, On, Or, PathNode, PathStep,
    PathStrategy, Sparse,
};
pub use query::{Built, Claim, ClaimShape, Init, Query, Setup, Tick, Update};
