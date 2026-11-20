use ph_util_foreach::impl_foreach;

impl_foreach!(range = 0..CASE_LEN, body = {
    pub const Row + #: &str = stringify!((T + #, U + #)[0..=#]);
});
