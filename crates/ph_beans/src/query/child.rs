//! A provided bean with its own pipeline, run by its parent.
//!
//! A parent names the child with `.forward(P::MUT)`. The walk then runs the
//! child's pipeline for the event the parent was emitted with, over the node
//! the parent keeps for it. The child's injects are slots of the parent: its
//! provides or its own injects. `Tick::init` of the parent runs the child's
//! init system the same way, before the parent's own.

use crate::bean::BeanInfo;
use crate::borrow::TakeView;
use crate::context::list::{Kept, Pass, WalkStrategy};
use crate::core::each::Each;
use crate::descriptor::Provided;
use crate::query::args::ClaimShape;
use crate::query::claim::Claim;
use crate::query::dyn_claim::Holds;
use crate::query::lend::Reborrow;
use crate::query::setup::Setup;
use crate::query::slot::{Part, Pick, Slot};
use crate::query::store::{Keeps, Node, Stored};
use crate::query::update::Update;

/// `Id::MUT` as a child's inject: the whole slot of the bean.
#[doc(hidden)]
pub struct SlotPart;

impl Part for SlotPart {
    type Of<P: BeanInfo> = Slot<P>;

    fn get<P: BeanInfo>(slot: &Slot<P>) -> &Slot<P> {
        slot
    }

    fn get_mut<P: BeanInfo>(slot: &mut Slot<P>) -> &mut Slot<P> {
        slot
    }
}

/// An index tree read as whole slots: what a parent lends a child.
#[doc(hidden)]
pub trait Whole {
    /// The same tree with every node on [`SlotPart`].
    type Slots;
}

impl Whole for () {
    type Slots = ();
}

// Generics: `K` part, `I` slot index, `A` access, `P` bean.
impl<K, I, A, P> Whole for Pick<K, I, A, P> {
    type Slots = Pick<SlotPart, I, A, P>;
}

// Generics: `X` left tree, `Y` right tree.
impl<X: Whole, Y: Whole> Whole for (X, Y) {
    type Slots = (X::Slots, Y::Slots);
}

/// The borrows `&mut R` of a parent lend the slots at indexes `I` as `O`.
///
/// Generics: `I` index tree, `O` what is lent.
#[doc(hidden)]
pub trait Lease<I, O> {
    /// Takes `O` out of the borrows.
    fn lease(self) -> O;
}

// Generics: `R` borrows, `I` index tree, `O` what is lent, `V` the
// reborrowed slots of `R`.
impl<'a, R, I, O, V> Lease<I, O> for &'a mut R
where
    &'a mut R: Each<Reborrow, Out = V>,
    I: TakeView<V, O>,
{
    fn lease(self) -> O {
        let (lent, _rest) = I::take_view(self.each());
        lent
    }
}

/// Bean `Self` of module `B` can be run from the borrows `R` of `B`.
///
/// Generics: `B` parent module, `R` its borrows.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `PipelineError<6>` (child `{Self}`)",
    label = "fix Descriptor::Status",
    note = "`{B}` cannot run the pipeline of this bean; PipelineError<6> — docs/errors.md"
)]
pub trait Child<B, R> {
    /// Runs the child's pipeline for the walk its parent is in.
    ///
    /// Generics: `W` walk ([`Pass`]).
    fn run<W: Pass>(res: &mut R, walk: W);
    /// Runs the child's init system and keeps what it made.
    fn make(res: &mut R);
}

// The parent's entry for `P` says how the node is seen: `&mut` under `Mut`,
// `Cold` under `Get`.
// Generics: `B` parent module, `R` its borrows, `P` child bean, `Q` its
// injects, `X` their indexes in `B`, `E` the entry of `P` in `B`, `J` its
// slot, `A` its access.
#[diagnostic::do_not_recommend]
impl<B, R, P, Q, X, E, J, A> Child<B, R> for P
where
    P: Setup + Default + Stored<Injects = Q>,
    Q: ClaimShape + Claim<B, Indexes = X>,
    B: Holds<P, Entry = E, Slot = J>,
    E: Provided<Access = A>,
    A: Keeps,
    X: Whole,
    for<'a> &'a mut R: Lease<(J, X::Slots), (A::Seen<'a, Node<P>>, Q::Lent<'a>)>,
{
    fn run<W: Pass>(res: &mut R, walk: W) {
        let (seen, injects) = res.lease();
        let tick = P::default().setup();
        Update::drive_parts(&tick, &mut A::open(seen).provides, injects, walk);
    }

    fn make(res: &mut R) {
        let (seen, injects) = res.lease();
        let node = A::open(seen);
        let tick = P::default().setup();
        let made = Update::boot_parts(&tick, injects);
        node.slot.state = made.state;
        node.provides = made.provides;
    }
}

/// The mark of a provide slot whose bean a `.forward` names.
/// Generics: `P` child bean.
pub struct Made<P> {
    _markers: core::marker::PhantomData<P>,
}

/// The walk of `Tick::init` over the init part: runs the init system of
/// every child in it.
#[derive(Clone, Copy)]
pub(crate) struct Boot;

impl WalkStrategy for Boot {
    fn keep<F: Kept>(&self) -> bool {
        false
    }

    fn forward<F: Kept, P: Child<B, R>, B, R>(&self, res: &mut R) {
        P::make(res);
    }
}
