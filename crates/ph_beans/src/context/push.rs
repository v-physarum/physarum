//! Append one entry on the shared [`List`](super::list::List).
//!
//! One module for the whole push table (rule 13). Shape:
//! - `0..=8` — [`Leaf`](super::list::Leaf); growth `1..=6` via [`AddItem`]
//! - `9..=15` — `(List<H>, Grow<R>)` + [`AddItem`] (`Args` / `TakeMany` style)
//! - `15→16` — reshape to four [`List`] groups; no 17th impl

use super::list::Grow;
use super::list::Leaf;
use super::list::List;

/// Appends one entry. Sixteen entries have no impl: that is the cap.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "at most 16 map systems are supported (Status = PipelineError<2>)",
    label = "fix Descriptor::Status",
    note = "MAX_MAP_SYSTEMS = 16; PipelineError<2> — docs/errors.md"
)]
// Generics: `E` entry.
pub trait PushEntry<E> {
    /// The list with `E` appended.
    type Out;
    /// Moves `entry` onto the end of the list.
    fn push(self, entry: E) -> Self::Out;
}

/// One shared append step for a leaf or a group tuple.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "at most 16 map systems are supported (Status = PipelineError<2>)",
    label = "fix Descriptor::Status",
    note = "MAX_MAP_SYSTEMS = 16; PipelineError<2> — docs/errors.md"
)]
// Generics: `N` new entry.
pub trait PushShape<N> {
    /// List after `N` is appended.
    type Out;
    /// Appends `item` using the shared group layout.
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

// --- AddItem (TakeMany role): sealed arity 1..=6 --------------------------------

mod seal {
    pub trait Cap6 {}

    impl<E0> Cap6 for (E0,) {}
    impl<E0, E1> Cap6 for (E0, E1) {}
    impl<E0, E1, E2> Cap6 for (E0, E1, E2) {}
    impl<E0, E1, E2, E3> Cap6 for (E0, E1, E2, E3) {}
    impl<E0, E1, E2, E3, E4> Cap6 for (E0, E1, E2, E3, E4) {}
    impl<E0, E1, E2, E3, E4, E5> Cap6 for (E0, E1, E2, E3, E4, E5) {}
}

/// Appends `N` onto a short leaf. Only lengths 1..=6; sealed vs the 15→16 reshape.
/// Generics: `N` new entry.
#[doc(hidden)]
pub trait AddItem<N>: seal::Cap6 {
    /// Leaf after `N` is appended.
    type Out;
    /// Moves `item` onto the end of the leaf.
    fn add_item(self, item: N) -> Self::Out;
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

impl<N, E0, E1, E2> AddItem<N> for (E0, E1, E2) {
    type Out = (E0, E1, E2, N);

    fn add_item(self, item: N) -> Self::Out {
        (self.0, self.1, self.2, item)
    }
}

// Generics: `N` new entry, `E0` slot 0, `E1` slot 1, `E2` slot 2, `E3` slot 3.
impl<N, E0, E1, E2, E3> AddItem<N> for (E0, E1, E2, E3) {
    type Out = (E0, E1, E2, E3, N);

    fn add_item(self, item: N) -> Self::Out {
        (self.0, self.1, self.2, self.3, item)
    }
}

impl<N, E0, E1, E2, E3, E4> AddItem<N> for (E0, E1, E2, E3, E4) {
    type Out = (E0, E1, E2, E3, E4, N);

    fn add_item(self, item: N) -> Self::Out {
        (self.0, self.1, self.2, self.3, self.4, item)
    }
}

// Generics: `N` new entry, `E0` slot 0, `E1` slot 1, `E2` slot 2, `E3` slot 3, `E4` slot 4, `E5` slot 5.
impl<N, E0, E1, E2, E3, E4, E5> AddItem<N> for (E0, E1, E2, E3, E4, E5) {
    type Out = (E0, E1, E2, E3, E4, E5, N);

    fn add_item(self, item: N) -> Self::Out {
        (self.0, self.1, self.2, self.3, self.4, self.5, item)
    }
}

// --- Leaf 0..=8 ----------------------------------------------------------------

impl<N> PushShape<N> for Leaf<()> {
    type Out = Leaf<(N,)>;

    fn push_shape(self, item: N) -> Self::Out {
        Leaf((item,))
    }
}

// Generics: `N` new entry, `T` tick / type / tip.
impl<N, T> PushShape<N> for Leaf<T>
where
    T: AddItem<N>,
{
    type Out = Leaf<T::Out>;

    fn push_shape(self, item: N) -> Self::Out {
        Leaf(self.0.add_item(item))
    }
}

impl<N, E0, E1, E2, E3, E4, E5, E6> PushShape<N> for Leaf<(E0, E1, E2, E3, E4, E5, E6)> {
    type Out = Leaf<(E0, E1, E2, E3, E4, E5, E6, N)>;

    fn push_shape(self, item: N) -> Self::Out {
        let Leaf((e0, e1, e2, e3, e4, e5, e6)) = self;
        Leaf((e0, e1, e2, e3, e4, e5, e6, item))
    }
}

// Generics: `N` new entry, `E0` slot 0, `E1` slot 1, `E2` slot 2, `E3` slot 3, `E4` slot 4, `E5` slot 5, `E6` slot 6, `E7` slot 7.
impl<N, E0, E1, E2, E3, E4, E5, E6, E7> PushShape<N> for Leaf<(E0, E1, E2, E3, E4, E5, E6, E7)> {
    type Out = (List<(E0, E1, E2, E3, E4, E5, E6, E7)>, Grow<(N,)>);

    fn push_shape(self, item: N) -> Self::Out {
        let Leaf(items) = self;
        (List(items), Grow((item,)))
    }
}

// --- Group 9..=15: one blanket, growth in AddItem --------------------------------

impl<N, H, R> PushShape<N> for (List<H>, Grow<R>)
where
    R: AddItem<N>,
{
    type Out = (List<H>, Grow<R::Out>);

    fn push_shape(self, item: N) -> Self::Out {
        let (head, Grow(rest)) = self;
        (head, Grow(rest.add_item(item)))
    }
}

// --- Wide 15→16 ----------------------------------------------------------------

// Generics: `N` new entry, `E0` slot 0, `E1` slot 1, `E2` slot 2, `E3` slot 3, `E4` slot 4, `E5` slot 5, `E6` slot 6, `E7` slot 7, `R0` rest group 0, `R1` rest group 1, `R2` rest group 2, `R3` rest group 3, `R4` rest group 4, `R5` rest group 5, `R6` rest group 6.
impl<N, E0, E1, E2, E3, E4, E5, E6, E7, R0, R1, R2, R3, R4, R5, R6> PushShape<N>
    for (
        List<(E0, E1, E2, E3, E4, E5, E6, E7)>,
        Grow<(R0, R1, R2, R3, R4, R5, R6)>,
    )
{
    type Out = (
        List<(E0, E1, E2, E3)>,
        List<(E4, E5, E6, E7)>,
        List<(R0, R1, R2, R3)>,
        List<(R4, R5, R6, N)>,
    );

    fn push_shape(self, item: N) -> Self::Out {
        let (List((e0, e1, e2, e3, e4, e5, e6, e7)), Grow((r0, r1, r2, r3, r4, r5, r6))) = self;
        (
            List((e0, e1, e2, e3)),
            List((e4, e5, e6, e7)),
            List((r0, r1, r2, r3)),
            List((r4, r5, r6, item)),
        )
    }
}
