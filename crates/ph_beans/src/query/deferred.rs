//! Deferred events: the buffer of a scope event, and the walk that runs one
//! of its entries.
//!
//! A deferred event waits in the state of its scope event
//! ([`EventBuf`]). The next emit of that
//! event runs the buffer in defer order, after the event's own maps. Each
//! entry runs like a direct emit of its path, inside the scope: the maps whose
//! filter that path matches, in pipeline order.
//!
//! Whether a buffer is read at all, and what can be deferred, is answered by
//! the tick's [`ModuleDesc`](crate::ModuleDesc) (PERF-1,
//! `docs/performance.md`).

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::context::list::{Kept, WalkStrategy};
use crate::context::{DeferState, EventBuf, ScopeHost};
use crate::core::each::Each;
use crate::descriptor::{Mut, State};
use crate::filter::eval::{Collect, EmitMask};
use crate::filter::shape::{Here, IdleEmit, NodeForm, PairBody, Skip};
use crate::filter::{EmitBit, EmitIx, On};
use crate::query::claim::Claim;
use crate::query::lend::Borrow;
use crate::query::sees::Sees;
use crate::query::store::Stored;

/// Runs one buffer entry: the maps whose filter matches the deferred `path`.
///
/// A map on the path of the emit `M` already ran in the direct walk; that is
/// a constant per map. The path test is the one run-time check.
/// Generics: `M` emit.
pub(crate) struct Queued<M> {
    path: EmitMask,
    _markers: PhantomData<M>,
}

// Generics: `M` emit.
impl<M> Queued<M> {
    pub(crate) const fn new(path: EmitMask) -> Self {
        Self { path, _markers: PhantomData }
    }
}

// Generics: `M` emit.
impl<M> Copy for Queued<M> {}

// Generics: `M` emit.
impl<M> Clone for Queued<M> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `M` emit.
impl<M: Collect> WalkStrategy for Queued<M> {
    fn keep<F: Kept>(&self) -> bool {
        let ran = const { M::MASK.contains(F::MASK) };
        !ran && F::matches(self.path)
    }
}

/// The buffer of a scope event, reached through the borrows `R` of module `B`.
///
/// The event is a bean of `B`, and its state holds the buffer
/// ([`DeferState`]). Generics: `B` module, `R` borrows over `B::Beans`.
#[doc(hidden)]
pub trait Scope<B, R> {
    /// Lends the buffer to `visit`.
    ///
    /// Generics: `V` visit, `O` its result.
    fn buf<V, O>(beans: &mut R, visit: V) -> O
    where
        V: Fn(&mut EventBuf) -> O;
}

// Generics: `B` module, `R` borrows, `S` scope event, `I` inner path.
impl<B, R, S, I> Scope<B, R> for On<S, I>
where
    S: BeanInfo<State: DeferState> + 'static,
    Mut<State<S>>: Sees<B, R> + for<'a> Claim<B, Arg<'a> = &'a mut S::State>,
{
    fn buf<V, O>(beans: &mut R, visit: V) -> O
    where
        V: Fn(&mut EventBuf) -> O,
    {
        <Mut<State<S>> as Sees<B, R>>::call(beans, &|state: &mut S::State| visit(state.deferred()))
    }
}

/// An event module `B` can take with `Tick::defer`: its scope event is a
/// bean `B` provides, and that event's state holds the buffer.
///
/// Generics: `B` module.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `FilterError<7>` (event `{Self}`)",
    label = "fix Descriptor::Status",
    note = "the module must provide the scope event under `Mut`, and that event's state must hold an `EventBuf` (`DeferState`); FilterError<7> — docs/errors.md"
)]
pub trait DeferInto<B: BeanInfo> {
    /// Appends `path` to the buffer of this event's scope event, in the
    /// states `B` owns.
    fn defer_into(states: &mut B::States, path: EmitMask)
    where
        B: Stored;
}

