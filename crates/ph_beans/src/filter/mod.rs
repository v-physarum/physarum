//! Filter AST: `And`, `Or`, `Not`, and `On`.

mod apply;
pub(crate) mod eval;
mod flag;
mod node;
mod ops;
pub(crate) mod path;
pub(crate) mod shape;

pub use eval::{Collect, ConstMatch, EmitBit, EmitMask, MatchEmit, UnionEmit};
pub use node::{And, Not, On, Or};
pub use path::{Dense, Gate, PathNode, PathStep, PathStrategy, Sparse};
pub use shape::{ApplyFilter, HasMut, NestOk, No, Spine, Under, Yes};
