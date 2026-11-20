#![allow(missing_docs)]

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

impl_foreach!(0..4 {
    impl_take!(
        args: (T[..=#]),
        ix: Ix #,
        out: T #,
        rest: T[..#] + Off[#] + T[#+1..=#],
    );
});

#[test]
fn fan_take_splits_at_the_outer_index() {
    assert_eq!(Ix0::ARGS, ["T0"]);
    assert_eq!(Ix0::OUT, "T0");
    assert_eq!(Ix0::REST, ["Off"]);

    assert_eq!(Ix1::ARGS, ["T0", "T1"]);
    assert_eq!(Ix1::OUT, "T1");
    assert_eq!(Ix1::REST, ["T0", "Off"]);

    assert_eq!(Ix2::ARGS, ["T0", "T1", "T2"]);
    assert_eq!(Ix2::OUT, "T2");
    assert_eq!(Ix2::REST, ["T0", "T1", "Off"]);

    assert_eq!(Ix3::ARGS, ["T0", "T1", "T2", "T3"]);
    assert_eq!(Ix3::OUT, "T3");
    assert_eq!(Ix3::REST, ["T0", "T1", "T2", "Off"]);
}
