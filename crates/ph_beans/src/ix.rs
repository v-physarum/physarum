//! Hidden index markers.
//!
//! `impl_foreach` declares [`Off`] and [`Ix0`] through [`Ix3`].

use ph_util_foreach::impl_foreach;

impl_foreach!(
    range = 0..1,
    body = {
        #[doc(hidden)]
        #[doc = "Hole left in a tuple after a slot is taken."]
        pub struct Off;
    }
);

impl_foreach!(range = 0..4, body = {
    #[doc(hidden)]
    #[doc = "Index of one provided bean."]
    pub struct Ix + #;
});
