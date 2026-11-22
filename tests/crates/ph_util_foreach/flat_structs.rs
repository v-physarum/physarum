#![allow(missing_docs)]

use ph_util_foreach::impl_foreach;

impl_foreach!(n in 0..4 {
    pub struct paste(Ix, n);
});

impl_foreach!(n in 0..2 {
    pub struct paste(Rev, n);
});

#[test]
fn indexes_zero_through_three() {
    let _ = (Ix0, Ix1, Ix2, Ix3, Rev0, Rev1);
}
