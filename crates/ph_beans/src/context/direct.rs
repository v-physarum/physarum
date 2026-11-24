//! Direct child emit and defer from inside a map.

use core::marker::PhantomData;

use crate::bean::EventInfo;
use crate::context::list::{ExactDrive, Lane};
use crate::context::scope::ScopeBufs;
use crate::filter::eval::{Collect, EmitBit};
use crate::filter::{NestOk, On};

/// Innermost event of a filter path.
pub(crate) trait TipEvent {
    /// Innermost event.
    type Tip;
}

// Generics: `E` event at this node.
impl<E> TipEvent for On<E, ()> {
    type Tip = E;
}

// Generics: `E` outer event, `C` next event, `I` rest of the path.
impl<E, C, I> TipEvent for On<E, On<C, I>>
where
    On<C, I>: TipEvent,
{
    type Tip = <On<C, I> as TipEvent>::Tip;
}

/// Appends `C` at the end of a filter path.
/// Generics: `C` child event.
#[doc(hidden)]
pub trait WithChild<C> {
    /// Path plus `On<C, ()>`.
    type Out: 'static;
}

/// Inner step of [`WithChild`].
#[doc(hidden)]
pub trait PushInner<C> {
    /// Path with `C` appended under this node.
    type Out: 'static;
}

impl<C: 'static> PushInner<C> for () {
    type Out = On<C, ()>;
}

// Generics: `E` event, `I` inner path, `C` child event.
impl<E, I, C> PushInner<C> for On<E, I>
where
    E: 'static,
    I: PushInner<C>,
{
    type Out = On<E, I::Out>;
}

// Generics: `E` outer event, `I` inner path, `C` child event.
impl<E, I, C> WithChild<C> for On<E, I>
where
    E: 'static,
    I: PushInner<C>,
{
    type Out = On<E, I::Out>;
}

/// `C`'s parent is exactly the current event.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `FilterError<3>`",
    label = "fix Descriptor::Status",
    note = "direct emit only reaches a child of the current event (FilterError<3>)"
)]
// Generics: `E` parent event.
pub trait ChildOf<E> {}

// Generics: `E` current event, `C` candidate child.
impl<E, C> ChildOf<E> for C where C: EventInfo<Parents = E> {}

/// `.map_at` handler. Generics: `E` current event, `P` filter path.
///
/// `run` is generic over the list so child emit stays const. A bare
/// `fn(&Direct<E, impl ExactDrive>)` cannot be stored in that list.
pub trait DirectFn<E, P = On<E, ()>> {
    /// Called when this map's filter matches.
    fn run<L: ExactDrive>(&self, go: &Direct<E, L, P>);
}

/// Handle for `.map_at`. Generics: `E` current event, `L` list, `P` filter path.
/// Emit only a child of `E`. Defer stores for the next enter of `E`.
/// ```compile_fail
/// use ph_beans::{BeanInfo, Direct, EventInfo, ExactDrive, Marker};
/// struct U; impl BeanInfo for U { type Provides=(); type Injects=(); type Beans=(); type State=(); }
/// impl EventInfo for U { type Parents=(); }
/// fn no(g: &Direct<U, impl ExactDrive>) { g.emit(U::ON); }
/// ```
pub struct Direct<'a, E, L, P = On<E, ()>> {
    scopes: &'a ScopeBufs,
    drive: &'a L,
    _markers: PhantomData<(E, P)>,
}

// Generics: `E` current event, `L` list ([`ExactDrive`]), `P` filter path.
impl<'a, E, L, P> Direct<'a, E, L, P> {
    pub(crate) fn from_lane(lane: &Lane<'a, L>) -> Self {
        Self { scopes: lane.scopes, drive: lane.root, _markers: PhantomData }
    }

    /// Runs the child now. Generics: `C` child event. Path via `const` mask.
    #[allow(clippy::needless_pass_by_value)]
    pub fn emit<C>(&self, child: On<C, ()>)
    where
        C: ChildOf<E> + EmitBit,
        P: WithChild<C>,
        L: ExactDrive,
        <P as WithChild<C>>::Out: Collect,
    {
        let _ = child;
        self.drive
            .drive_exact::<<P as WithChild<C>>::Out>(self.scopes);
    }

    /// Stores `N` on this scope. Runs the next time `E` is entered.
    #[allow(clippy::needless_pass_by_value)]
    pub fn defer<N>(&self, event: N)
    where
        E: EmitBit,
        N: Collect + NestOk,
    {
        let _ = event;
        self.scopes.or_bit(E::BIT, N::MASK);
    }
}
