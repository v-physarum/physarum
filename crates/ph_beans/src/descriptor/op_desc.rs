//! Local operator status for one claim list.

use core::marker::PhantomData;

use crate::descriptor::Valid;
use crate::descriptor::live::{Idle, Live};
use crate::filter::And;

/// Per-operator lens. Pipeline [`crate::Descriptor`] stays the tip status.
pub trait OpDesc {
    /// `Valid`, or a named claim error.
    type Status;
}

/// `Status` may pass the next gated step.
pub trait OpOk {}

impl OpOk for Valid {}

/// Shape of one claim list.
///
/// Duplicate and `T`+`Mut` statuses are named below. Stable Rust rejects a
/// second impl that would pick them out of a generic pair, so a well-formed
/// list stays [`Valid`] until that selection exists.
pub trait OpShape: OpDesc {}

impl<T> OpDesc for T
where
    T: OpLeaf,
{
    type Status = T::Leaf;
}

/// Leaf or combination status.
#[doc(hidden)]
pub trait OpLeaf {
    /// Operator status of this node.
    type Leaf;
}

impl OpLeaf for () {
    type Leaf = Valid;
}

impl<T> OpLeaf for crate::descriptor::Get<T> {
    type Leaf = Valid;
}

impl<T> OpLeaf for crate::descriptor::Mut<T> {
    type Leaf = Valid;
}

impl<C, V: ?Sized> OpLeaf for crate::descriptor::Dyn<C, V>
where
    C: OpLeaf,
{
    type Leaf = C::Leaf;
}

impl<A, B> OpLeaf for And<A, B>
where
    A: OpLeaf<Leaf = Valid>,
    B: OpLeaf<Leaf = Valid>,
{
    type Leaf = Valid;
}

impl<A, B> OpLeaf for (A, B)
where
    A: OpLeaf<Leaf = Valid>,
    B: OpLeaf<Leaf = Valid>,
{
    type Leaf = Valid;
}

impl<C> OpLeaf for Live<C>
where
    C: OpLeaf,
{
    type Leaf = C::Leaf;
}

impl<C> OpLeaf for Idle<C>
where
    C: OpLeaf,
{
    type Leaf = C::Leaf;
}

impl<T: OpLeaf> OpShape for T {}

/// The same bean was claimed twice.
pub struct ErrDuplicateResource<T>(PhantomData<T>);

/// `T` and `Mut` were both claimed for one bean.
pub struct ErrConflictAccess<T>(PhantomData<T>);
