//! [`Info`]: the descriptor of a bean, held and claimed like any bean.

use core::marker::PhantomData;

use crate::bean::{BeanInfo, NoState};
use crate::descriptor::ModuleDesc;
use crate::query::{ClaimShape, Setup, Tick};

/// The descriptor of bean `M` as a bean.
///
/// A module provides it under [`Get`](crate::Get) or injects it from its
/// parent, and a system claims it with `Info::<M>::T`: it takes `&Info<M>`.
/// `M` may be the module itself.
///
/// It stores nothing. [`Info::DESC`] is a constant of `M`'s pipeline: the
/// pipeline is not built and no instance of `M` is read.
/// Generics: `M` the described bean.
pub struct Info<M> {
    _markers: PhantomData<M>,
}

// Generics: `M` the described bean.
impl<M> BeanInfo for Info<M> {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

// Generics: `M` the described bean.
impl<M> Default for Info<M>
where
    M: Setup + BeanInfo<Injects: ClaimShape>,
{
    fn default() -> Self {
        Self { _markers: PhantomData }
    }
}

// Generics: `M` the described bean.
impl<M> Info<M>
where
    M: Setup + BeanInfo<Injects: ClaimShape>,
{
    /// What the pipeline of `M` holds and declares.
    pub const DESC: ModuleDesc = desc_of(M::setup);

    /// [`Info::DESC`], read through the claimed bean.
    #[must_use]
    pub const fn desc(&self) -> ModuleDesc {
        Self::DESC
    }
}

// `setup` is only named here, for the type of its tick: it is not called.
// Generics: `M` module, `T` its tick.
const fn desc_of<M, T>(_setup: fn(M) -> T) -> ModuleDesc
where
    M: BeanInfo<Injects: ClaimShape>,
    T: Tick<M>,
{
    <T as Tick<M>>::DESC
}
