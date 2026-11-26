//! One slot-wise walk over a flat tuple, shared by every trait that fans out.
//!
//! [`Each`] is the only walk by length. A feature names a strategy `F`
//! ([`SlotFn`] per slot) and bounds [`Each`]`<F>`. It does not add its own
//! impls per tuple length (rules 8, 11–12, 23).

/// What a walk does with one slot. Generics: `E` the slot it is handed.
pub trait SlotFn<E> {
    /// What the slot becomes.
    type Out;
    /// Maps one slot.
    fn slot(item: E) -> Self::Out;
}

/// Hands every slot of a borrowed tuple to strategy `F`, in order.
///
/// `Self` is `&'a mut` of the tuple, so a bound for every lifetime does not
/// ask the tuple to be `'static`. Generics: `F` strategy ([`SlotFn`]).
pub trait Each<F> {
    /// The tuple of mapped slots.
    type Out;
    /// Maps every slot.
    fn each(self) -> Self::Out;
}

// Generics: `F` strategy.
impl<F> Each<F> for &mut () {
    type Out = ();

    fn each(self) -> Self::Out {}
}

// Generics: `F` strategy, `E0` slot.
impl<'a, F, E0> Each<F> for &'a mut (E0,)
where
    F: SlotFn<&'a mut E0>,
{
    type Out = (<F as SlotFn<&'a mut E0>>::Out,);

    fn each(self) -> Self::Out {
        (<F as SlotFn<&'a mut E0>>::slot(&mut self.0),)
    }
}

// Generics: `F` strategy, `E0`…`E1` slots.
impl<'a, F, E0, E1> Each<F> for &'a mut (E0, E1)
where
    F: SlotFn<&'a mut E0> + SlotFn<&'a mut E1>,
{
    type Out = (
        <F as SlotFn<&'a mut E0>>::Out,
        <F as SlotFn<&'a mut E1>>::Out,
    );

    fn each(self) -> Self::Out {
        (
            <F as SlotFn<&'a mut E0>>::slot(&mut self.0),
            <F as SlotFn<&'a mut E1>>::slot(&mut self.1),
        )
    }
}

// Generics: `F` strategy, `E0`…`E2` slots.
impl<'a, F, E0, E1, E2> Each<F> for &'a mut (E0, E1, E2)
where
    F: SlotFn<&'a mut E0> + SlotFn<&'a mut E1> + SlotFn<&'a mut E2>,
{
    type Out = (
        <F as SlotFn<&'a mut E0>>::Out,
        <F as SlotFn<&'a mut E1>>::Out,
        <F as SlotFn<&'a mut E2>>::Out,
    );

    fn each(self) -> Self::Out {
        (
            <F as SlotFn<&'a mut E0>>::slot(&mut self.0),
            <F as SlotFn<&'a mut E1>>::slot(&mut self.1),
            <F as SlotFn<&'a mut E2>>::slot(&mut self.2),
        )
    }
}

// Generics: `F` strategy, `E0`…`E3` slots.
impl<'a, F, E0, E1, E2, E3> Each<F> for &'a mut (E0, E1, E2, E3)
where
    F: SlotFn<&'a mut E0> + SlotFn<&'a mut E1> + SlotFn<&'a mut E2> + SlotFn<&'a mut E3>,
{
    type Out = (
        <F as SlotFn<&'a mut E0>>::Out,
        <F as SlotFn<&'a mut E1>>::Out,
        <F as SlotFn<&'a mut E2>>::Out,
        <F as SlotFn<&'a mut E3>>::Out,
    );

    fn each(self) -> Self::Out {
        (
            <F as SlotFn<&'a mut E0>>::slot(&mut self.0),
            <F as SlotFn<&'a mut E1>>::slot(&mut self.1),
            <F as SlotFn<&'a mut E2>>::slot(&mut self.2),
            <F as SlotFn<&'a mut E3>>::slot(&mut self.3),
        )
    }
}

// Generics: `F` strategy, `E0`…`E4` slots.
impl<'a, F, E0, E1, E2, E3, E4> Each<F> for &'a mut (E0, E1, E2, E3, E4)
where
    F: SlotFn<&'a mut E0>,
    F: SlotFn<&'a mut E1>,
    F: SlotFn<&'a mut E2>,
    F: SlotFn<&'a mut E3>,
    F: SlotFn<&'a mut E4>,
{
    type Out = (
        <F as SlotFn<&'a mut E0>>::Out,
        <F as SlotFn<&'a mut E1>>::Out,
        <F as SlotFn<&'a mut E2>>::Out,
        <F as SlotFn<&'a mut E3>>::Out,
        <F as SlotFn<&'a mut E4>>::Out,
    );

    fn each(self) -> Self::Out {
        (
            <F as SlotFn<&'a mut E0>>::slot(&mut self.0),
            <F as SlotFn<&'a mut E1>>::slot(&mut self.1),
            <F as SlotFn<&'a mut E2>>::slot(&mut self.2),
            <F as SlotFn<&'a mut E3>>::slot(&mut self.3),
            <F as SlotFn<&'a mut E4>>::slot(&mut self.4),
        )
    }
}

// Generics: `F` strategy, `E0`…`E5` slots.
impl<'a, F, E0, E1, E2, E3, E4, E5> Each<F> for &'a mut (E0, E1, E2, E3, E4, E5)
where
    F: SlotFn<&'a mut E0>,
    F: SlotFn<&'a mut E1>,
    F: SlotFn<&'a mut E2>,
    F: SlotFn<&'a mut E3>,
    F: SlotFn<&'a mut E4>,
    F: SlotFn<&'a mut E5>,
{
    type Out = (
        <F as SlotFn<&'a mut E0>>::Out,
        <F as SlotFn<&'a mut E1>>::Out,
        <F as SlotFn<&'a mut E2>>::Out,
        <F as SlotFn<&'a mut E3>>::Out,
        <F as SlotFn<&'a mut E4>>::Out,
        <F as SlotFn<&'a mut E5>>::Out,
    );

    fn each(self) -> Self::Out {
        (
            <F as SlotFn<&'a mut E0>>::slot(&mut self.0),
            <F as SlotFn<&'a mut E1>>::slot(&mut self.1),
            <F as SlotFn<&'a mut E2>>::slot(&mut self.2),
            <F as SlotFn<&'a mut E3>>::slot(&mut self.3),
            <F as SlotFn<&'a mut E4>>::slot(&mut self.4),
            <F as SlotFn<&'a mut E5>>::slot(&mut self.5),
        )
    }
}

// Generics: `F` strategy, `E0`…`E6` slots.
impl<'a, F, E0, E1, E2, E3, E4, E5, E6> Each<F> for &'a mut (E0, E1, E2, E3, E4, E5, E6)
where
    F: SlotFn<&'a mut E0>,
    F: SlotFn<&'a mut E1>,
    F: SlotFn<&'a mut E2>,
    F: SlotFn<&'a mut E3>,
    F: SlotFn<&'a mut E4>,
    F: SlotFn<&'a mut E5>,
    F: SlotFn<&'a mut E6>,
{
    type Out = (
        <F as SlotFn<&'a mut E0>>::Out,
        <F as SlotFn<&'a mut E1>>::Out,
        <F as SlotFn<&'a mut E2>>::Out,
        <F as SlotFn<&'a mut E3>>::Out,
        <F as SlotFn<&'a mut E4>>::Out,
        <F as SlotFn<&'a mut E5>>::Out,
        <F as SlotFn<&'a mut E6>>::Out,
    );

    fn each(self) -> Self::Out {
        (
            <F as SlotFn<&'a mut E0>>::slot(&mut self.0),
            <F as SlotFn<&'a mut E1>>::slot(&mut self.1),
            <F as SlotFn<&'a mut E2>>::slot(&mut self.2),
            <F as SlotFn<&'a mut E3>>::slot(&mut self.3),
            <F as SlotFn<&'a mut E4>>::slot(&mut self.4),
            <F as SlotFn<&'a mut E5>>::slot(&mut self.5),
            <F as SlotFn<&'a mut E6>>::slot(&mut self.6),
        )
    }
}

// Generics: `F` strategy, `E0`…`E7` slots.
impl<'a, F, E0, E1, E2, E3, E4, E5, E6, E7> Each<F> for &'a mut (E0, E1, E2, E3, E4, E5, E6, E7)
where
    F: SlotFn<&'a mut E0>,
    F: SlotFn<&'a mut E1>,
    F: SlotFn<&'a mut E2>,
    F: SlotFn<&'a mut E3>,
    F: SlotFn<&'a mut E4>,
    F: SlotFn<&'a mut E5>,
    F: SlotFn<&'a mut E6>,
    F: SlotFn<&'a mut E7>,
{
    type Out = (
        <F as SlotFn<&'a mut E0>>::Out,
        <F as SlotFn<&'a mut E1>>::Out,
        <F as SlotFn<&'a mut E2>>::Out,
        <F as SlotFn<&'a mut E3>>::Out,
        <F as SlotFn<&'a mut E4>>::Out,
        <F as SlotFn<&'a mut E5>>::Out,
        <F as SlotFn<&'a mut E6>>::Out,
        <F as SlotFn<&'a mut E7>>::Out,
    );

    fn each(self) -> Self::Out {
        (
            <F as SlotFn<&'a mut E0>>::slot(&mut self.0),
            <F as SlotFn<&'a mut E1>>::slot(&mut self.1),
            <F as SlotFn<&'a mut E2>>::slot(&mut self.2),
            <F as SlotFn<&'a mut E3>>::slot(&mut self.3),
            <F as SlotFn<&'a mut E4>>::slot(&mut self.4),
            <F as SlotFn<&'a mut E5>>::slot(&mut self.5),
            <F as SlotFn<&'a mut E6>>::slot(&mut self.6),
            <F as SlotFn<&'a mut E7>>::slot(&mut self.7),
        )
    }
}
