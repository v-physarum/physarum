//! The state of a bean that owns none: [`NoState`].
//!
//! A bean names its state in [`BeanInfo::State`](crate::BeanInfo::State). A
//! bean without state names [`NoState`]. Only the library makes that value,
//! so such a module takes no `.init_state` and is built without it.

/// The state of a bean that owns no state: `type State = NoState`.
///
/// It has no public constructor and no `Default`: an init function cannot
/// return it.
pub struct NoState {
    _sealed: (),
}

/// A state the library can make without an init function: the start value
/// of a provided bean's slot.
#[doc(hidden)]
pub trait Blank {
    /// The start value.
    fn blank() -> Self;
}

// Generics: `T` state with a default.
impl<T: Default> Blank for T {
    fn blank() -> Self {
        T::default()
    }
}

impl Blank for NoState {
    fn blank() -> Self {
        Self { _sealed: () }
    }
}
