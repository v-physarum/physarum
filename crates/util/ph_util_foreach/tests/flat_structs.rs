#![allow(missing_docs)]

use ph_util_foreach::impl_foreach;

impl_foreach!(0..4, {
    pub struct Ix + #;
});

#[test]
fn indexes_zero_through_three() {
    let _ = (Ix0, Ix1, Ix2, Ix3);
}
