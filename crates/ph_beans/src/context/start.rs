//! Empty pipeline tip for a bean — a [`Context`] (hence a [`Descriptor`](crate::Descriptor)).

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::context::Ctx;
use crate::context::Leaf;
use crate::context::List;
use crate::context::Pipe;
use crate::context::maps::MapList;
use crate::context::tick::NoSystems;
use crate::descriptor::NeedInit;
use crate::descriptor::live::Live;

/// Fresh context tip at [`Query`](crate::query::Query)`::query`.
///
/// The tip type stays on this descriptor — not on [`BeanInfo`] and not spelled
/// on the `query` return line.
pub trait StartDesc: Sized {
    /// Empty [`Context`] for this bean (`NeedInit`, no args, no maps).
    type Ctx: Context<Bean = Self>;
    /// Builds that tip.
    fn start_ctx() -> Self::Ctx;
}

#[allow(clippy::type_complexity)]
impl<B> StartDesc for B
where
    B: BeanInfo,
{
    type Ctx = Ctx<B, Live<()>, NeedInit, Pipe<NoSystems, MapList<List<Leaf<()>>, (), ()>>>;

    fn start_ctx() -> Self::Ctx {
        Ctx::new(Pipe::new(NoSystems, MapList::empty()))
    }
}