// Generics: `B` module, `N` deferred event.
#[diagnostic::do_not_recommend]
impl<B, N> DeferInto<B> for N
where
    B: Stored,
    for<'s> &'s mut B::States: Each<Borrow>,
    for<'s> N: Scope<B, <&'s mut B::States as Each<Borrow>>::Out>,
{
    fn defer_into(states: &mut B::States, path: EmitMask) {
        N::buf(&mut states.each(), |buf| buf.push(path));
    }
}

/// Runs the buffers of the scope events an emit tree declares.
///
/// `Self` is the tree the pipeline declared with `.emit`. An emit `M` runs
/// the buffer of its own scope event, when the tree names that event.
/// Generics: `B` module, `R` borrows of the running emit.
#[doc(hidden)]
pub trait Drain<B, R> {
    /// Runs `run` for every event deferred into the scope of `M`, oldest
    /// first, then empties that buffer.
    ///
    /// Generics: `M` emit, `F` what one entry runs.
    fn drain<M: ScopeHost, F: FnMut(&mut R, EmitMask)>(beans: &mut R, run: &mut F);
}

/// [`Drain`] of a [`NodeForm::Body`]. Generics: `B` module, `R` borrows.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `FilterError<7>` (emit `{Self}`)",
    label = "fix Descriptor::Status",
    note = "the module must hold the scope event of a declared `.emit` under `Mut`, and that event's state must hold an `EventBuf` (`DeferState`); FilterError<7> — docs/errors.md"
)]
pub trait AsDrain<B, R> {
    /// See [`Drain::drain`].
    fn drain<M: ScopeHost, F: FnMut(&mut R, EmitMask)>(beans: &mut R, run: &mut F);
}

// Generics: `B` module, `R` borrows, `T` idle body.
impl<B, R, T: IdleEmit> AsDrain<B, R> for T {
    fn drain<M: ScopeHost, F: FnMut(&mut R, EmitMask)>(_beans: &mut R, _run: &mut F) {}
}

// Generics: `B` module, `R` borrows, `A` left tree, `C` right tree, `K` join kind.
impl<B, R, A, C, K> AsDrain<B, R> for PairBody<A, C, K>
where
    A: Drain<B, R>,
    C: Drain<B, R>,
{
    fn drain<M: ScopeHost, F: FnMut(&mut R, EmitMask)>(beans: &mut R, run: &mut F) {
        A::drain::<M, F>(beans, run);
        C::drain::<M, F>(beans, run);
    }
}

// A negated tree declares nothing. Generics: `B` module, `R` borrows, `A` child.
impl<B, R, A> AsDrain<B, R> for Skip<A> {
    fn drain<M: ScopeHost, F: FnMut(&mut R, EmitMask)>(_beans: &mut R, _run: &mut F) {}
}

// Generics: `B` module, `R` borrows, `S` scope event, `I` inner path.
impl<B, R, S, I> AsDrain<B, R> for Here<S, I>
where
    S: EmitBit,
    On<S, I>: Scope<B, R>,
{
    fn drain<M: ScopeHost, F: FnMut(&mut R, EmitMask)>(beans: &mut R, run: &mut F) {
        if const { M::SCOPED && EmitMask::from_bit(M::BIT).same(EmitMask::from_bit(<S as EmitIx>::BIT)) }
        {
            let pending = <On<S, I>>::buf(beans, |buf| buf.len());
            for index in 0..pending {
                if let Some(path) = <On<S, I>>::buf(beans, |buf| buf.path_at(index)) {
                    run(beans, path);
                }
            }
            if pending != 0 {
                <On<S, I>>::buf(beans, EventBuf::clear);
            }
        }
    }
}

// Generics: `B` module, `R` borrows, `T` emit tree ([`NodeForm`]).
impl<B, R, T> Drain<B, R> for T
where
    T: NodeForm,
    T::Body: AsDrain<B, R>,
{
    fn drain<M: ScopeHost, F: FnMut(&mut R, EmitMask)>(beans: &mut R, run: &mut F) {
        <T::Body as AsDrain<B, R>>::drain::<M, F>(beans, run);
    }
}
