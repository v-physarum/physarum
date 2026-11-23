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

/// Wait policy on [`Yes`] / [`No`]: what happens to a [`Live`] claim.
///
/// Marker on the flag — not a Live×flag matrix.
#[doc(hidden)]
pub trait WaitFlag<C> {
    /// Args after applying this wait to [`Live`]`<C>`.
    type OnLive;
}

impl<C> WaitFlag<C> for Yes {
    type OnLive = Idle<C>;
}

impl<C> WaitFlag<C> for No {
    type OnLive = Live<C>;
}

/// Applies a wait flag to the current args.
///
/// One blanket for [`Live`] (via [`WaitFlag`]), one for [`Idle`] (always idle).
#[doc(hidden)]
pub trait Rewait<F> {
    /// Args after the filter.
    type Out;
}

impl<C, F> Rewait<F> for Live<C>
where
    F: WaitFlag<C>,
{
    type Out = F::OnLive;
}

impl<C, F> Rewait<F> for Idle<C> {
    type Out = Idle<C>;
}
