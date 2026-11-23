//! Filter AST: `And`, `Or`, `Not`, and `On`.

mod apply;
pub(crate) mod eval;
mod flag;
mod node;
mod ops;
pub(crate) mod shape;

pub use eval::{Collect, MatchEmit, UnionEmit};
pub use node::{And, Not, On, Or};
pub use shape::{ApplyFilter, HasMut, NestOk, No, Spine, Under, Yes};
