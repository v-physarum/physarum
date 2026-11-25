//! [`CarrierDesc`]: the payload of a newtype list carrier.

use crate::core::list::{End, Full, Group, Head, List};

/// Newtype list carrier: payload is [`Inner`](CarrierDesc::Inner).
///
/// [`List`], [`Head`], [`Group`], [`Full`], and [`End`] set this once
/// ([`ClaimDesc`](crate::ClaimDesc) style).
/// [`Hand`] forwards to [`Inner`](Self::Inner).
#[doc(hidden)]
pub trait CarrierDesc {
    /// Inner payload.
    type Inner: ?Sized;
    /// Borrows the inner payload.
    fn carrier_inner(&self) -> &Self::Inner;
}

// Generics: `I` inner payload.
impl<I> CarrierDesc for List<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        self.inner()
    }
}

impl CarrierDesc for End {
    type Inner = Self;

    fn carrier_inner(&self) -> &Self {
        self
    }
}

// Generics: `I` items.
impl<I> CarrierDesc for Group<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        self.inner()
    }
}

// Generics: `I` items.
impl<I> CarrierDesc for Full<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        self.inner()
    }
}

// Generics: `I` closed groups.
impl<I> CarrierDesc for Head<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        self.inner()
    }
}
