//! Filter AST: `And`, `Or`, `Not`, and `On`.

mod apply;
pub(crate) mod eval;
mod flag;
pub(crate) mod mask;
pub(crate) mod nest;
mod node;
mod ops;
pub(crate) mod path;
pub(crate) mod shape;
mod slot;

pub use eval::{Collect, ConstMatch, EmitMask, MatchEmit, UnionEmit};
pub use node::{And, Not, On, Or};
pub use path::{Dense, Gate, PathNode, PathStep, PathStrategy, Sparse};
pub use shape::{ApplyFilter, HasMut, NestOk, No, Spine, Under, Yes};
pub use slot::{Bit, BitIx, EmitBit, EmitIx, EventReg, HostedBy, InReg, SlotOf};
