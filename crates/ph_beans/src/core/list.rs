//! Grouped list: a closed [`Head`] and one open slot.
//!
//! An open slot is [`End`] or a [`Group`] of one to three items. Four items
//! close into one [`Full`]. At most four groups. Append is [`PushShape`].

use crate::core::config::Config;
use crate::core::config::OfConfig;

/// List carrier. Generics: `I` payload.
pub struct List<I>(I);

impl<I> List<I> {
    /// Wraps `inner`.
    #[must_use]
    pub const fn new(inner: I) -> Self {
        Self(inner)
    }

    /// Borrows the payload.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.0
    }
}

/// Empty list: [`Head`]`<()>` beside [`End`].
pub struct EmptySlots;

impl EmptySlots {
    /// Builds [`OfConfig`]`<Self>`.
    #[must_use]
    pub fn empty() -> OfConfig<Self> {
        List::new((Head(()), End))
    }
}

impl Config for EmptySlots {
    type Type = List<(Head<()>, End)>;
}

/// Closed groups already stored.
///
/// Generics: `I` closed groups (`()` or a tuple of [`Full`]).
pub struct Head<I>(I);

impl<I> Head<I> {
    /// Borrows the closed groups.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.0
    }
}

/// Open slot with no items yet.
#[derive(Clone, Copy)]
pub struct End;

/// Open group of one to three items.
///
/// Generics: `I` items.
pub struct Group<I>(I);

impl<I> Group<I> {
    /// Borrows the items.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.0
    }
}

/// Closed group of four items.
///
/// Generics: `I` items.
pub struct Full<I>(I);

impl<I> Full<I> {
    /// Borrows the items.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.0
    }
}

/// Appends one entry. Sixteen entries have no impl: that is the cap.
#[diagnostic::on_unimplemented(
    message = "at most 16 entries are supported",
    label = "the list is full",
    note = "four groups of four"
)]
// Generics: `E` entry.
pub trait PushEntry<E> {
    /// The list with `E` appended.
    type Out;
    /// Moves `entry` onto the end of the list.
    fn push(self, entry: E) -> Self::Out;
}

/// Appends into a [`Head`] plus one open slot.
#[diagnostic::on_unimplemented(
    message = "at most 16 entries are supported",
    label = "the list is full",
    note = "four groups of four"
)]
// Generics: `N` new entry.
pub trait PushShape<N> {
    /// Payload after `N` is appended.
    type Out;
    /// Appends `item`.
    fn push_shape(self, item: N) -> Self::Out;
}

impl<I, N> PushEntry<N> for List<I>
where
    I: PushShape<N>,
{
    type Out = List<I::Out>;

    fn push(self, entry: N) -> Self::Out {
        List(self.0.push_shape(entry))
    }
}

mod seal {
    pub trait Cap {}

    impl Cap for () {}
    impl<E0> Cap for (E0,) {}
    impl<E0, E1> Cap for (E0, E1) {}

    pub trait Placed {}

    impl Placed for super::End {}

    // Generics: `I` items still short of three.
    impl<I: Cap> Placed for super::Group<I> {}

    impl<E0, E1, E2> Placed for super::Group<(E0, E1, E2)> {}
}

/// Appends `N` onto a short tuple. Lengths `()`, 1, and 2 only.
///
/// Generics: `N` new entry.
pub trait AddItem<N>: seal::Cap {
    /// Tuple after `N` is appended.
    type Out;
    /// Moves `item` onto the end.
    fn add_item(self, item: N) -> Self::Out;
}

// Generics: `N` new entry.
impl<N> AddItem<N> for () {
    type Out = (N,);

    fn add_item(self, item: N) -> Self::Out {
        (item,)
    }
}

impl<N, E0> AddItem<N> for (E0,) {
    type Out = (E0, N);

    fn add_item(self, item: N) -> Self::Out {
        (self.0, item)
    }
}

// Generics: `N` new entry, `E0` slot 0, `E1` slot 1.
impl<N, E0, E1> AddItem<N> for (E0, E1) {
    type Out = (E0, E1, N);

    fn add_item(self, item: N) -> Self::Out {
        (self.0, self.1, item)
    }
}

/// A slot that can take `N`. [`Full`] is not [`seal::Placed`].
///
/// Generics: `N` new entry.
pub trait Place<N>: seal::Placed {
    /// Slot after `N` is placed.
    type Out;
    /// Stores `item` in this slot.
    fn place(self, item: N) -> Self::Out;
}

impl<N> Place<N> for End {
    type Out = Group<(N,)>;

    fn place(self, item: N) -> Self::Out {
        Group((item,))
    }
}

// Generics: `N` new entry, `I` items ([`AddItem`]).
impl<N, I> Place<N> for Group<I>
where
    I: AddItem<N>,
{
    type Out = Group<I::Out>;

    fn place(self, item: N) -> Self::Out {
        Group(self.0.add_item(item))
    }
}

// Generics: `N` new entry, `E0` slot 0, `E1` slot 1, `E2` slot 2.
impl<N, E0, E1, E2> Place<N> for Group<(E0, E1, E2)> {
    type Out = Full<(E0, E1, E2, N)>;

    fn place(self, item: N) -> Self::Out {
        let Group((e0, e1, e2)) = self;
        Full((e0, e1, e2, item))
    }
}

/// Head plus the slot after [`Place`].
pub trait Pack {
    /// Payload after this place.
    type Out;
    /// Stores the placed slot on the head.
    fn pack(self) -> Self::Out;
}

// Generics: `H` head, `I` open group.
impl<H, I> Pack for (Head<H>, Group<I>) {
    type Out = (Head<H>, Group<I>);

    fn pack(self) -> Self::Out {
        self
    }
}

// Generics: `H` head ([`AddItem`]), `A` closed group.
impl<H, A> Pack for (Head<H>, Full<A>)
where
    H: AddItem<Full<A>>,
{
    type Out = (Head<H::Out>, End);

    fn pack(self) -> Self::Out {
        let (Head(head), full) = self;
        (Head(head.add_item(full)), End)
    }
}

// Generics: `E0`…`E2` closed groups, `A` last group. No open slot remains.
impl<E0, E1, E2, A> Pack for (Head<(E0, E1, E2)>, Full<A>) {
    type Out = Head<(E0, E1, E2, Full<A>)>;

    fn pack(self) -> Self::Out {
        let (Head((e0, e1, e2)), full) = self;
        Head((e0, e1, e2, full))
    }
}

// Generics: `N` new entry, `H` head, `R` open slot.
impl<N, H, R> PushShape<N> for (Head<H>, R)
where
    R: Place<N>,
    (Head<H>, R::Out): Pack,
{
    type Out = <(Head<H>, R::Out) as Pack>::Out;

    fn push_shape(self, item: N) -> Self::Out {
        let (head, slot) = self;
        (head, slot.place(item)).pack()
    }
}
