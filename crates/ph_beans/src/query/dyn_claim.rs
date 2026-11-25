//! What an entry of a module's bean list lends to a claim on that bean.
//!
//! A claim names a bean; the module's descriptor says how the bean is held.
//! A plain entry (`Get<P>` / `Mut<P>`) is a [`Slot`](crate::Slot), and the
//! claim reads a part of it. A [`Dyn`] entry (`P::T as Trait` on the inject)
//! is a trait object the caller lends, and the same claim reads that object.
//! The operator `.inject_resources` takes the ordinary claim either way.

use crate::bean::BeanInfo;
use crate::borrow::{At, Ix, Off, Take};
use crate::descriptor::{Dyn, DynObject, Get, Mut, Provided};
use crate::query::args::ClaimShape;
use crate::query::slot::{Part, Pick};

/// One entry of [`BeanInfo::Beans`], as a claim on its bean sees it.
#[doc(hidden)]
pub trait Entry {
    /// What a claim borrows from the call: `&Target` or `&mut Target`.
    /// Generics: `K` part, `P` bean.
    type Target<K: Part, P: BeanInfo>: ?Sized;
    /// Index node of the claim. Generics: `K` part, `I` slot, `A` access,
    /// `P` bean.
    type Index<K, I, A, P>;
}

/// What a system takes for a borrow `'a` of an [`Entry`]'s target.
///
/// The lifetime is a parameter of the trait: the argument of a system may
/// name it inside the type (`&'a mut (dyn Trait + 'a)`).
/// Generics: `K` part, `P` bean.
#[doc(hidden)]
pub trait Lends<'a, K: Part, P: BeanInfo>: Entry {
    /// What the argument points at: `&'a To` shared, `&'a mut To` exclusive.
    type To: ?Sized + 'a;
    /// Hands a shared borrow to a system.
    fn shared(target: &'a Self::Target<K, P>) -> &'a Self::To;
    /// Hands an exclusive borrow to a system.
    fn excl(target: &'a mut Self::Target<K, P>) -> &'a mut Self::To;
}

// A slot: the claim reads part `K` of it. Generics: `E` entry ([`Provided`]).
impl<E: Provided> Entry for E {
    type Target<K: Part, P: BeanInfo> = K::Of<P>;
    type Index<K, I, A, P> = Pick<K, I, A, P>;
}

// Generics: `E` entry ([`Provided`]), `K` part, `P` bean.
impl<'a, E, K, P> Lends<'a, K, P> for E
where
    E: Provided,
    K: Part,
    P: BeanInfo,
    K::Of<P>: 'a,
{
    type To = K::Of<P>;

    fn shared(target: &'a K::Of<P>) -> &'a Self::To {
        target
    }

    fn excl(target: &'a mut K::Of<P>) -> &'a mut Self::To {
        target
    }
}

// A trait object: the claim reads it as it was lent.
// Generics: `C` marker of the bean, `V` view ([`DynObject`]).
impl<C, V: ?Sized> Entry for Dyn<C, V> {
    type Target<K: Part, P: BeanInfo> = V;
    type Index<K, I, A, P> = I;
}

// The system takes the object for the borrow of that call: `&mut dyn Trait`.
// Generics: `C` marker of the bean, `V` view ([`DynObject`]), `K` part, `P` bean.
impl<'a, C, V, K, P> Lends<'a, K, P> for Dyn<C, V>
where
    V: ?Sized + DynObject<'a>,
    K: Part,
    P: BeanInfo,
{
    type To = V::Obj;

    fn shared(target: &'a V) -> &'a Self::To {
        target.shared()
    }

    fn excl(target: &'a mut V) -> &'a mut Self::To {
        target.exclusive()
    }
}

/// Module `Self` holds bean `P`: the entry of its bean list at `P`'s index.
///
/// Generics: `P` bean.
#[doc(hidden)]
pub trait Holds<P> {
    /// The entry of [`BeanInfo::Beans`] for `P`.
    type Entry;
    /// The slot of that entry.
    type Slot;
}

// Generics: `B` module, `P` bean.
impl<B, P> Holds<P> for B
where
    B: BeanInfo + At<P>,
    B::Beans: Take<B::Ix>,
{
    type Entry = <B::Beans as Take<B::Ix>>::Out;
    type Slot = B::Ix;
}

// A bean list of one entry may be written bare. It is slot 0.
// Generics: `E` entry ([`Entry`]).
impl<E: Entry> Take<Ix<0>> for E {
    type Out = Self;
    type Rest = Off;
    type With<X> = X;

    fn take(self) -> (Self::Out, Self::Rest) {
        (self, Off)
    }
}

// As a module's inject, `Dyn<Get<P>, V>` is lent as `&V`: the caller passes
// its concrete value and the call coerces it.
// Generics: `P` bean, `V` view ([`DynObject`]).
impl<P, V: ?Sized> ClaimShape for Dyn<Get<P>, V>
where
    V: DynObject<'static>,
{
    type Lent<'a>
        = &'a V
    where
        Self: 'a;
}

// Generics: `P` bean, `V` view ([`DynObject`]).
impl<P, V: ?Sized> ClaimShape for Dyn<Mut<P>, V>
where
    V: DynObject<'static>,
{
    type Lent<'a>
        = &'a mut V
    where
        Self: 'a;
}
