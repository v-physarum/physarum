//! Pull slots out of a fixed-width tuple.
//!
//! [`Take`] leaves [`Off`] in the hole and keeps the width, so later indexes
//! still name the same positions. [`TakeMany`] walks a list of those indexes.

use super::{Ix, Off};

/// Pulls slot `I` out of a fixed-width tuple.
/// Generics: `I` index.
#[doc(hidden)]
pub trait Take<I> {
    /// The value that occupied slot `I`.
    type Out;
    /// `Self` with [`Off`] in slot `I`.
    type Rest;

    /// Splits `self` into the slot and the tuple with that hole.
    #[must_use]
    fn take(self) -> (Self::Out, Self::Rest);
}

/// Pulls every index in `I` out of a tuple.
///
/// Generics: `I` index list, `O` projected args in index order.
#[doc(hidden)]
pub trait TakeMany<I, O> {
    /// Projects `O` from `self`.
    #[must_use]
    fn take_many(self) -> O;
}

impl<T> TakeMany<(), ()> for T {
    fn take_many(self) {}
}

impl<T, I0, O0> TakeMany<(I0,), (O0,)> for T
where
    T: Take<I0, Out = O0>,
{
    fn take_many(self) -> (O0,) {
        let (first, _) = self.take();
        (first,)
    }
}

// Generics: `T` tuple, `I0` index 0, `I1` index 1, `O0` out 0, `O1` out 1.
impl<T, I0, I1, O0, O1> TakeMany<(I0, I1), (O0, O1)> for T
where
    T: Take<I0, Out = O0>,
    T::Rest: Take<I1, Out = O1>,
{
    fn take_many(self) -> (O0, O1) {
        let (first, rest) = self.take();
        let (second, _) = rest.take();
        (first, second)
    }
}

impl<T, I0, I1, I2, O0, O1, O2> TakeMany<(I0, I1, I2), (O0, O1, O2)> for T
where
    T: Take<I0, Out = O0>,
    T::Rest: Take<I1, Out = O1>,
    <T::Rest as Take<I1>>::Rest: Take<I2, Out = O2>,
{
    fn take_many(self) -> (O0, O1, O2) {
        let (first, rest) = self.take();
        let (second, rest) = rest.take();
        let (third, _) = rest.take();
        (first, second, third)
    }
}

// Generics: `T0` slot 0.
impl<T0> Take<Ix<0>> for (T0,) {
    type Out = T0;
    type Rest = (Off,);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0,) = self;
        (t0, (Off,))
    }
}

impl<T0, T1> Take<Ix<0>> for (T0, T1) {
    type Out = T0;
    type Rest = (Off, T1);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0, t1) = self;
        (t0, (Off, t1))
    }
}

// Generics: `T0` slot 0, `T1` slot 1.
impl<T0, T1> Take<Ix<1>> for (T0, T1) {
    type Out = T1;
    type Rest = (T0, Off);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t1, t0) = (self.1, self.0);
        (t1, (t0, Off))
    }
}

impl<T0, T1, T2> Take<Ix<0>> for (T0, T1, T2) {
    type Out = T0;
    type Rest = (Off, T1, T2);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0, t1, t2) = self;
        (t0, (Off, t1, t2))
    }
}

// Generics: `T0` slot 0, `T1` slot 1, `T2` slot 2.
impl<T0, T1, T2> Take<Ix<1>> for (T0, T1, T2) {
    type Out = T1;
    type Rest = (T0, Off, T2);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0, t1, t2) = self;
        (t1, (t0, Off, t2))
    }
}

impl<T0, T1, T2> Take<Ix<2>> for (T0, T1, T2) {
    type Out = T2;
    type Rest = (T0, T1, Off);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0, t1, t2) = self;
        (t2, (t0, t1, Off))
    }
}

// Generics: `T0` slot 0, `T1` slot 1, `T2` slot 2, `T3` slot 3.
impl<T0, T1, T2, T3> Take<Ix<0>> for (T0, T1, T2, T3) {
    type Out = T0;
    type Rest = (Off, T1, T2, T3);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0, t1, t2, t3) = self;
        (t0, (Off, t1, t2, t3))
    }
}

impl<T0, T1, T2, T3> Take<Ix<1>> for (T0, T1, T2, T3) {
    type Out = T1;
    type Rest = (T0, Off, T2, T3);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0, t1, t2, t3) = self;
        (t1, (t0, Off, t2, t3))
    }
}

// Generics: `T0` slot 0, `T1` slot 1, `T2` slot 2, `T3` slot 3.
impl<T0, T1, T2, T3> Take<Ix<2>> for (T0, T1, T2, T3) {
    type Out = T2;
    type Rest = (T0, T1, Off, T3);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0, t1, t2, t3) = self;
        (t2, (t0, t1, Off, t3))
    }
}

impl<T0, T1, T2, T3> Take<Ix<3>> for (T0, T1, T2, T3) {
    type Out = T3;
    type Rest = (T0, T1, T2, Off);

    fn take(self) -> (Self::Out, Self::Rest) {
        let (t0, t1, t2, t3) = self;
        (t3, (t0, t1, t2, Off))
    }
}
