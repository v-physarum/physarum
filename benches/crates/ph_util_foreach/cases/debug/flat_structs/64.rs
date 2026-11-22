use ph_util_foreach::impl_foreach;

impl_foreach!(n in 0..CASE_LEN {
    pub struct paste(Ix, n);
});
