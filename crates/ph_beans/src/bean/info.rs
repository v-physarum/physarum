//! The descriptors of a bean: [`BeanInfo`] and [`EventInfo`] declare it,
//! [`Info`] carries the descriptor of its pipeline as a bean.

use core::marker::PhantomData;

use crate::bean::NoState;
use crate::descriptor::ModuleDesc;
use crate::filter::On;
use crate::query::{ClaimShape, Setup, Tick};

/// Standard bean descriptor.
///
/// Provides, injects, the combined bean list, and the owned state. A bean
/// without state names [`NoState`].
///
/// Meta trait — frozen without owner approval (rule 14). Use [`Query`](crate::Query)
/// to start a pipeline.
pub trait BeanInfo: Sized {
    /// Beans this type provides.
    type Provides;
    /// Beans this type injects from its parent.
    type Injects;
    /// Provides and injects together.
    type Beans;
    /// Owned state of this bean only. [`NoState`] when it owns none.
    type State;
}

/// Extra descriptor for an event, beside [`BeanInfo`].
///
/// An event may list several parents. Meta trait — frozen without owner
/// approval (rule 14).
pub trait EventInfo: BeanInfo {
    /// Parent events of this event.
    type Parents;

    /// `Ev::ON[inner]` starts an event node.
    const ON: On<Self, ()> = On::new();
}

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
