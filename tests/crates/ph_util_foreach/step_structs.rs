#![allow(missing_docs)]

use ph_util_foreach::impl_foreach;

impl_foreach!(n in (0..10).step_by(2) {
    pub struct paste(Ix, n);
});

#[test]
fn even_indexes() {
    let _ = (Ix0, Ix2, Ix4, Ix6, Ix8);
}
