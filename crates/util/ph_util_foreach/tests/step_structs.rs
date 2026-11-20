#![allow(missing_docs)]

use ph_util_foreach::impl_foreach;

impl_foreach!(0..10::2, {
    pub struct Ix + #;
});

#[test]
fn even_indexes() {
    let _ = (Ix0, Ix2, Ix4, Ix6, Ix8);
}
