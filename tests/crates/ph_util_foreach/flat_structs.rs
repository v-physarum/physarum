#![allow(missing_docs)]

use ph_util_foreach::impl_foreach;

impl_foreach!(range = 0..4, body = {
    pub struct Ix + #;
});

impl_foreach!(body = { pub struct Rev + #; }, range = 0..2);

#[test]
fn indexes_zero_through_three() {
    let _ = (Ix0, Ix1, Ix2, Ix3, Rev0, Rev1);
}
