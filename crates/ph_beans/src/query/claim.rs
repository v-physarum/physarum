//! A claim read under a module: what a system takes for it, and where the
//! claimed beans sit in the module ([`Claim`]).

use crate::descriptor::live::ClaimDesc;
use crate::descriptor::{Exclusive, Get, Mut, Shared};
use crate::filter::And;
use crate::query::args::{ClaimLeaf, ClaimShape};
use crate::query::dyn_claim::{Entry, Holds, Lends};

/// Resource claim `B` can project.
///
/// Leaves carry `B: At<P>`. Lists only require each part to be a [`Claim`].
/// Generics: `B` bean surface.
pub trait Claim<B>: ClaimShape {
    /// What this claim borrows from one call of module `B`. A plain bean
    /// lends a part of its slot. A bean `B` injects as `P::T as Trait` lends
    /// the trait object the caller passed.
    type Seen<'a>
    where
        Self: 'a;
    /// What a system takes: [`Seen`](Self::Seen), with a trait object named
    /// for that borrow (`&'a mut (dyn Trait + 'a)`).
    type Arg<'a>
    where
        Self: 'a;
    /// Hands the borrows to a system.
    fn arg<'a>(seen: Self::Seen<'a>) -> Self::Arg<'a>
    where
        Self: 'a;
    /// Absolute indexes of the claimed beans.
    type Indexes;
}

// Generics: `B` bean surface.
impl<B> Claim<B> for () {
    type Seen<'a>
        = ()
    where
        Self: 'a;
    type Arg<'a>
        = ()
    where
        Self: 'a;

    fn arg<'a>((): ())
    where
        Self: 'a,
    {
    }

    type Indexes = ();
}

// Generics: `B` bean surface, `P` bean.
impl<B, P> Claim<B> for Get<P>
where
    P: ClaimLeaf,
    B: Holds<P::Bean, Entry: for<'a> Lends<'a, P::Part, P::Bean>>,
    <B::Entry as Entry>::Target<P::Part, P::Bean>: 'static,
{
    type Seen<'a>
        = &'a <B::Entry as Entry>::Target<P::Part, P::Bean>
    where
        Self: 'a;
    type Arg<'a>
        = &'a <B::Entry as Lends<'a, P::Part, P::Bean>>::To
    where
        Self: 'a;

    fn arg<'a>(seen: Self::Seen<'a>) -> Self::Arg<'a>
    where
        Self: 'a,
    {
        <B::Entry as Lends<'a, P::Part, P::Bean>>::shared(seen)
    }

    type Indexes = <B::Entry as Entry>::Index<P::Part, B::Slot, Shared, P::Bean>;
}

// Generics: `B` bean surface, `P` bean.
impl<B, P> Claim<B> for Mut<P>
where
    P: ClaimLeaf,
    B: Holds<P::Bean, Entry: for<'a> Lends<'a, P::Part, P::Bean>>,
    <B::Entry as Entry>::Target<P::Part, P::Bean>: 'static,
{
    type Seen<'a>
        = &'a mut <B::Entry as Entry>::Target<P::Part, P::Bean>
    where
        Self: 'a;
    type Arg<'a>
        = &'a mut <B::Entry as Lends<'a, P::Part, P::Bean>>::To
    where
        Self: 'a;

    fn arg<'a>(seen: Self::Seen<'a>) -> Self::Arg<'a>
    where
        Self: 'a,
    {
        <B::Entry as Lends<'a, P::Part, P::Bean>>::excl(seen)
    }

    type Indexes = <B::Entry as Entry>::Index<P::Part, B::Slot, Exclusive, P::Bean>;
}

// Generics: `B` bean surface, `A` left child, `C` context.
impl<B, A, C> Claim<B> for And<A, C>
where
    A: Claim<B>,
    C: Claim<B>,
{
    type Seen<'a>
        = (A::Seen<'a>, C::Seen<'a>)
    where
        Self: 'a;
    type Arg<'a>
        = (A::Arg<'a>, C::Arg<'a>)
    where
        Self: 'a;

    fn arg<'a>(seen: Self::Seen<'a>) -> Self::Arg<'a>
    where
        Self: 'a,
    {
        (A::arg(seen.0), C::arg(seen.1))
    }

    type Indexes = (A::Indexes, C::Indexes);
}

// Generics: `B` bean surface, `A` left child, `C` context.
impl<B, A, C> Claim<B> for (A, C)
where
    A: Claim<B>,
    C: Claim<B>,
{
    type Seen<'a>
        = (A::Seen<'a>, C::Seen<'a>)
    where
        Self: 'a;
    type Arg<'a>
        = (A::Arg<'a>, C::Arg<'a>)
    where
        Self: 'a;

    fn arg<'a>(seen: Self::Seen<'a>) -> Self::Arg<'a>
    where
        Self: 'a,
    {
        (A::arg(seen.0), C::arg(seen.1))
    }

    type Indexes = (A::Indexes, C::Indexes);
}

// Generics: `B` bean surface, `T` operator.
impl<B, T> Claim<B> for T
where
    T: ClaimDesc,
    T::Inner: Claim<B>,
{
    type Seen<'a>
        = <T::Inner as Claim<B>>::Seen<'a>
    where
        Self: 'a;
    type Arg<'a>
        = <T::Inner as Claim<B>>::Arg<'a>
    where
        Self: 'a;

    fn arg<'a>(seen: Self::Seen<'a>) -> Self::Arg<'a>
    where
        Self: 'a,
    {
        <T::Inner as Claim<B>>::arg(seen)
    }

    type Indexes = <T::Inner as Claim<B>>::Indexes;
}
