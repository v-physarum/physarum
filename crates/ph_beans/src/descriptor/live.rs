//! Live resource set carried in the context args.

use core::marker::PhantomData;

use crate::filter::shape::{No, Yes};

/// The claim `C` is live.
pub struct Live<C>(PhantomData<C>);

/// A wait cleared the live claim. `C` is remembered for the init function.
pub struct Idle<C>(PhantomData<C>);

/// Projects the live accessor set.
pub trait AccSet {
    /// Markers still live for the next `.inject_resources`.
    type Set;
}

impl<C> AccSet for Live<C> {
    type Set = C;
}

impl<C> AccSet for Idle<C> {
    type Set = ();
}

/// Applies a wait flag to the current args.
#[doc(hidden)]
pub trait Rewait<F> {
    /// Args after the filter.
    type Out;
}

impl<C> Rewait<Yes> for Live<C> {
    type Out = Idle<C>;
}

impl<C> Rewait<No> for Live<C> {
    type Out = Live<C>;
}

impl<C> Rewait<Yes> for Idle<C> {
    type Out = Idle<C>;
}

impl<C> Rewait<No> for Idle<C> {
    type Out = Idle<C>;
}
