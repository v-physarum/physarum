use ph_util_foreach::impl_foreach;

impl_foreach!(n in 0..CASE_LEN {
    pub const paste(Row, n): &str = stringify!((paste(T, n), paste(U, n))[0..=n]);
});
