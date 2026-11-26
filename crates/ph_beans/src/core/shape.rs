//! Slot-wise type map over a flat tuple.
//!
//! [`Shape`] names a tuple whose every slot went through [`SlotType`]. It has
//! no values: [`Each`](super::each::Each) is the walk that maps them.

/// What one slot becomes. Generics: `E` the slot.
pub trait SlotType<E> {
    /// The mapped slot.
    type Out;
}

/// A flat tuple with every slot mapped by `F`. One impl per length.
///
/// Generics: `F` type map ([`SlotType`]).
pub trait Shape<F> {
    /// The tuple of mapped slots.
    type Out;
}

// Generics: `F` type map.
impl<F> Shape<F> for () {
    type Out = ();
}

// Generics: `F` type map, `E0` slot.
impl<F, E0> Shape<F> for (E0,)
where
    F: SlotType<E0>,
{
    type Out = (<F as SlotType<E0>>::Out,);
}

// Generics: `F` type map, `E0`…`E1` slots.
impl<F, E0, E1> Shape<F> for (E0, E1)
where
    F: SlotType<E0> + SlotType<E1>,
{
    type Out = (<F as SlotType<E0>>::Out, <F as SlotType<E1>>::Out);
}

// Generics: `F` type map, `E0`…`E2` slots.
impl<F, E0, E1, E2> Shape<F> for (E0, E1, E2)
where
    F: SlotType<E0> + SlotType<E1> + SlotType<E2>,
{
    type Out = (
        <F as SlotType<E0>>::Out,
        <F as SlotType<E1>>::Out,
        <F as SlotType<E2>>::Out,
    );
}

// Generics: `F` type map, `E0`…`E3` slots.
impl<F, E0, E1, E2, E3> Shape<F> for (E0, E1, E2, E3)
where
    F: SlotType<E0> + SlotType<E1> + SlotType<E2> + SlotType<E3>,
{
    type Out = (
        <F as SlotType<E0>>::Out,
        <F as SlotType<E1>>::Out,
        <F as SlotType<E2>>::Out,
        <F as SlotType<E3>>::Out,
    );
}

// Generics: `F` type map, `E0`…`E4` slots.
impl<F, E0, E1, E2, E3, E4> Shape<F> for (E0, E1, E2, E3, E4)
where
    F: SlotType<E0> + SlotType<E1> + SlotType<E2> + SlotType<E3> + SlotType<E4>,
{
    type Out = (
        <F as SlotType<E0>>::Out,
        <F as SlotType<E1>>::Out,
        <F as SlotType<E2>>::Out,
        <F as SlotType<E3>>::Out,
        <F as SlotType<E4>>::Out,
    );
}

// Generics: `F` type map, `E0`…`E5` slots.
impl<F, E0, E1, E2, E3, E4, E5> Shape<F> for (E0, E1, E2, E3, E4, E5)
where
    F: SlotType<E0> + SlotType<E1> + SlotType<E2> + SlotType<E3> + SlotType<E4> + SlotType<E5>,
{
    type Out = (
        <F as SlotType<E0>>::Out,
        <F as SlotType<E1>>::Out,
        <F as SlotType<E2>>::Out,
        <F as SlotType<E3>>::Out,
        <F as SlotType<E4>>::Out,
        <F as SlotType<E5>>::Out,
    );
}

// Generics: `F` type map, `E0`…`E6` slots.
impl<F, E0, E1, E2, E3, E4, E5, E6> Shape<F> for (E0, E1, E2, E3, E4, E5, E6)
where
    F: SlotType<E0>,
    F: SlotType<E1>,
    F: SlotType<E2>,
    F: SlotType<E3>,
    F: SlotType<E4>,
    F: SlotType<E5>,
    F: SlotType<E6>,
{
    type Out = (
        <F as SlotType<E0>>::Out,
        <F as SlotType<E1>>::Out,
        <F as SlotType<E2>>::Out,
        <F as SlotType<E3>>::Out,
        <F as SlotType<E4>>::Out,
        <F as SlotType<E5>>::Out,
        <F as SlotType<E6>>::Out,
    );
}

// Generics: `F` type map, `E0`…`E7` slots.
impl<F, E0, E1, E2, E3, E4, E5, E6, E7> Shape<F> for (E0, E1, E2, E3, E4, E5, E6, E7)
where
    F: SlotType<E0>,
    F: SlotType<E1>,
    F: SlotType<E2>,
    F: SlotType<E3>,
    F: SlotType<E4>,
    F: SlotType<E5>,
    F: SlotType<E6>,
    F: SlotType<E7>,
{
    type Out = (
        <F as SlotType<E0>>::Out,
        <F as SlotType<E1>>::Out,
        <F as SlotType<E2>>::Out,
        <F as SlotType<E3>>::Out,
        <F as SlotType<E4>>::Out,
        <F as SlotType<E5>>::Out,
        <F as SlotType<E6>>::Out,
        <F as SlotType<E7>>::Out,
    );
}
