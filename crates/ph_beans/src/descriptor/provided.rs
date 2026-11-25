//! What an entry of `Provides` or `Injects` names: a bean and an access.

use crate::bean::BeanInfo;
use crate::descriptor::{Get, Mut};

/// The bean may only be read.
pub struct Shared;

/// The bean may be read or changed.
pub struct Exclusive;

/// The entry names no access. A provide must name one (`PipelineError<5>`).
pub struct Unmarked;

/// An access an entry can name: [`Shared`] or [`Exclusive`].
#[doc(hidden)]
pub trait Named {}

impl Named for Shared {}

impl Named for Exclusive {}

/// Access `Self` is enough to hand out access `A`.
///
/// [`Exclusive`] hands out either access. [`Shared`] hands out [`Shared`].
/// Generics: `A` access asked for.
#[doc(hidden)]
pub trait Allows<A> {}

// Generics: `A` access asked for.
impl<A> Allows<A> for Exclusive {}

impl Allows<Shared> for Shared {}

/// One entry of [`BeanInfo::Provides`] or [`BeanInfo::Injects`]: a bean and
/// the access its marker names.
///
/// `Id::T` ([`Get`]) and `Id::MUT` ([`Mut`]) both name `Id`. Under [`Get`]
/// the bean is made by `init` and never changes, under [`Mut`] it can be
/// taken either way. A bean with no marker is [`Unmarked`]: it is described
/// here so the failure can name its own code, and no list accepts it.
#[doc(hidden)]
pub trait Provided {
    /// The bean.
    type Bean: BeanInfo;
    /// [`Shared`], [`Exclusive`], or [`Unmarked`].
    type Access;
}

// Generics: `P` bean with no marker.
impl<P: BeanInfo> Provided for P {
    type Bean = P;
    type Access = Unmarked;
}

// Generics: `P` bean, shared.
impl<P: BeanInfo> Provided for Get<P> {
    type Bean = P;
    type Access = Shared;
}

// Generics: `P` bean, exclusive.
impl<P: BeanInfo> Provided for Mut<P> {
    type Bean = P;
    type Access = Exclusive;
}
