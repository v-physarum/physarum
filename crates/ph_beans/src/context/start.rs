//! Empty pipeline tip for a bean — a [`Context`] (hence a [`Descriptor`](crate::Descriptor)).

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::context::Ctx;
use crate::context::Pipe;
use crate::context::maps::MapList;
use crate::context::tick::NoSystems;
use crate::core::config::Config;
use crate::core::config::OfConfig;
use crate::core::list::EmptySlots;
use crate::descriptor::NeedInit;
use crate::descriptor::live::Live;

/// Empty map list.
pub struct EmptyMap;

impl Config for EmptyMap {
    type Type = MapList<OfConfig<EmptySlots>, (), ()>;
}

/// Fresh context tip at [`Query`](crate::query::Query)`::query`.
///
/// The tip is [`OfConfig`]`<`[`StartCfg`]`<Self>>`.
/// It is not spelled on `query`.
pub trait StartDesc: BeanInfo + Sized {
    /// Empty [`Context`] for this bean (`NeedInit`, no args, no maps).
    type Ctx: Context<Bean = Self>;
    /// Builds that tip.
    fn start_ctx() -> Self::Ctx;
}

/// Config: empty args, [`NeedInit`], no init system, empty map list.
///
/// Generics: `H` bean.
#[doc(hidden)]
pub struct StartCfg<H>(PhantomData<H>);

impl Config for NoSystems {
    type Type = Pipe<Self, OfConfig<EmptyMap>>;
}

// Generics: `H` bean.
impl<H> Config for StartCfg<H>
where
    H: BeanInfo,
{
    type Type = Ctx<H, Live<()>, NeedInit, OfConfig<NoSystems>>;
}

// Generics: `H` bean.
impl<H> StartCfg<H>
where
    H: BeanInfo,
{
    fn apply(self) -> OfConfig<Self> {
        let StartCfg(_) = self;
        Ctx::new(Pipe::new(NoSystems, MapList::empty()))
    }
}

// Generics: `B` bean.
impl<B> StartDesc for B
where
    B: BeanInfo,
{
    type Ctx = OfConfig<StartCfg<B>>;

    fn start_ctx() -> Self::Ctx {
        StartCfg(PhantomData).apply()
    }
}
