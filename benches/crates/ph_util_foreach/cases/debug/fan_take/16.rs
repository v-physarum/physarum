use ph_util_foreach::impl_foreach;

macro_rules! impl_take {
    (
        args: ($($args:ident),* $(,)?),
        ix: $ix:ident,
        out: $out:ident,
        rest: $($rest:ident),* $(,)?
    ) => {
        #[allow(non_snake_case)]
        mod $ix {
            pub const ARGS: &[&str] = &[$(stringify!($args)),*];
            pub const OUT: &str = stringify!($out);
            pub const REST: &[&str] = &[$(stringify!($rest)),*];
        }
    };
}

impl_foreach!(range = 0..CASE_LEN, body = {
    impl_take!(
        args: (T[..=#]),
        ix: Ix + #,
        out: T + #,
        rest: T[..#] + Off[#] + T[#+1..=#],
    );
});
