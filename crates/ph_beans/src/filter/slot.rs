//! Emit bits as slots of a registry.
//!
//! An event names one [`Bit`] of one registry type. The app states the owner
//! of that slot once with [`SlotOf`]. A second owner of the same slot is two
//! impls of one trait for one type, which the compiler rejects (`E0119`).
//! Nothing here exists at run time.

use crate::filter::shape::{Here, IdleEmit, NodeForm, PairBody, Skip};

/// One emit bit as a type. Generics: `N` bit index (`0..EmitMask::BITS`).
pub struct Bit<const N: u32>;

mod seal {
    pub trait Slot {}

    // Generics: `N` bit index.
    impl<const N: u32> Slot for super::Bit<N> {}
}

/// Index of a slot. Only [`Bit`] implements this.
pub trait BitIx: seal::Slot {
    /// Bit index of this slot.
    const IX: u32;
}

// Generics: `N` bit index.
impl<const N: u32> BitIx for Bit<N> {
    const IX: u32 = N;
}

/// Slot `Self` of registry `R` belongs to [`Owner`](Self::Owner).
///
/// Generics: `R` registry, a type local to the app.
/// ```compile_fail
/// use ph_beans::{Bit, SlotOf};
/// struct Events; struct Up; struct Low;
/// impl SlotOf<Events> for Bit<0> { type Owner = Up; }
/// impl SlotOf<Events> for Bit<0> { type Owner = Low; }
/// ```
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `FilterError<5>` (slot `{Self}`)",
    label = "fix Descriptor::Status",
    note = "registry `{R}` names no owner for this slot; FilterError<5> — docs/errors.md"
)]
pub trait SlotOf<R> {
    /// The one event that holds this slot.
    type Owner;
}

/// Event bit for the emit mask. Separate from [`EventInfo`](crate::EventInfo).
///
/// The slot must name this event as its owner.
/// ```compile_fail
/// use ph_beans::{Bit, EmitBit, SlotOf};
/// struct Events; struct Up; struct Low;
/// impl SlotOf<Events> for Bit<0> { type Owner = Up; }
/// impl EmitBit for Low { type Reg = Events; type Slot = Bit<0>; }
/// ```
pub trait EmitBit: Sized {
    /// Registry that hands out the bits.
    type Reg;
    /// This event's slot in that registry.
    type Slot: SlotOf<Self::Reg, Owner = Self> + BitIx;
}

/// Bit index of an event. Comes from [`EmitBit::Slot`] and cannot be set by hand.
pub trait EmitIx {
    /// Bit index in [`EmitMask`](crate::EmitMask).
    const BIT: u32;
}

// Generics: `E` event.
impl<E: EmitBit> EmitIx for E {
    const BIT: u32 = <E::Slot as BitIx>::IX;
}

/// Every event of this filter or emit tree is in registry `R`.
///
/// Generics: `R` registry.
pub trait InReg<R> {}

/// [`InReg`] of a [`NodeForm::Body`]. Generics: `R` registry.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `FilterError<4>` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "every event on this path needs EmitBit in registry `{R}`; FilterError<4> — docs/errors.md"
)]
pub trait AsReg<R> {}

// Generics: `R` registry, `T` idle body.
impl<R, T: IdleEmit> AsReg<R> for T {}

// Generics: `R` registry, `A` left child, `B` right child, `K` join kind.
impl<R, A: InReg<R>, B: InReg<R>, K> AsReg<R> for PairBody<A, B, K> {}

// Generics: `R` registry, `A` child.
impl<R, A: InReg<R>> AsReg<R> for Skip<A> {}

// Generics: `R` registry, `E` event, `I` inner path.
impl<R, E: EmitBit<Reg = R>, I: InReg<R>> AsReg<R> for Here<E, I> {}

// Generics: `R` registry, `T` operator ([`NodeForm`]).
impl<R, T> InReg<R> for T
where
    T: NodeForm,
    T::Body: AsReg<R>,
{
}

/// The registry a module's pipeline takes its event bits from.
///
/// A module that filters on events or emits them names one registry here.
/// A module with no events needs no impl.
pub trait EventReg {
    /// Registry of every event this module's pipeline uses.
    type Reg;
}

/// Every event of this tree has a bit in the registry of module `M`.
///
/// `.filter`, `.emit`, and `Tick::emit` ask for this, so the error is on
/// that call. Generics: `M` module ([`EventReg`]).
pub trait HostedBy<M> {}

/// [`HostedBy`] of a [`NodeForm::Body`]. Generics: `M` module.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `FilterError<4>` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "an event here needs EmitBit in the EventReg registry of `{M}`; FilterError<4> — docs/errors.md"
)]
pub trait AsHosted<M> {}

// Generics: `M` module, `T` idle body.
impl<M, T: IdleEmit> AsHosted<M> for T {}

// Generics: `M` module, `A` left child, `B` right child, `K` join kind.
impl<M, A: HostedBy<M>, B: HostedBy<M>, K> AsHosted<M> for PairBody<A, B, K> {}

// Generics: `M` module, `A` child.
impl<M, A: HostedBy<M>> AsHosted<M> for Skip<A> {}

// Generics: `M` module, `E` event, `I` inner path.
impl<M, E, I> AsHosted<M> for Here<E, I>
where
    E: EmitBit,
    M: EventReg<Reg = E::Reg>,
    I: InReg<E::Reg>,
{
}

// Generics: `M` module, `T` operator ([`NodeForm`]).
impl<M, T> HostedBy<M> for T
where
    T: NodeForm,
    T::Body: AsHosted<M>,
{
}
