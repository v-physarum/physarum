//! Hidden index markers.
//!
//! [`Off`] is written here. `impl_foreach` declares [`Ix0`] through [`Ix3`].

use ph_util_foreach::impl_foreach;

/// Hole left in a tuple after a slot is taken.
#[doc(hidden)]
pub struct Off;

impl_foreach!(range = 0..4, body = {
    #[doc(hidden)]
    #[doc = "Index of one provided bean."]
    pub struct Ix + #;
});
