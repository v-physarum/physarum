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

impl_foreach!(n in 0..CASE_LEN {
    impl_take!(
        args: (T[..=n]),
        ix: paste(Ix, n),
        out: paste(T, n),
        rest: T[..n] + Off[n] + T[n + 1..=n],
    );
});
