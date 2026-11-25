//! Live resource set carried in the context args.

use core::marker::PhantomData;

use crate::filter::shape::{No, Yes};

/// The claim `C` is live.
/// Generics: `C` claim set.
pub struct Live<C> {
    _markers: PhantomData<C>,
}

/// A wait cleared the live claim. `C` is remembered for the init function.
pub struct Idle<C> {
    _markers: PhantomData<C>,
}

/// Wrapper whose claim shape / indexes are those of [`Inner`](ClaimDesc::Inner).
///
/// [`Live`] / [`Idle`] (and later similar carriers) implement this once;
/// [`ClaimShape`](crate::ClaimShape) / [`Claim`](crate::Claim) blanket over it
/// instead of per-wrapper walks.
pub trait ClaimDesc {
    /// Inner claim node (view + status + indexes).
    type Inner: ?Sized;
}

// Generics: `C` claim.
impl<C> ClaimDesc for Live<C> {
    type Inner = C;
}

// Generics: `C` claim set.
impl<C> ClaimDesc for Idle<C> {
    type Inner = C;
}

/// Projects the live accessor set.
pub trait AccSet {
    /// Markers still live for the next `.inject_resources`.
    type Set;
}

// Generics: `C` claim set.
impl<C> AccSet for Live<C> {
    type Set = C;
}

// Generics: `C` claim.
impl<C> AccSet for Idle<C> {
    type Set = ();
}

/// Wait policy on [`Yes`] / [`No`]: what happens to a [`Live`] claim.
///
/// Marker on the flag — not a Live×flag matrix.
/// Generics: `C` claim set.
#[doc(hidden)]
pub trait WaitFlag<C> {
    /// Args after applying this wait to [`Live`]`<C>`.
    type OnLive;
}

// Generics: `C` claim set.
impl<C> WaitFlag<C> for Yes {
    type OnLive = Idle<C>;
}

// Generics: `C` claim set.
impl<C> WaitFlag<C> for No {
    type OnLive = Live<C>;
}

/// Applies a wait flag to the current args.
///
/// One blanket for [`Live`] (via [`WaitFlag`]), one for [`Idle`] (always idle).
/// Generics: `F` wait flag.
#[doc(hidden)]
pub trait Rewait<F> {
    /// Args after the filter.
    type Out;
}

// Generics: `C` claim set, `F` wait flag.
impl<C, F> Rewait<F> for Live<C>
where
    F: WaitFlag<C>,
{
    type Out = F::OnLive;
}

// Generics: `C` claim set, `F` wait flag.
impl<C, F> Rewait<F> for Idle<C> {
    type Out = Idle<C>;
}
