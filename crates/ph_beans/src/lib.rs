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

pub use bean::{
    BeanInfo, EventInfo, FromParent, Info, Module, NoState, OfInject, OfParent, OfProvide,
};
pub use borrow::{At, Ix, Off, Take, TakeMany};
pub use context::{
    CarrierDesc, Context, ContextTick, Ctx, DeferState, End, EventBuf, Full, Group, Head, List,
    MAX_MAP_SYSTEMS, MapEntry, MapList, NoSystems, Owned, Pipe, ScopeHost, StartDesc,
};
pub use descriptor::{
    Accessors, ClaimDesc, ClaimError, Descriptor, Dyn, DynObject, FilterError, Get, Idle, Live,
    Marker, ModuleDesc, Mut, NeedBuild, NeedInit, OpDesc, OpOk, PipelineError, PipelineOk, State,
    Valid, WarnUnusedArgs,
};
pub use filter::{
    And, ApplyFilter, Bit, BitIx, Dense, EmitBit, EmitIx, EmitMask, EventReg, Gate, HostedBy,
    InReg, NestOk, Not, On, Or, PathNode, PathStep, PathStrategy, SlotOf, Sparse,
};
pub use query::{Built, Claim, ClaimShape, Init, Node, Query, Setup, Slot, Tick, Update};
